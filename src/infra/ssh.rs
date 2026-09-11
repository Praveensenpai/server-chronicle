use std::process::Command;

use crate::domain::models::SshSession;

pub fn read_active_ssh_sessions() -> Vec<SshSession> {
    let mut sessions = Vec::new();

    // 1. First check `w -h`
    if let Ok(out) = Command::new("w").args(["-h"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let user = parts[0].to_string();
                    let tty = parts[1].to_string();
                    let ip = parts[2].trim_matches(|c| c == '(' || c == ')').to_string();
                    let connected = parts[3].to_string();
                    sessions.push(SshSession {
                        user,
                        client_ip: ip,
                        tty_or_port: tty,
                        connected_at: connected,
                    });
                }
            }
        }
    }

    if !sessions.is_empty() {
        return sessions;
    }

    // 2. Fallback to `ss -tn state established '( sport = :22 )'`
    if let Ok(out) = Command::new("ss")
        .args(["-tn", "state", "established", "(", "sport", "=", ":22", ")"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let remote = parts[3].to_string();
                    let (ip, port) = remote.rsplit_once(':').unwrap_or((remote.as_str(), ""));
                    sessions.push(SshSession {
                        user: "ssh-client".to_string(),
                        client_ip: ip.to_string(),
                        tty_or_port: format!("port {port}"),
                        connected_at: "active".to_string(),
                    });
                }
            }
        }
    }

    sessions
}
