use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::{fmt, io::BufReader, path::PathBuf};

pub const APP_NAME: &str = "Gnome Alert Scheduler";
pub const REPEAT_ALERT_FILE_PATH: &str = "./repeat_alerts.txt";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSchedule {
    pub title: String,
    pub message: String,
    pub repeat_interval_in_seconds: u64,
}

impl fmt::Display for AlertSchedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - {} (every {} seconds)",
            self.title, self.message, self.repeat_interval_in_seconds
        )
    }
}

impl AlertSchedule {
    pub fn new(
        title: String,
        message: String,
        repeat_interval_in_seconds: u64,
    ) -> Result<Self, ScheduleError> {
        if title.chars().count() > 100 {
            return Err(ScheduleError::TitleTooLong);
        }
        if message.chars().count() > 500 {
            return Err(ScheduleError::MessageTooLong);
        }

        Ok(Self {
            title,
            message,
            repeat_interval_in_seconds,
        })
    }
}

#[derive(Debug)]
pub enum ScheduleError {
    TitleTooLong,
    MessageTooLong,
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScheduleError::TitleTooLong => write!(f, "Title exceeds 100 characters"),
            ScheduleError::MessageTooLong => write!(f, "Message exceeds 500 characters"),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// Get the path to the data file.
fn data_file() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap_or(std::env::current_dir().unwrap());
    dir.push(APP_NAME);
    fs::create_dir_all(&dir).ok();
    dir.push(REPEAT_ALERT_FILE_PATH);
    dir
}

/// Load the list of scheduled alerts from a file.
pub fn load_schedules() -> std::io::Result<Vec<AlertSchedule>> {
    let path = data_file();
    match File::open(&path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let people = serde_json::from_reader(reader).unwrap_or_else(|_| Vec::new());
            Ok(people)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}
