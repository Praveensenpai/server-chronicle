use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
    Frame,
};

use super::render_compact_gauge;
use super::services::render_containers_and_ssh;
use crate::domain::battery_types::BatterySnapshot;
use crate::domain::models::{ProcessSortMode, ServerSnapshot};
use crate::tui::theme::{
    COLOR_BORDER, COLOR_DANGER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS,
    COLOR_WARNING,
};

pub fn render_telemetry(
    frame: &mut Frame,
    area: Rect,
    snapshot: &ServerSnapshot,
    battery: &BatterySnapshot,
    selected_proc_idx: usize,
    sort_mode: ProcessSortMode,
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
    render_processes(frame, chunks[2], snapshot, selected_proc_idx, sort_mode);
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

    let temp_color = sys.cpu_temp_c.map_or(COLOR_MUTED, |t| {
        if t >= 90.0 {
            COLOR_DANGER
        } else if t >= 70.0 {
            COLOR_WARNING
        } else {
            COLOR_SUCCESS
        }
    });

    let temp_str = sys
        .cpu_temp_c
        .map_or_else(|| "N/A".to_string(), |t| format!("{t:.1}°C"));
    let fan_str = sys.fan_display();
    let cores_str = if sys.cpu_core_temps_c.is_empty() {
        String::new()
    } else {
        sys.cpu_core_temps_c
            .iter()
            .map(|t| format!("{t:.0}°"))
            .collect::<Vec<_>>()
            .join("  ")
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title(" ⚡ CPU Load ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }

    // Row 0: bar
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

    // Row 2+: details (skip row 1 as spacer, matching compact gauge)
    if inner.height > 2 {
        let mut lines: Vec<Line> = vec![Line::from(vec![
            Span::styled(" 🌡 ", Style::default().fg(COLOR_MUTED)),
            Span::styled(
                temp_str,
                Style::default().fg(temp_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   🌀 ", Style::default().fg(COLOR_MUTED)),
            Span::styled(fan_str, Style::default().fg(COLOR_MUTED)),
        ])];
        if !cores_str.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(" cores: ", Style::default().fg(COLOR_MUTED)),
                Span::styled(cores_str, Style::default().fg(COLOR_MUTED)),
            ]));
        }
        let avail = inner.height.saturating_sub(2);
        let details_area = Rect::new(
            inner.x,
            inner.y + 2,
            inner.width,
            (lines.len() as u16).min(avail),
        );
        frame.render_widget(Paragraph::new(lines), details_area);
    }
}

fn render_processes(
    frame: &mut Frame,
    area: Rect,
    snapshot: &ServerSnapshot,
    selected: usize,
    sort_mode: ProcessSortMode,
) {
    let total_procs = snapshot.top_processes.len();
    let visible_capacity = (area.height.saturating_sub(3) as usize).max(1);
    let offset = if selected >= visible_capacity {
        selected - visible_capacity + 1
    } else {
        0
    };

    let rows: Vec<Row> = snapshot
        .top_processes
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_capacity)
        .map(|(i, p)| {
            let is_sel = i == selected;
            let cursor = if is_sel { "▶ " } else { "  " };
            let style = if is_sel {
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default()
            };
            Row::new(vec![
                format!("{cursor}{}", p.pid),
                p.name.clone(),
                format!("{:.1}%", p.cpu_percent),
                p.mem_summary(),
            ])
            .style(style)
        })
        .collect();

    let sort_label = sort_mode.label();
    let pos_hint = if total_procs > 0 {
        format!(" ({}/{})", selected + 1, total_procs)
    } else {
        String::new()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),     // PID
            Constraint::Percentage(50), // Process Name
            Constraint::Length(10),     // CPU %
            Constraint::Length(18),     // Memory (RAM)
        ],
    )
    .header(
        Row::new(vec!["  PID", "Process Name", "CPU %", "Memory (RAM)"]).style(
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(format!(
                " ⚙️ Top Resource Processes [Sort: {sort_label} ▼]{pos_hint} (Press 's' to sort, 'K' to kill) "
            )),
    );

    frame.render_widget(table, area);
}
