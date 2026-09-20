use std::fs;
use std::mem::MaybeUninit;
use std::process::Command;

use crate::domain::models::{ProcessItem, SystemMetrics};

pub fn read_system_metrics() -> SystemMetrics {
    let hostname = read_hostname();
    let load_avg = read_load_avg();
    let uptime_secs = read_uptime_secs();
    let (mem_used_bytes, mem_total_bytes, swap_used_bytes, swap_total_bytes) = read_meminfo();
    let (disk_used_bytes, disk_total_bytes) = read_disk_space("/");
    let cpu_percent = read_cpu_percent();
    let cpu_temp_c = read_cpu_temperature();
    let cpu_core_temps_c = read_cpu_core_temps();
    let (fan_speed_rpm, fan_status) = read_fan_info();

    SystemMetrics {
        cpu_percent,
        cpu_temp_c,
        cpu_core_temps_c,
        fan_speed_rpm,
        fan_status,
        mem_used_bytes,
        mem_total_bytes,
        swap_used_bytes,
        swap_total_bytes,
        disk_used_bytes,
        disk_total_bytes,
        load_avg,
        uptime_secs,
        hostname,
    }
}

use crate::infra::thermal::{read_cpu_core_temps, read_cpu_temperature, read_fan_info};

fn read_hostname() -> String {
    Command::new("hostname").output().map_or_else(
        |_| "ubuntu-server".to_string(),
        |o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                "ubuntu-server".to_string()
            } else {
                s
            }
        },
    )
}

fn read_load_avg() -> (f64, f64, f64) {
    if let Ok(content) = fs::read_to_string("/proc/loadavg") {
        let mut parts = content.split_whitespace();
        let l1 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0.0);
        let l5 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0.0);
        let l15 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0.0);
        return (l1, l5, l15);
    }
    (0.0, 0.0, 0.0)
}

fn read_uptime_secs() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/uptime") {
        if let Some(sec_str) = content.split_whitespace().next() {
            let int_part = sec_str.split('.').next().unwrap_or(sec_str);
            if let Ok(s) = int_part.parse::<u64>() {
                return s;
            }
        }
    }
    0
}

fn read_meminfo() -> (u64, u64, u64, u64) {
    let Ok(content) = fs::read_to_string("/proc/meminfo") else {
        return (0, 0, 0, 0);
    };

    let mut mem_total_kb = 0u64;
    let mut mem_avail_kb = 0u64;
    let mut swap_total_kb = 0u64;
    let mut swap_free_kb = 0u64;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            mem_total_kb = parse_mem_line(line);
        } else if line.starts_with("MemAvailable:") {
            mem_avail_kb = parse_mem_line(line);
        } else if line.starts_with("SwapTotal:") {
            swap_total_kb = parse_mem_line(line);
        } else if line.starts_with("SwapFree:") {
            swap_free_kb = parse_mem_line(line);
        }
    }

    let mem_total = mem_total_kb.saturating_mul(1024);
    let mem_used = mem_total.saturating_sub(mem_avail_kb.saturating_mul(1024));
    let swap_total = swap_total_kb.saturating_mul(1024);
    let swap_used = swap_total.saturating_sub(swap_free_kb.saturating_mul(1024));

    (mem_used, mem_total, swap_used, swap_total)
}

fn parse_mem_line(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

fn read_disk_space(mount: &str) -> (u64, u64) {
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    let Ok(c_mount) = std::ffi::CString::new(mount) else {
        return (0, 0);
    };
    let res = unsafe { libc::statvfs(c_mount.as_ptr(), stat.as_mut_ptr()) };
    if res != 0 {
        return (0, 0);
    }
    let stat = unsafe { stat.assume_init() };
    let total = stat.f_blocks.saturating_mul(stat.f_frsize);
    let avail = stat.f_bavail.saturating_mul(stat.f_frsize);
    let used = total.saturating_sub(avail);
    (used, total)
}

static LAST_CPU_JIFFIES: std::sync::Mutex<(u64, u64)> = std::sync::Mutex::new((0, 0));

#[must_use]
pub fn parse_cpu_stat_line(line: &str) -> Option<(u64, u64)> {
    if !line.starts_with("cpu ") {
        return None;
    }
    let parts: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|p| p.parse::<u64>().ok())
        .collect();

    if parts.len() < 4 {
        return None;
    }

    let idle = parts[3] + parts.get(4).copied().unwrap_or(0);
    let total: u64 = parts.iter().sum();
    let work = total.saturating_sub(idle);
    Some((work, total))
}

fn read_cpu_percent() -> f64 {
    let Ok(content) = fs::read_to_string("/proc/stat") else {
        return 0.0;
    };
    let Some(first_line) = content.lines().next() else {
        return 0.0;
    };
    let Some((work, total)) = parse_cpu_stat_line(first_line) else {
        return 0.0;
    };

    let mut lock = match LAST_CPU_JIFFIES.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    let (prev_work, prev_total) = *lock;
    *lock = (work, total);

    if prev_total == 0 || total <= prev_total {
        return 0.0;
    }

    let delta_total = total - prev_total;
    let delta_work = work.saturating_sub(prev_work);

    ((delta_work as f64 / delta_total as f64) * 100.0).clamp(0.0, 100.0)
}

pub fn normalize_process_cpu(raw_cpu: f64, num_cpus: usize) -> f64 {
    let divisor = num_cpus.max(1) as f64;
    (raw_cpu / divisor).clamp(0.0, 100.0)
}

pub fn resolve_process_name(comm: &str, args: &str) -> String {
    let lower = comm.to_lowercase();
    let is_interpreter = lower.starts_with("python")
        || lower == "node"
        || lower == "ruby"
        || lower == "perl"
        || lower == "bash"
        || lower == "sh";

    if !is_interpreter || args.trim().is_empty() {
        return comm.to_string();
    }

    let tokens: Vec<&str> = args.split_whitespace().collect();
    for (i, token) in tokens.iter().enumerate() {
        if *token == "-m" {
            if let Some(module) = tokens.get(i + 1) {
                return format!("{comm}: {module}");
            }
        }
    }

    for token in tokens.iter().skip(1) {
        if token.starts_with('-') {
            continue;
        }
        let file_name = std::path::Path::new(token)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(token);
        return format!("{comm}: {file_name}");
    }

    comm.to_string()
}

pub fn parse_process_line(line: &str, num_cpus: usize) -> Option<ProcessItem> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let pid = parts[0].parse::<u32>().ok()?;
    let raw_cpu = parts[1].parse::<f64>().unwrap_or(0.0);
    let mem_percent = parts[2].parse::<f64>().unwrap_or(0.0);
    let rss_kb = parts[3].parse::<u64>().unwrap_or(0);
    let comm = parts[4];
    let args = if parts.len() > 5 {
        parts[5..].join(" ")
    } else {
        String::new()
    };
    let name = resolve_process_name(comm, &args);
    let cpu_percent = normalize_process_cpu(raw_cpu, num_cpus);
    let mem_bytes = rss_kb.saturating_mul(1024);

    Some(ProcessItem {
        pid,
        name,
        cpu_percent,
        mem_percent,
        mem_bytes,
    })
}

fn fetch_ps_processes(sort_arg: &str, sample_size: usize, num_cpus: usize) -> Vec<ProcessItem> {
    let Ok(out) = Command::new("ps")
        .args(["-eo", "pid,%cpu,%mem,rss,comm,args", sort_arg])
        .output()
    else {
        return Vec::new();
    };

    let s = String::from_utf8_lossy(&out.stdout);
    s.lines()
        .skip(1)
        .take(sample_size)
        .filter_map(|line| parse_process_line(line, num_cpus))
        .collect()
}

pub fn read_top_processes(limit: usize) -> Vec<ProcessItem> {
    let num_cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut map = std::collections::HashMap::new();

    for proc in fetch_ps_processes("--sort=-%cpu", limit, num_cpus) {
        map.insert(proc.pid, proc);
    }
    for proc in fetch_ps_processes("--sort=-rss", limit, num_cpus) {
        map.entry(proc.pid).or_insert(proc);
    }

    let mut items: Vec<ProcessItem> = map.into_values().collect();
    items.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent));
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mem_line() {
        let line = "MemTotal:       16307136 kB";
        assert_eq!(parse_mem_line(line), 16307136);

        let bogus = "InvalidLine";
        assert_eq!(parse_mem_line(bogus), 0);
    }

    #[test]
    fn test_parse_cpu_stat_line() {
        let line = "cpu  53508 0 6772 97024 572 0 265 0 0 0";
        let (work, total) = parse_cpu_stat_line(line).expect("must parse valid cpu line");
        assert_eq!(total, 53508 + 6772 + 97024 + 572 + 265);
        let idle = 97024 + 572;
        assert_eq!(work, total - idle);

        let invalid = "cpu0 100 200";
        assert!(parse_cpu_stat_line(invalid).is_none());
    }

    #[test]
    fn test_normalize_process_cpu() {
        // 150% raw on 4 cores = 37.5% total host CPU
        assert!((normalize_process_cpu(150.0, 4) - 37.5).abs() < 0.01);
        // 100% raw on 4 cores = 25.0%
        assert!((normalize_process_cpu(100.0, 4) - 25.0).abs() < 0.01);
        // Edge cases
        assert_eq!(normalize_process_cpu(500.0, 4), 100.0);
        assert_eq!(normalize_process_cpu(50.0, 0), 50.0);
    }

    #[test]
    fn test_resolve_process_name() {
        assert_eq!(
            resolve_process_name(
                "python3",
                "/home/neko/spotiflac-bot/.venv/bin/python3 -m spotiflac_bot"
            ),
            "python3: spotiflac_bot"
        );
        assert_eq!(
            resolve_process_name("python3", "python3 /var/www/script.py --verbose"),
            "python3: script.py"
        );
        assert_eq!(
            resolve_process_name("node", "node /app/server.js"),
            "node: server.js"
        );
        assert_eq!(
            resolve_process_name("jellyfin", "/jellyfin/jellyfin"),
            "jellyfin"
        );
    }

    #[test]
    fn test_parse_process_line() {
        let line = "2471 1.2 4.6 344052 jellyfin /jellyfin/jellyfin";
        let proc = parse_process_line(line, 1).expect("must parse valid ps line");
        assert_eq!(proc.pid, 2471);
        assert_eq!(proc.name, "jellyfin");
        assert!((proc.cpu_percent - 1.2).abs() < 0.01);
        assert!((proc.mem_percent - 4.6).abs() < 0.01);
        assert_eq!(proc.mem_bytes, 344052 * 1024);

        let py_line = "163989 0.0 2.9 223436 python3 /venv/bin/python3 -m spotiflac_bot";
        let proc2 = parse_process_line(py_line, 1).expect("must parse python line");
        assert_eq!(proc2.name, "python3: spotiflac_bot");
        assert_eq!(proc2.mem_bytes, 223436 * 1024);

        assert!(parse_process_line("invalid", 1).is_none());
    }
}
