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
    },
    GenericNote {
        message: String,
    },
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub mem_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerSnapshot {
    pub system: SystemMetrics,
    pub containers: Vec<ContainerInfo>,
    pub ssh_sessions: Vec<SshSession>,
    pub torrents: Vec<TorrentSnapshot>,
    pub top_processes: Vec<ProcessItem>,
}
