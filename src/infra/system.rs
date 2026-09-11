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

fn read_cpu_temperature() -> Option<f64> {
    // Prefer hwmon coretemp (most accurate on x86 — reads the CPU package sensor directly)
    if let Some(t) = read_coretemp_package() {
        return Some(t);
    }

    // Fall back to thermal_zone scanning
    let entries = fs::read_dir("/sys/class/thermal").ok()?;
    let mut selected: Option<(i32, f64)> = None;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("thermal_zone") {
            continue;
        }

        let Ok(raw_value) = fs::read_to_string(entry.path().join("temp")) else {
            continue;
        };
        let Ok(value) = raw_value.trim().parse::<f64>() else {
            continue;
        };
        // Kernel reports millidegrees on most platforms
        let temperature = if value.abs() > 200.0 {
            value / 1000.0
        } else {
            value
        };
        // Skip obviously bogus readings
        if !(0.0..=120.0).contains(&temperature) {
            continue;
        }
        let sensor_type = fs::read_to_string(entry.path().join("type")).unwrap_or_default();
        let sensor_type = sensor_type.trim().to_ascii_lowercase();
        let score = if sensor_type.contains("package") {
            5
        } else if sensor_type == "x86_pkg_temp" {
            5
        } else if sensor_type.contains("x86") {
            4
        } else if sensor_type.contains("cpu") || sensor_type.contains("core") {
            3
        } else if sensor_type.contains("acpitz") {
            2
        } else {
            1
        };

        if selected.is_none_or(|(current_score, _)| score > current_score) {
            selected = Some((score, temperature));
        }
    }

    selected.map(|(_, temperature)| temperature)
}

/// Read per-core temperatures from hwmon coretemp (temp2_input, temp3_input, …).
/// temp1_input is the package sensor; temp2+ are individual cores.
pub fn read_cpu_core_temps() -> Vec<f64> {
    let Ok(entries) = fs::read_dir("/sys/class/hwmon") else {
        return Vec::new();
    };
    for hwmon in entries.flatten() {
        let name = fs::read_to_string(hwmon.path().join("name")).unwrap_or_default();
        if !name.trim().eq_ignore_ascii_case("coretemp") {
            continue;
        }
        let mut cores: Vec<(u32, f64)> = Vec::new();
        let Ok(files) = fs::read_dir(hwmon.path()) else {
            continue;
        };
        for f in files.flatten() {
            let fname = f.file_name().to_string_lossy().to_string();
            // tempN_input where N >= 2 are individual cores (N=1 is the package)
            if let Some(rest) = fname.strip_prefix("temp") {
                if let Some(idx_str) = rest.strip_suffix("_input") {
                    if let Ok(idx) = idx_str.parse::<u32>() {
                        if idx >= 2 {
                            if let Ok(raw) = fs::read_to_string(f.path()) {
                                if let Ok(v) = raw.trim().parse::<f64>() {
                                    let temp = if v > 200.0 { v / 1000.0 } else { v };
                                    if (0.0..=120.0).contains(&temp) {
                                        cores.push((idx, temp));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        cores.sort_by_key(|(idx, _)| *idx);
        return cores.into_iter().map(|(_, t)| t).collect();
    }
    Vec::new()
}

/// Read the CPU package temperature from hwmon coretemp driver.
/// Returns the highest temp1_input (package) from any coretemp hwmon device.
fn read_coretemp_package() -> Option<f64> {
    let entries = fs::read_dir("/sys/class/hwmon").ok()?;
    for hwmon in entries.flatten() {
        let name_path = hwmon.path().join("name");
        let name = fs::read_to_string(&name_path).unwrap_or_default();
        if !name.trim().eq_ignore_ascii_case("coretemp") {
            continue;
        }
        // temp1_input is conventionally the package-level sensor in coretemp
        if let Ok(raw) = fs::read_to_string(hwmon.path().join("temp1_input")) {
            if let Ok(v) = raw.trim().parse::<f64>() {
                let temp = if v > 200.0 { v / 1000.0 } else { v };
                if (0.0..=120.0).contains(&temp) {
                    return Some(temp);
                }
            }
        }
    }
    None
}

fn read_fan_speed() -> Option<u32> {
    // Primary: read fan RPM directly from the Embedded Controller (EC) registers.
    // On the HP Laptop 14q-cs0xxx, the standard hwmon/hp driver exposes only
    // pwm1_enable with no fan tachometer input — the EC holds the actual RPM at
    // offset 0x70–0x71 as a 16-bit big-endian value.
    if let Some(rpm) = read_fan_speed_from_ec() {
        return Some(rpm);
    }

    // Fallback: scan hwmon fan*_input files (works on other hardware).
    let hwmons = fs::read_dir("/sys/class/hwmon").ok()?;
    let mut max_rpm: Option<u32> = None;

    for hwmon in hwmons.flatten() {
        let base = hwmon.path();
        // Some kernels expose sensors under hwmonN/device/, others directly under hwmonN/
        let search_dirs = [base.clone(), base.join("device")];
        for dir in &search_dirs {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with("fan") && fname.ends_with("_input") {
                    if let Ok(raw) = fs::read_to_string(entry.path()) {
                        if let Ok(rpm) = raw.trim().parse::<u32>() {
                            if rpm > 0 {
                                max_rpm = Some(max_rpm.map_or(rpm, |cur| cur.max(rpm)));
                            }
                        }
                    }
                }
            }
        }
    }

    max_rpm
}

/// Read fan RPM from the HP Embedded Controller register space.
///
/// The EC IO space is exposed at `/sys/kernel/debug/ec/ec0/io` (256 bytes) by
/// the `ec_sys` kernel module.  On the HP 14q-cs0xxx the fan tachometer is a
/// 16-bit big-endian value stored at offset 0x70.  A value of 0 means the fan
/// has stopped (or the EC hasn't populated the register yet), so we return
/// `None` in that case to avoid displaying a misleading zero.
fn read_fan_speed_from_ec() -> Option<u32> {
    const EC_IO_PATH: &str = "/sys/kernel/debug/ec/ec0/io";
    const FAN_OFFSET: usize = 0x70;

    let data = fs::read(EC_IO_PATH).ok()?;
    if data.len() <= FAN_OFFSET + 1 {
        return None;
    }

    let high = data[FAN_OFFSET] as u32;
    let low = data[FAN_OFFSET + 1] as u32;
    let rpm = (high << 8) | low; // big-endian u16

    if rpm == 0 || rpm > 20_000 {
        // 0 = fan stopped or register unpopulated; >20 000 is clearly bogus
        return None;
    }

    Some(rpm)
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
