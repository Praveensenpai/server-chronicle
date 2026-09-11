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
    let fan_speed_rpm = read_fan_speed();

    SystemMetrics {
        cpu_percent,
        cpu_temp_c,
        cpu_core_temps_c,
        fan_speed_rpm,
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

use crate::infra::thermal::{read_cpu_core_temps, read_cpu_temperature, read_fan_speed};

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

pub fn read_top_processes(limit: usize) -> Vec<ProcessItem> {
    let Ok(out) = Command::new("ps")
        .args(["-eo", "pid,comm,%cpu,%mem", "--sort=-%cpu"])
        .output()
    else {
        return Vec::new();
    };

    let s = String::from_utf8_lossy(&out.stdout);
    let mut items = Vec::new();

    for line in s.lines().skip(1).take(limit) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let pid = parts[0].parse::<u32>().unwrap_or(0);
            let name = parts[1].to_string();
            let cpu_percent = parts[2].parse::<f64>().unwrap_or(0.0);
            let mem_percent = parts[3].parse::<f64>().unwrap_or(0.0);
            items.push(ProcessItem {
                pid,
                name,
                cpu_percent,
                mem_percent,
            });
        }
    }
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
}
