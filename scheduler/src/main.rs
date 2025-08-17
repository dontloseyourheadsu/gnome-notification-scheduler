use anyhow::{Context, Result};
use clap::Parser;
use futures::{SinkExt, StreamExt};
use shared_lib::{Request, Response, AlertSchedule};
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec};

mod cli_parser_models;
use cli_parser_models::{Cli, Commands};

const SOCKET_PATH: &str = "/tmp/gnome-alert-scheduler.sock";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let req = match &cli.command {
        Commands::Schedule { title, message, interval } => {
            Request::Add { title: title.clone(), message: message.clone(), interval: *interval }
        }
        Commands::List => Request::List,
        Commands::Update { id, title, message, interval } => {
            Request::Update {
                id: *id,
                title: title.clone(),
                message: message.clone(),
                interval: *interval,
            }
        }
        Commands::Remove { id } => Request::Remove { id: *id },
    };

    let mut framed = Framed::new(
        UnixStream::connect(SOCKET_PATH)
            .await
            .with_context(|| format!("connect {}", SOCKET_PATH))?,
        LinesCodec::new(),
    );

    framed.send(serde_json::to_string(&req)?).await?;
    if let Some(line) = framed.next().await {
        match line {
            Ok(s) => match serde_json::from_str::<Response>(&s) {
                Ok(Response::Ok(val)) => {
                    // Pretty-print useful outputs
                    if let Some(_arr) = val.get("schedules").and_then(|v| v.as_array()) {
                        // Show as list using your Display impl
                        let schedules: Vec<AlertSchedule> = serde_json::from_value(val["schedules"].clone()).unwrap_or_default();
                        if schedules.is_empty() {
                            println!("(no alerts)");
                        } else {
                            for sched in schedules {
                                println!("{sched}");
                            }
                        }
                    } else {
                        println!("{}", serde_json::to_string_pretty(&val)?);
                    }
                }
                Ok(Response::Err(msg)) => eprintln!("Error: {msg}"),
                Err(_) => println!("{s}"),
            },
            Err(e) => eprintln!("read error: {e:#}"),
        }
    }

    Ok(())
}
