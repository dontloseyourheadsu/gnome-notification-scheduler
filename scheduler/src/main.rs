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
            Ok(mut schedule) => {
                if let Ok(mut schedules) = load_alert_schedules() {
                    let last_schedule_id = schedules.last().map(|s| s.id).unwrap_or(1);
                    schedule.id = last_schedule_id + 1;

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
        Commands::Update {
            id,
            title,
            message,
            interval,
        } => {
            if let Ok(mut schedules) = load_alert_schedules() {
                if let Some(schedule) = schedules.iter_mut().find(|s| s.id == *id) {
                    schedule.title = title.clone();
                    schedule.message = message.clone();
                    schedule.repeat_interval_in_seconds = *interval;

                    if let Err(e) = save_alert_schedules(&schedules) {
                        eprintln!("Error saving schedules: {}", e);
                    }
                } else {
                    eprintln!("Error: Alert ID {} does not exist.", id);
                }
            } else {
                eprintln!("Error loading existing schedules.");
            }
        }
        Commands::Remove { id } => {
            println!("Removing alert with ID: {}", id);

            if let Ok(mut schedules) = load_alert_schedules() {
                if *id as usize >= schedules.len() {
                    eprintln!("Error: Alert ID {} does not exist.", id);
                    return;
                }
                schedules.remove(*id as usize);

                if let Err(e) = save_alert_schedules(&schedules) {
                    eprintln!("Error saving schedules: {}", e);
                } else {
                    println!("Alert with ID {} removed successfully.", id);
                }
            } else {
                eprintln!("Error loading existing schedules.");
            }
        }
    }
}
