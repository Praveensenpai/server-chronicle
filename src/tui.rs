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
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use std::io::stdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::domain::battery_types::BatterySnapshot;
use crate::domain::models::{EventRecord, ProcessSortMode, ServerSnapshot};
use crate::infra::{
    battery::read_battery_snapshot, capture_server_snapshot, storage::read_today_events,
};
use app::{ActiveTab, App};

struct TelemetryUpdate {
    snapshot: ServerSnapshot,
    battery: BatterySnapshot,
    events: Vec<EventRecord>,
}

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

fn spawn_telemetry_worker(running: Arc<AtomicBool>) -> Receiver<TelemetryUpdate> {
    let (tx, rx) = mpsc::channel();
    let _ = thread::Builder::new()
        .name("chronicle-telemetry".into())
        .spawn(move || {
            while running.load(Ordering::Relaxed) {
                for _ in 0..20 {
                    if !running.load(Ordering::Relaxed) {
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                let update = TelemetryUpdate {
                    snapshot: capture_server_snapshot(),
                    battery: read_battery_snapshot(),
                    events: read_today_events(),
                };

                if tx.send(update).is_err() {
                    break;
                }
            }
        });
    rx
}

fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let rx = spawn_telemetry_worker(Arc::clone(&running));

    let res = event_loop(terminal, app, &rx);

    running.store(false, Ordering::Relaxed);
    res
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    rx: &Receiver<TelemetryUpdate>,
) -> Result<()> {
    let poll_timeout = Duration::from_millis(30);

    loop {
        while let Ok(update) = rx.try_recv() {
            app.apply_telemetry(update.snapshot, update.battery, update.events);
        }

        terminal.draw(|f| ui(f, app))?;
        app.check_status_expiration();

        if event::poll(poll_timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && handle_key(app, key.code) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode) -> bool {
    if app.is_searching {
        handle_search_key(app, code);
        return false;
    }
    if app.show_kill_modal {
        handle_kill_modal_key(app, code);
        return false;
    }
    handle_navigation_key(app, code)
}

fn handle_search_key(app: &mut App, code: KeyCode) {
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
}

fn handle_kill_modal_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
            let _ = app.kill_selected_process();
        }
        KeyCode::Char('n' | 'N') | KeyCode::Esc => {
            app.show_kill_modal = false;
        }
        _ => {}
    }
}

fn handle_navigation_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Tab => app.cycle_tab(),
        KeyCode::Char('1') => set_active_tab(app, ActiveTab::Telemetry),
        KeyCode::Char('2') => set_active_tab(app, ActiveTab::Battery),
        KeyCode::Char('3') => set_active_tab(app, ActiveTab::Chronicle),
        KeyCode::Char('e') => app.export_ai_prompt(),
        KeyCode::Char('/') if app.active_tab == ActiveTab::Chronicle => {
            app.is_searching = true;
        }
        KeyCode::Char('K') if app.active_tab == ActiveTab::Telemetry => {
            app.show_kill_modal = true;
        }
        KeyCode::Char('s') if app.active_tab == ActiveTab::Telemetry => {
            app.toggle_process_sort();
        }
        KeyCode::Char('c') if app.active_tab == ActiveTab::Telemetry => {
            app.set_process_sort(ProcessSortMode::Cpu);
        }
        KeyCode::Char('m') if app.active_tab == ActiveTab::Telemetry => {
            app.set_process_sort(ProcessSortMode::Ram);
        }
        KeyCode::Down | KeyCode::Char('j') => handle_scroll_down(app),
        KeyCode::Up | KeyCode::Char('k') => handle_scroll_up(app),
        _ => {}
    }
    false
}

fn set_active_tab(app: &mut App, tab: ActiveTab) {
    app.active_tab = tab;
    app.scroll_offset = 0;
}

fn handle_scroll_down(app: &mut App) {
    if app.active_tab == ActiveTab::Telemetry {
        app.next_process();
    } else if app.active_tab == ActiveTab::Chronicle {
        app.scroll_offset = app.scroll_offset.saturating_add(1);
    }
}

fn handle_scroll_up(app: &mut App) {
    if app.active_tab == ActiveTab::Telemetry {
        app.prev_process();
    } else if app.active_tab == ActiveTab::Chronicle {
        app.scroll_offset = app.scroll_offset.saturating_sub(1);
    }
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

    views::render_header(frame, chunks[0], app);

    match app.active_tab {
        ActiveTab::Telemetry => {
            views::render_telemetry(
                frame,
                chunks[1],
                &app.snapshot,
                &app.battery,
                app.selected_proc_idx,
                app.proc_sort_mode,
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

    views::render_footer(frame, chunks[2], app);

    if app.show_kill_modal {
        views::render_kill_modal(frame, size, app);
    }
}
