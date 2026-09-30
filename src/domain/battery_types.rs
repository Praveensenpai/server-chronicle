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
pub struct BatterySnapshot {
    pub state: PowerState,
    pub ac_online: bool,
    pub capacity: u8,
    pub charge_now_uah: u64,
    pub charge_full_uah: u64,
    pub charge_full_design_uah: u64,
    pub voltage_now_uv: u64,
    pub power_now_uw: Option<u64>,
    pub health_percent: f64,
    pub calculated_rate_pct_hr: f64,
    pub mins_per_percent: f64,
    pub estimated_minutes_left: Option<u64>,
    pub brackets: Vec<BracketStat>,
    pub discharge_brackets: Vec<BracketStat>,
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
            power_now_uw: None,
            health_percent: 100.0,
            calculated_rate_pct_hr: 0.0,
            mins_per_percent: 0.0,
            estimated_minutes_left: None,
            brackets: init_default_brackets(),
            discharge_brackets: init_default_discharge_brackets(),
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
pub fn init_default_discharge_brackets() -> Vec<BracketStat> {
    let mut out = Vec::with_capacity(10);
    for i in (0..10).rev() {
        let high = (i + 1) * 10;
        let low = i * 10;
        out.push(BracketStat {
            label: format!("{high}% - {low}%"),
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
pub fn charging_bracket_index(cap: u8) -> usize {
    if cap >= 100 {
        9
    } else {
        (cap as usize / 10).min(9)
    }
}

#[must_use]
pub fn discharging_bracket_index(cap: u8) -> usize {
    if cap >= 100 {
        0
    } else {
        let drop = 100usize.saturating_sub(cap as usize);
        if drop == 0 {
            0
        } else {
            ((drop.saturating_sub(1)) / 10).min(9)
        }
    }
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

        // Zero duration returns 0.0
        assert_eq!(calculate_rate_pct_per_hour(10, 20, 0), 0.0);
    }

    #[test]
    fn test_calculate_health_percent() {
        let health = calculate_health_percent(3600000, 3600000);
        assert!((health - 100.0).abs() < 0.01);

        let degraded = calculate_health_percent(1800000, 3600000);
        assert!((degraded - 50.0).abs() < 0.01);

        // Zero design capacity returns 100.0
        assert_eq!(calculate_health_percent(1800000, 0), 100.0);
    }

    #[test]
    fn test_power_state_parsing() {
        assert_eq!(PowerState::from_str("Charging"), PowerState::Charging);
        assert_eq!(PowerState::from_str("Discharging"), PowerState::Discharging);
        assert_eq!(PowerState::from_str("Full"), PowerState::Full);
        assert_eq!(
            PowerState::from_str("Not Charging"),
            PowerState::NotCharging
        );
        assert_eq!(PowerState::from_str("anything_else"), PowerState::Unknown);
    }

    #[test]
    fn test_init_brackets() {
        let charge_b = init_default_brackets();
        assert_eq!(charge_b.len(), 10);
        assert_eq!(charge_b[0].label, "0% - 10%");
        assert_eq!(charge_b[9].label, "90% - 100%");

        let discharge_b = init_default_discharge_brackets();
        assert_eq!(discharge_b.len(), 10);
        assert_eq!(discharge_b[0].label, "100% - 90%");
        assert_eq!(discharge_b[9].label, "10% - 0%");
    }

    #[test]
    fn test_bracket_indices() {
        assert_eq!(charging_bracket_index(0), 0);
        assert_eq!(charging_bracket_index(9), 0);
        assert_eq!(charging_bracket_index(10), 1);
        assert_eq!(charging_bracket_index(56), 5);
        assert_eq!(charging_bracket_index(99), 9);
        assert_eq!(charging_bracket_index(100), 9);

        assert_eq!(discharging_bracket_index(100), 0);
        assert_eq!(discharging_bracket_index(91), 0);
        assert_eq!(discharging_bracket_index(90), 0);
        assert_eq!(discharging_bracket_index(89), 1);
        assert_eq!(discharging_bracket_index(56), 4);
        assert_eq!(discharging_bracket_index(1), 9);
        assert_eq!(discharging_bracket_index(0), 9);
    }
}
