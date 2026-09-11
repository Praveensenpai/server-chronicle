use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn install_user_service() -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let user_dir = Path::new(&home).join(".config/systemd/user");
    fs::create_dir_all(&user_dir)?;

    let service_file = user_dir.join("server-chronicle.service");
    let bin = Path::new(&home).join(".local/bin/server-chronicle");
    let bin_path = if bin.exists() {
        bin.display().to_string()
    } else {
        std::env::current_exe()?.display().to_string()
    };

    let content = format!(
        "[Unit]\n\
        Description=Server Chronicle Background Logger and Battery UPS Monitor\n\
        After=network.target\n\n\
        [Service]\n\
        Type=simple\n\
        ExecStart={bin_path} daemon --interval 5\n\
        Restart=always\n\
        RestartSec=5\n\n\
        [Install]\n\
        WantedBy=default.target\n"
    );

    fs::write(&service_file, content)
        .context("Failed to write server-chronicle.service unit file")?;

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();
    let _ = Command::new("systemctl")
        .args(["--user", "enable", "--now", "server-chronicle.service"])
        .output();

    if let Ok(user) = std::env::var("USER") {
        let _ = Command::new("loginctl")
            .args(["enable-linger", &user])
            .output();
    }

    println!("  ✔ Installed and started user systemd service: server-chronicle.service");
    println!("  ✔ Persistent user lingering enabled for headless boot");
    Ok(())
}
