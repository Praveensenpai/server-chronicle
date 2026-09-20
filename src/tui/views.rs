pub mod battery_view;
pub mod chronicle;
pub mod services;
pub mod telemetry;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Gauge},
    Frame,
};

pub use battery_view::render_battery_view;
pub use chronicle::render_chronicle;
pub use telemetry::render_telemetry;

pub fn render_compact_gauge(
    frame: &mut Frame,
    area: Rect,
    title: String,
    color: Color,
    percent: u16,
    label: String,
) {
    use crate::tui::theme::COLOR_BORDER;
    use ratatui::{text::Line, widgets::Paragraph};

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.is_empty() {
        return;
    }

    // Bar always at the top row of the inner area
    let gauge_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color))
        .percent(percent)
        .label(Span::styled(
            label.clone(),
            Style::default()
                .fg(Color::Black)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, gauge_area);

    // Percentage text on the line below the bar
    if inner.height > 2 {
        let text_area = Rect::new(inner.x, inner.y + 2, inner.width, 1);
        let p = Paragraph::new(Line::from(Span::styled(
            format!(" {label}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(p, text_area);
    }
}

pub fn render_kill_modal(frame: &mut Frame, area: Rect, app: &crate::tui::app::App) {
    use crate::tui::theme::{COLOR_BORDER, COLOR_DANGER, COLOR_MUTED, COLOR_WARNING};
    use ratatui::{
        layout::Alignment,
        text::Line,
        widgets::{Clear, Paragraph},
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title(" ⚠️ Terminate Process Confirmation ");

    let proc_name = app
        .snapshot
        .top_processes
        .get(app.selected_proc_idx)
        .map_or("Unknown", |p| p.name.as_str());
    let pid = app
        .snapshot
        .top_processes
        .get(app.selected_proc_idx)
        .map_or(0, |p| p.pid);

    let modal_area = centered_rect(50, 20, area);
    frame.render_widget(Clear, modal_area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Terminate "),
            Span::styled(
                format!("{proc_name} (PID {pid})"),
                Style::default()
                    .fg(COLOR_WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" with SIGTERM?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [y] Confirm ",
                Style::default()
                    .fg(COLOR_DANGER)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" [n / Esc] Cancel ", Style::default().fg(COLOR_MUTED)),
        ]),
    ];

    let p = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(block);
    frame.render_widget(p, modal_area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn render_header(frame: &mut Frame, area: Rect, app: &crate::tui::app::App) {
    use crate::tui::app::ActiveTab;
    use crate::tui::theme::{COLOR_BORDER, COLOR_MUTED, COLOR_PRIMARY};
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        text::Line,
        widgets::{Paragraph, Tabs},
    };

    let has_room = area.width >= 90;
    let constraints = if has_room {
        vec![
            Constraint::Length(22),
            Constraint::Min(38),
            Constraint::Length(26),
        ]
    } else {
        vec![Constraint::Length(22), Constraint::Min(20)]
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let title_line = Line::from(vec![Span::styled(
        " ⏱ CHRONICLE ",
        Style::default()
            .fg(COLOR_PRIMARY)
            .add_modifier(Modifier::BOLD),
    )]);
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER));
    frame.render_widget(Paragraph::new(title_line).block(title_block), cols[0]);

    let tab_titles = vec![
        " [1] 🖥️ Telemetry ",
        " [2] 🔋 Battery UPS ",
        " [3] 📜 Chronicle ",
    ];
    let selected_idx = match app.active_tab {
        ActiveTab::Telemetry => 0,
        ActiveTab::Battery => 1,
        ActiveTab::Chronicle => 2,
    };

    let tabs = Tabs::new(tab_titles)
        .select(selected_idx)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
        .style(Style::default().fg(COLOR_MUTED))
        .highlight_style(
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, cols[1]);

    if has_room && cols.len() > 2 {
        render_host_badge(frame, cols[2], app);
    }
}

fn render_host_badge(frame: &mut Frame, area: Rect, app: &crate::tui::app::App) {
    use crate::tui::theme::{COLOR_BORDER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY};
    use ratatui::{text::Line, widgets::Paragraph};

    let uptime_h = app.snapshot.system.uptime_secs / 3600;
    let uptime_d = uptime_h / 24;
    let host_line = Line::from(vec![
        Span::styled("🌐 ", Style::default().fg(COLOR_PRIMARY)),
        Span::styled(
            &app.snapshot.system.hostname,
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" • ⬆️ {}d {}h", uptime_d, uptime_h % 24),
            Style::default().fg(COLOR_MUTED),
        ),
    ]);
    let host_p = Paragraph::new(host_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );
    frame.render_widget(host_p, area);
}

pub fn render_footer(frame: &mut Frame, area: Rect, app: &crate::tui::app::App) {
    use crate::tui::theme::{
        COLOR_DANGER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS, COLOR_WARNING,
    };
    use ratatui::{text::Line, widgets::Paragraph};

    if let Some((msg, _)) = &app.status_message {
        let p = Paragraph::new(format!("  {msg}")).style(
            Style::default()
                .fg(COLOR_SUCCESS)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(p, area);
        return;
    }

    let sort_hint = format!(" Sort:{} ", app.proc_sort_mode.label());
    let keys = vec![
        Span::styled(
            " [Tab/1-3]",
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Views ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            "[s]",
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(sort_hint, Style::default().fg(COLOR_MUTED)),
        Span::styled(
            "[e]",
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" AI Export ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            "[K]",
            Style::default()
                .fg(COLOR_DANGER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Kill Proc ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            "[/]",
            Style::default()
                .fg(COLOR_WARNING)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Filter ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            "[q]",
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Exit", Style::default().fg(COLOR_MUTED)),
    ];
    frame.render_widget(Paragraph::new(Line::from(keys)), area);
}
