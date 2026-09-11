use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::battery_types::{
    calculate_health_percent, calculate_rate_pct_per_hour, init_default_brackets, BatterySample,
    BatterySnapshot, BracketStat, PowerState,
};

const BATTERY_SYS_PATH: &str = "/sys/class/power_supply/BAT0";
const AC_SYS_PATH: &str = "/sys/class/power_supply/AC/online";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistentBatteryTracker {
    pub last_capacity: u8,
    pub last_state: String,
    pub last_timestamp: Option<DateTime<Utc>>,
    pub session_start_cap: u8,
    pub session_start_time: Option<DateTime<Utc>>,
    pub brackets: Vec<BracketStat>,
    pub samples: Vec<BatterySample>,
}

impl PersistentBatteryTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_capacity: 0,
            last_state: "Unknown".to_string(),
            last_timestamp: None,
            session_start_cap: 0,
            session_start_time: None,
            brackets: init_default_brackets(),
            samples: Vec::new(),
        }
    }

    pub fn load_or_init() -> Self {
        let path = get_tracker_file();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(mut tracker) = serde_json::from_str::<Self>(&data) {
                if tracker.brackets.len() != 10 {
                    tracker.brackets = init_default_brackets();
                }
                return tracker;
            }
        }
        Self::new()
    }

    pub fn save(&self) -> Result<()> {
        let path = get_tracker_file();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn update(&mut self, current_cap: u8, state: &PowerState) {
        let now = Utc::now();
        let state_str = state.as_str().to_string();

        if self.last_state != state_str {
            self.session_start_cap = current_cap;
            self.session_start_time = Some(now);
            self.last_state = state_str.clone();
        }

        if self.session_start_time.is_none() {
            self.session_start_cap = current_cap;
            self.session_start_time = Some(now);
        }

        self.samples.push(BatterySample {
            timestamp: now,
            capacity: current_cap,
            state: state.clone(),
        });

        // Retain rolling 1 hour of samples (max 720 samples at 5s intervals)
        if self.samples.len() > 720 {
            self.samples.remove(0);
        }

        // Bracket checking
        self.check_brackets(current_cap, now);

        self.last_capacity = current_cap;
        self.last_timestamp = Some(now);
        let _ = self.save();
    }

    fn check_brackets(&mut self, current_cap: u8, now: DateTime<Utc>) {
        let bracket_idx = (current_cap as usize / 10).min(9);
        if let Some(start_time) = self.session_start_time {
            let elapsed_secs = (now - start_time).num_seconds().max(0) as u64;
            if current_cap > self.session_start_cap && elapsed_secs > 0 {
                let rate =
                    calculate_rate_pct_per_hour(self.session_start_cap, current_cap, elapsed_secs);
                if bracket_idx < self.brackets.len() {
                    let b = &mut self.brackets[bracket_idx];
                    b.duration_secs = elapsed_secs;
                    b.rate_pct_per_hour = rate;
                    if current_cap.is_multiple_of(10) || current_cap == 100 {
                        b.completed = true;
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn current_speed_pct_per_hour(&self, current_cap: u8) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        // Use up to 30 mins window of samples for smoothing
        let now = Utc::now();
        let oldest = self
            .samples
            .iter()
            .find(|s| (now - s.timestamp).num_minutes() <= 30)
            .unwrap_or_else(|| &self.samples[0]);

        let duration_secs = (now - oldest.timestamp).num_seconds().max(0) as u64;
        calculate_rate_pct_per_hour(oldest.capacity, current_cap, duration_secs)
    }
}

fn get_tracker_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    Path::new(&home).join(".local/share/server-chronicle/battery_tracker.json")
}

pub fn read_battery_snapshot() -> BatterySnapshot {
    let mut snap = BatterySnapshot::default();
    let bat_dir = Path::new(BATTERY_SYS_PATH);

    if !bat_dir.exists() {
        return snap;
    }

    let status_str =
        read_sys_file(&bat_dir.join("status")).unwrap_or_else(|| "Unknown".to_string());
    snap.state = PowerState::from_str(&status_str);
    snap.capacity = read_sys_u8(&bat_dir.join("capacity")).unwrap_or(0);
    snap.charge_now_uah = read_sys_u64(&bat_dir.join("charge_now")).unwrap_or(0);
    snap.charge_full_uah = read_sys_u64(&bat_dir.join("charge_full")).unwrap_or(0);
    snap.charge_full_design_uah = read_sys_u64(&bat_dir.join("charge_full_design")).unwrap_or(0);
    snap.voltage_now_uv = read_sys_u64(&bat_dir.join("voltage_now")).unwrap_or(0);

    let ac_path = Path::new(AC_SYS_PATH);
    snap.ac_online = read_sys_u8(ac_path).unwrap_or(0) == 1;

    snap.health_percent =
        calculate_health_percent(snap.charge_full_uah, snap.charge_full_design_uah);

    let mut tracker = PersistentBatteryTracker::load_or_init();
    tracker.update(snap.capacity, &snap.state);

    snap.calculated_rate_pct_hr = tracker.current_speed_pct_per_hour(snap.capacity);
    if snap.calculated_rate_pct_hr > 0.01 {
        snap.mins_per_percent = 60.0 / snap.calculated_rate_pct_hr;
    }

    snap.brackets = tracker.brackets;

    if snap.state == PowerState::Discharging && snap.calculated_rate_pct_hr > 0.1 {
        // Estimate minutes left until 5% threshold
        let usable_pct = snap.capacity.saturating_sub(5);
        let mins = ((usable_pct as f64 / snap.calculated_rate_pct_hr) * 60.0) as u64;
        snap.estimated_minutes_left = Some(mins);
    } else if snap.state == PowerState::Charging && snap.calculated_rate_pct_hr > 0.1 {
        // Estimate minutes to reach 100%
        let to_charge = 100u8.saturating_sub(snap.capacity);
        let mins = ((to_charge as f64 / snap.calculated_rate_pct_hr) * 60.0) as u64;
        snap.estimated_minutes_left = Some(mins);
    }

    snap
}

fn read_sys_file(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_sys_u8(path: &Path) -> Option<u8> {
    read_sys_file(path).and_then(|s| s.parse().ok())
}

fn read_sys_u64(path: &Path) -> Option<u64> {
    read_sys_file(path).and_then(|s| s.parse().ok())
}
