<div align="center">

# ⏱️ サーバー・クロニクル • SERVER-CHRONICLE
### *High-Performance Server Activity Logger, Battery UPS Monitor & AI Context Generator*

[![Latest Release](https://img.shields.io/github/v/release/Praveensenpai/server-chronicle?style=for-the-badge&color=blue)](https://github.com/Praveensenpai/server-chronicle/releases)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-DEA584?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Ubuntu_Server-E95420?style=for-the-badge&logo=ubuntu)](https://ubuntu.com)
[![Systemd](https://img.shields.io/badge/Daemon-Systemd_User_Unit-89b4fa?style=for-the-badge&logo=systemd)](https://systemd.io)
[![License: MIT](https://img.shields.io/badge/License-MIT-a6e3a1?style=for-the-badge)](LICENSE)

*Tailored for headless Ubuntu servers, self-hosted Docker media rigs, and laptop home servers acting as built-in battery UPS systems.*

[⚡ Quick Install](#-quick-start) • [✨ Features](#-key-features) • [💻 How to Use](#-how-to-use) • [⌨️ TUI Controls](#%EF%B8%8F-interactive-tui-controls) • [🌀 Fan Configuration](#-hardware-fan-speed-setup-hp--acpi) • [📁 Storage & Service](#-storage--systemd-service)

</div>

---

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                       ⏱️ SERVER-CHRONICLE ARCHITECTURE                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   [ ⚡ Hardware / ACPI ] ──➔ [ Battery UPS Engine ] ──➔ [ Dual Brackets ]   │
│   (HP /sys/class/power)       (Speed: %/hr over Δt)     (Charge & Drain 10) │
│                                                                             │
│   [ 🌀 Thermal & Cooling ] ─➔ [ Fan Resolver ]     ──➔ [ Tachometer / ACPI] │
│   (ec_sys, cooling_device)    (Auto Silent/Active)      (Exact RPM or Mode) │
│                                                                             │
│   [ 🐳 Docker Engine ]  ──➔ [ Container Tracker ]  ──➔ [ State Events ]     │
│   (qbittorrent, stats)        (CPU %, Memory load)      (Started / Stopped) │
│                                                                             │
│   [ 👤 OpenSSH / Sockets ] ➔ [ Session Inspector ] ──➔ [ Security Audit ]   │
│   (Active IPs, Tailscale)     (User, Duration, TTY)     (Login / Logouts)   │
│                                       │                                     │
│                                       ▼                                     │
│                     ┌───────────────────────────────────┐                   │
│                     │ 💾 Rolling JSONL Event Storage    │                   │
│                     │ (~/.local/share/server-chronicle) │                   │
│                     └─────────────────┬─────────────────┘                   │
│                                       │                                     │
│                 ┌─────────────────────┴─────────────────────┐               │
│                 ▼                                           ▼               │
│   ┌───────────────────────────┐               ┌───────────────────────────┐ │
│   │ 📊 Asynchronous 60FPS TUI │               │ 🤖 1-Keypress AI Exporter │ │
│   │   • 🖥️ Live Telemetry     │               │   • Markdown Prompt Gen   │ │
│   │   • 🔋 Battery UPS Dual   │               │   • Clipboard Pipe (wl/pb)│ │
│   │   • 📜 Daily Chronicle    │               │   • Resource Spike Audit  │ │
│   └───────────────────────────┘               └───────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## ✨ Key Features

| Feature | Description |
| :--- | :--- |
| 🔋 **Calculated Battery UPS Analytics** | Computes empirical charging & discharging speeds (`%/hr`) across dual 10-bracket tables (`0%–10%`, `10%–20%`... `100%`) without relying on missing ACPI registers. |
| 🚨 **Power Outage Detection** | Detects AC disconnects immediately, logs UPS mode transitions, and projects remaining runtime before emergency depletion. |
| 🏎️ **Zero-Latency 60 FPS TUI** | Telemetry fetching (`docker stats`, qBittorrent, sensors) runs decoupled on a background worker thread; key navigation and scrolling respond with **0ms latency**. |
| 📊 **Normalized CPU (Solaris Mode)** | Process CPU usage is normalized across all logical cores (`0.0% – 100.0%`), directly matching the system load gauge. |
| 🌀 **Intelligent Fan State & RPM** | Multi-source fan resolver supporting hardware tachometers, HP Embedded Controller (EC) register decoding, and ACPI cooling device fallbacks (`Auto (Silent)` / `Auto (Active)`). |
| 🐳 **Docker & Torrent Telemetry** | Real-time container statuses, CPU/memory consumption, and active download metrics without external monitoring daemons. |
| 👤 **SSH Security Chronicle** | Tracks active sessions, client IPs (including Tailscale mesh nodes), and session lifetimes. |
| 🤖 **1-Keypress AI Context Export (`e`)** | Generates a clean, comprehensive Markdown summary of the day's server operations, peak resource loads, and container metrics ready for AI paste. |
| ⚙️ **Ultra-Lightweight Daemon** | Runs silently under `systemd --user` with linger enabled, consuming under **2 MB of RAM**. |

---

## 🚀 Quick Start

### 🪄 One-Liner Install

```bash
curl -fsSL https://raw.githubusercontent.com/Praveensenpai/server-chronicle/main/remote-install.sh | bash
```

### 📦 Or Install via Cargo

```bash
cargo install --git https://github.com/Praveensenpai/server-chronicle
```

---

## 💻 How to Use

`server-chronicle` is designed to be both an **interactive terminal cockpit** and an **unattended background observer**.

### 1. Launch the Interactive Dashboard

To open the full-screen terminal dashboard:

```bash
server-chronicle
# or explicitly:
server-chronicle status
```

### 2. Enable the Background Logging Daemon

Run the daemon in the background to continuously record battery discharge milestones, container states, SSH logins, and resource spikes into daily JSONL logs:

```bash
# Automatically installs and enables the systemd user service:
server-chronicle install-service
```

Verify service operation:
```bash
systemctl --user status server-chronicle.service
journalctl --user -u server-chronicle.service -f
```

### 3. Generate AI Context Prompts (`export`)

Need to feed your server's current status, power trends, and recent events to an AI assistant?

```bash
# Print formatted Markdown directly to stdout:
server-chronicle export

# Pipe directly to system clipboard (Wayland):
server-chronicle export | wl-copy

# Pipe directly to system clipboard (X11):
server-chronicle export | xclip -selection clipboard
```

### 4. Quick Terminal Battery Snapshot

Inspect battery UPS status, charge/discharge rates, and health without launching the full TUI:

```bash
server-chronicle battery
```

---

## ⌨️ Interactive TUI Controls

| Key | Action | Description |
| :---: | :--- | :--- |
| **`Tab`** | **Cycle Views** | Rotate sequentially through **Telemetry** ➔ **Battery UPS** ➔ **Chronicle** |
| **`1`** | **Telemetry View** | Direct jump to Host metrics, CPU core temps, fan speed, Docker containers & Top Processes |
| **`2`** | **Battery UPS View** | Direct jump to live charging/discharging speeds and dual 10-bracket historical rate tables |
| **`3`** | **Chronicle View** | Direct jump to searchable daily event log & timeline |
| **`e`** | **Instant AI Export** | Formats today's activity into Markdown and copies to clipboard or disk |
| **`K`** | **Terminate Process** | Opens red confirmation modal to send `SIGTERM` to selected process *(Telemetry tab)* |
| **`/`** | **Fuzzy Search** | Filter today's timeline events in real-time *(Chronicle tab)* |
| **`j` / `↓`** | **Select Next** | Move selection down through processes or scroll timeline |
| **`k` / `↑`** | **Select Previous** | Move selection up through processes or scroll timeline |
| **`q` / `Esc`** | **Exit TUI** | Return to terminal shell |

---

## 🌀 Hardware Fan Speed Setup (HP & ACPI)

### Standard Motherboards & Laptops
On hardware with standard hwmon tachometer sensors (ThinkPad, Dell, desktop motherboards), fan RPM works out-of-the-box with **zero extra configuration**.

### Consumer Laptops (HP 14q / Pavilion / EliteBook)
On HP laptops, the BIOS does not expose tachometer readings to standard `hwmon`. By default, Server Chronicle uses the kernel's **ACPI Cooling Device** subsystem, displaying:
- **`🌀 Auto (Silent)`**: Fan idle or off.
- **`🌀 Auto (Active)`**: Fan actively spinning under thermal load.

#### Unlocking Live Numerical RPM on HP Laptops
To read live numerical RPM (e.g. `2520 RPM`) directly from the HP Embedded Controller (EC) register `0x70`:

1. **Enable the `ec_sys` kernel module on boot**:
   ```bash
   echo "ec_sys" | sudo tee /etc/modules-load.d/ec_sys.conf
   echo "options ec_sys write_support=1" | sudo tee /etc/modprobe.d/ec_sys.conf
   sudo modprobe ec_sys write_support=1
   ```

2. **Grant read permissions to EC debugfs**:
   ```bash
   sudo chmod 755 /sys/kernel/debug
   sudo chmod a+r /sys/kernel/debug/ec/ec0/io
   ```

Server Chronicle will automatically detect `/sys/kernel/debug/ec/ec0/io` and decode the little-endian register into live RPM!

---

## 📁 Storage & Systemd Service

- **Daemon Status**: `systemctl --user status server-chronicle.service`
- **Daily JSONL Logs**: `~/.local/share/server-chronicle/logs/activity-YYYY-MM-DD.jsonl`
- **Battery Tracker Database**: `~/.local/share/server-chronicle/battery_tracker.json`
- **Binary Locations**: `~/.local/bin/server-chronicle` and `~/.cargo/bin/server-chronicle`

---

## 📄 License

MIT License © [Praveensenpai](https://github.com/Praveensenpai)
