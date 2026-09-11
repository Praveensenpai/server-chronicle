use anyhow::Result;
use chrono::{DateTime, Utc};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::domain::models::EventRecord;

pub fn get_log_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    Path::new(&home).join(".local/share/server-chronicle/logs")
}

pub fn get_log_path(date: DateTime<Utc>) -> PathBuf {
    let filename = format!("activity-{}.jsonl", date.format("%Y-%m-%d"));
    get_log_dir().join(filename)
}

pub fn append_event(record: &EventRecord) -> Result<()> {
    let dir = get_log_dir();
    fs::create_dir_all(&dir)?;

    let path = get_log_path(record.timestamp);
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

    let line = serde_json::to_string(record)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn read_today_events() -> Vec<EventRecord> {
    read_events_for_date(Utc::now())
}

pub fn read_events_for_date(date: DateTime<Utc>) -> Vec<EventRecord> {
    let path = get_log_path(date);
    read_events_from_path(&path)
}

pub fn read_events_from_path(path: &Path) -> Vec<EventRecord> {
    if !path.exists() {
        return Vec::new();
    }

    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };

    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<EventRecord>(&line) {
            records.push(rec);
        }
    }
    records
}
