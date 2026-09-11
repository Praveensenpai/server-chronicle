use anyhow::Result;
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::battery_types::{BatterySnapshot, PowerState};
use crate::domain::models::{EventRecord, ServerActivityEvent, ServerSnapshot};
use crate::infra::{
    battery::read_battery_snapshot, capture_server_snapshot, storage::append_event,
};

pub fn run_daemon(interval_secs: u64) -> Result<()> {
    println!(
        "  ⚡ Starting server-chronicle background daemon (sampling every {interval_secs}s)..."
    );

    let initial_bat = read_battery_snapshot();
    let initial_snap = capture_server_snapshot();

    record_and_log(ServerActivityEvent::SystemStartup {
        version: env!("CARGO_PKG_VERSION").to_string(),
        hostname: initial_snap.system.hostname.clone(),
        uptime_secs: initial_snap.system.uptime_secs,
    });

    let mut last_bat_state = initial_bat.state;
    let mut last_capacity = initial_bat.capacity;
    let mut known_containers = extract_containers(&initial_snap);
    let mut known_ssh_sessions: HashMap<String, Instant> = initial_snap
        .ssh_sessions
        .iter()
        .map(|s| (format!("{}:{}", s.user, s.client_ip), Instant::now()))
        .collect();
    let mut known_torrents: HashMap<String, f64> = initial_snap
        .torrents
        .iter()
        .map(|t| (t.name.clone(), t.progress))
        .collect();
    let mut last_spike_check = Instant::now();
    let mut last_heartbeat = Instant::now();

    loop {
        thread::sleep(Duration::from_secs(interval_secs));

        let bat = read_battery_snapshot();
        let snap = capture_server_snapshot();

        check_power_events(&bat, &mut last_bat_state, &mut last_capacity);
        check_container_events(&snap, &mut known_containers);
        check_ssh_events(&snap, &mut known_ssh_sessions);
        check_torrent_events(&snap, &mut known_torrents);

        if last_spike_check.elapsed() > Duration::from_secs(300) {
            check_resource_spikes(&snap);
            last_spike_check = Instant::now();
        }

        if last_heartbeat.elapsed() > Duration::from_secs(3600) {
            emit_heartbeat(&snap, &bat);
            last_heartbeat = Instant::now();
        }
    }
}

#[must_use]
pub fn simplify_container_status(status: &str) -> &'static str {
    let s = status.trim().to_lowercase();
    if s.starts_with("up") {
        "running"
    } else if s.starts_with("exit") {
        "exited"
    } else if s.starts_with("pause") {
        "paused"
    } else if s.starts_with("restart") {
        "restarting"
    } else {
        "stopped"
    }
}

fn extract_containers(snap: &ServerSnapshot) -> HashMap<String, String> {
    snap.containers
        .iter()
        .map(|c| {
            (
                c.name.clone(),
                simplify_container_status(&c.status).to_string(),
            )
        })
        .collect()
}

fn check_power_events(bat: &BatterySnapshot, last_state: &mut PowerState, last_cap: &mut u8) {
    if bat.state != *last_state {
        let event = match (&*last_state, &bat.state) {
            (_, PowerState::Discharging) => {
                let est_mins = bat.estimated_minutes_left.unwrap_or_else(|| {
                    let usable = bat.capacity.saturating_sub(5) as f64;
                    ((usable / 20.0) * 60.0) as u64
                });
                ServerActivityEvent::PowerOutageAlert {
                    capacity: bat.capacity,
                    estimated_runtime_mins: est_mins,
                }
            }
            (PowerState::Discharging, PowerState::Charging | PowerState::Full) => {
                ServerActivityEvent::PowerRestoredAlert {
                    capacity: bat.capacity,
                }
            }
            _ => ServerActivityEvent::BatteryStateChanged {
                from_status: last_state.as_str().to_string(),
                to_status: bat.state.as_str().to_string(),
                capacity: bat.capacity,
            },
        };
        record_and_log(event);
        *last_state = bat.state.clone();
    }

    if bat.capacity != *last_cap {
        if bat.capacity.is_multiple_of(10) || bat.capacity == 100 || bat.capacity == 0 {
            emit_bracket_event(bat);
        }
        *last_cap = bat.capacity;
    }
}

fn emit_bracket_event(bat: &BatterySnapshot) {
    if bat.state == PowerState::Charging {
        let bracket_idx = (bat.capacity as usize / 10).saturating_sub(1);
        if let Some(b) = bat.brackets.get(bracket_idx) {
            if b.duration_secs > 0 {
                record_and_log(ServerActivityEvent::BatteryBracketCompleted {
                    bracket: b.label.clone(),
                    duration_secs: b.duration_secs,
                    rate_pct_per_hour: b.rate_pct_per_hour,
                });
            }
        }
    } else if bat.state == PowerState::Discharging {
        let drop = 100usize.saturating_sub(bat.capacity as usize);
        let bracket_idx = (drop / 10).saturating_sub(1);
        if let Some(b) = bat.discharge_brackets.get(bracket_idx) {
            if b.duration_secs > 0 {
                record_and_log(ServerActivityEvent::BatteryBracketCompleted {
                    bracket: b.label.clone(),
                    duration_secs: b.duration_secs,
                    rate_pct_per_hour: b.rate_pct_per_hour,
                });
            }
        }
    }
}

fn check_container_events(snap: &ServerSnapshot, known: &mut HashMap<String, String>) {
    let current = extract_containers(snap);
    for (name, status) in &current {
        if let Some(prev_status) = known.get(name) {
            if prev_status != status {
                record_and_log(ServerActivityEvent::ContainerStateChanged {
                    name: name.clone(),
                    status: format!("{prev_status} ➔ {status}"),
                });
            }
        }
    }

    let removed: Vec<String> = known
        .keys()
        .filter(|name| !current.contains_key(*name))
        .cloned()
        .collect();

    for name in removed {
        record_and_log(ServerActivityEvent::ContainerStateChanged {
            name,
            status: "stopped".to_string(),
        });
    }

    *known = current;
}

fn check_ssh_events(snap: &ServerSnapshot, known: &mut HashMap<String, Instant>) {
    let current: HashMap<String, String> = snap
        .ssh_sessions
        .iter()
        .map(|s| (format!("{}:{}", s.user, s.client_ip), s.client_ip.clone()))
        .collect();

    // Logins
    for (sess_key, client_ip) in &current {
        if !known.contains_key(sess_key) {
            let user = sess_key.split(':').next().unwrap_or("unknown");
            record_and_log(ServerActivityEvent::SshLogin {
                user: user.to_string(),
                client_ip: client_ip.clone(),
            });
            known.insert(sess_key.clone(), Instant::now());
        }
    }

    // Logouts
    let logged_out: Vec<String> = known
        .keys()
        .filter(|k| !current.contains_key(*k))
        .cloned()
        .collect();

    for sess_key in logged_out {
        if let Some(start_time) = known.remove(&sess_key) {
            let duration_secs = start_time.elapsed().as_secs();
            let mut parts = sess_key.split(':');
            let user = parts.next().unwrap_or("unknown").to_string();
            let client_ip = parts.next().unwrap_or("unknown").to_string();
            record_and_log(ServerActivityEvent::SshLogout {
                user,
                client_ip,
                duration_secs,
            });
        }
    }
}

fn check_torrent_events(snap: &ServerSnapshot, known: &mut HashMap<String, f64>) {
    for t in &snap.torrents {
        let prev_prog = known.get(&t.name).copied().unwrap_or(0.0);
        let state_lower = t.state.to_lowercase();
        let is_complete =
            t.progress >= 100.0 || state_lower.contains("seed") || state_lower.contains("upload");

        if prev_prog < 100.0 && is_complete {
            record_and_log(ServerActivityEvent::TorrentCompleted {
                name: t.name.clone(),
                size_bytes: t.size,
            });
        }
        known.insert(t.name.clone(), t.progress);
    }
}

fn check_resource_spikes(snap: &ServerSnapshot) {
    if snap.system.cpu_percent > 85.0 {
        let offender = snap
            .top_processes
            .iter()
            .max_by(|a, b| a.cpu_percent.total_cmp(&b.cpu_percent))
            .map(|p| format!("{} (PID {}) at {:.1}% CPU", p.name, p.pid, p.cpu_percent));

        record_and_log(ServerActivityEvent::ResourceSpike {
            metric: "CPU Usage".to_string(),
            value: snap.system.cpu_percent,
            threshold: 85.0,
            offender,
        });
    }

    if snap.system.mem_percent() > 90.0 {
        let offender = snap
            .top_processes
            .iter()
            .max_by(|a, b| a.mem_percent.total_cmp(&b.mem_percent))
            .map(|p| format!("{} (PID {}) at {:.1}% MEM", p.name, p.pid, p.mem_percent));

        record_and_log(ServerActivityEvent::ResourceSpike {
            metric: "RAM Usage".to_string(),
            value: snap.system.mem_percent(),
            threshold: 90.0,
            offender,
        });
    }

    if let Some(t) = snap.system.cpu_temp_c {
        if t >= 85.0 {
            record_and_log(ServerActivityEvent::ThermalAlert {
                temp_c: t,
                threshold_c: 85.0,
            });
        }
    }

    if snap.system.disk_percent() > 88.0 {
        record_and_log(ServerActivityEvent::DiskMilestone {
            path: "/".to_string(),
            percent: snap.system.disk_percent(),
            threshold: 88.0,
        });
    }
}

fn emit_heartbeat(snap: &ServerSnapshot, bat: &BatterySnapshot) {
    let temp_str = snap
        .system
        .cpu_temp_c
        .map_or_else(|| "N/A".to_string(), |t| format!("{t:.1}°C"));
    let summary = format!(
        "Heartbeat: CPU {:.1}% ({temp_str}) • RAM {:.1}% ({:.1}G) • Disk {:.1}% • Bat {}% ({}) • {} containers",
        snap.system.cpu_percent,
        snap.system.mem_percent(),
        snap.system.mem_used_bytes as f64 / 1_073_741_824.0,
        snap.system.disk_percent(),
        bat.capacity,
        bat.state.as_str(),
        snap.containers.len(),
    );
    record_and_log(ServerActivityEvent::SystemHeartbeat { summary });
}

fn record_and_log(event: ServerActivityEvent) {
    let record = EventRecord::new(event);
    let time = record.timestamp.format("%Y-%m-%d %H:%M:%S");
    let summary = record.event.summary();
    println!("⏱ [{time}] {summary}");
    let _ = append_event(&record);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplify_container_status() {
        assert_eq!(simplify_container_status("Up 6 weeks"), "running");
        assert_eq!(
            simplify_container_status("Up 14 minutes (healthy)"),
            "running"
        );
        assert_eq!(
            simplify_container_status("Exited (0) 2 minutes ago"),
            "exited"
        );
        assert_eq!(simplify_container_status("Paused"), "paused");
        assert_eq!(
            simplify_container_status("Restarting (1) 5s ago"),
            "restarting"
        );
        assert_eq!(simplify_container_status("unknown status"), "stopped");
    }
}
