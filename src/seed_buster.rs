use std::sync::Arc;

use dashmap::DashMap;
use transmission::Torrent;

use crate::structures::TorrentState;

pub async fn seed_buster(torrents: Arc<DashMap<i32, Torrent>>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        {
            for torrent in torrents.iter() {
                let stat = torrent.value().stats();
                if stat.percent_done >= 1.0
                    && TorrentState::from(stat.state) == TorrentState::Seeding
                {
                    torrent.value().stop();
                }
            }
        }
    }
}
