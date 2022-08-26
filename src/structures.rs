use std::sync::{Arc, Mutex};

use async_graphql::*;
use futures_util::Stream;
use tokio_stream::StreamExt;

use crate::context::SharedData;

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
}

struct Torrent {
    torrent: transmission::Torrent,
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

    async fn total_bytes(&self) -> Result<u64> {
        Ok(self.torrent.stats().size_when_done)
    }

    async fn total_downloaded(&self) -> Result<u64> {
        Ok(self.torrent.stats().size_when_done - self.torrent.stats().left_until_done)
    }

    async fn peers_connected(&self) -> Result<i32> {
        Ok(self.torrent.stats().peers_connected)
    }
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn monitor_torrent<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        torrent_id: i32,
        #[graphql(default = 500)] refresh_duration_millis: u64,
    ) -> Result<impl Stream<Item = Torrent>> {
        let data = ctx.data::<SharedData>()?;
        let torrent = data
            .torrents
            .get(&torrent_id)
            .ok_or("Torrent not found")?
            .clone();
        let ttorrent = torrent.clone();
        let interval =
            tokio::time::interval(std::time::Duration::from_millis(refresh_duration_millis));
        let last_sent = Arc::new(Mutex::new(None));
        let interval_stream = tokio_stream::wrappers::IntervalStream::new(interval);

        let str = async_stream::stream! {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(refresh_duration_millis)).await;
                let tmp_torrent = ttorrent.clone();
                if tmp_torrent.stats().percent_complete < 1.0 {
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
                                torrent:tmp_torrent
                            }
                        }
                    }
                }else{
                    break
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
