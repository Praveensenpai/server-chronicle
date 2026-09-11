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
    if let Some(rpm) = read_fan_speed_from_ec() {
        return Some(rpm);
    }

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
