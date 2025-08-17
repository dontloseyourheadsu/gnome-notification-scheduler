use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::{fmt, io::BufReader, path::PathBuf};

pub const APP_NAME: &str = "Gnome Alert Scheduler";
pub const INTERNAL_APP_NAME: &str = "gnome-alert-scheduler";
pub const ALERT_SCHEDULES_FILE_PATH: &str = "./alert-schedules.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSchedule {
    pub id: u64,
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

        let id: u64 = 0;

        Ok(Self {
            id, // Id will be assigned by other method
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
    InvalidId,
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScheduleError::TitleTooLong => write!(f, "Title exceeds 100 characters"),
            ScheduleError::MessageTooLong => write!(f, "Message exceeds 500 characters"),
            ScheduleError::InvalidId => write!(f, "Invalid alert ID"),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// Get the path to the data file.
fn data_file() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap_or(std::env::current_dir().unwrap());
    dir.push(INTERNAL_APP_NAME);
    fs::create_dir_all(&dir).ok();
    dir.push(ALERT_SCHEDULES_FILE_PATH);
    dir
}

/// Save the list of scheduled alerts to a file.
pub fn save_alert_schedules(schedules: &[AlertSchedule]) -> std::io::Result<()> {
    let path = data_file();
    let tmp = path.with_extension("json.tmp");

    // Write atomically: write to temp, flush, then rename
    let file = File::create(&tmp)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, schedules).expect("serialize");

    // Ensure bytes hit by general OS
    writer.flush()?;
    // Ensure bytes hit Unix systems.
    writer.get_ref().sync_all()?;

    fs::rename(tmp, path)?; // atomic on most platforms
    Ok(())
}

/// Load the list of scheduled alerts from a file.
pub fn load_alert_schedules() -> std::io::Result<Vec<AlertSchedule>> {
    let path = data_file();
    match File::open(&path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let schedules = serde_json::from_reader(reader).unwrap_or_else(|_| Vec::new());
            Ok(schedules)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}
