pub mod battery_view;
pub mod chronicle;
pub mod telemetry;

use ratatui::{
    layout::Rect,
    style::{Color, Style},
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
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.is_empty() {
        return;
    }

    let gauge_area = Rect::new(
        inner.x,
        inner.y + inner.height.saturating_sub(1) / 2,
        inner.width,
        1,
    );
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color))
        .percent(percent)
        .label(label);
    frame.render_widget(gauge, gauge_area);
}
