use clap::Parser;
use shared_lib::{AlertSchedule, load_alert_schedules, save_alert_schedules};

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
                if let Ok(mut schedules) = load_alert_schedules() {
                    schedules.push(schedule);

                    if let Err(e) = save_alert_schedules(&schedules) {
                        eprintln!("Error saving schedules: {}", e);
                    }
                } else {
                    println!("No existing schedules found.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        },
        Commands::List => {
            println!("Listing all scheduled alerts...");
            if let Ok(schedules) = load_alert_schedules() {
                for schedule in schedules {
                    print!("{}", schedule);
                }
            }
        }
        Commands::Remove { id } => {
            println!("Removing alert with ID: {}", id);
        }
    }
}
