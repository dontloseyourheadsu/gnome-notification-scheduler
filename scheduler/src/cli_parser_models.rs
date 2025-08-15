use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "alert-scheduler")]
#[command(about = "A CLI tool for scheduling alerts")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Schedule a new alert
    Schedule {
        /// Alert title
        #[arg(short, long)]
        title: String,
        /// Alert message
        #[arg(short, long)]
        message: String,
        /// Repeat interval in seconds
        #[arg(short, long)]
        interval: u64,
    },
    /// List all scheduled alerts
    List,
    /// Remove a scheduled alert
    Remove {
        /// Alert ID to remove
        id: u64,
    },
}