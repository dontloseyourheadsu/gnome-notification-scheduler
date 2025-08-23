use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use notify_rust::Notification;
use shared_lib::{load_alert_schedules, save_alert_schedules, AlertSchedule, Request, Response};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Mutex, RwLock};
use tokio::time::Duration;
use tokio_util::codec::{Framed, LinesCodec};

const SOCKET_PATH: &str = "/tmp/gnome-alert-scheduler.sock";

type TimerHandles = Arc<Mutex<HashMap<u64, tokio::task::JoinHandle<()>>>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Load on startup
    let schedules = load_alert_schedules().context("load schedules")?;
    let state = Arc::new(RwLock::new(schedules));

    // Track running timers for each alert
    let timer_handles: TimerHandles = Arc::new(Mutex::new(HashMap::new()));

    // Start timers for all non-stopped schedules
    {
        let schedules = state.read().await;
        for schedule in schedules.iter() {
            if !schedule.stopped {
                start_timer_for_schedule(schedule.clone(), timer_handles.clone()).await;
            }
        }
    }

    // Prepare socket
    if Path::new(SOCKET_PATH).exists() {
        let _ = fs::remove_file(SOCKET_PATH);
    }
    let listener =
        UnixListener::bind(SOCKET_PATH).with_context(|| format!("bind {}", SOCKET_PATH))?;
    println!("daemon listening on {}", SOCKET_PATH);

    // Setup graceful shutdown
    let timer_handles_shutdown = timer_handles.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\nShutting down gracefully...");

        // Cancel all timers
        let mut handles = timer_handles_shutdown.lock().await;
        for (id, handle) in handles.drain() {
            println!("Stopping timer for alert {}", id);
            handle.abort();
        }

        // Remove socket file
        let _ = fs::remove_file(SOCKET_PATH);
        std::process::exit(0);
    });

    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();
        let timer_handles = timer_handles.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, state, timer_handles).await {
                eprintln!("client error: {e:#}");
            }
        });
    }
}

async fn handle_client(
    stream: UnixStream,
    state: Arc<RwLock<Vec<AlertSchedule>>>,
    timer_handles: TimerHandles,
) -> Result<()> {
    let mut framed = Framed::new(stream, LinesCodec::new());
    while let Some(line) = framed.next().await {
        match line {
            Ok(line) => {
                let resp = handle_line(&line, &state, &timer_handles).await;
                let out = serde_json::to_string(&resp).unwrap();
                framed.send(out).await?;
            }
            Err(e) => {
                eprintln!("decode error: {e:#}");
                break;
            }
        }
    }
    Ok(())
}

async fn handle_line(
    line: &str,
    state: &Arc<RwLock<Vec<AlertSchedule>>>,
    timer_handles: &TimerHandles,
) -> Response {
    let req: Result<Request, _> = serde_json::from_str(line);
    match req {
        Ok(Request::List) => {
            let guard = state.read().await;
            Response::Ok(serde_json::json!({ "schedules": &*guard }))
        }
        Ok(Request::Add {
            title,
            message,
            interval,
        }) => {
            match AlertSchedule::new(title, message, interval) {
                Ok(mut schedule) => {
                    let mut guard = state.write().await;
                    // assign id = max_id + 1
                    let next_id = guard.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                    schedule.id = next_id;
                    guard.push(schedule.clone());
                    if let Err(e) = save_alert_schedules(&guard) {
                        return Response::Err(format!("save error: {e}"));
                    }

                    // Start timer for new schedule
                    start_timer_for_schedule(schedule.clone(), timer_handles.clone()).await;

                    Response::Ok(serde_json::json!({ "created": schedule }))
                }
                Err(e) => Response::Err(e.to_string()),
            }
        }
        Ok(Request::Update {
            id,
            title,
            message,
            interval,
        }) => {
            let mut guard = state.write().await;
            if let Some(s) = guard.iter_mut().find(|s| s.id == id) {
                s.title = title;
                s.message = message;
                s.repeat_interval_in_seconds = interval;
                let updated = s.clone();
                if let Err(e) = save_alert_schedules(&*guard) {
                    return Response::Err(format!("save error: {e}"));
                }

                // Restart timer with new interval if not stopped
                if !updated.stopped {
                    stop_timer_for_schedule(id, timer_handles).await;
                    start_timer_for_schedule(updated.clone(), timer_handles.clone()).await;
                }

                Response::Ok(serde_json::json!({ "updated": updated }))
            } else {
                Response::Err(format!("Alert ID {} not found", id))
            }
        }
        Ok(Request::Remove { id }) => {
            let mut guard = state.write().await;
            let before = guard.len();
            guard.retain(|s| s.id != id);
            if guard.len() == before {
                return Response::Err(format!("Alert ID {} not found", id));
            }
            if let Err(e) = save_alert_schedules(&guard) {
                return Response::Err(format!("save error: {e}"));
            }

            // Stop and remove timer
            stop_timer_for_schedule(id, timer_handles).await;

            Response::Ok(serde_json::json!({ "removed_id": id }))
        }
        Ok(Request::Stop { id }) => {
            let mut guard = state.write().await;
            if let Some(s) = guard.iter_mut().find(|s| s.id == id) {
                s.stopped = true;
                let updated = s.clone();
                if let Err(e) = save_alert_schedules(&*guard) {
                    return Response::Err(format!("save error: {e}"));
                }

                // Stop the timer
                stop_timer_for_schedule(id, timer_handles).await;

                Response::Ok(serde_json::json!({ "stopped": updated }))
            } else {
                Response::Err(format!("Alert ID {} not found", id))
            }
        }
        Ok(Request::Start { id }) => {
            let mut guard = state.write().await;
            if let Some(s) = guard.iter_mut().find(|s| s.id == id) {
                s.stopped = false;
                let updated = s.clone();
                if let Err(e) = save_alert_schedules(&*guard) {
                    return Response::Err(format!("save error: {e}"));
                }

                // Start the timer
                start_timer_for_schedule(updated.clone(), timer_handles.clone()).await;

                Response::Ok(serde_json::json!({ "started": updated }))
            } else {
                Response::Err(format!("Alert ID {} not found", id))
            }
        }
        Err(e) => Response::Err(format!("bad request JSON: {e}")),
    }
}

/// Start a timer for the given schedule
async fn start_timer_for_schedule(schedule: AlertSchedule, timer_handles: TimerHandles) {
    let id = schedule.id;
    let interval_secs = schedule.repeat_interval_in_seconds;

    println!("Starting timer for alert {} (every {}s)", id, interval_secs);

    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

        // Skip the first tick (immediate fire)
        interval.tick().await;

        loop {
            interval.tick().await;

            // Execute the alert
            if let Err(e) = execute_alert(&schedule).await {
                eprintln!("Failed to execute alert {}: {}", id, e);
            }
        }
    });

    timer_handles.lock().await.insert(id, handle);
}

/// Stop and remove timer for the given schedule ID
async fn stop_timer_for_schedule(id: u64, timer_handles: &TimerHandles) {
    let mut handles = timer_handles.lock().await;
    if let Some(handle) = handles.remove(&id) {
        println!("Stopping timer for alert {}", id);
        handle.abort();
    }
}

/// Execute an alert by showing a desktop notification
async fn execute_alert(schedule: &AlertSchedule) -> Result<()> {
    println!(
        "EXECUTING ALERT {}: {} - {}",
        schedule.id, schedule.title, schedule.message
    );

    // Show desktop notification
    Notification::new()
        .summary(&schedule.title)
        .body(&schedule.message)
        .appname("Gnome Alert Scheduler")
        .timeout(notify_rust::Timeout::Milliseconds(6000)) // 6 seconds
        .show()
        .context("failed to show notification")?;

    Ok(())
}
