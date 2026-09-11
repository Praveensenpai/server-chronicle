use anyhow::Result;
use std::process::Command;

use crate::api::export::generate_ai_report;
use crate::domain::battery_types::BatterySnapshot;
use crate::domain::models::{EventRecord, ServerSnapshot};
use crate::infra::{
    battery::read_battery_snapshot, capture_server_snapshot, storage::read_today_events,
};

#[derive(Debug, PartialEq, Eq)]
pub enum ActiveTab {
    Telemetry,
    Battery,
    Chronicle,
}

pub struct App {
    pub active_tab: ActiveTab,
    pub snapshot: ServerSnapshot,
    pub battery: BatterySnapshot,
    pub events: Vec<EventRecord>,
    pub search_query: String,
    pub is_searching: bool,
    pub scroll_offset: usize,
    pub selected_proc_idx: usize,
    pub show_kill_modal: bool,
    pub status_message: Option<(String, std::time::Instant)>,
}

impl App {
    #[must_use]
    pub fn new() -> Self {
        let snapshot = capture_server_snapshot();
        let battery = read_battery_snapshot();
        let events = read_today_events();

        Self {
            active_tab: ActiveTab::Telemetry,
            snapshot,
            battery,
            events,
            search_query: String::new(),
            is_searching: false,
            scroll_offset: 0,
            selected_proc_idx: 0,
            show_kill_modal: false,
            status_message: None,
        }
    }

    pub fn refresh(&mut self) {
        self.snapshot = capture_server_snapshot();
        self.battery = read_battery_snapshot();
        self.events = read_today_events();
    }

    pub fn cycle_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Telemetry => ActiveTab::Battery,
            ActiveTab::Battery => ActiveTab::Chronicle,
            ActiveTab::Chronicle => ActiveTab::Telemetry,
        };
        self.scroll_offset = 0;
    }

    pub fn next_process(&mut self) {
        if !self.snapshot.top_processes.is_empty() {
            self.selected_proc_idx =
                (self.selected_proc_idx + 1) % self.snapshot.top_processes.len();
        }
    }

    pub fn prev_process(&mut self) {
        if !self.snapshot.top_processes.is_empty() {
            if self.selected_proc_idx == 0 {
                self.selected_proc_idx = self.snapshot.top_processes.len() - 1;
            } else {
                self.selected_proc_idx -= 1;
            }
        }
    }

    pub fn kill_selected_process(&mut self) -> Result<()> {
        if let Some(proc) = self.snapshot.top_processes.get(self.selected_proc_idx) {
            let pid_str = proc.pid.to_string();
            let _ = Command::new("kill").args(["-15", &pid_str]).output();
            self.set_status(format!("Sent SIGTERM to {} (PID {})", proc.name, proc.pid));
            self.refresh();
        }
        self.show_kill_modal = false;
        Ok(())
    }

    pub fn export_ai_prompt(&mut self) {
        let report = generate_ai_report(&self.snapshot, &self.battery, &self.events);

        // 1. Emit OSC 52 sequence so terminal clipboard captures it (even over remote SSH)
        let osc52 = format!("\x1b]52;c;{}\x07", to_base64(report.as_bytes()));
        use std::io::Write;
        let _ = std::io::stdout().write_all(osc52.as_bytes());
        let _ = std::io::stdout().flush();

        // 2. Try desktop clipboard via wl-copy or xclip
        let mut copied = false;
        if let Ok(mut child) = Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(report.as_bytes());
                copied = true;
            }
        }

        if !copied {
            if let Ok(mut child) = Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(report.as_bytes());
                    copied = true;
                }
            }
        }

        // 3. Always write to latest_export.md as permanent fallback
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let path =
            std::path::Path::new(&home).join(".local/share/server-chronicle/latest_export.md");
        if let Some(p) = path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let _ = std::fs::write(&path, &report);

        if copied {
            self.set_status("✨ Exported to clipboard & latest_export.md!".to_string());
        } else {
            self.set_status("✨ Copied via OSC52 & saved to latest_export.md!".to_string());
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
    }

    pub fn check_status_expiration(&mut self) {
        if let Some((_, instant)) = &self.status_message {
            if instant.elapsed().as_secs() > 4 {
                self.status_message = None;
            }
        }
    }
}

#[must_use]
pub fn to_base64(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARSET[(triple >> 18) & 0x3F] as char);
        out.push(CHARSET[(triple >> 12) & 0x3F] as char);
        if chunk.len() > 1 {
            out.push(CHARSET[(triple >> 6) & 0x3F] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARSET[triple & 0x3F] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_base64() {
        assert_eq!(to_base64(b""), "");
        assert_eq!(to_base64(b"f"), "Zg==");
        assert_eq!(to_base64(b"fo"), "Zm8=");
        assert_eq!(to_base64(b"foo"), "Zm9v");
        assert_eq!(to_base64(b"Hello World!"), "SGVsbG8gV29ybGQh");
    }
}
