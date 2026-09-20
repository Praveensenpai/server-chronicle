use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum ServerActivityEvent {
    BatteryStateChanged {
        from_status: String,
        to_status: String,
        capacity: u8,
    },
    BatteryBracketCompleted {
        bracket: String,
        duration_secs: u64,
        rate_pct_per_hour: f64,
    },
    PowerOutageAlert {
        capacity: u8,
        estimated_runtime_mins: u64,
    },
    PowerRestoredAlert {
        capacity: u8,
    },
    SshLogin {
        user: String,
        client_ip: String,
    },
    SshLogout {
        user: String,
        client_ip: String,
        duration_secs: u64,
    },
    ContainerStateChanged {
        name: String,
        status: String,
    },
    TorrentCompleted {
        name: String,
        size_bytes: u64,
    },
    ResourceSpike {
        metric: String,
        value: f64,
        threshold: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offender: Option<String>,
    },
    SystemStartup {
        version: String,
        hostname: String,
        uptime_secs: u64,
    },
    SystemHeartbeat {
        summary: String,
    },
    ThermalAlert {
        temp_c: f64,
        threshold_c: f64,
    },
    DiskMilestone {
        path: String,
        percent: f64,
        threshold: f64,
    },
    GenericNote {
        message: String,
    },
}

impl ServerActivityEvent {
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::SystemStartup {
                version,
                hostname,
                uptime_secs,
            } => {
                format!(
                    "Daemon v{version} online on {hostname} (uptime {}h {}m)",
                    uptime_secs / 3600,
                    (uptime_secs % 3600) / 60
                )
            }
            Self::SystemHeartbeat { summary } => summary.clone(),
            Self::BatteryStateChanged {
                from_status,
                to_status,
                capacity,
            } => {
                format!("{from_status} ➔ {to_status} ({capacity}%)")
            }
            Self::BatteryBracketCompleted {
                bracket,
                duration_secs,
                rate_pct_per_hour,
            } => {
                format!(
                    "{bracket} in {}m {}s ({rate_pct_per_hour:.1}%/hr)",
                    duration_secs / 60,
                    duration_secs % 60
                )
            }
            Self::PowerOutageAlert {
                capacity,
                estimated_runtime_mins,
            } => {
                format!("AC Lost! Battery at {capacity}%, ~{estimated_runtime_mins}m left")
            }
            Self::PowerRestoredAlert { capacity } => {
                format!("AC Power Restored! Capacity: {capacity}%")
            }
            Self::ThermalAlert {
                temp_c,
                threshold_c,
            } => {
                format!(
                    "CPU thermal alert: {temp_c:.1}°C exceeded safe ceiling ({threshold_c:.1}°C)"
                )
            }
            Self::DiskMilestone {
                path,
                percent,
                threshold,
            } => {
                format!("Disk milestone: {path} reached {percent:.1}% (threshold {threshold:.1}%)")
            }
            Self::SshLogin { user, client_ip } => {
                format!("SSH login from {user}@{client_ip}")
            }
            Self::SshLogout {
                user,
                client_ip,
                duration_secs,
            } => {
                format!(
                    "SSH closed: {user}@{client_ip} (active {}m)",
                    duration_secs / 60
                )
            }
            Self::ContainerStateChanged { name, status } => {
                format!("Container {name}: {status}")
            }
            Self::TorrentCompleted { name, size_bytes } => {
                format!(
                    "Downloaded {name} ({:.1} GB)",
                    *size_bytes as f64 / 1_000_000_000.0
                )
            }
            Self::ResourceSpike {
                metric,
                value,
                threshold,
                offender,
            } => {
                if let Some(off) = offender {
                    format!("{metric} spike: {value:.1}% > {threshold:.1}% (top: {off})")
                } else {
                    format!("{metric} spike: {value:.1}% > {threshold:.1}%")
                }
            }
            Self::GenericNote { message } => message.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub timestamp: DateTime<Utc>,
    pub event: ServerActivityEvent,
}

impl EventRecord {
    #[must_use]
    pub fn new(event: ServerActivityEvent) -> Self {
        Self {
            timestamp: Utc::now(),
            event,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerInfo {
    pub name: String,
    pub status: String,
    pub cpu_percent: f64,
    pub memory_usage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshSession {
    pub user: String,
    pub client_ip: String,
    pub tty_or_port: String,
    pub connected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TorrentSnapshot {
    pub name: String,
    pub progress: f64,
    pub state: String,
    pub dlspeed: u64,
    pub upspeed: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemMetrics {
    pub cpu_percent: f64,
    pub cpu_temp_c: Option<f64>,
    pub cpu_core_temps_c: Vec<f64>,
    pub fan_speed_rpm: Option<u32>,
    #[serde(default)]
    pub fan_status: String,
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub load_avg: (f64, f64, f64),
    pub uptime_secs: u64,
    pub hostname: String,
}

impl SystemMetrics {
    #[must_use]
    pub fn fan_display(&self) -> &str {
        if !self.fan_status.is_empty() {
            &self.fan_status
        } else {
            "N/A"
        }
    }

    #[must_use]
    pub fn mem_percent(&self) -> f64 {
        if self.mem_total_bytes == 0 {
            0.0
        } else {
            (self.mem_used_bytes as f64 / self.mem_total_bytes as f64) * 100.0
        }
    }

    #[must_use]
    pub fn disk_percent(&self) -> f64 {
        if self.disk_total_bytes == 0 {
            0.0
        } else {
            (self.disk_used_bytes as f64 / self.disk_total_bytes as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProcessSortMode {
    #[default]
    Cpu,
    Ram,
}

impl ProcessSortMode {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Ram => "RAM",
        }
    }

    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::Cpu => Self::Ram,
            Self::Ram => Self::Cpu,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub mem_percent: f64,
    pub mem_bytes: u64,
}

impl ProcessItem {
    #[must_use]
    pub fn formatted_mem(&self) -> String {
        const KIB: u64 = 1024;
        const MIB: u64 = 1024 * 1024;
        const GIB: u64 = 1024 * 1024 * 1024;

        if self.mem_bytes >= GIB {
            format!("{:.2} GiB", self.mem_bytes as f64 / GIB as f64)
        } else if self.mem_bytes >= MIB {
            format!("{:.1} MiB", self.mem_bytes as f64 / MIB as f64)
        } else if self.mem_bytes >= KIB {
            format!("{:.0} KiB", self.mem_bytes as f64 / KIB as f64)
        } else {
            format!("{} B", self.mem_bytes)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerSnapshot {
    pub system: SystemMetrics,
    pub containers: Vec<ContainerInfo>,
    pub ssh_sessions: Vec<SshSession>,
    pub torrents: Vec<TorrentSnapshot>,
    pub top_processes: Vec<ProcessItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_metrics_percentages() {
        let metrics = SystemMetrics {
            mem_used_bytes: 4 * 1024 * 1024 * 1024,
            mem_total_bytes: 8 * 1024 * 1024 * 1024,
            disk_used_bytes: 50 * 1000 * 1000 * 1000,
            disk_total_bytes: 100 * 1000 * 1000 * 1000,
            ..Default::default()
        };
        assert!((metrics.mem_percent() - 50.0).abs() < 0.01);
        assert!((metrics.disk_percent() - 50.0).abs() < 0.01);

        let zero_metrics = SystemMetrics::default();
        assert_eq!(zero_metrics.mem_percent(), 0.0);
        assert_eq!(zero_metrics.disk_percent(), 0.0);
    }

    #[test]
    fn test_process_sort_and_mem() {
        let mode = ProcessSortMode::Cpu;
        assert_eq!(mode.label(), "CPU");
        assert_eq!(mode.toggle(), ProcessSortMode::Ram);
        assert_eq!(mode.toggle().label(), "RAM");
        assert_eq!(mode.toggle().toggle(), ProcessSortMode::Cpu);

        let proc = ProcessItem {
            pid: 42,
            name: "jellyfin".into(),
            cpu_percent: 1.2,
            mem_percent: 5.0,
            mem_bytes: 350 * 1024 * 1024,
        };
        assert_eq!(proc.formatted_mem(), "350.0 MiB");
        let gig_proc = ProcessItem {
            mem_bytes: 2 * 1024 * 1024 * 1024,
            ..Default::default()
        };
        assert_eq!(gig_proc.formatted_mem(), "2.00 GiB");
    }

    #[test]
    fn test_event_roundtrip_serde() {
        let events = vec![
            ServerActivityEvent::PowerOutageAlert {
                capacity: 85,
                estimated_runtime_mins: 120,
            },
            ServerActivityEvent::SshLogin {
                user: "neko".into(),
                client_ip: "100.108.154.50".into(),
            },
            ServerActivityEvent::ResourceSpike {
                metric: "RAM".into(),
                value: 95.5,
                threshold: 90.0,
                offender: Some("ffmpeg".into()),
            },
            ServerActivityEvent::GenericNote {
                message: "system check".into(),
            },
        ];

        for ev in events {
            let record = EventRecord::new(ev.clone());
            let json = serde_json::to_string(&record).expect("serialization failed");
            let back: EventRecord = serde_json::from_str(&json).expect("deserialization failed");
            assert_eq!(back.event, ev);
        }
    }
}
