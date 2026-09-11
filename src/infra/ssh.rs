use std::process::Command;

use crate::domain::models::SshSession;

#[must_use]
pub fn is_remote_ip(ip: &str) -> bool {
    let trimmed = ip.trim();
    if trimmed.is_empty() || trimmed == "-" || trimmed.starts_with(':') {
        return false;
    }
    trimmed.contains('.') || trimmed.contains(':')
}

#[must_use]
pub fn parse_w_line(line: &str) -> Option<SshSession> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 4 {
        let user = parts[0].to_string();
        let tty = parts[1].to_string();
        let ip = parts[2].trim_matches(|c| c == '(' || c == ')').to_string();
        let connected = parts[3].to_string();
        Some(SshSession {
            user,
            client_ip: ip,
            tty_or_port: tty,
            connected_at: connected,
        })
    } else {
        None
    }
}

pub fn read_active_ssh_sessions() -> Vec<SshSession> {
    let mut sessions = Vec::new();

    // 1. First check `w -h`
    if let Ok(out) = Command::new("w").args(["-h"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                if let Some(sess) = parse_w_line(line) {
                    if is_remote_ip(&sess.client_ip) {
                        sessions.push(sess);
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_remote_ip() {
        assert!(is_remote_ip("100.108.154.50"));
        assert!(is_remote_ip("192.168.1.10"));
        assert!(!is_remote_ip("-"));
        assert!(!is_remote_ip(":0"));
        assert!(!is_remote_ip(""));
    }

    #[test]
    fn test_parse_w_line() {
        let line = "neko     pts/0    100.108.154.50   19:49    3.00s  2.32s   ?    tmux";
        let sess = parse_w_line(line).expect("must parse w output");
        assert_eq!(sess.user, "neko");
        assert_eq!(sess.tty_or_port, "pts/0");
        assert_eq!(sess.client_ip, "100.108.154.50");
        assert_eq!(sess.connected_at, "19:49");
    }
}
