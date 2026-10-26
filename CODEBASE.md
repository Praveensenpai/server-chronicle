# CODEBASE.md: server-chronicle Semantic Digest

> **Notice**: AI-optimized semantic index. Do not write narrative prose. Keep token density high.

## 1. System Topology & Data Flow
```text
Entrypoint ──> CLI/Parser ──> Domain Logic ──> Infra/IO
```

## 2. Global Constraints & Architecture Patterns
- **Primary Language**: Rust 2021 edition
- **Architectural Paradigm**: Role-based (domain/, infra/, api/cli/, tui/)
- **Hard Constraints**: <400 lines/file, <60 lines/fn, zero production unwrap(), 0 warnings.
- **Target Distribution**: Linux x86_64 standalone binary

## 3. Module & Interface Skeleton

### `src/api/export.rs` (Role: api, Lines: 385)
- **Responsibility**: Core api logic in src/api/export.rs
- **Imports**: use chrono :: Utc , use std :: fmt :: Write as _ , use crate :: domain :: battery_types :: BatterySnapshot , use crate :: domain :: models :: { EventRecord , ServerActivityEvent , ServerSnapshot } 
- **Public Functions & Signatures**:
  ```rust
  fn generate_ai_report (snapshot : & ServerSnapshot , battery : & BatterySnapshot , events : & [EventRecord] ,) -> String
  ```

### `src/api.rs` (Role: api, Lines: 3)
- **Responsibility**: Core api logic in src/api.rs
- **Imports**: pub use export :: generate_ai_report 

### `src/daemon.rs` (Role: general, Lines: 347)
- **Responsibility**: Core general logic in src/daemon.rs
- **Imports**: use anyhow :: Result , use std :: collections :: HashMap , use std :: thread , use std :: time :: { Duration , Instant } , use crate :: domain :: battery_types :: { BatterySnapshot , PowerState } , use crate :: domain :: models :: { EventRecord , ServerActivityEvent , ServerSnapshot } , use crate :: infra :: { battery :: read_battery_snapshot , capture_server_snapshot , storage :: append_event , } 
- **Public Functions & Signatures**:
  ```rust
  fn run_daemon (interval_secs : u64) -> Result < () >
  fn simplify_container_status (status : & str) -> & 'static str
  ```

### `src/domain/battery_types.rs` (Role: domain, Lines: 223)
- **Responsibility**: Core domain logic in src/domain/battery_types.rs
- **Imports**: use serde :: { Deserialize , Serialize } 
- **Types & Enums**:
  ```rust
  pub enum PowerState
  pub struct BracketStat
  pub struct BatterySnapshot
  ```
- **Public Functions & Signatures**:
  ```rust
  fn from_str (s : & str) -> Self
  fn as_str (& self) -> & 'static str
  fn init_default_brackets () -> Vec < BracketStat >
  fn init_default_discharge_brackets () -> Vec < BracketStat >
  fn calculate_rate_pct_per_hour (start_cap : u8 , end_cap : u8 , duration_secs : u64) -> f64
  fn charging_bracket_index (cap : u8) -> usize
  fn discharging_bracket_index (cap : u8) -> usize
  fn calculate_health_percent (full_uah : u64 , design_uah : u64) -> f64
  ```

### `src/domain/models.rs` (Role: domain, Lines: 397)
- **Responsibility**: Core domain logic in src/domain/models.rs
- **Imports**: use chrono :: { DateTime , Utc } , use serde :: { Deserialize , Serialize } 
- **Types & Enums**:
  ```rust
  pub enum ServerActivityEvent
  pub struct EventRecord
  pub struct ContainerInfo
  pub struct SshSession
  pub struct TorrentSnapshot
  pub struct SystemMetrics
  pub enum ProcessSortMode
  pub struct ProcessItem
  pub struct ServerSnapshot
  ```
- **Public Functions & Signatures**:
  ```rust
  fn summary (& self) -> String
  fn new (event : ServerActivityEvent) -> Self
  fn fan_display (& self) -> & str
  fn mem_percent (& self) -> f64
  fn disk_percent (& self) -> f64
  fn label (self) -> & 'static str
  fn toggle (self) -> Self
  fn formatted_mem (& self) -> String
  fn mem_summary (& self) -> String
  ```

### `src/domain.rs` (Role: domain, Lines: 2)
- **Responsibility**: Core domain logic in src/domain.rs

### `src/infra/battery.rs` (Role: infra, Lines: 384)
- **Responsibility**: Core infra logic in src/infra/battery.rs
- **Imports**: use anyhow :: Result , use chrono :: { DateTime , Utc } , use serde :: { Deserialize , Serialize } , use std :: fs , use std :: path :: { Path , PathBuf } , use crate :: domain :: battery_types :: { calculate_health_percent , calculate_rate_pct_per_hour , charging_bracket_index , discharging_bracket_index , init_default_brackets , init_default_discharge_brackets , BatterySnapshot , BracketStat , PowerState , } 
- **Types & Enums**:
  ```rust
  pub struct PersistentBatteryTracker
  ```
- **Public Functions & Signatures**:
  ```rust
  fn new () -> Self
  fn load_or_init () -> Self
  fn save (& self) -> Result < () >
  fn update (& mut self , current_cap : u8 , state : & PowerState)
  fn current_speed_pct_per_hour (& self , current_cap : u8 , state : & PowerState) -> f64
  fn read_battery_snapshot () -> BatterySnapshot
  ```

### `src/infra/docker.rs` (Role: infra, Lines: 89)
- **Responsibility**: Core infra logic in src/infra/docker.rs
- **Imports**: use std :: process :: Command , use crate :: domain :: models :: ContainerInfo 
- **Public Functions & Signatures**:
  ```rust
  fn read_containers () -> Vec < ContainerInfo >
  ```

### `src/infra/ssh.rs` (Role: infra, Lines: 102)
- **Responsibility**: Core infra logic in src/infra/ssh.rs
- **Imports**: use std :: process :: Command , use crate :: domain :: models :: SshSession 
- **Public Functions & Signatures**:
  ```rust
  fn is_remote_ip (ip : & str) -> bool
  fn parse_w_line (line : & str) -> Option < SshSession >
  fn read_active_ssh_sessions () -> Vec < SshSession >
  ```

### `src/infra/storage.rs` (Role: infra, Lines: 61)
- **Responsibility**: Core infra logic in src/infra/storage.rs
- **Imports**: use anyhow :: Result , use chrono :: { DateTime , Utc } , use std :: fs :: { self , OpenOptions } , use std :: io :: { BufRead , BufReader , Write } , use std :: path :: { Path , PathBuf } , use crate :: domain :: models :: EventRecord 
- **Public Functions & Signatures**:
  ```rust
  fn get_log_dir () -> PathBuf
  fn get_log_path (date : DateTime < Utc >) -> PathBuf
  fn append_event (record : & EventRecord) -> Result < () >
  fn read_today_events () -> Vec < EventRecord >
  fn read_events_for_date (date : DateTime < Utc >) -> Vec < EventRecord >
  fn read_events_from_path (path : & Path) -> Vec < EventRecord >
  ```

### `src/infra/system.rs` (Role: infra, Lines: 357)
- **Responsibility**: Core infra logic in src/infra/system.rs
- **Imports**: use std :: fs , use std :: mem :: MaybeUninit , use std :: process :: Command , use crate :: domain :: models :: { ProcessItem , SystemMetrics } , use crate :: infra :: thermal :: { read_cpu_core_temps , read_cpu_temperature , read_fan_info } 
- **Public Functions & Signatures**:
  ```rust
  fn read_system_metrics () -> SystemMetrics
  fn parse_cpu_stat_line (line : & str) -> Option < (u64 , u64) >
  fn normalize_process_cpu (raw_cpu : f64 , num_cpus : usize) -> f64
  fn resolve_process_name (comm : & str , args : & str) -> String
  fn parse_process_line (line : & str , num_cpus : usize) -> Option < ProcessItem >
  fn read_top_processes (limit : usize) -> Vec < ProcessItem >
  ```

### `src/infra/thermal.rs` (Role: infra, Lines: 285)
- **Responsibility**: Core infra logic in src/infra/thermal.rs
- **Imports**: use std :: fs 
- **Public Functions & Signatures**:
  ```rust
  fn read_cpu_temperature () -> Option < f64 >
  fn read_cpu_core_temps () -> Vec < f64 >
  fn read_fan_speed () -> Option < u32 >
  fn read_fan_info () -> (Option < u32 > , String)
  fn parse_ec_fan_rpm (data : & [u8]) -> Option < u32 >
  ```

### `src/infra/torrent.rs` (Role: infra, Lines: 76)
- **Responsibility**: Core infra logic in src/infra/torrent.rs
- **Imports**: use reqwest :: blocking :: Client , use serde :: Deserialize , use std :: time :: Duration , use crate :: domain :: models :: TorrentSnapshot 
- **Types & Enums**:
  ```rust
  pub struct QbitTorrentItem
  ```
- **Public Functions & Signatures**:
  ```rust
  fn read_torrents () -> Vec < TorrentSnapshot >
  ```

### `src/infra.rs` (Role: infra, Lines: 25)
- **Responsibility**: Core infra logic in src/infra.rs
- **Imports**: use crate :: domain :: models :: ServerSnapshot 
- **Public Functions & Signatures**:
  ```rust
  fn capture_server_snapshot () -> ServerSnapshot
  ```

### `src/main.rs` (Role: general, Lines: 115)
- **Responsibility**: Core general logic in src/main.rs
- **Imports**: use anyhow :: Result , use clap :: { Parser , Subcommand } 

### `src/service.rs` (Role: general, Lines: 51)
- **Responsibility**: Core general logic in src/service.rs
- **Imports**: use anyhow :: { Context , Result } , use std :: fs , use std :: path :: Path , use std :: process :: Command 
- **Public Functions & Signatures**:
  ```rust
  fn install_user_service () -> Result < () >
  ```

### `src/tui/app.rs` (Role: tui, Lines: 277)
- **Responsibility**: Core tui logic in src/tui/app.rs
- **Imports**: use anyhow :: Result , use std :: process :: Command , use crate :: api :: export :: generate_ai_report , use crate :: domain :: battery_types :: BatterySnapshot , use crate :: domain :: models :: { EventRecord , ProcessItem , ProcessSortMode , ServerSnapshot } , use crate :: infra :: { battery :: read_battery_snapshot , capture_server_snapshot , storage :: read_today_events , } 
- **Types & Enums**:
  ```rust
  pub enum ActiveTab
  pub struct App
  ```
- **Public Functions & Signatures**:
  ```rust
  fn new () -> Self
  fn sort_process_list (processes : & mut [ProcessItem] , mode : ProcessSortMode)
  fn toggle_process_sort (& mut self)
  fn set_process_sort (& mut self , mode : ProcessSortMode)
  fn apply_telemetry (& mut self , mut snapshot : ServerSnapshot , battery : BatterySnapshot , events : Vec < EventRecord > ,)
  fn cycle_tab (& mut self)
  fn next_process (& mut self)
  fn prev_process (& mut self)
  fn kill_selected_process (& mut self) -> Result < () >
  fn export_ai_prompt (& mut self)
  fn set_status (& mut self , msg : String)
  fn check_status_expiration (& mut self)
  fn to_base64 (bytes : & [u8]) -> String
  ```

### `src/tui/theme.rs` (Role: tui, Lines: 11)
- **Responsibility**: Core tui logic in src/tui/theme.rs
- **Imports**: use ratatui :: style :: Color 

### `src/tui/views/battery_view.rs` (Role: tui, Lines: 359)
- **Responsibility**: Core tui logic in src/tui/views/battery_view.rs
- **Imports**: use ratatui :: { layout :: { Constraint , Direction , Layout , Rect } , style :: { Modifier , Style } , widgets :: { Block , Borders , Paragraph } , Frame , } , use super :: render_compact_gauge , use crate :: domain :: battery_types :: { BatterySnapshot , PowerState } , use crate :: tui :: theme :: { COLOR_BORDER , COLOR_DANGER , COLOR_MUTED , COLOR_PRIMARY , COLOR_SECONDARY , COLOR_SUCCESS , COLOR_TEXT , COLOR_WARNING , } 
- **Public Functions & Signatures**:
  ```rust
  fn render_battery_view (frame : & mut Frame , area : Rect , battery : & BatterySnapshot)
  ```

### `src/tui/views/chronicle.rs` (Role: tui, Lines: 141)
- **Responsibility**: Core tui logic in src/tui/views/chronicle.rs
- **Imports**: use ratatui :: { layout :: { Constraint , Direction , Layout , Rect } , style :: { Modifier , Style } , widgets :: { Block , Borders , List , ListItem , Paragraph } , Frame , } , use crate :: domain :: models :: { EventRecord , ServerActivityEvent } , use crate :: tui :: theme :: { COLOR_BORDER , COLOR_DANGER , COLOR_MUTED , COLOR_PRIMARY , COLOR_SECONDARY , COLOR_SUCCESS , COLOR_WARNING , } 
- **Public Functions & Signatures**:
  ```rust
  fn render_chronicle (frame : & mut Frame , area : Rect , events : & [EventRecord] , search_query : & str , is_searching : bool , scroll_offset : usize ,)
  ```

### `src/tui/views/services.rs` (Role: tui, Lines: 178)
- **Responsibility**: Core tui logic in src/tui/views/services.rs
- **Imports**: use ratatui :: { layout :: { Constraint , Direction , Layout , Rect } , style :: { Modifier , Style } , widgets :: { Block , Borders , Cell , Paragraph , Row , Table } , Frame , } , use crate :: domain :: models :: ServerSnapshot , use crate :: tui :: theme :: { COLOR_ACCENT , COLOR_BORDER , COLOR_DANGER , COLOR_MUTED , COLOR_PRIMARY , COLOR_SECONDARY , COLOR_SUCCESS , COLOR_WARNING , } 
- **Public Functions & Signatures**:
  ```rust
  fn render_containers_and_ssh (frame : & mut Frame , area : Rect , snapshot : & ServerSnapshot)
  fn render_docker_table (frame : & mut Frame , area : Rect , snapshot : & ServerSnapshot)
  fn render_torrents_table (frame : & mut Frame , area : Rect , snapshot : & ServerSnapshot)
  fn render_ssh_list (frame : & mut Frame , area : Rect , snapshot : & ServerSnapshot)
  ```

### `src/tui/views/telemetry.rs` (Role: tui, Lines: 278)
- **Responsibility**: Core tui logic in src/tui/views/telemetry.rs
- **Imports**: use ratatui :: { layout :: { Constraint , Direction , Layout , Rect } , style :: { Modifier , Style } , text :: { Line , Span } , widgets :: { Block , Borders , Gauge , Paragraph , Row , Table } , Frame , } , use super :: render_compact_gauge , use super :: services :: render_containers_and_ssh , use crate :: domain :: battery_types :: BatterySnapshot , use crate :: domain :: models :: { ProcessSortMode , ServerSnapshot } , use crate :: tui :: theme :: { COLOR_BORDER , COLOR_DANGER , COLOR_MUTED , COLOR_PRIMARY , COLOR_SECONDARY , COLOR_SUCCESS , COLOR_WARNING , } 
- **Public Functions & Signatures**:
  ```rust
  fn render_telemetry (frame : & mut Frame , area : Rect , snapshot : & ServerSnapshot , battery : & BatterySnapshot , selected_proc_idx : usize , sort_mode : ProcessSortMode ,)
  ```

### `src/tui/views.rs` (Role: tui, Lines: 300)
- **Responsibility**: Core tui logic in src/tui/views.rs
- **Imports**: use ratatui :: { layout :: Rect , style :: { Color , Modifier , Style } , text :: Span , widgets :: { Block , Borders , Gauge } , Frame , } , pub use battery_view :: render_battery_view , pub use chronicle :: render_chronicle , pub use telemetry :: render_telemetry 
- **Public Functions & Signatures**:
  ```rust
  fn render_compact_gauge (frame : & mut Frame , area : Rect , title : String , color : Color , percent : u16 , label : String ,)
  fn render_kill_modal (frame : & mut Frame , area : Rect , app : & crate :: tui :: app :: App)
  fn centered_rect (percent_x : u16 , percent_y : u16 , r : Rect) -> Rect
  fn render_header (frame : & mut Frame , area : Rect , app : & crate :: tui :: app :: App)
  fn render_footer (frame : & mut Frame , area : Rect , app : & crate :: tui :: app :: App)
  ```

### `src/tui.rs` (Role: tui, Lines: 253)
- **Responsibility**: Core tui logic in src/tui.rs
- **Imports**: use anyhow :: Result , use crossterm :: { event :: { self , Event , KeyCode , KeyEventKind } , execute , terminal :: { disable_raw_mode , enable_raw_mode , EnterAlternateScreen , LeaveAlternateScreen } , } , use ratatui :: { backend :: CrosstermBackend , layout :: { Constraint , Direction , Layout } , Frame , Terminal , } , use std :: io :: stdout , use std :: sync :: atomic :: { AtomicBool , Ordering } , use std :: sync :: mpsc :: { self , Receiver } , use std :: sync :: Arc , use std :: thread , use std :: time :: Duration , use crate :: domain :: battery_types :: BatterySnapshot , use crate :: domain :: models :: { EventRecord , ProcessSortMode , ServerSnapshot } , use crate :: infra :: { battery :: read_battery_snapshot , capture_server_snapshot , storage :: read_today_events , } , use app :: { ActiveTab , App } 
- **Public Functions & Signatures**:
  ```rust
  fn run_tui () -> Result < () >
  ```

## 4. Execution Lifecycle Trace
1. **Startup**: Entrypoint parses CLI flags & dispatches command.
2. **Execution**: Core domain logic processes inputs and evaluates rules.
3. **Persistence / I/O**: Domain logic calls infra for disk/terminal I/O.
4. **Exit**: Graceful termination with standard exit codes.

## 5. Verification Commands
```bash
cargo build --release --target x86_64-unknown-linux-gnu
cargo test --all-targets
cargo clippy --all-targets -- -D warnings && cargo fmt --check
```
