use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
    Frame,
};

use crate::domain::battery_types::{BatterySnapshot, PowerState};
use crate::tui::theme::{
    COLOR_DANGER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS, COLOR_TEXT,
    COLOR_WARNING,
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
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" 🔋 Battery Capacity • {ac_badge} ")),
        )
        .gauge_style(Style::default().fg(bat_color))
        .percent(b.capacity as u16)
        .label(format!("{}% — {}", b.capacity, b.state.as_str()));
    frame.render_widget(gauge, cols[0]);

    // Health Card
    let health_color = if b.health_percent > 80.0 {
        COLOR_SUCCESS
    } else {
        COLOR_WARNING
    };
    let health_text = vec![
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

    let p = Paragraph::new(health_text).block(
        Block::default()
            .borders(Borders::ALL)
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

    let speed_label = if b.calculated_rate_pct_hr > 0.05 {
        format!("+{:.2}% / hour", b.calculated_rate_pct_hr)
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
    .block(Block::default().borders(Borders::ALL).title(time_title));
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
            .title(" 🔄 Full 0% ➔ 100% Cycle Speed "),
    );
    frame.render_widget(p3, cols[2]);
}

fn render_brackets_table(frame: &mut Frame, area: Rect, b: &BatterySnapshot) {
    let rows: Vec<Row> = b
        .brackets
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
                format!("{:.1}% / hr", br.rate_pct_per_hour)
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
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .header(
        Row::new(vec![
            "Charge Bracket",
            "Time Taken",
            "Average Speed",
            "Status",
        ])
        .style(
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 📈 Bracket Breakdown (0-10%, 10-20% ... 90-100%) "),
    );

    frame.render_widget(table, area);
}
