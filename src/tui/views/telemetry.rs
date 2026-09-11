use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
    Frame,
};

use super::render_compact_gauge;
use crate::domain::battery_types::BatterySnapshot;
use crate::domain::models::ServerSnapshot;
use crate::tui::theme::{
    COLOR_DANGER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS, COLOR_WARNING,
};

pub fn render_telemetry(
    frame: &mut Frame,
    area: Rect,
    snapshot: &ServerSnapshot,
    battery: &BatterySnapshot,
    selected_proc_idx: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // Gauges
            Constraint::Length(10), // Containers & SSH
            Constraint::Min(8),     // Top Processes
        ])
        .split(area);

    render_gauges(frame, chunks[0], snapshot, battery);
    render_containers_and_ssh(frame, chunks[1], snapshot);
    render_processes(frame, chunks[2], snapshot, selected_proc_idx);
}

fn render_gauges(
    frame: &mut Frame,
    area: Rect,
    snapshot: &ServerSnapshot,
    battery: &BatterySnapshot,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let sys = &snapshot.system;

    render_cpu_gauge(frame, cols[0], sys);

    // RAM Gauge
    let ram_pct = sys.mem_percent();
    render_compact_gauge(
        frame,
        cols[1],
        " 🧠 Memory (RAM) ".to_string(),
        if ram_pct > 85.0 {
            COLOR_DANGER
        } else {
            COLOR_SECONDARY
        },
        ram_pct.min(100.0) as u16,
        format!(
            "{:.1}% ({:.1}G)",
            ram_pct,
            sys.mem_used_bytes as f64 / 1_073_741_824.0
        ),
    );

    // Root Disk Gauge
    let disk_pct = sys.disk_percent();
    render_compact_gauge(
        frame,
        cols[2],
        " 💾 Root Disk (/) ".to_string(),
        if disk_pct > 85.0 {
            COLOR_DANGER
        } else {
            COLOR_SUCCESS
        },
        disk_pct.min(100.0) as u16,
        format!(
            "{:.1}% ({:.0}G)",
            disk_pct,
            sys.disk_used_bytes as f64 / 1_000_000_000.0
        ),
    );

    // Battery mini Gauge
    let bat_color = if battery.capacity > 50 {
        COLOR_SUCCESS
    } else if battery.capacity > 20 {
        COLOR_WARNING
    } else {
        COLOR_DANGER
    };
    let ac_label = if battery.ac_online {
        "⚡ AC"
    } else {
        "🔋 UPS"
    };
    render_compact_gauge(
        frame,
        cols[3],
        format!(" 🔋 Battery ({ac_label}) "),
        bat_color,
        battery.capacity as u16,
        format!("{}% ({})", battery.capacity, battery.state.as_str()),
    );
}

fn render_cpu_gauge(frame: &mut Frame, area: Rect, sys: &crate::domain::models::SystemMetrics) {
    let color = if sys.cpu_percent > 85.0 {
        COLOR_DANGER
    } else {
        COLOR_PRIMARY
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚡ CPU Load ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }

    let gauge_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color))
        .percent(sys.cpu_percent.min(100.0) as u16)
        .label(Span::styled(
            format!("{:.1}%", sys.cpu_percent),
            Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, gauge_area);

    let temperature = sys
        .cpu_temp_c
        .map_or_else(|| "--".to_string(), |value| format!("{value:.1}°C"));
    let fan_speed = sys
        .fan_speed_rpm
        .map_or_else(|| "--".to_string(), |value| format!("{value} RPM"));
    let details = Paragraph::new(vec![
        Line::from(format!("🌡 Temp: {temperature}")),
        Line::from(format!("🌀 Fan:  {fan_speed}")),
    ])
    .style(Style::default().fg(COLOR_MUTED));
    let details_area = Rect::new(inner.x, inner.y.saturating_add(2), inner.width, 2);
    frame.render_widget(details, details_area);
}

fn render_containers_and_ssh(frame: &mut Frame, area: Rect, snapshot: &ServerSnapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Docker Containers Table
    let rows: Vec<Row> = snapshot
        .containers
        .iter()
        .map(|c| {
            Row::new(vec![
                c.name.clone(),
                c.status.clone(),
                format!("{:.1}%", c.cpu_percent),
                c.memory_usage.clone(),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
        ],
    )
    .header(
        Row::new(vec!["Container", "Status", "CPU", "Memory"]).style(
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 🐳 Docker Containers "),
    );

    frame.render_widget(table, cols[0]);

    // SSH Sessions List
    let mut ssh_lines = Vec::new();
    if snapshot.ssh_sessions.is_empty() {
        ssh_lines.push(ratatui::text::Line::from(
            "  No remote SSH sessions connected.",
        ));
    } else {
        for s in &snapshot.ssh_sessions {
            ssh_lines.push(ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(
                    format!("  👤 {} ", s.user),
                    Style::default()
                        .fg(COLOR_SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ),
                ratatui::text::Span::styled(
                    format!("from {} ", s.client_ip),
                    Style::default().fg(COLOR_PRIMARY),
                ),
                ratatui::text::Span::styled(
                    format!("({})", s.tty_or_port),
                    Style::default().fg(COLOR_MUTED),
                ),
            ]));
        }
    }

    let p = Paragraph::new(ssh_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 👤 Active SSH Sessions "),
    );
    frame.render_widget(p, cols[1]);
}

fn render_processes(frame: &mut Frame, area: Rect, snapshot: &ServerSnapshot, selected: usize) {
    let rows: Vec<Row> = snapshot
        .top_processes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == selected {
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Row::new(vec![
                p.pid.to_string(),
                p.name.clone(),
                format!("{:.1}%", p.cpu_percent),
                format!("{:.1}%", p.mem_percent),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Percentage(50),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ],
    )
    .header(
        Row::new(vec!["PID", "Process Name", "CPU %", "MEM %"]).style(
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" ⚙️ Top Resource Processes (Press 'K' to kill) "),
    );

    frame.render_widget(table, area);
}
