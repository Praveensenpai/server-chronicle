use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
    Frame,
};

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

    // CPU Gauge
    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ⚡ CPU Load "),
        )
        .gauge_style(Style::default().fg(if sys.cpu_percent > 85.0 {
            COLOR_DANGER
        } else {
            COLOR_PRIMARY
        }))
        .percent(sys.cpu_percent.min(100.0) as u16)
        .label(format!("{:.1}%", sys.cpu_percent));
    frame.render_widget(cpu_gauge, cols[0]);

    // RAM Gauge
    let ram_pct = sys.mem_percent();
    let ram_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 🧠 Memory (RAM) "),
        )
        .gauge_style(Style::default().fg(if ram_pct > 85.0 {
            COLOR_DANGER
        } else {
            COLOR_SECONDARY
        }))
        .percent(ram_pct.min(100.0) as u16)
        .label(format!(
            "{:.1}% ({:.1}G)",
            ram_pct,
            sys.mem_used_bytes as f64 / 1_073_741_824.0
        ));
    frame.render_widget(ram_gauge, cols[1]);

    // Root Disk Gauge
    let disk_pct = sys.disk_percent();
    let disk_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 💾 Root Disk (/) "),
        )
        .gauge_style(Style::default().fg(if disk_pct > 85.0 {
            COLOR_DANGER
        } else {
            COLOR_SUCCESS
        }))
        .percent(disk_pct.min(100.0) as u16)
        .label(format!(
            "{:.1}% ({:.0}G)",
            disk_pct,
            sys.disk_used_bytes as f64 / 1_000_000_000.0
        ));
    frame.render_widget(disk_gauge, cols[2]);

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
    let bat_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" 🔋 Battery ({ac_label}) ")),
        )
        .gauge_style(Style::default().fg(bat_color))
        .percent(battery.capacity as u16)
        .label(format!(
            "{}% ({})",
            battery.capacity,
            battery.state.as_str()
        ));
    frame.render_widget(bat_gauge, cols[3]);
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
