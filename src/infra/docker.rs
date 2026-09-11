use std::process::Command;

use crate::domain::models::ContainerInfo;

pub fn read_containers() -> Vec<ContainerInfo> {
    let Ok(out) = Command::new("docker")
        .args([
            "stats",
            "--no-stream",
            "--format",
            "{{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}",
        ])
        .output()
    else {
        return read_containers_fallback();
    };

    if !out.status.success() {
        return read_containers_fallback();
    }

    let s = String::from_utf8_lossy(&out.stdout);
    let mut map = std::collections::HashMap::new();

    for line in s.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let name = parts[0].trim().to_string();
            let cpu_str = parts[1].trim().trim_end_matches('%');
            let cpu_percent = cpu_str.parse::<f64>().unwrap_or(0.0);
            let mem_usage = parts[2].trim().to_string();
            map.insert(name, (cpu_percent, mem_usage));
        }
    }

    let mut result = Vec::new();
    let ps_out = Command::new("docker")
        .args(["ps", "-a", "--format", "{{.Names}}\t{{.Status}}"])
        .output();

    if let Ok(po) = ps_out {
        let ps_s = String::from_utf8_lossy(&po.stdout);
        for line in ps_s.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let name = parts[0].trim().to_string();
                let status = parts[1].trim().to_string();
                let (cpu, mem) = map.remove(&name).unwrap_or((0.0, "0B / 0B".to_string()));
                result.push(ContainerInfo {
                    name,
                    status,
                    cpu_percent: cpu,
                    memory_usage: mem,
                });
            }
        }
    }

    result
}

fn read_containers_fallback() -> Vec<ContainerInfo> {
    let Ok(out) = Command::new("docker")
        .args(["ps", "-a", "--format", "{{.Names}}\t{{.Status}}"])
        .output()
    else {
        return Vec::new();
    };

    if !out.status.success() {
        return Vec::new();
    }

    let s = String::from_utf8_lossy(&out.stdout);
    let mut result = Vec::new();

    for line in s.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            result.push(ContainerInfo {
                name: parts[0].trim().to_string(),
                status: parts[1].trim().to_string(),
                cpu_percent: 0.0,
                memory_usage: "N/A".to_string(),
            });
        }
    }
    result
}
