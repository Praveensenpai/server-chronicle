pub mod battery;
pub mod docker;
pub mod ssh;
pub mod storage;
pub mod system;
pub mod thermal;
pub mod torrent;

use crate::domain::models::ServerSnapshot;

pub fn capture_server_snapshot() -> ServerSnapshot {
    let system = system::read_system_metrics();
    let containers = docker::read_containers();
    let ssh_sessions = ssh::read_active_ssh_sessions();
    let torrents = torrent::read_torrents();
    let top_processes = system::read_top_processes(20);

    ServerSnapshot {
        system,
        containers,
        ssh_sessions,
        torrents,
        top_processes,
    }
}
