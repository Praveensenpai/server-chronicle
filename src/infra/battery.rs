use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::battery_types::{
    calculate_health_percent, calculate_rate_pct_per_hour, init_default_brackets,
    init_default_discharge_brackets, BatterySample, BatterySnapshot, BracketStat, PowerState,
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
    #[serde(default = "init_default_brackets")]
    pub brackets: Vec<BracketStat>,
    #[serde(default = "init_default_discharge_brackets")]
    pub discharge_brackets: Vec<BracketStat>,
    #[serde(default)]
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
            discharge_brackets: init_default_discharge_brackets(),
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
                if tracker.discharge_brackets.len() != 10 {
                    tracker.discharge_brackets = init_default_discharge_brackets();
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
        self.check_brackets(current_cap, now, state);

        self.last_capacity = current_cap;
        self.last_timestamp = Some(now);
    }

    fn check_brackets(&mut self, current_cap: u8, now: DateTime<Utc>, state: &PowerState) {
        let Some(start_time) = self.session_start_time else {
            return;
        };
        let elapsed_secs = (now - start_time).num_seconds().max(0) as u64;
        if elapsed_secs == 0 {
            return;
        }

        if *state == PowerState::Charging && current_cap > self.session_start_cap {
            let rate =
                calculate_rate_pct_per_hour(self.session_start_cap, current_cap, elapsed_secs);
            let bracket_idx = (current_cap as usize / 10).min(9);
            if bracket_idx < self.brackets.len() {
                let b = &mut self.brackets[bracket_idx];
                b.duration_secs = elapsed_secs;
                b.rate_pct_per_hour = rate;
                if current_cap.is_multiple_of(10) || current_cap == 100 {
                    b.completed = true;
                }
            }
        } else if *state == PowerState::Discharging && current_cap < self.session_start_cap {
            let rate =
                calculate_rate_pct_per_hour(self.session_start_cap, current_cap, elapsed_secs);
            let drop_amount = 100usize.saturating_sub(current_cap as usize);
            let bracket_idx = (drop_amount / 10).min(9);
            if bracket_idx < self.discharge_brackets.len() {
                let b = &mut self.discharge_brackets[bracket_idx];
                b.duration_secs = elapsed_secs;
                b.rate_pct_per_hour = rate;
                if current_cap.is_multiple_of(10) || current_cap == 0 {
                    b.completed = true;
                }
            }
        }
    }

    #[must_use]
    pub fn current_speed_pct_per_hour(&self, current_cap: u8, state: &PowerState) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let now = Utc::now();
        let state_samples: Vec<&BatterySample> =
            self.samples.iter().filter(|s| s.state == *state).collect();

        if state_samples.len() < 2 {
            return 0.0;
        }

        // Use up to 30 mins window of state-isolated samples for smoothing
        let oldest = state_samples
            .iter()
            .find(|s| (now - s.timestamp).num_minutes() <= 30)
            .copied()
            .unwrap_or(state_samples[0]);

        let duration_secs = (now - oldest.timestamp).num_seconds().max(0) as u64;
        if duration_secs < 20 {
            return 0.0;
        }
        let raw = calculate_rate_pct_per_hour(oldest.capacity, current_cap, duration_secs);
        raw.min(150.0)
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

    let power_path = bat_dir.join("power_now");
    if power_path.exists() {
        snap.power_now_uw = read_sys_u64(&power_path);
    } else if snap.voltage_now_uv > 0 {
        let current_path = bat_dir.join("current_now");
        if let Some(curr_ua) = read_sys_u64(&current_path) {
            snap.power_now_uw = Some((snap.voltage_now_uv / 1000) * (curr_ua / 1000));
        }
    }

    let ac_path = Path::new(AC_SYS_PATH);
    snap.ac_online = read_sys_u8(ac_path).unwrap_or(0) == 1;

    snap.health_percent =
        calculate_health_percent(snap.charge_full_uah, snap.charge_full_design_uah);

    let mut tracker = PersistentBatteryTracker::load_or_init();
    tracker.update(snap.capacity, &snap.state);
    let _ = tracker.save();

    snap.calculated_rate_pct_hr = tracker.current_speed_pct_per_hour(snap.capacity, &snap.state);
    if snap.calculated_rate_pct_hr > 0.01 {
        snap.mins_per_percent = 60.0 / snap.calculated_rate_pct_hr;
    }

    snap.brackets = tracker.brackets;
    snap.discharge_brackets = tracker.discharge_brackets;

    if snap.state == PowerState::Discharging {
        if snap.calculated_rate_pct_hr > 0.1 {
            let usable_pct = snap.capacity.saturating_sub(5);
            let mins = ((usable_pct as f64 / snap.calculated_rate_pct_hr) * 60.0) as u64;
            snap.estimated_minutes_left = Some(mins);
        } else if let Some(start_time) = tracker.session_start_time {
            let elapsed_secs = (Utc::now() - start_time).num_seconds().max(0) as u64;
            if tracker.session_start_cap > snap.capacity && elapsed_secs > 30 {
                let session_rate = calculate_rate_pct_per_hour(
                    tracker.session_start_cap,
                    snap.capacity,
                    elapsed_secs,
                );
                if session_rate > 0.1 {
                    let usable_pct = snap.capacity.saturating_sub(5);
                    let mins = ((usable_pct as f64 / session_rate) * 60.0) as u64;
                    snap.estimated_minutes_left = Some(mins);
                }
            }
        }
    } else if snap.state == PowerState::Charging && snap.calculated_rate_pct_hr > 0.1 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_bracket_updates() {
        let mut tracker = PersistentBatteryTracker::new();
        let state = PowerState::Discharging;

        // Start session at 100%
        tracker.update(100, &state);
        assert_eq!(tracker.session_start_cap, 100);

        // Simulate 5 minutes passing and drop to 95%
        tracker.update(95, &state);
        assert_eq!(tracker.last_capacity, 95);

        // Test charging transition
        let charge_state = PowerState::Charging;
        tracker.update(95, &charge_state);
        assert_eq!(tracker.session_start_cap, 95);
        assert_eq!(tracker.last_state, "Charging");
    }
}
