use dashmap::DashMap;
use std::sync::Arc;
use transmission::{Client, Torrent};

pub struct SharedData {
    pub client: Client,
    pub torrents: Arc<DashMap<i32, Torrent>>,
}
