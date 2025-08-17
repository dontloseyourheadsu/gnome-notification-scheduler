use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::{fmt, io::BufReader, path::PathBuf};

pub const APP_NAME: &str = "Gnome Alert Scheduler";
pub const INTERNAL_APP_NAME: &str = "gnome-alert-scheduler";
pub const ALERT_SCHEDULES_FILE_NAME: &str = "alert-schedules.json";

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
            "#{:>3}  {} - {} (every {}s)",
            self.id, self.title, self.message, self.repeat_interval_in_seconds
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
            id: 0, // Will be assigned by daemon
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

/// Path: ~/.local/share/gnome-alert-scheduler/alert-schedules.json (on Linux)
fn data_file() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap_or(std::env::current_dir().unwrap());
    dir.push(INTERNAL_APP_NAME);
    fs::create_dir_all(&dir).ok();
    dir.push(ALERT_SCHEDULES_FILE_NAME);
    dir
}

/// Save schedules atomically.
pub fn save_alert_schedules(schedules: &[AlertSchedule]) -> std::io::Result<()> {
    let path = data_file();
    let tmp = path.with_extension("json.tmp");
    let file = File::create(&tmp)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, schedules).expect("serialize");
    writer.flush()?;
    writer.get_ref().sync_all()?;
    fs::rename(tmp, path)?;
    Ok(())
}

/// Load schedules (empty if missing/corrupt).
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

/// ===== Wire protocol shared by client & daemon =====
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    Add { title: String, message: String, interval: u64 },
    List,
    Update { id: u64, title: String, message: String, interval: u64 },
    Remove { id: u64 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum Response {
    Ok(serde_json::Value),
    Err(String),
}
