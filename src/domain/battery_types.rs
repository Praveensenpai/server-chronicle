use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PowerState {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

impl PowerState {
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "charging" => Self::Charging,
            "discharging" => Self::Discharging,
            "full" => Self::Full,
            "not charging" => Self::NotCharging,
            _ => Self::Unknown,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Charging => "Charging",
            Self::Discharging => "Discharging (On Battery)",
            Self::Full => "Full (AC Connected)",
            Self::NotCharging => "Not Charging",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BracketStat {
    pub label: String,
    pub duration_secs: u64,
    pub rate_pct_per_hour: f64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatterySample {
    pub timestamp: DateTime<Utc>,
    pub capacity: u8,
    pub state: PowerState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatterySnapshot {
    pub state: PowerState,
    pub ac_online: bool,
    pub capacity: u8,
    pub charge_now_uah: u64,
    pub charge_full_uah: u64,
    pub charge_full_design_uah: u64,
    pub voltage_now_uv: u64,
    pub health_percent: f64,
    pub calculated_rate_pct_hr: f64,
    pub mins_per_percent: f64,
    pub estimated_minutes_left: Option<u64>,
    pub brackets: Vec<BracketStat>,
}

impl Default for BatterySnapshot {
    fn default() -> Self {
        Self {
            state: PowerState::Unknown,
            ac_online: false,
            capacity: 0,
            charge_now_uah: 0,
            charge_full_uah: 0,
            charge_full_design_uah: 0,
            voltage_now_uv: 0,
            health_percent: 100.0,
            calculated_rate_pct_hr: 0.0,
            mins_per_percent: 0.0,
            estimated_minutes_left: None,
            brackets: init_default_brackets(),
        }
    }
}

#[must_use]
pub fn init_default_brackets() -> Vec<BracketStat> {
    let mut out = Vec::with_capacity(10);
    for i in 0..10 {
        let low = i * 10;
        let high = (i + 1) * 10;
        out.push(BracketStat {
            label: format!("{low}% - {high}%"),
            duration_secs: 0,
            rate_pct_per_hour: 0.0,
            completed: false,
        });
    }
    out
}

#[must_use]
pub fn calculate_rate_pct_per_hour(start_cap: u8, end_cap: u8, duration_secs: u64) -> f64 {
    if duration_secs == 0 {
        return 0.0;
    }
    let delta = (end_cap as f64 - start_cap as f64).abs();
    (delta / duration_secs as f64) * 3600.0
}

#[must_use]
pub fn calculate_health_percent(full_uah: u64, design_uah: u64) -> f64 {
    if design_uah == 0 {
        return 100.0;
    }
    ((full_uah as f64 / design_uah as f64) * 100.0).min(100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rate_pct_per_hour() {
        // Charged 10% in 1800 seconds (30 mins) -> 20% per hour
        let rate = calculate_rate_pct_per_hour(10, 20, 1800);
        assert!((rate - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_health_percent() {
        let health = calculate_health_percent(3600000, 3600000);
        assert!((health - 100.0).abs() < 0.01);

        let degraded = calculate_health_percent(1800000, 3600000);
        assert!((degraded - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_power_state_parsing() {
        assert_eq!(PowerState::from_str("Charging"), PowerState::Charging);
        assert_eq!(PowerState::from_str("Discharging"), PowerState::Discharging);
        assert_eq!(PowerState::from_str("Full"), PowerState::Full);
    }
}
