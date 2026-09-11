use std::fs;

pub fn read_cpu_temperature() -> Option<f64> {
    if let Some(t) = read_coretemp_package() {
        return Some(t);
    }

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
        let temperature = if value.abs() > 200.0 {
            value / 1000.0
        } else {
            value
        };
        if !(0.0..=120.0).contains(&temperature) {
            continue;
        }
        let sensor_type = fs::read_to_string(entry.path().join("type")).unwrap_or_default();
        let sensor_type = sensor_type.trim().to_ascii_lowercase();
        let score = if sensor_type.contains("package") || sensor_type == "x86_pkg_temp" {
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

#[must_use]
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

fn read_coretemp_package() -> Option<f64> {
    let entries = fs::read_dir("/sys/class/hwmon").ok()?;
    for hwmon in entries.flatten() {
        let name_path = hwmon.path().join("name");
        let name = fs::read_to_string(&name_path).unwrap_or_default();
        if !name.trim().eq_ignore_ascii_case("coretemp") {
            continue;
        }
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

pub fn read_fan_speed() -> Option<u32> {
    read_fan_speed_from_hwmon().or_else(read_fan_speed_from_ec)
}

pub fn read_fan_info() -> (Option<u32>, String) {
    if let Some(rpm) = read_fan_speed() {
        return (Some(rpm), format!("{rpm} RPM"));
    }

    if let Some(acpi_status) = read_acpi_cooling_fan_status() {
        return (None, acpi_status);
    }

    if let Some(pwm_status) = read_hwmon_pwm_status() {
        return (None, pwm_status);
    }

    (None, "N/A".to_string())
}

fn read_fan_speed_from_hwmon() -> Option<u32> {
    let hwmons = fs::read_dir("/sys/class/hwmon").ok()?;
    let mut max_rpm: Option<u32> = None;

    for hwmon in hwmons.flatten() {
        let base = hwmon.path();
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

fn read_acpi_cooling_fan_status() -> Option<String> {
    let entries = fs::read_dir("/sys/class/thermal").ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.starts_with("cooling_device") {
            continue;
        }

        let type_path = path.join("type");
        let Ok(type_str) = fs::read_to_string(type_path) else {
            continue;
        };

        if type_str.trim().eq_ignore_ascii_case("fan") {
            let cur_state = fs::read_to_string(path.join("cur_state"))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(0);

            let status = if cur_state > 0 {
                "Auto (Active)".to_string()
            } else {
                "Auto (Silent)".to_string()
            };
            return Some(status);
        }
    }

    None
}

fn read_hwmon_pwm_status() -> Option<String> {
    let hwmons = fs::read_dir("/sys/class/hwmon").ok()?;

    for hwmon in hwmons.flatten() {
        let pwm_enable_path = hwmon.path().join("pwm1_enable");
        if let Ok(val) = fs::read_to_string(pwm_enable_path) {
            if val.trim() == "2" {
                return Some("Auto (BIOS)".to_string());
            }
        }
    }

    None
}

fn read_fan_speed_from_ec() -> Option<u32> {
    const EC_IO_PATH: &str = "/sys/kernel/debug/ec/ec0/io";
    const FAN_OFFSET: usize = 0x70;

    let data = fs::read(EC_IO_PATH).ok()?;
    if data.len() <= FAN_OFFSET + 1 {
        return None;
    }

    let high = data[FAN_OFFSET] as u32;
    let low = data[FAN_OFFSET + 1] as u32;
    let rpm = (high << 8) | low;

    if rpm == 0 || rpm > 20_000 {
        return None;
    }

    Some(rpm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_fan_info_structure() {
        let (rpm, status) = read_fan_info();
        if let Some(r) = rpm {
            assert!(r > 0);
            assert!(status.contains("RPM"));
        } else {
            assert!(!status.is_empty());
        }
    }
}
