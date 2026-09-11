use chrono::Utc;
use std::fmt::Write as _;

use crate::domain::battery_types::BatterySnapshot;
use crate::domain::models::{EventRecord, ServerActivityEvent, ServerSnapshot};

#[must_use]
pub fn generate_ai_report(
    snapshot: &ServerSnapshot,
    battery: &BatterySnapshot,
    events: &[EventRecord],
) -> String {
    let now = Utc::now();
    let date_str = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let mut doc = String::new();

    let _ = writeln!(doc, "# 🖥️ Ubuntu Server Chronicle — {date_str}\n");

    render_battery_section(&mut doc, battery);
    render_system_section(&mut doc, snapshot);
    render_docker_section(&mut doc, snapshot);
    render_ssh_section(&mut doc, snapshot);
    render_events_section(&mut doc, events);

    doc
}

fn render_battery_section(doc: &mut String, b: &BatterySnapshot) {
    let _ = writeln!(doc, "## ⚡ Power & Battery UPS Telemetry");
    let _ = writeln!(doc, "- **State**: {}", b.state.as_str());
    let _ = writeln!(
        doc,
        "- **AC Mains Connected**: {}",
        if b.ac_online {
            "Yes (Online)"
        } else {
            "No (Running on UPS Battery!)"
        }
    );
    let _ = writeln!(doc, "- **Capacity**: {}%", b.capacity);
    let _ = writeln!(
        doc,
        "- **Battery Health**: {:.1}% ({} / {} mAh)",
        b.health_percent,
        b.charge_full_uah / 1000,
        b.charge_full_design_uah / 1000
    );

    if b.calculated_rate_pct_hr > 0.05 {
        let _ = writeln!(
            doc,
            "- **Calculated Rate**: {:.2}% / hour (~{:.1} mins per 1%)",
            b.calculated_rate_pct_hr, b.mins_per_percent
        );
    } else {
        let _ = writeln!(doc, "- **Calculated Rate**: Stable / Idle");
    }

    if let Some(left) = b.estimated_minutes_left {
        let label = if b.ac_online {
            "Estimated Time to 100%"
        } else {
            "Estimated Runtime Left"
        };
        let _ = writeln!(doc, "- **{label}**: {left} mins");
    }

    let _ = writeln!(doc, "\n### ⏱️ Charging Brackets Breakdown");
    let _ = writeln!(doc, "| Bracket | Duration | Rate (%/hr) | Status |");
    let _ = writeln!(doc, "| :--- | :--- | :--- | :--- |");
    for br in &b.brackets {
        let mins = br.duration_secs / 60;
        let secs = br.duration_secs % 60;
        let status = if br.completed {
            "✔ Done"
        } else if br.duration_secs > 0 {
            "▶ In Progress"
        } else {
            "Pending"
        };
        let _ = writeln!(
            doc,
            "| `{}` | {}m {}s | {:.1}% | {status} |",
            br.label, mins, secs, br.rate_pct_per_hour
        );
    }
    let _ = writeln!(doc);
}

fn render_system_section(doc: &mut String, snap: &ServerSnapshot) {
    let sys = &snap.system;
    let _ = writeln!(doc, "## 📊 System Resources");
    let _ = writeln!(doc, "- **Hostname**: {}", sys.hostname);
    let _ = writeln!(doc, "- **CPU Usage**: {:.1}%", sys.cpu_percent);
    let _ = writeln!(
        doc,
        "- **Memory**: {:.1}% ({:.2} GiB / {:.2} GiB)",
        sys.mem_percent(),
        sys.mem_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
        sys.mem_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        doc,
        "- **Root Disk (/)**: {:.1}% ({:.1} GB / {:.1} GB)",
        sys.disk_percent(),
        sys.disk_used_bytes as f64 / (1000.0 * 1000.0 * 1000.0),
        sys.disk_total_bytes as f64 / (1000.0 * 1000.0 * 1000.0)
    );
    let fan_str = sys
        .fan_speed_rpm
        .map_or_else(|| "N/A".to_string(), |r| format!("{r} RPM"));
    let _ = writeln!(doc, "- **Fan Speed**: {fan_str}");
    let _ = writeln!(
        doc,
        "- **Load Average**: {:.2}, {:.2}, {:.2}",
        sys.load_avg.0, sys.load_avg.1, sys.load_avg.2
    );
    let _ = writeln!(
        doc,
        "- **Uptime**: {}d {}h {}m\n",
        sys.uptime_secs / 86400,
        (sys.uptime_secs % 86400) / 3600,
        (sys.uptime_secs % 3600) / 60
    );
}

fn render_docker_section(doc: &mut String, snap: &ServerSnapshot) {
    let _ = writeln!(doc, "## 🐳 Docker Containers");
    if snap.containers.is_empty() {
        let _ = writeln!(doc, "*No running containers found.*\n");
        return;
    }
    let _ = writeln!(doc, "| Name | Status | CPU% | Memory |");
    let _ = writeln!(doc, "| :--- | :--- | :--- | :--- |");
    for c in &snap.containers {
        let _ = writeln!(
            doc,
            "| **{}** | `{}` | {:.1}% | {} |",
            c.name, c.status, c.cpu_percent, c.memory_usage
        );
    }
    let _ = writeln!(doc);
}

fn render_ssh_section(doc: &mut String, snap: &ServerSnapshot) {
    let _ = writeln!(doc, "## 👤 Active SSH Sessions");
    if snap.ssh_sessions.is_empty() {
        let _ = writeln!(doc, "*No active remote SSH sessions.*\n");
        return;
    }
    for s in &snap.ssh_sessions {
        let _ = writeln!(
            doc,
            "- User: `{}` from `{}` ({})",
            s.user, s.client_ip, s.tty_or_port
        );
    }
    let _ = writeln!(doc);
}

fn render_events_section(doc: &mut String, events: &[EventRecord]) {
    let _ = writeln!(doc, "## 📜 Activity Log Timeline (Today)");
    if events.is_empty() {
        let _ = writeln!(doc, "*No logged events recorded today yet.*\n");
        return;
    }
    for e in events.iter().rev().take(30) {
        let time = e.timestamp.format("%H:%M:%S").to_string();
        let desc = format_event(&e.event);
        let _ = writeln!(doc, "- `[{time}]` {desc}");
    }
    let _ = writeln!(doc);
}

fn format_event(e: &ServerActivityEvent) -> String {
    match e {
        ServerActivityEvent::BatteryStateChanged {
            from_status,
            to_status,
            capacity,
        } => {
            format!("⚡ Power state changed: {from_status} ➔ {to_status} ({capacity}%)")
        }
        ServerActivityEvent::BatteryBracketCompleted {
            bracket,
            duration_secs,
            rate_pct_per_hour,
        } => {
            format!(
                "⏱ Bracket {bracket} finished in {}m {}s ({rate_pct_per_hour:.1}%/hr)",
                duration_secs / 60,
                duration_secs % 60
            )
        }
        ServerActivityEvent::PowerOutageAlert {
            capacity,
            estimated_runtime_mins,
        } => {
            format!("🚨 AC POWER LOST! Running on UPS ({capacity}%). Estimated runtime: {estimated_runtime_mins} mins")
        }
        ServerActivityEvent::PowerRestoredAlert { capacity } => {
            format!("✨ AC Power Restored! ({capacity}%)")
        }
        ServerActivityEvent::SshLogin { user, client_ip } => {
            format!("👤 SSH login: `{user}` from `{client_ip}`")
        }
        ServerActivityEvent::SshLogout {
            user,
            client_ip,
            duration_secs,
        } => {
            format!(
                "👤 SSH session closed: `{user}` from `{client_ip}` (duration: {}m)",
                duration_secs / 60
            )
        }
        ServerActivityEvent::ContainerStateChanged { name, status } => {
            format!("🐳 Container `{name}`: {status}")
        }
        ServerActivityEvent::TorrentCompleted { name, size_bytes } => {
            format!(
                "📥 Download completed: `{name}` ({:.2} GB)",
                *size_bytes as f64 / 1_000_000_000.0
            )
        }
        ServerActivityEvent::ResourceSpike {
            metric,
            value,
            threshold,
        } => {
            format!("⚠️ High resource load: {metric} at {value:.1}% (threshold {threshold:.1}%)")
        }
        ServerActivityEvent::GenericNote { message } => format!("ℹ️ {message}"),
    }
}
