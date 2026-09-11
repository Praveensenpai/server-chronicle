<div align="center">

# ⏱️ サーバー・クロニクル • SERVER-CHRONICLE
### *High-Performance Server Activity Logger, Battery UPS Monitor & AI Context Generator*

[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Ubuntu_Server-E95420.svg?style=flat-square&logo=ubuntu)](https://ubuntu.com)
[![Systemd](https://img.shields.io/badge/Daemon-Systemd_User_Unit-blue.svg?style=flat-square&logo=systemd)](https://systemd.io)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)](LICENSE)

*Tailored for headless Ubuntu servers, self-hosted media setups, and laptop home servers acting as built-in UPS systems.*

</div>

---

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                       ⏱️ SERVER-CHRONICLE ARCHITECTURE                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   [ ⚡ Hardware / ACPI ] ──➔ [ Battery UPS Engine ] ──➔ [ Charge Brackets ] │
│   (HP /sys/class/power)       (Speed: %/hr over Δt)     (0-10%, 10-20%...)  │
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
│   │ 📊 Interactive Ratatui    │               │ 🤖 1-Keypress AI Exporter │ │
│   │   • 🖥️ Live Telemetry     │               │   • Markdown Prompt Gen   │ │
│   │   • 🔋 Battery UPS        │               │   • Clipboard Pipe (wl)   │ │
│   │   • 📜 Daily Timeline     │               │   • Audit Trail Sync      │ │
│   └───────────────────────────┘               └───────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## ✨ Key Features

| Feature | Description |
| :--- | :--- |
| 🔋 **Calculated Battery UPS Analytics** | Computes empirical charging & discharging speeds (`%/hr`) and bracket metrics (`0%–10%`, `10%–20%`... `100%`) without relying on missing ACPI `current_now` registers. |
| 🚨 **Power Outage Detection** | Detects AC disconnects immediately, logs UPS mode transition, and calculates remaining runtime minutes before emergency threshold. |
| 🐳 **Docker Container Telemetry** | Real-time monitoring of container states, CPU%, and memory usage (e.g. `qbittorrent`) without heavy external daemons. |
| 👤 **SSH Security Chronicle** | Tracks active sessions, client IPs (including Tailscale mesh nodes), and session lifetimes. |
| 📊 **3-Tab Ratatui TUI** | Dedicated views for Live Telemetry, Battery UPS Breakdown, and Daily Activity Timeline with real-time fuzzy filtering (`/`). |
| 🤖 **1-Keypress AI Context Export (`e`)** | Generates a clean, comprehensive Markdown summary of the day's server operations, peak resource loads, and container metrics ready for AI. |
| ⚙️ **Ultra-Lightweight Daemon** | Runs silently under `systemd --user` with linger enabled, consuming under **1 MB of RAM**. |

---

## 🚀 Quick Start

### 🪄 One-Liner Install

```bash
curl -fsSL https://raw.githubusercontent.com/Praveensenpai/server-chronicle/main/remote-install.sh | bash
```

---

## ⌨️ Interactive TUI Controls

Launch the interactive dashboard anytime:

```bash
server-chronicle status
```

| Key | Action | Description |
| :---: | :--- | :--- |
| **`Tab`** | **Cycle Views** | Switch between **Live Telemetry** ➔ **Battery UPS** ➔ **Daily Chronicle** |
| **`e`** | **Instant AI Export** | Formats today's activity into Markdown and copies to clipboard or disk |
| **`K`** | **Terminate Process** | Opens red confirmation modal to send `SIGTERM` to selected process *(Telemetry tab)* |
| **`/`** | **Fuzzy Search** | Filter today's timeline events in real-time *(Chronicle tab)* |
| **`j` / `↓`** | **Select Next** | Move selection down |
| **`k` / `↑`** | **Select Previous** | Move selection up |
| **`q` / `Esc`** | **Exit TUI** | Return to terminal shell |

---

## 💻 CLI Commands

```bash
# Launch interactive Ratatui dashboard
server-chronicle status

# Quick battery UPS & charging rate breakdown in terminal
server-chronicle battery

# Export today's full audit report to stdout (pipe to clipboard or AI)
server-chronicle export

# Run background monitoring daemon in foreground
server-chronicle daemon --interval 5

# Install and enable background user systemd unit
server-chronicle install-service
```

---

## 📁 Storage & Systemd Service

- **Daemon Status**: `systemctl --user status server-chronicle.service`
- **Daily JSONL Logs**: `~/.local/share/server-chronicle/logs/activity-YYYY-MM-DD.jsonl`
- **Battery Tracker**: `~/.local/share/server-chronicle/battery_tracker.json`

---

## 📄 License

MIT License © [Praveensenpai](https://github.com/Praveensenpai)
