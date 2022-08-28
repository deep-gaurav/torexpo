use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};

use async_graphql::*;
use chrono::{DateTime, Duration, Utc};
use futures_util::Stream;
use magic_crypt::MagicCryptTrait;
use serde::{Deserialize, Serialize};

use crate::{
    context::SharedData,
    torrent_struc::{TorrentInfo, TorrentStats},
    DOWNLOAD_DIR, MCRYPT,
};

pub type MainSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    pub async fn add_magnet_link<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        magnet_link: String,
    ) -> Result<i32> {
        let data = ctx.data::<SharedData>()?;
        let torrent = data.client.add_torrent_magnet(&magnet_link)?;
        let id = torrent.id();
        data.torrents.insert(id, torrent);
        Ok(id)
    }

    pub async fn add_torrent_file<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        torrent: Upload,
    ) -> Result<i32> {
        let torrent_file = torrent.value(ctx)?;
        let tmpdir = tempfile::tempdir()?;
        let path = tmpdir.path().join(&torrent_file.filename);
        let mut file = std::fs::File::open(&path)?;
        // let bufWrite = BufWriter::new(file);
        let _res = tokio::task::spawn_blocking(move || {
            let mut read = torrent_file.into_read();
            std::io::copy(&mut read, &mut file)
        })
        .await??;
        let data = ctx.data::<SharedData>()?;
        let torrent = data
            .client
            .add_torrent_file(path.to_str().ok_or("Not valid path")?)?;
        let id = torrent.id();
        data.torrents.insert(id, torrent);
        Ok(id)
    }

    pub async fn remove<'ctx>(&self, ctx: &Context<'ctx>, torrent_id: i32) -> Result<String> {
        let data = ctx.data::<SharedData>()?;
        if let Some((_id, torrent)) = data.torrents.remove(&torrent_id) {
            torrent.remove(true);
            Ok("success".into())
        } else {
            Err("Torrent not found".into())
        }
    }

    pub async fn start<'ctx>(&self, ctx: &Context<'ctx>, torrent_id: i32) -> Result<String> {
        let data = ctx.data::<SharedData>()?;
        if let Some(torrent) = &data.torrents.get(&torrent_id) {
            torrent.start();
            Ok("success".into())
        } else {
            Err("Torrent not found".into())
        }
    }

    pub async fn stop<'ctx>(&self, ctx: &Context<'ctx>, torrent_id: i32) -> Result<String> {
        let data = ctx.data::<SharedData>()?;
        if let Some(torrent) = &data.torrents.get(&torrent_id) {
            torrent.stop();
            Ok("success".into())
        } else {
            Err("Torrent not found".into())
        }
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn torrents<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Vec<Torrent>> {
        let data = ctx.data::<SharedData>()?;
        let torrents = data
            .torrents
            .iter()
            .map(|f| f.value().clone())
            .map(|f| Torrent { torrent: f })
            .collect::<Vec<_>>();
        Ok(torrents)
    }
    async fn torrent<'ctx>(&self, ctx: &Context<'ctx>, torrent_id: i32) -> Result<Option<Torrent>> {
        let data = ctx.data::<SharedData>()?;
        let torrent = data.torrents.get(&torrent_id).map(|t| Torrent {
            torrent: t.value().clone(),
        });
        Ok(torrent)
    }
}

pub struct Torrent {
    pub torrent: transmission::Torrent,
}

#[Object]
impl Torrent {
    async fn id(&self) -> i32 {
        self.torrent.id()
    }

    async fn name(&self) -> Result<String> {
        Ok(self.torrent.name().into())
    }

    async fn state(&self) -> Result<TorrentState> {
        let status = self.torrent.stats().state;
        Ok(status.into())
    }

    async fn info(&self) -> Result<TorrentInfo> {
        Ok(self.torrent.info().into())
    }

    // async fn set_seed_ratio(&self, ratio: f64) -> Result<String> {
    //     self.torrent.clone().set_ratio(ratio);
    //     Ok("success".into())
    // }

    async fn stats(&self) -> Result<TorrentStats> {
        Ok(self.torrent.stats().into())
    }
}

#[derive(Debug, SimpleObject)]
#[graphql(complex)]
pub struct TorrentFile {
    /// The length of the file in bytes
    pub length: u64,
    /// Name of the file
    pub name: String,
    /// Download priority of the file
    pub dnd: i8,
    /// Was the file renamed?
    pub is_renamed: bool,
    pub first_piece: u32,
    pub last_piece: u32,
    pub offset: u64,
}

#[ComplexObject]
impl TorrentFile {
    async fn download_link(&self, expiry_secs: Option<u64>) -> Option<String> {
        let path = std::path::Path::new(&DOWNLOAD_DIR.clone()).join(&self.name);
        let pathcheck = path.clone();
        let exists = tokio::task::spawn_blocking(move || pathcheck.exists()).await;
        match exists {
            Ok(exists) => {
                if exists {
                    let coded = DownloadLinkStructure {
                        file: path.to_string_lossy(),
                        expiry: expiry_secs
                            .map(|secs| chrono::Utc::now() + Duration::seconds(secs as i64)),
                    };
                    let bincoded = bincode::serialize(&coded);
                    match bincoded {
                        Ok(bincoded) => {
                            let encrypted = MCRYPT.encrypt_bytes_to_base64(&bincoded);
                            let urlencoded = urlencoding::encode(&encrypted);
                            Some(format!("/download/{}", urlencoded))
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadLinkStructure<'a> {
    pub file: Cow<'a, str>,
    pub expiry: Option<DateTime<Utc>>,
}

impl From<transmission::torrent::torrentinfo::TorrentFile> for TorrentFile {
    fn from(file: transmission::torrent::torrentinfo::TorrentFile) -> Self {
        Self {
            length: file.length,
            name: file.name,
            dnd: file.dnd,
            is_renamed: file.is_renamed,
            first_piece: file.first_piece,
            last_piece: file.last_piece,
            offset: file.offset,
        }
    }
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn monitor_torrent<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        torrent_id: i32,
        #[graphql(default = true)] auto_stop: bool,
        #[graphql(default = 500)] refresh_duration_millis: u64,
    ) -> Result<impl Stream<Item = Torrent>> {
        let data = ctx.data::<SharedData>()?;
        let torrent = data
            .torrents
            .get(&torrent_id)
            .ok_or("Torrent not found")?
            .clone();
        drop(torrent);
        let torrents = data.torrents.clone();
        let last_sent = Arc::new(Mutex::new(None));

        let str = async_stream::stream! {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(refresh_duration_millis)).await;
                {
                    let tmp_torrent = torrents.get(&torrent_id);
                    let tmp_torrent = match tmp_torrent {
                        Some(torrent) => torrent.clone(),
                        None => break
                    };
                    let be = bincode::serialize(&tmp_torrent);
                    if let Ok(be) = be {
                        let mut should_yield = false;
                        match &*last_sent.lock().unwrap(){
                            Some(be2)=>{
                                if &be != be2{
                                    should_yield = true
                                }
                            }
                            None => should_yield=true
                        }
                        if should_yield {
                            {
                                *last_sent.lock().unwrap() = Some(be);
                            }
                            yield Torrent{
                                torrent:tmp_torrent.clone()
                            }
                        }
                    };
                    if auto_stop && tmp_torrent.stats().percent_done >= 1.0 {
                        tmp_torrent.stop();
                        break;
                    }
                }
            }
        };

        Ok(str)
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(remote = "transmission::torrent::TorrentState")]
pub enum TorrentState {
    /// The torrent is downloading
    Downloading,
    /// The torrent is waiting to download
    DownloadingWait,
    /// The torrent is seeding
    Seeding,
    /// The torrent is waiting to seed
    SeedingWait,
    /// The torrent is stopped
    Stopped,
    /// The torrent is being checked
    Checking,
    /// The torrent is waiting to be checked
    CheckingWait,
    /// The torrent has errored
    Error,
}
