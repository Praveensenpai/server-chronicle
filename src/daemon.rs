use anyhow::Result;
use std::collections::HashSet;
use std::thread;
use std::time::Duration;

use crate::domain::battery_types::PowerState;
use crate::domain::models::{EventRecord, ServerActivityEvent};
use crate::infra::{
    battery::read_battery_snapshot, capture_server_snapshot, storage::append_event,
};

pub fn run_daemon(interval_secs: u64) -> Result<()> {
    println!(
        "  ⚡ Starting server-chronicle background daemon (sampling every {interval_secs}s)..."
    );

    let initial_bat = read_battery_snapshot();
    let mut last_bat_state = initial_bat.state;
    let mut last_capacity = initial_bat.capacity;
    let mut known_containers: HashSet<String> = HashSet::new();
    let mut known_ssh_clients: HashSet<String> = HashSet::new();
    let mut last_spike_check = std::time::Instant::now();

    loop {
        thread::sleep(Duration::from_secs(interval_secs));

        let bat = read_battery_snapshot();
        let snap = capture_server_snapshot();

        // 1. Check Power / Battery changes
        if bat.state != last_bat_state {
            let event = match (&last_bat_state, &bat.state) {
                (_, PowerState::Discharging) => ServerActivityEvent::PowerOutageAlert {
                    capacity: bat.capacity,
                    estimated_runtime_mins: bat.estimated_minutes_left.unwrap_or(0),
                },
                (PowerState::Discharging, PowerState::Charging | PowerState::Full) => {
                    ServerActivityEvent::PowerRestoredAlert {
                        capacity: bat.capacity,
                    }
                }
                _ => ServerActivityEvent::BatteryStateChanged {
                    from_status: last_bat_state.as_str().to_string(),
                    to_status: bat.state.as_str().to_string(),
                    capacity: bat.capacity,
                },
            };
            let _ = append_event(&EventRecord::new(event));
            last_bat_state = bat.state.clone();
        }

        // Capacity milestone checks (every 10%)
        if bat.capacity != last_capacity {
            if bat.capacity.is_multiple_of(10) || bat.capacity == 100 {
                let bracket_idx = (bat.capacity as usize / 10).saturating_sub(1);
                if let Some(b) = bat.brackets.get(bracket_idx) {
                    if b.duration_secs > 0 {
                        let _ = append_event(&EventRecord::new(
                            ServerActivityEvent::BatteryBracketCompleted {
                                bracket: b.label.clone(),
                                duration_secs: b.duration_secs,
                                rate_pct_per_hour: b.rate_pct_per_hour,
                            },
                        ));
                    }
                }
            }
            last_capacity = bat.capacity;
        }

        // 2. Check Docker Container transitions
        let current_containers: HashSet<String> = snap
            .containers
            .iter()
            .map(|c| format!("{}:{}", c.name, c.status))
            .collect();

        for c_str in current_containers.difference(&known_containers) {
            if let Some((name, status)) = c_str.split_once(':') {
                let _ = append_event(&EventRecord::new(
                    ServerActivityEvent::ContainerStateChanged {
                        name: name.to_string(),
                        status: status.to_string(),
                    },
                ));
            }
        }
        known_containers = current_containers;

        // 3. Check SSH Logins
        let current_ssh: HashSet<String> = snap
            .ssh_sessions
            .iter()
            .map(|s| format!("{}:{}", s.user, s.client_ip))
            .collect();

        for s_str in current_ssh.difference(&known_ssh_clients) {
            if let Some((user, ip)) = s_str.split_once(':') {
                let _ = append_event(&EventRecord::new(ServerActivityEvent::SshLogin {
                    user: user.to_string(),
                    client_ip: ip.to_string(),
                }));
            }
        }
        known_ssh_clients = current_ssh;

        // 4. Check Resource Spikes (rate-limited every 5 mins)
        if last_spike_check.elapsed() > Duration::from_secs(300) {
            if snap.system.mem_percent() > 90.0 {
                let _ = append_event(&EventRecord::new(ServerActivityEvent::ResourceSpike {
                    metric: "RAM Usage".to_string(),
                    value: snap.system.mem_percent(),
                    threshold: 90.0,
                }));
            }
            if snap.system.disk_percent() > 88.0 {
                let _ = append_event(&EventRecord::new(ServerActivityEvent::ResourceSpike {
                    metric: "Root Disk Usage".to_string(),
                    value: snap.system.disk_percent(),
                    threshold: 88.0,
                }));
            }
            last_spike_check = std::time::Instant::now();
        }
    }
}
