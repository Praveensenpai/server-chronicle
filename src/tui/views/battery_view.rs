use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::render_compact_gauge;
use crate::domain::battery_types::{BatterySnapshot, PowerState};
use crate::tui::theme::{
    COLOR_BORDER, COLOR_DANGER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS,
    COLOR_TEXT, COLOR_WARNING,
};

pub fn render_battery_view(frame: &mut Frame, area: Rect, battery: &BatterySnapshot) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Battery gauge & health card
            Constraint::Length(6), // Live rate & UPS estimate stats
            Constraint::Min(8),    // Detailed electrical & operational diagnostics
        ])
        .split(area);

    render_header_card(frame, chunks[0], battery);
    render_speed_stats(frame, chunks[1], battery);
    render_power_diagnostics(frame, chunks[2], battery);
}

fn render_header_card(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_battery_gauge(frame, cols[0], b);
    render_health_card(frame, cols[1], b);
}

fn render_battery_gauge(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let bat_color = if b.capacity > 50 {
        COLOR_SUCCESS
    } else if b.capacity > 20 {
        COLOR_WARNING
    } else {
        COLOR_DANGER
    };

    let ac_badge = if b.ac_online {
        "⚡ AC Connected (Mains)"
    } else {
        "🚨 ON BATTERY (UPS MODE)"
    };

    render_compact_gauge(
        frame,
        area,
        format!(" 🔋 Battery Capacity • {ac_badge} "),
        bat_color,
        b.capacity as u16,
        format!("{}% — {}", b.capacity, b.state.as_str()),
    );
}

fn render_health_card(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let health_color = if b.health_percent > 80.0 {
        COLOR_SUCCESS
    } else {
        COLOR_WARNING
    };

    let mut health_text = vec![
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Battery Health: "),
            ratatui::text::Span::styled(
                format!("{:.1}%", b.health_percent),
                Style::default()
                    .fg(health_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Full Capacity:  "),
            ratatui::text::Span::styled(
                format!("{} mAh", b.charge_full_uah / 1000),
                Style::default().fg(COLOR_TEXT),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Design Spec:    "),
            ratatui::text::Span::styled(
                format!("{} mAh", b.charge_full_design_uah / 1000),
                Style::default().fg(COLOR_MUTED),
            ),
        ]),
    ];

    if let Some(uw) = b.power_now_uw {
        health_text.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Power Draw:     "),
            ratatui::text::Span::styled(
                format!("{:.2} W", uw as f64 / 1_000_000.0),
                Style::default().fg(COLOR_PRIMARY),
            ),
        ]));
    }

    let p = Paragraph::new(health_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(" 🩺 Hardware Health & Wear "),
    );
    frame.render_widget(p, area);
}

fn render_speed_stats(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    render_rate_card(frame, cols[0], b);
    render_estimate_card(frame, cols[1], b);
    render_cycle_card(frame, cols[2], b);
}

fn render_rate_card(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let sign = if b.state == PowerState::Discharging {
        "-"
    } else {
        "+"
    };
    let speed_label = if b.calculated_rate_pct_hr > 0.05 {
        format!("{sign}{:.2}% / hour", b.calculated_rate_pct_hr)
    } else {
        "Stable / Balanced".to_string()
    };
    let pace_label = if b.mins_per_percent > 0.05 {
        format!("~{:.1} mins per 1%", b.mins_per_percent)
    } else {
        "--".to_string()
    };

    let p = Paragraph::new(vec![
        ratatui::text::Line::from(ratatui::text::Span::styled(
            speed_label,
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )),
        ratatui::text::Line::from(ratatui::text::Span::styled(
            pace_label,
            Style::default().fg(COLOR_MUTED),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(" ⚡ Calculated Rate (x% - y% / time) "),
    );
    frame.render_widget(p, area);
}

fn render_estimate_card(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let (time_title, time_val) = match b.state {
        PowerState::Charging => (
            " ⏱ Estimated Time to 100% ",
            b.estimated_minutes_left
                .map_or("Calculating...".to_string(), |m| format!("{m} minutes")),
        ),
        PowerState::Discharging => (
            " 🚨 Estimated UPS Runtime Left ",
            b.estimated_minutes_left
                .map_or("Calculating...".to_string(), |m| {
                    format!("{m} minutes (to 5%)")
                }),
        ),
        _ => (" ⏱ Status ", "Connected to Mains (Full)".to_string()),
    };

    let p = Paragraph::new(ratatui::text::Line::from(ratatui::text::Span::styled(
        time_val,
        Style::default()
            .fg(COLOR_WARNING)
            .add_modifier(Modifier::BOLD),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(time_title),
    );
    frame.render_widget(p, area);
}

fn render_cycle_card(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let full_est = if b.calculated_rate_pct_hr > 0.1 {
        let total_mins = ((100.0 / b.calculated_rate_pct_hr) * 60.0) as u64;
        format!(
            "{}h {}m (estimated full cycle)",
            total_mins / 60,
            total_mins % 60
        )
    } else {
        "--".to_string()
    };

    let p = Paragraph::new(ratatui::text::Line::from(ratatui::text::Span::styled(
        full_est,
        Style::default()
            .fg(COLOR_SECONDARY)
            .add_modifier(Modifier::BOLD),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(" 🔄 Full 0% ➔ 100% Cycle Speed "),
    );
    frame.render_widget(p, area);
}

fn render_power_diagnostics(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_electrical_specs_card(frame, cols[0], b);
    render_operational_diagnostics_card(frame, cols[1], b);
}

fn render_electrical_specs_card(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let volt_str = if b.voltage_now_uv > 0 {
        format!("{:.2} V", b.voltage_now_uv as f64 / 1_000_000.0)
    } else {
        "--".to_string()
    };

    let power_str = if let Some(uw) = b.power_now_uw {
        format!("{:.2} W", uw as f64 / 1_000_000.0)
    } else {
        "--".to_string()
    };

    let remaining_uah = b.charge_full_uah.saturating_sub(b.charge_now_uah);

    let electrical_lines = vec![
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Current Voltage:     "),
            ratatui::text::Span::styled(
                volt_str,
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Live Power Flow:     "),
            ratatui::text::Span::styled(
                power_str,
                Style::default()
                    .fg(COLOR_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Current Energy:      "),
            ratatui::text::Span::styled(
                format!("{} mAh", b.charge_now_uah / 1000),
                Style::default().fg(COLOR_TEXT),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Remaining to Full:   "),
            ratatui::text::Span::styled(
                format!("{} mAh", remaining_uah / 1000),
                Style::default().fg(COLOR_MUTED),
            ),
        ]),
    ];

    let p = Paragraph::new(electrical_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(" ⚡ Electrical Specifications "),
    );
    frame.render_widget(p, area);
}

fn render_operational_diagnostics_card(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let wear_pct = (100.0 - b.health_percent).max(0.0);
    let health_grade = if b.health_percent >= 90.0 {
        ("Optimal / Excellent", COLOR_SUCCESS)
    } else if b.health_percent >= 75.0 {
        ("Normal / Good", COLOR_SECONDARY)
    } else if b.health_percent >= 50.0 {
        ("Degraded / Fair", COLOR_WARNING)
    } else {
        ("Critical / Service Required", COLOR_DANGER)
    };

    let (source_text, source_color) = if b.ac_online {
        ("AC Mains (Connected)", COLOR_SUCCESS)
    } else {
        ("Battery (UPS Operating)", COLOR_WARNING)
    };

    let diagnostics_lines = vec![
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Power Source:        "),
            ratatui::text::Span::styled(
                source_text,
                Style::default()
                    .fg(source_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Condition Assessment:"),
            ratatui::text::Span::styled(
                health_grade.0,
                Style::default()
                    .fg(health_grade.1)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Wear Level:          "),
            ratatui::text::Span::styled(
                format!("{wear_pct:.1}% capacity lost"),
                Style::default().fg(if wear_pct > 20.0 {
                    COLOR_WARNING
                } else {
                    COLOR_MUTED
                }),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Operating State:     "),
            ratatui::text::Span::styled(b.state.as_str(), Style::default().fg(COLOR_TEXT)),
        ]),
    ];

    let p = Paragraph::new(diagnostics_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(" 🩺 Operational Diagnostics "),
    );
    frame.render_widget(p, area);
}
