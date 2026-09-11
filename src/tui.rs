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
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io::stdout;
use std::time::{Duration, Instant};

use app::{ActiveTab, App};
use theme::{
    COLOR_DANGER, COLOR_MUTED, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS, COLOR_WARNING,
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
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
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
        render_kill_modal(frame, size, app);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(20)])
        .split(area);

    let title_line = Line::from(vec![
        Span::styled(
            " ⏱️ SERVER ",
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "CHRONICLE ",
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let title_block = Paragraph::new(title_line).block(Block::default().borders(Borders::ALL));
    frame.render_widget(title_block, cols[0]);

    let tab_titles = vec![
        " [Tab] 🖥️ Telemetry ",
        " [Tab] 🔋 Battery UPS ",
        " [Tab] 📜 Chronicle ",
    ];
    let selected_idx = match app.active_tab {
        ActiveTab::Telemetry => 0,
        ActiveTab::Battery => 1,
        ActiveTab::Chronicle => 2,
    };

    let tabs = Tabs::new(tab_titles)
        .select(selected_idx)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(COLOR_MUTED))
        .highlight_style(
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, cols[1]);
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
            " Tab",
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Cycle Views • "),
        Span::styled(
            "e",
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" AI Export • "),
        Span::styled(
            "K",
            Style::default()
                .fg(COLOR_DANGER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Kill Proc • "),
        Span::styled(
            "/",
            Style::default()
                .fg(COLOR_WARNING)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Filter • "),
        Span::styled(
            "q",
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Exit"),
    ];
    let p = Paragraph::new(Line::from(keys));
    frame.render_widget(p, area);
}

fn render_kill_modal(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_DANGER))
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
