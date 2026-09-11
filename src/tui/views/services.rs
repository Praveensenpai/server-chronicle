use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::domain::models::ServerSnapshot;
use crate::tui::theme::{
    COLOR_ACCENT, COLOR_BORDER, COLOR_DANGER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY,
    COLOR_SUCCESS, COLOR_WARNING,
};

pub fn render_containers_and_ssh(frame: &mut Frame, area: Rect, snapshot: &ServerSnapshot) {
    if !snapshot.torrents.is_empty() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(42),
                Constraint::Percentage(32),
                Constraint::Percentage(26),
            ])
            .split(area);

        render_docker_table(frame, cols[0], snapshot);
        render_torrents_table(frame, cols[1], snapshot);
        render_ssh_list(frame, cols[2], snapshot);
    } else {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        render_docker_table(frame, cols[0], snapshot);
        render_ssh_list(frame, cols[1], snapshot);
    }
}

pub fn render_docker_table(frame: &mut Frame, area: Rect, snapshot: &ServerSnapshot) {
    let rows: Vec<Row> = snapshot
        .containers
        .iter()
        .map(|c| {
            let status_lower = c.status.to_lowercase();
            let status_cell = if status_lower.contains("running") || status_lower.starts_with("up")
            {
                ratatui::text::Span::styled(
                    format!("● {}", c.status),
                    Style::default().fg(COLOR_SUCCESS),
                )
            } else if status_lower.contains("exited") || status_lower.contains("stopped") {
                ratatui::text::Span::styled(
                    format!("○ {}", c.status),
                    Style::default().fg(COLOR_DANGER),
                )
            } else if status_lower.contains("paused") {
                ratatui::text::Span::styled(
                    format!("◐ {}", c.status),
                    Style::default().fg(COLOR_WARNING),
                )
            } else {
                ratatui::text::Span::styled(
                    format!("▲ {}", c.status),
                    Style::default().fg(COLOR_ACCENT),
                )
            };

            Row::new(vec![
                Cell::from(c.name.clone()),
                Cell::from(status_cell),
                Cell::from(format!("{:.1}%", c.cpu_percent)),
                Cell::from(c.memory_usage.clone()),
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
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(" 🐳 Docker Containers "),
    );

    frame.render_widget(table, area);
}

pub fn render_torrents_table(frame: &mut Frame, area: Rect, snapshot: &ServerSnapshot) {
    let rows: Vec<Row> = snapshot
        .torrents
        .iter()
        .map(|t| {
            let rate = if t.dlspeed > 0 {
                format!("↓{:.1}M", t.dlspeed as f64 / 1_000_000.0)
            } else if t.upspeed > 0 {
                format!("↑{:.1}K", t.upspeed as f64 / 1_000.0)
            } else {
                t.state.clone()
            };
            Row::new(vec![t.name.clone(), format!("{:.0}%", t.progress), rate])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(55),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
        ],
    )
    .header(
        Row::new(vec!["Torrent", "Done", "Speed"]).style(
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(" 📥 Active Downloads "),
    );

    frame.render_widget(table, area);
}

pub fn render_ssh_list(frame: &mut Frame, area: Rect, snapshot: &ServerSnapshot) {
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
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(" 👤 Active SSH Sessions "),
    );
    frame.render_widget(p, area);
}
