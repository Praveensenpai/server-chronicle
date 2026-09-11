pub mod app;
pub mod theme;
pub mod views;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io::stdout;
use std::time::{Duration, Instant};

use app::{ActiveTab, App};
use theme::{
    COLOR_BORDER, COLOR_DANGER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS,
    COLOR_WARNING,
};

pub fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = main_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut last_refresh = Instant::now();
    let refresh_interval = Duration::from_secs(2);

    loop {
        terminal.draw(|f| ui(f, app))?;
        app.check_status_expiration();

        let timeout = Duration::from_millis(200);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && handle_key(app, key.code) {
                    break;
                }
            }
        }

        if last_refresh.elapsed() >= refresh_interval {
            app.refresh();
            last_refresh = Instant::now();
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode) -> bool {
    // If search mode is active
    if app.is_searching {
        match code {
            KeyCode::Esc | KeyCode::Enter => app.is_searching = false,
            KeyCode::Backspace => {
                app.search_query.pop();
                app.scroll_offset = 0;
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.scroll_offset = 0;
            }
            _ => {}
        }
        return false;
    }

    // If kill modal is active
    if app.show_kill_modal {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                let _ = app.kill_selected_process();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.show_kill_modal = false;
            }
            _ => {}
        }
        return false;
    }

    // Normal navigation
    match code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Tab => app.cycle_tab(),
        KeyCode::Char('1') => {
            app.active_tab = ActiveTab::Telemetry;
            app.scroll_offset = 0;
        }
        KeyCode::Char('2') => {
            app.active_tab = ActiveTab::Battery;
            app.scroll_offset = 0;
        }
        KeyCode::Char('3') => {
            app.active_tab = ActiveTab::Chronicle;
            app.scroll_offset = 0;
        }
        KeyCode::Char('e') => app.export_ai_prompt(),
        KeyCode::Char('/') => {
            if app.active_tab == ActiveTab::Chronicle {
                app.is_searching = true;
            }
        }
        KeyCode::Char('K') => {
            if app.active_tab == ActiveTab::Telemetry {
                app.show_kill_modal = true;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.active_tab == ActiveTab::Telemetry {
                app.next_process();
            } else if app.active_tab == ActiveTab::Chronicle {
                app.scroll_offset = app.scroll_offset.saturating_add(1);
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.active_tab == ActiveTab::Telemetry {
                app.prev_process();
            } else if app.active_tab == ActiveTab::Chronicle {
                app.scroll_offset = app.scroll_offset.saturating_sub(1);
            }
        }
        _ => {}
    }
    false
}

fn ui(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Footer / Keybindings
        ])
        .split(size);

    render_header(frame, chunks[0], app);

    match app.active_tab {
        ActiveTab::Telemetry => {
            views::render_telemetry(
                frame,
                chunks[1],
                &app.snapshot,
                &app.battery,
                app.selected_proc_idx,
            );
        }
        ActiveTab::Battery => {
            views::render_battery_view(frame, chunks[1], &app.battery);
        }
        ActiveTab::Chronicle => {
            views::render_chronicle(
                frame,
                chunks[1],
                &app.events,
                &app.search_query,
                app.is_searching,
                app.scroll_offset,
            );
        }
    }

    render_footer(frame, chunks[2], app);

    if app.show_kill_modal {
        views::render_kill_modal(frame, size, app);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
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
    let title_block = Paragraph::new(title_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );
    frame.render_widget(title_block, cols[0]);

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
        frame.render_widget(host_p, cols[2]);
    }
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    if let Some((msg, _)) = &app.status_message {
        let p = Paragraph::new(format!("  {msg}")).style(
            Style::default()
                .fg(COLOR_SUCCESS)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(p, area);
        return;
    }

    let keys = vec![
        Span::styled(
            " [Tab/1-3]",
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Views ", Style::default().fg(COLOR_MUTED)),
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
    let p = Paragraph::new(Line::from(keys));
    frame.render_widget(p, area);
}
