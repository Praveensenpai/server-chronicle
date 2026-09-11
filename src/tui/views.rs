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
