use clap::Parser;
use shared_lib::{load_schedules, AlertSchedule};

mod cli_parser_models;
use cli_parser_models::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Schedule {
            title,
            message,
            interval,
        } => match AlertSchedule::new(title.clone(), message.clone(), *interval) {
            Ok(schedule) => {
                println!("Scheduled: {}", schedule);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        },
        Commands::List => {
            println!("Listing all scheduled alerts...");
            if let Ok(schedules) = load_schedules() {
                for schedule in  schedules {
                    print!("{}", schedule);
                }
            }
        }
        Commands::Remove { id } => {
            println!("Removing alert with ID: {}", id);
        }
    }
}
