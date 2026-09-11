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

    SystemMetrics {
        cpu_percent,
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

fn read_cpu_percent() -> f64 {
    let Ok(out) = Command::new("ps").args(["-A", "-o", "%cpu"]).output() else {
        return 0.0;
    };
    let s = String::from_utf8_lossy(&out.stdout);
    let mut total = 0.0;
    for line in s.lines().skip(1) {
        if let Ok(v) = line.trim().parse::<f64>() {
            total += v;
        }
    }
    let cpus = num_cpus();
    (total / cpus as f64).min(100.0)
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
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
