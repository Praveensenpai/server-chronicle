use reqwest::blocking::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::domain::models::TorrentSnapshot;

#[derive(Debug, Deserialize)]
struct QbitTorrentItem {
    name: Option<String>,
    progress: Option<f64>,
    state: Option<String>,
    dlspeed: Option<u64>,
    upspeed: Option<u64>,
    size: Option<u64>,
}

pub fn read_torrents() -> Vec<TorrentSnapshot> {
    let client = Client::builder()
        .timeout(Duration::from_millis(600))
        .build();

    let Ok(c) = client else {
        return Vec::new();
    };

    let Ok(resp) = c.get("http://127.0.0.1:6881/api/v2/torrents/info").send() else {
        return Vec::new();
    };

    if !resp.status().is_success() {
        return Vec::new();
    }

    let Ok(items) = resp.json::<Vec<QbitTorrentItem>>() else {
        return Vec::new();
    };

    items
        .into_iter()
        .map(|t| TorrentSnapshot {
            name: t.name.unwrap_or_else(|| "Unknown".to_string()),
            progress: (t.progress.unwrap_or(0.0) * 100.0).min(100.0),
            state: t.state.unwrap_or_else(|| "idle".to_string()),
            dlspeed: t.dlspeed.unwrap_or(0),
            upspeed: t.upspeed.unwrap_or(0),
            size: t.size.unwrap_or(0),
        })
        .collect()
}
