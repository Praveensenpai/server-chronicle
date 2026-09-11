use reqwest::blocking::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::domain::models::TorrentSnapshot;

const QBIT_PORTS: [u16; 2] = [6881, 8080];

#[derive(Debug, Deserialize)]
pub struct QbitTorrentItem {
    pub name: Option<String>,
    pub progress: Option<f64>,
    pub state: Option<String>,
    pub dlspeed: Option<u64>,
    pub upspeed: Option<u64>,
    pub size: Option<u64>,
}

impl From<QbitTorrentItem> for TorrentSnapshot {
    fn from(t: QbitTorrentItem) -> Self {
        Self {
            name: t.name.unwrap_or_else(|| "Unknown".to_string()),
            progress: (t.progress.unwrap_or(0.0) * 100.0).clamp(0.0, 100.0),
            state: t.state.unwrap_or_else(|| "idle".to_string()),
            dlspeed: t.dlspeed.unwrap_or(0),
            upspeed: t.upspeed.unwrap_or(0),
            size: t.size.unwrap_or(0),
        }
    }
}

pub fn read_torrents() -> Vec<TorrentSnapshot> {
    let client = Client::builder()
        .timeout(Duration::from_millis(600))
        .build();

    let Ok(c) = client else {
        return Vec::new();
    };

    for port in QBIT_PORTS {
        let url = format!("http://127.0.0.1:{port}/api/v2/torrents/info");
        if let Ok(resp) = c.get(&url).send() {
            if resp.status().is_success() {
                if let Ok(items) = resp.json::<Vec<QbitTorrentItem>>() {
                    return items.into_iter().map(Into::into).collect();
                }
            }
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qbit_item_conversion() {
        let item = QbitTorrentItem {
            name: Some("Ubuntu 24.04.iso".to_string()),
            progress: Some(0.85),
            state: Some("downloading".to_string()),
            dlspeed: Some(5_000_000),
            upspeed: Some(100_000),
            size: Some(4_000_000_000),
        };
        let snap: TorrentSnapshot = item.into();
        assert_eq!(snap.name, "Ubuntu 24.04.iso");
        assert!((snap.progress - 85.0).abs() < 0.01);
        assert_eq!(snap.state, "downloading");
        assert_eq!(snap.dlspeed, 5_000_000);
        assert_eq!(snap.size, 4_000_000_000);
    }
}
