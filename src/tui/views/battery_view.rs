use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table},
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
            Constraint::Length(7), // Big battery gauge & health card
            Constraint::Length(6), // Live calculated speed & UPS runtime stats
            Constraint::Min(10),   // 10-bracket table
        ])
        .split(area);

    render_header_card(frame, chunks[0], battery);
    render_speed_stats(frame, chunks[1], battery);
    render_brackets_table(frame, chunks[2], battery);
}

fn render_header_card(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

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
        cols[0],
        format!(" 🔋 Battery Capacity • {ac_badge} "),
        bat_color,
        b.capacity as u16,
        format!("{}% — {}", b.capacity, b.state.as_str()),
    );

    // Health Card
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
    frame.render_widget(p, cols[1]);
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

    let p1 = Paragraph::new(vec![
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
    frame.render_widget(p1, cols[0]);

    // Estimate Card
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

    let p2 = Paragraph::new(ratatui::text::Line::from(ratatui::text::Span::styled(
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
    frame.render_widget(p2, cols[1]);

    // Full 0% to 100% calculation
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

    let p3 = Paragraph::new(ratatui::text::Line::from(ratatui::text::Span::styled(
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
    frame.render_widget(p3, cols[2]);
}

fn render_brackets_table(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    if area.width >= 90 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        render_single_bracket_table(
            frame,
            cols[0],
            " 📈 Charging Brackets (0% ➔ 100%) ",
            "Charge",
            &b.brackets,
            COLOR_PRIMARY,
        );
        render_single_bracket_table(
            frame,
            cols[1],
            " 📉 Discharging / Drain (100% ➔ 0%) ",
            "Drain",
            &b.discharge_brackets,
            COLOR_SECONDARY,
        );
    } else if b.state == PowerState::Discharging {
        render_single_bracket_table(
            frame,
            area,
            " 📉 Discharging / Drain Brackets (100% ➔ 0%) ",
            "Drain",
            &b.discharge_brackets,
            COLOR_SECONDARY,
        );
    } else {
        render_single_bracket_table(
            frame,
            area,
            " 📈 Charging Brackets (0% ➔ 100%) ",
            "Charge",
            &b.brackets,
            COLOR_PRIMARY,
        );
    }
}

fn render_single_bracket_table(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    header_label: &str,
    brackets: &[crate::domain::battery_types::BracketStat],
    header_color: ratatui::style::Color,
) {
    let rows: Vec<Row> = brackets
        .iter()
        .map(|br| {
            let mins = br.duration_secs / 60;
            let secs = br.duration_secs % 60;
            let (status_text, style) = if br.completed {
                ("✔ Completed", Style::default().fg(COLOR_SUCCESS))
            } else if br.duration_secs > 0 {
                (
                    "▶ In Progress",
                    Style::default()
                        .fg(COLOR_WARNING)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("Pending", Style::default().fg(COLOR_MUTED))
            };

            let rate_str = if br.rate_pct_per_hour > 0.01 {
                format!("{:.1}%/hr", br.rate_pct_per_hour)
            } else {
                "--".to_string()
            };

            Row::new(vec![
                br.label.clone(),
                format!("{mins}m {secs}s"),
                rate_str,
                status_text.to_string(),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(28),
            Constraint::Percentage(24),
            Constraint::Percentage(24),
            Constraint::Percentage(24),
        ],
    )
    .header(
        Row::new(vec![header_label, "Time", "Speed", "Status"]).style(
            Style::default()
                .fg(header_color)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(title),
    );

    frame.render_widget(table, area);
}
