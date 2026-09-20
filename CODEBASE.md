# CODEBASE.md: Server Chronicle Semantic Digest

> **Notice**: This file is an AI-optimized semantic index. Do not write narrative prose. Keep token density high.

## 1. System Topology & Data Flow
```text
src/main.rs ──> Clap CLI
                 ├── run_daemon (src/daemon.rs) ──> Polling Loop ──> capture_server_snapshot
                 │                                                ──> append_event (src/infra/storage.rs)
                 └── run_tui (src/tui.rs) ───────> mpsc Telemetry Worker ──> Event Loop ──> Ratatui Frame
                                                   └── src/tui/app.rs (App State) ──> src/tui/views/*.rs
```

## 2. Global Constraints & Architecture Patterns
- **Primary Language & Edition**: Rust 2021 edition
- **Architectural Paradigm**: Role-based (`domain/`, `infra/`, `api/`, `tui/`)
- **Hard Constraints**: <400 lines/file (300 soft), <60 lines/fn (40 soft), max 4 parameters, max 3 nesting depth, zero production `unwrap()` / `expect()`, 0 compiler/clippy warnings
- **Target Distribution**: Linux x86_64 & aarch64 standalone binary via GitHub Releases

## 3. Module & Interface Skeleton

### `src/domain/models.rs` (Role: domain, Lines: ~392)
- **Responsibility**: Core domain models for server metrics, activity events, process telemetry, and daily snapshots.
- **Imports**: `chrono::{DateTime, Utc}`, `serde::{Deserialize, Serialize}`
- **Types & Enums**:
  ```rust
  pub enum ServerActivityEvent { ... }
  pub struct EventRecord { pub id: String, pub timestamp: DateTime<Utc>, pub event: ServerActivityEvent }
  pub struct SystemMetrics { pub hostname: String, pub uptime_secs: u64, pub cpu_percent: f64, ... }
  pub enum ProcessSortMode { Cpu, Ram }
  pub struct ProcessItem { pub pid: u32, pub name: String, pub cpu_percent: f64, pub mem_percent: f64, pub mem_bytes: u64 }
  pub struct ServerSnapshot { pub system: SystemMetrics, pub containers: Vec<ContainerInfo>, pub ssh_sessions: Vec<SshSession>, pub torrents: Vec<TorrentSnapshot>, pub top_processes: Vec<ProcessItem> }
  ```
- **Public Functions & Signatures**:
  ```rust
  impl ProcessSortMode { pub fn label(self) -> &'static str; pub fn toggle(self) -> Self; }
  impl ProcessItem { pub fn formatted_mem(&self) -> String; }
  impl SystemMetrics { pub fn mem_percent(&self) -> f64; pub fn disk_percent(&self) -> f64; }
  ```
- **Consumers**: `src/infra/system.rs`, `src/infra.rs`, `src/daemon.rs`, `src/tui/app.rs`, `src/tui/views/telemetry.rs`, `src/api/export.rs`
- **Side Effects / I/O**: None (pure data structures and serialization)

### `src/domain/battery_types.rs` (Role: domain, Lines: ~148)
- **Responsibility**: Battery UPS telemetry models, power states, and discharge bracket metrics.
- **Imports**: `chrono::{DateTime, Utc}`, `serde::{Deserialize, Serialize}`
- **Types & Enums**:
  ```rust
  pub enum PowerState { Discharging, Charging, Full, NotCharging, Unknown }
  pub struct BracketRecord { pub bracket_name: String, pub start_pct: u8, pub end_pct: u8, pub duration_secs: u64, ... }
  pub struct BatterySnapshot { pub state: PowerState, pub capacity: u8, pub rate_pct_per_hour: Option<f64>, ... }
  ```

### `src/infra/system.rs` (Role: infra, Lines: ~293)
- **Responsibility**: Linux `/proc` and `/sys` filesystem telemetry extraction and `ps` process monitoring.
- **Imports**: `crate::domain::models::{ProcessItem, SystemMetrics}`, `std::process::Command`
- **Public Functions & Signatures**:
  ```rust
  pub fn read_system_metrics() -> SystemMetrics;
  pub fn read_top_processes(limit: usize) -> Vec<ProcessItem>;
  pub fn parse_process_line(line: &str, num_cpus: usize) -> Option<ProcessItem>;
  pub fn normalize_process_cpu(raw_cpu: f64, num_cpus: usize) -> f64;
  ```
- **Consumers**: `src/infra.rs`
- **Side Effects / I/O**: Reads `/proc/meminfo`, `/proc/stat`, `/proc/uptime`, `/sys/class/thermal`, executes `ps -eo pid,comm,%cpu,%mem,rss`

### `src/infra/battery.rs` (Role: infra, Lines: ~194)
- **Responsibility**: Live battery sensor parsing and 10-bracket discharge rate tracking.
- **Public Functions & Signatures**:
  ```rust
  pub fn read_battery_snapshot() -> BatterySnapshot;
  ```
- **Side Effects / I/O**: Reads `/sys/class/power_supply/`

### `src/infra/storage.rs` (Role: infra, Lines: ~78)
- **Responsibility**: Local event chronicle persistence via JSON lines in `~/.local/share/server-chronicle/`.
- **Public Functions & Signatures**:
  ```rust
  pub fn append_event(event: &ServerActivityEvent) -> Result<()>;
  pub fn read_today_events() -> Vec<EventRecord>;
  ```

### `src/tui/app.rs` (Role: tui, Lines: ~277)
- **Responsibility**: Interactive application state, tab navigation, process selection, sort toggling, AI prompt generation.
- **Types & Enums**:
  ```rust
  pub enum ActiveTab { Telemetry, Battery, Chronicle }
  pub struct App { pub active_tab: ActiveTab, pub snapshot: ServerSnapshot, pub battery: BatterySnapshot, pub events: Vec<EventRecord>, pub selected_proc_idx: usize, pub proc_sort_mode: ProcessSortMode, ... }
  ```
- **Public Functions & Signatures**:
  ```rust
  impl App {
      pub fn new() -> Self;
      pub fn sort_process_list(processes: &mut [ProcessItem], mode: ProcessSortMode);
      pub fn toggle_process_sort(&mut self);
      pub fn set_process_sort(&mut self, mode: ProcessSortMode);
      pub fn apply_telemetry(&mut self, snapshot: ServerSnapshot, battery: BatterySnapshot, events: Vec<EventRecord>);
      pub fn next_process(&mut self);
      pub fn prev_process(&mut self);
      pub fn kill_selected_process(&mut self) -> Result<()>;
      pub fn export_ai_prompt(&mut self);
  }
  ```

### `src/tui/views/telemetry.rs` (Role: tui, Lines: ~264)
- **Responsibility**: Renders resource gauges, active containers, SSH sessions, and top processes table with PID, name, CPU%, MEM%, and RAM usage.
- **Public Functions & Signatures**:
  ```rust
  pub fn render_telemetry(frame: &mut Frame, area: Rect, snapshot: &ServerSnapshot, battery: &BatterySnapshot, selected_proc_idx: usize, sort_mode: ProcessSortMode);
  ```

### `src/tui.rs` (Role: tui, Lines: ~253)
- **Responsibility**: TUI initialization, background telemetry polling thread, and terminal key event loop.
- **Public Functions & Signatures**:
  ```rust
  pub fn run_tui() -> Result<()>;
  ```

### `src/daemon.rs` (Role: daemon, Lines: ~341)
- **Responsibility**: Background monitoring daemon, event detection (power loss, SSH, docker, spikes), and hourly heartbeats.
- **Public Functions & Signatures**:
  ```rust
  pub fn run_daemon(interval_secs: u64) -> Result<()>;
  ```

## 4. Execution Lifecycle Trace
1. **Startup**: `src/main.rs` matches CLI arguments via `clap`.
2. **Dispatch**:
   - `server-chronicle tui` (or default without flags): Invokes `src/tui::run_tui()`.
   - `server-chronicle daemon`: Invokes `src/daemon::run_daemon(interval)`.
   - `server-chronicle export`: Generates AI Markdown report and prints to stdout.
3. **Execution**:
   - TUI renders telemetry, battery UPS, and chronicle timelines. Keypress `s` toggles process sorting by CPU vs RAM.
4. **Shutdown**: Signal handling or `q`/`Esc` restores terminal raw mode and exits cleanly.

## 5. Verification Commands
```bash
cargo build --release --target x86_64-unknown-linux-gnu
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## 6. Recent Iteration Changes
- **2026-09-20**:
  - `src/domain/models.rs`: Added `ProcessSortMode` enum (`Cpu`, `Ram`) and `mem_bytes` field to `ProcessItem` with `formatted_mem()` helper.
  - `src/infra/system.rs`: Updated `read_top_processes()` and `parse_process_line()` to query `rss` and combine top CPU and RAM processes.
  - `src/tui/app.rs`: Added `proc_sort_mode` state, `sort_process_list`, `toggle_process_sort`, and `set_process_sort`.
  - `src/tui/views/telemetry.rs`: Added RAM usage column (`RAM`) to processes table and dynamic sort indicator in table title.
  - `src/tui/views.rs`: Added `[s] Sort: CPU/RAM` shortcut hint in footer.
  - `src/tui.rs`: Added navigation keybindings for `s` (toggle sort), `c` (CPU sort), and `m` (RAM sort).
