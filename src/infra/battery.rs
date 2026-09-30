use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::battery_types::{
    calculate_health_percent, calculate_rate_pct_per_hour, charging_bracket_index,
    discharging_bracket_index, init_default_brackets, init_default_discharge_brackets,
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
    #[serde(default)]
    pub current_bracket_start_cap: u8,
    #[serde(default)]
    pub current_bracket_start_time: Option<DateTime<Utc>>,
    #[serde(default = "init_default_brackets")]
    pub brackets: Vec<BracketStat>,
    #[serde(default = "init_default_discharge_brackets")]
    pub discharge_brackets: Vec<BracketStat>,
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
            current_bracket_start_cap: 0,
            current_bracket_start_time: None,
            brackets: init_default_brackets(),
            discharge_brackets: init_default_discharge_brackets(),
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

        if self.last_state != state_str || self.session_start_time.is_none() {
            self.session_start_cap = current_cap;
            self.session_start_time = Some(now);
            self.current_bracket_start_cap = current_cap;
            self.current_bracket_start_time = Some(now);
            self.last_state = state_str.clone();
        }

        // Bracket tracking
        match state {
            PowerState::Charging => {
                self.update_charging_brackets(current_cap, now);
            }
            PowerState::Discharging => {
                self.update_discharging_brackets(current_cap, now);
            }
            _ => {
                self.current_bracket_start_cap = current_cap;
                self.current_bracket_start_time = Some(now);
            }
        }

        self.last_capacity = current_cap;
        self.last_timestamp = Some(now);
    }

    fn update_charging_brackets(&mut self, current_cap: u8, now: DateTime<Utc>) {
        let curr_idx = charging_bracket_index(current_cap);
        let prev_idx = charging_bracket_index(self.last_capacity);

        if current_cap > self.last_capacity && prev_idx < curr_idx {
            for k in prev_idx..curr_idx {
                if k < self.brackets.len() {
                    let target_cap = ((k + 1) as u8 * 10).min(100);
                    let start_t = self.current_bracket_start_time.unwrap_or(now);
                    finalize_bracket(
                        &mut self.brackets[k],
                        self.current_bracket_start_cap,
                        target_cap,
                        start_t,
                        now,
                    );
                    self.current_bracket_start_cap = target_cap;
                    self.current_bracket_start_time = Some(now);
                }
            }
        }

        if current_cap >= 100 && !self.brackets[9].completed {
            let start_t = self.current_bracket_start_time.unwrap_or(now);
            finalize_bracket(
                &mut self.brackets[9],
                self.current_bracket_start_cap,
                100,
                start_t,
                now,
            );
        } else if curr_idx < self.brackets.len() && !self.brackets[curr_idx].completed {
            update_live_bracket(
                &mut self.brackets[curr_idx],
                self.current_bracket_start_cap,
                current_cap,
                self.current_bracket_start_time.unwrap_or(now),
                now,
            );
        }
    }

    fn update_discharging_brackets(&mut self, current_cap: u8, now: DateTime<Utc>) {
        let curr_idx = discharging_bracket_index(current_cap);
        let prev_idx = discharging_bracket_index(self.last_capacity);

        if current_cap < self.last_capacity && prev_idx < curr_idx {
            for k in prev_idx..curr_idx {
                if k < self.discharge_brackets.len() {
                    let target_cap = (10u8.saturating_sub((k + 1) as u8)) * 10;
                    let start_t = self.current_bracket_start_time.unwrap_or(now);
                    finalize_bracket(
                        &mut self.discharge_brackets[k],
                        self.current_bracket_start_cap,
                        target_cap,
                        start_t,
                        now,
                    );
                    self.current_bracket_start_cap = target_cap;
                    self.current_bracket_start_time = Some(now);
                }
            }
        }

        if current_cap == 0 && !self.discharge_brackets[9].completed {
            let start_t = self.current_bracket_start_time.unwrap_or(now);
            finalize_bracket(
                &mut self.discharge_brackets[9],
                self.current_bracket_start_cap,
                0,
                start_t,
                now,
            );
        } else if curr_idx < self.discharge_brackets.len()
            && !self.discharge_brackets[curr_idx].completed
        {
            update_live_bracket(
                &mut self.discharge_brackets[curr_idx],
                self.current_bracket_start_cap,
                current_cap,
                self.current_bracket_start_time.unwrap_or(now),
                now,
            );
        }
    }

    #[must_use]
    pub fn current_speed_pct_per_hour(&self, current_cap: u8, state: &PowerState) -> f64 {
        if *state != PowerState::Charging && *state != PowerState::Discharging {
            return 0.0;
        }
        let now = Utc::now();
        if let Some(start_time) = self.current_bracket_start_time {
            let duration_secs = (now - start_time).num_seconds().max(0) as u64;
            if duration_secs >= 20 && self.current_bracket_start_cap != current_cap {
                return calculate_rate_pct_per_hour(
                    self.current_bracket_start_cap,
                    current_cap,
                    duration_secs,
                )
                .min(150.0);
            }
        }
        if let Some(session_start) = self.session_start_time {
            let duration_secs = (now - session_start).num_seconds().max(0) as u64;
            if duration_secs >= 30 && self.session_start_cap != current_cap {
                return calculate_rate_pct_per_hour(
                    self.session_start_cap,
                    current_cap,
                    duration_secs,
                )
                .min(150.0);
            }
        }
        0.0
    }
}

fn finalize_bracket(
    bracket: &mut BracketStat,
    start_cap: u8,
    target_cap: u8,
    start_t: DateTime<Utc>,
    now: DateTime<Utc>,
) {
    let duration = (now - start_t).num_seconds().max(1) as u64;
    bracket.duration_secs = duration;
    bracket.rate_pct_per_hour = calculate_rate_pct_per_hour(start_cap, target_cap, duration);
    bracket.completed = true;
}

fn update_live_bracket(
    bracket: &mut BracketStat,
    start_cap: u8,
    current_cap: u8,
    start_t: DateTime<Utc>,
    now: DateTime<Utc>,
) {
    let duration = (now - start_t).num_seconds().max(0) as u64;
    if duration > 0 && start_cap != current_cap {
        bracket.duration_secs = duration;
        bracket.rate_pct_per_hour = calculate_rate_pct_per_hour(start_cap, current_cap, duration);
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

    let status_str = read_sys_str(&bat_dir.join("status")).unwrap_or_else(|| "Unknown".to_string());
    snap.state = PowerState::from_str(&status_str);
    snap.capacity = read_sys_num(&bat_dir.join("capacity")).unwrap_or(0);
    snap.charge_now_uah = read_sys_num(&bat_dir.join("charge_now")).unwrap_or(0);
    snap.charge_full_uah = read_sys_num(&bat_dir.join("charge_full")).unwrap_or(0);
    snap.charge_full_design_uah = read_sys_num(&bat_dir.join("charge_full_design")).unwrap_or(0);
    snap.voltage_now_uv = read_sys_num(&bat_dir.join("voltage_now")).unwrap_or(0);

    let power_path = bat_dir.join("power_now");
    if power_path.exists() {
        snap.power_now_uw = read_sys_num(&power_path);
    } else if snap.voltage_now_uv > 0 {
        let current_path = bat_dir.join("current_now");
        if let Some(curr_ua) = read_sys_num::<u64>(&current_path) {
            snap.power_now_uw = Some((snap.voltage_now_uv / 1000) * (curr_ua / 1000));
        }
    }

    snap.ac_online = read_sys_num::<u8>(Path::new(AC_SYS_PATH)).unwrap_or(0) == 1;
    snap.health_percent =
        calculate_health_percent(snap.charge_full_uah, snap.charge_full_design_uah);

    let mut tracker = PersistentBatteryTracker::load_or_init();
    tracker.update(snap.capacity, &snap.state);
    let _ = tracker.save();

    snap.calculated_rate_pct_hr = tracker.current_speed_pct_per_hour(snap.capacity, &snap.state);
    if snap.calculated_rate_pct_hr > 0.01 {
        snap.mins_per_percent = 60.0 / snap.calculated_rate_pct_hr;
    }

    compute_runtime_estimate(&mut snap, &tracker);
    snap.brackets = tracker.brackets;
    snap.discharge_brackets = tracker.discharge_brackets;

    snap
}

fn compute_runtime_estimate(snap: &mut BatterySnapshot, tracker: &PersistentBatteryTracker) {
    if snap.state == PowerState::Discharging {
        if snap.calculated_rate_pct_hr > 0.1 {
            let mins = ((snap.capacity.saturating_sub(5) as f64 / snap.calculated_rate_pct_hr)
                * 60.0) as u64;
            snap.estimated_minutes_left = Some(mins);
        } else if let Some(st) = tracker.session_start_time {
            let secs = (Utc::now() - st).num_seconds().max(0) as u64;
            if tracker.session_start_cap > snap.capacity && secs > 30 {
                let rate =
                    calculate_rate_pct_per_hour(tracker.session_start_cap, snap.capacity, secs);
                if rate > 0.1 {
                    let mins = ((snap.capacity.saturating_sub(5) as f64 / rate) * 60.0) as u64;
                    snap.estimated_minutes_left = Some(mins);
                }
            }
        }
    } else if snap.state == PowerState::Charging && snap.calculated_rate_pct_hr > 0.1 {
        let mins = ((100u8.saturating_sub(snap.capacity) as f64 / snap.calculated_rate_pct_hr)
            * 60.0) as u64;
        snap.estimated_minutes_left = Some(mins);
    }
}

fn read_sys_str(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_sys_num<T: std::str::FromStr>(path: &Path) -> Option<T> {
    read_sys_str(path).and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_bracket_updates() {
        let mut tracker = PersistentBatteryTracker::new();
        let state = PowerState::Discharging;
        tracker.update(100, &state);
        assert_eq!(tracker.session_start_cap, 100);

        tracker.update(95, &state);
        assert_eq!(tracker.last_capacity, 95);

        let charge_state = PowerState::Charging;
        tracker.update(95, &charge_state);
        assert_eq!(tracker.session_start_cap, 95);
        assert_eq!(tracker.last_state, "Charging");
    }

    #[test]
    fn test_charging_bracket_progression() {
        let mut tracker = PersistentBatteryTracker::new();
        let state = PowerState::Charging;
        tracker.update(0, &state);
        tracker.update(5, &state);
        assert!(!tracker.brackets[0].completed);

        tracker.update(10, &state);
        assert!(tracker.brackets[0].completed);
        assert_eq!(tracker.brackets[0].label, "0% - 10%");
        assert!(tracker.brackets[0].rate_pct_per_hour > 0.0);

        tracker.update(20, &state);
        assert!(tracker.brackets[1].completed);
        assert_eq!(tracker.brackets[1].label, "10% - 20%");
    }

    #[test]
    fn test_discharging_bracket_progression() {
        let mut tracker = PersistentBatteryTracker::new();
        let state = PowerState::Discharging;
        tracker.update(100, &state);
        tracker.update(95, &state);
        assert!(!tracker.discharge_brackets[0].completed);

        tracker.update(89, &state);
        assert!(tracker.discharge_brackets[0].completed);
        assert_eq!(tracker.discharge_brackets[0].label, "100% - 90%");
        assert!(tracker.discharge_brackets[0].rate_pct_per_hour > 0.0);
    }
}
