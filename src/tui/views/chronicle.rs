use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::domain::models::{EventRecord, ServerActivityEvent};
use crate::tui::theme::{
    COLOR_DANGER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS, COLOR_WARNING,
};

pub fn render_chronicle(
    frame: &mut Frame,
    area: Rect,
    events: &[EventRecord],
    search_query: &str,
    is_searching: bool,
    scroll_offset: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);

    render_search_bar(frame, chunks[0], search_query, is_searching);
    render_events_list(frame, chunks[1], events, search_query, scroll_offset);
}

fn render_search_bar(frame: &mut Frame, area: Rect, query: &str, is_searching: bool) {
    let border_style = if is_searching {
        Style::default().fg(COLOR_PRIMARY)
    } else {
        Style::default().fg(COLOR_MUTED)
    };

    let title = if is_searching {
        " 🔍 Search Filter (Press Enter or Esc to finish) "
    } else {
        " 🔍 Filter Timeline (Press '/' to search) "
    };

    let p = Paragraph::new(format!(" {query}"))
        .style(Style::default().fg(if is_searching {
            COLOR_PRIMARY
        } else {
            COLOR_MUTED
        }))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        );
    frame.render_widget(p, area);
}

fn render_events_list(
    frame: &mut Frame,
    area: Rect,
    events: &[EventRecord],
    query: &str,
    scroll: usize,
) {
    let filtered: Vec<&EventRecord> = events
        .iter()
        .filter(|e| {
            if query.is_empty() {
                return true;
            }
            let desc = format_event_text(&e.event);
            desc.to_lowercase().contains(&query.to_lowercase())
        })
        .collect();

    let visible_count = area.height.saturating_sub(2) as usize;
    let items: Vec<ListItem> = filtered
        .iter()
        .rev()
        .skip(scroll)
        .take(visible_count)
        .map(|e| {
            let time = e.timestamp.format("%H:%M:%S").to_string();
            let (badge, badge_color) = get_event_badge(&e.event);
            let desc = format_event_text(&e.event);

            let line = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(
                    format!(" [{time}] "),
                    Style::default().fg(COLOR_MUTED),
                ),
                ratatui::text::Span::styled(
                    format!("{badge} "),
                    Style::default()
                        .fg(badge_color)
                        .add_modifier(Modifier::BOLD),
                ),
                ratatui::text::Span::raw(desc),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
        " 📜 Daily Timeline ({}/{}) ",
        filtered.len(),
        events.len()
    )));

    frame.render_widget(list, area);
}

fn get_event_badge(e: &ServerActivityEvent) -> (&'static str, ratatui::style::Color) {
    match e {
        ServerActivityEvent::BatteryStateChanged { .. } => ("[⚡ POWER]", COLOR_PRIMARY),
        ServerActivityEvent::BatteryBracketCompleted { .. } => ("[⏱ BRACKET]", COLOR_SUCCESS),
        ServerActivityEvent::PowerOutageAlert { .. } => ("[🚨 OUTAGE]", COLOR_DANGER),
        ServerActivityEvent::PowerRestoredAlert { .. } => ("[✨ RESTORED]", COLOR_SUCCESS),
        ServerActivityEvent::SshLogin { .. } => ("[👤 LOGIN]", COLOR_PRIMARY),
        ServerActivityEvent::SshLogout { .. } => ("[👤 LOGOUT]", COLOR_MUTED),
        ServerActivityEvent::ContainerStateChanged { .. } => ("[🐳 DOCKER]", COLOR_SECONDARY),
        ServerActivityEvent::TorrentCompleted { .. } => ("[📥 TORRENT]", COLOR_SUCCESS),
        ServerActivityEvent::ResourceSpike { .. } => ("[⚠️ SPIKE]", COLOR_WARNING),
        ServerActivityEvent::GenericNote { .. } => ("[ℹ️ NOTE]", COLOR_MUTED),
    }
}

fn format_event_text(e: &ServerActivityEvent) -> String {
    match e {
        ServerActivityEvent::BatteryStateChanged {
            from_status,
            to_status,
            capacity,
        } => {
            format!("{from_status} ➔ {to_status} ({capacity}%)")
        }
        ServerActivityEvent::BatteryBracketCompleted {
            bracket,
            duration_secs,
            rate_pct_per_hour,
        } => {
            format!(
                "{bracket} in {}m {}s ({rate_pct_per_hour:.1}%/hr)",
                duration_secs / 60,
                duration_secs % 60
            )
        }
        ServerActivityEvent::PowerOutageAlert {
            capacity,
            estimated_runtime_mins,
        } => {
            format!("AC Lost! Battery at {capacity}%, ~{estimated_runtime_mins}m left")
        }
        ServerActivityEvent::PowerRestoredAlert { capacity } => {
            format!("AC Power Restored! Capacity: {capacity}%")
        }
        ServerActivityEvent::SshLogin { user, client_ip } => {
            format!("SSH login from {user}@{client_ip}")
        }
        ServerActivityEvent::SshLogout {
            user,
            client_ip,
            duration_secs,
        } => {
            format!(
                "SSH closed: {user}@{client_ip} (active {}m)",
                duration_secs / 60
            )
        }
        ServerActivityEvent::ContainerStateChanged { name, status } => {
            format!("Container {name}: {status}")
        }
        ServerActivityEvent::TorrentCompleted { name, size_bytes } => {
            format!(
                "Downloaded {name} ({:.1} GB)",
                *size_bytes as f64 / 1_000_000_000.0
            )
        }
        ServerActivityEvent::ResourceSpike {
            metric,
            value,
            threshold,
        } => {
            format!("{metric} spike: {value:.1}% > {threshold:.1}%")
        }
        ServerActivityEvent::GenericNote { message } => message.clone(),
    }
}
