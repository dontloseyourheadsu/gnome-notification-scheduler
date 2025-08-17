use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use shared_lib::{
    load_alert_schedules, save_alert_schedules, AlertSchedule, Request, Response,
};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;
use tokio_util::codec::{Framed, LinesCodec};

const SOCKET_PATH: &str = "/tmp/gnome-alert-scheduler.sock";

#[tokio::main]
async fn main() -> Result<()> {
    // Load on startup
    let schedules = load_alert_schedules().context("load schedules")?;
    let state = Arc::new(RwLock::new(schedules));

    // Prepare socket
    if Path::new(SOCKET_PATH).exists() {
        let _ = fs::remove_file(SOCKET_PATH);
    }
    let listener = UnixListener::bind(SOCKET_PATH)
        .with_context(|| format!("bind {}", SOCKET_PATH))?;
    println!("daemon listening on {}", SOCKET_PATH);

    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, state).await {
                eprintln!("client error: {e:#}");
            }
        });
    }
}

async fn handle_client(stream: UnixStream, state: Arc<RwLock<Vec<AlertSchedule>>>) -> Result<()> {
    let mut framed = Framed::new(stream, LinesCodec::new());
    while let Some(line) = framed.next().await {
        match line {
            Ok(line) => {
                let resp = handle_line(&line, &state).await;
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

async fn handle_line(line: &str, state: &Arc<RwLock<Vec<AlertSchedule>>>) -> Response {
    let req: Result<Request, _> = serde_json::from_str(line);
    match req {
        Ok(Request::List) => {
            let guard = state.read().await;
            Response::Ok(serde_json::json!({ "schedules": &*guard }))
        }
        Ok(Request::Add { title, message, interval }) => {
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
                    Response::Ok(serde_json::json!({ "created": schedule }))
                }
                Err(e) => Response::Err(e.to_string()),
            }
        }
        Ok(Request::Update { id, title, message, interval }) => {
            let mut guard = state.write().await;
            if let Some(s) = guard.iter_mut().find(|s| s.id == id) {
                s.title = title;
                s.message = message;
                s.repeat_interval_in_seconds = interval;
                let updated = s.clone();
                if let Err(e) = save_alert_schedules(&*guard) {
                    return Response::Err(format!("save error: {e}"));
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
            Response::Ok(serde_json::json!({ "removed_id": id }))
        }
        Err(e) => Response::Err(format!("bad request JSON: {e}")),
    }
}
