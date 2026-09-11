pub mod battery_view;
pub mod chronicle;
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
    use ratatui::{text::Line, widgets::Paragraph};

    let block = Block::default().borders(Borders::ALL).title(title);
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
