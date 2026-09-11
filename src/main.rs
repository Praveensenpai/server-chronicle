mod api;
mod daemon;
mod domain;
mod infra;
mod service;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "server-chronicle")]
#[command(author = "Praveensenpai <pvnt20@gmail.com>")]
#[command(version)]
#[command(about = "⏱️ Server activity logger, battery UPS monitor, and AI context generator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch interactive Ratatui TUI dashboard (default)
    Status,
    /// Run background activity & battery UPS logger daemon
    Daemon {
        /// Sampling interval in seconds
        #[arg(short, long, default_value_t = 5)]
        interval: u64,
    },
    /// Export today's complete server chronicle formatted for AI prompts
    Export,
    /// Quick CLI battery UPS & charging speed snapshot
    Battery,
    /// Install & enable systemd background user service
    InstallService,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Status) {
        Commands::Status => {
            tui::run_tui()?;
        }
        Commands::Daemon { interval } => {
            daemon::run_daemon(interval)?;
        }
        Commands::Export => {
            let snap = infra::capture_server_snapshot();
            let bat = infra::battery::read_battery_snapshot();
            let events = infra::storage::read_today_events();
            let report = api::generate_ai_report(&snap, &bat, &events);
            print!("{report}");
        }
        Commands::Battery => {
            print_battery_summary();
        }
        Commands::InstallService => {
            service::install_user_service()?;
        }
    }

    Ok(())
}

fn print_battery_summary() {
    let b = infra::battery::read_battery_snapshot();
    println!("🌸 ========================================= 🌸");
    println!("     ⏱️ Server Chronicle — Battery UPS Telemetry    ");
    println!("🌸 ========================================= 🌸");
    println!("  • State:            {}", b.state.as_str());
    println!(
        "  • AC Mains Online:  {}",
        if b.ac_online {
            "Yes (Plugged In)"
        } else {
            "No (Running on Battery!)"
        }
    );
    println!("  • Current Capacity: {}%", b.capacity);
    println!(
        "  • Battery Health:   {:.1}% ({} / {} mAh)",
        b.health_percent,
        b.charge_full_uah / 1000,
        b.charge_full_design_uah / 1000
    );
    if b.calculated_rate_pct_hr > 0.01 {
        println!(
            "  • Calculated Rate:  +{:.2}% / hr (~{:.1}m per 1%)",
            b.calculated_rate_pct_hr, b.mins_per_percent
        );
    } else {
        println!("  • Calculated Rate:  Stable / Balanced");
    }
    if let Some(left) = b.estimated_minutes_left {
        let label = if b.ac_online {
            "Time to 100%"
        } else {
            "UPS Runtime Left"
        };
        println!("  • Estimated {label}: {left} minutes");
    }
    println!("\n  📈 Charging Brackets Breakdown:");
    for br in &b.brackets {
        let status = if br.completed {
            "✔ Done"
        } else if br.duration_secs > 0 {
            "▶ In Progress"
        } else {
            "Pending"
        };
        let rate_str = if br.rate_pct_per_hour > 0.01 {
            format!("{:.1}%/hr", br.rate_pct_per_hour)
        } else {
            "--".to_string()
        };
        println!(
            "    • {:12} : {:4}m {:2}s | {:10} | {}",
            br.label,
            br.duration_secs / 60,
            br.duration_secs % 60,
            rate_str,
            status
        );
    }
    println!("🌸 ========================================= 🌸");
}
