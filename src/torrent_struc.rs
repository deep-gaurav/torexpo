use async_graphql::*;
use chrono::NaiveDateTime;

use crate::structures::{TorrentFile, TorrentState};

#[derive(SimpleObject)]
pub struct TorrentStats {
    /// The ID of the torrent.
    pub id: i32,
    /// The state of the torrent. Internally Transmission calls this the "activity",
    pub state: TorrentState,
    /// The error state (if any).
    pub error: TorrentError,
    /// A string describing the above error if any.
    pub error_string: String,
    /// Progress rechecking a torrent.
    pub recheck_progress: f32,
    /// Percent of the total download completed.
    pub percent_complete: f32,
    /// Percent of the metadata download completed.
    pub metadata_percent_complete: f32,
    /// Percent of the desired download completed.
    /// This differs from [`torrent::TorrentStats::percent_complete`] if the user only wants some of a torrent's files.
    pub percent_done: f32,
    /// Percent of the seed ratio uploaded. 1 if completed or infinite.
    pub seed_ratio_percent_done: f32,
    /// The raw upload speed.
    pub raw_upload_speed_kbps: f32,
    /// The raw download speed.
    pub raw_download_speed_kbps: f32,
    /// The actual piece upload speed.
    pub piece_upload_speed_kbps: f32,
    /// The actual piece download speed.
    pub piece_download_speed_kbps: f32,
    /// Estimated time of arrival (completion)
    pub eta: i32,
    pub eta_idle: i32,
    /// Number of peers connected for this torrent.
    pub peers_connected: i32,
    pub peers_from: [i32; 7],
    /// Peers we are downloading from.
    pub peers_sending_to_us: i32,
    /// Peers we are uploading to.
    pub peers_getting_from_us: i32,
    /// Webseeds we are downlading from.
    pub webseeds_sending_to_us: i32,
    /// Size in bytes when completed.
    pub size_when_done: u64,
    /// Bytes until download is finished.
    pub left_until_done: u64,
    pub desired_available: u64,
    pub corrupt_ever: u64,
    pub uploaded_ever: u64,
    pub downloaded_ever: u64,
    pub have_valid: u64,
    pub have_unchecked: u64,
    pub manual_announce_time: NaiveDateTime,
    /// Seed ratio
    pub ratio: f32,
    /// Date and time added
    pub added_date: NaiveDateTime,
    /// Date and time finished
    pub done_date: NaiveDateTime,
    /// Date and time started
    pub start_date: NaiveDateTime,
    /// Date and time of last activity
    pub activity_date: NaiveDateTime,
    /// How long it has been idle
    pub idle_secs: i32,
    /// How long it has been downloading
    pub seconds_downloading: i32,
    /// How log it has been seeding
    pub seconds_seeding: i32,
    /// Is the torrent finished
    pub finished: bool,
    /// What position in the queue is the torrent
    pub queue_position: i32,
    /// Is the torrent stalled
    pub is_stalled: bool,
}

impl From<transmission::torrent::TorrentStats> for TorrentStats {
    fn from(torrent: transmission::torrent::TorrentStats) -> Self {
        Self {
            id: torrent.id,
            state: torrent.state.into(),
            error: torrent.error.into(),
            error_string: torrent.error_string,
            recheck_progress: torrent.recheck_progress,
            percent_complete: torrent.percent_complete,
            metadata_percent_complete: torrent.metadata_percent_complete,
            percent_done: torrent.percent_done,
            seed_ratio_percent_done: torrent.seed_ratio_percent_done,
            raw_upload_speed_kbps: torrent.raw_upload_speed_kbps,
            raw_download_speed_kbps: torrent.raw_download_speed_kbps,
            piece_upload_speed_kbps: torrent.piece_upload_speed_kbps,
            piece_download_speed_kbps: torrent.piece_download_speed_kbps,
            eta: torrent.eta,
            eta_idle: torrent.eta_idle,
            peers_connected: torrent.peers_connected,
            peers_from: torrent.peers_from,
            peers_sending_to_us: torrent.peers_sending_to_us,
            peers_getting_from_us: torrent.peers_getting_from_us,
            webseeds_sending_to_us: torrent.webseeds_sending_to_us,
            size_when_done: torrent.size_when_done,
            left_until_done: torrent.left_until_done,
            desired_available: torrent.desired_available,
            corrupt_ever: torrent.corrupt_ever,
            uploaded_ever: torrent.uploaded_ever,
            downloaded_ever: torrent.downloaded_ever,
            have_valid: torrent.have_valid,
            have_unchecked: torrent.have_unchecked,
            manual_announce_time: torrent.manual_announce_time,
            ratio: torrent.ratio,
            added_date: torrent.added_date,
            done_date: torrent.done_date,
            start_date: torrent.start_date,
            activity_date: torrent.activity_date,
            idle_secs: torrent.idle_secs,
            seconds_downloading: torrent.seconds_downloading,
            seconds_seeding: torrent.seconds_seeding,
            finished: torrent.finished,
            queue_position: torrent.queue_position,
            is_stalled: torrent.is_stalled,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(remote = "transmission::error::Error")]
pub enum TorrentError {
    /// A general state of non-error.
    /// If this is is ever the `Err` of a `Result` please file a bug report.
    NoError,
    /// For all errors with unknown causes.
    Unknown,
    /// An error occured in file I/O.
    IOError,
    /// Error in parsing a torrent.
    ParseErr,
    /// When parsing a torrent if it is a duplicate.
    ParseDuplicate,
    /// Local error when getting a torrent's stats.
    StatLocal,
    /// Tracker error when getting a torrent's stats.
    StatTracker,
    /// Tracker warning when getting a torrent's stats.
    StatTrackerWarn,
    /// An error with the URL when getting metainfo.
    MakeMetaUrl,
    /// Getting metainfo was cancelled.
    MakeMetaCancelled,
}

#[derive(SimpleObject)]
pub struct TorrentInfo {
    /// Total download size in bytes
    pub total_size: u64,
    /// Original name of the torrent
    pub original_name: String,
    /// Name of the torrent
    pub name: String,
    pub torrent: String,
    /// Webseeds of the torrent
    pub webseeds: Vec<String>,
    /// Comment on the torrent
    pub comment: String,
    /// The torrent's creator
    pub creator: String,
    /// Files of the torrent
    pub files: Vec<TorrentFile>,
    /// Pieces of the torrent
    ///
    /// This is skipped in Serialization due to it's size.
    /// If you want it serialized you will have to do it manually.
    pub pieces: Vec<TorrentPiece>,
    /// Trackers of the torrent
    pub trackers: Vec<TrackerInfo>,
    /// Date the torrent was created
    pub date_created: NaiveDateTime,
    /// Number of trackers
    pub tracker_count: u32,
    /// Number of webseeds
    pub webseed_count: u32,
    /// Number of files
    pub file_count: u32,
    /// Sice of pieces in bytes
    pub piece_size: u32,
    /// Number of pieces
    pub piece_count: u32,
    pub hash: [u8; 20],
    /// String hash of the torrent
    pub hash_string: String,
    pub is_private: bool,
    /// Is it a torrent of a folder?
    pub is_folder: bool,
}

impl From<transmission::torrent::TorrentInfo> for TorrentInfo {
    fn from(torrent_info: transmission::torrent::TorrentInfo) -> Self {
        Self {
            total_size: torrent_info.total_size,
            original_name: torrent_info.original_name,
            name: torrent_info.name,
            torrent: torrent_info.torrent,
            webseeds: torrent_info.webseeds,
            comment: torrent_info.comment,
            creator: torrent_info.creator,
            files: torrent_info.files.into_iter().map(|f| f.into()).collect(),
            pieces: torrent_info.pieces.into_iter().map(|f| f.into()).collect(),
            trackers: torrent_info
                .trackers
                .into_iter()
                .map(|f| f.into())
                .collect(),
            date_created: torrent_info.date_created,
            tracker_count: torrent_info.tracker_count,
            webseed_count: torrent_info.webseed_count,
            file_count: torrent_info.file_count,
            piece_size: torrent_info.piece_size,
            piece_count: torrent_info.piece_count,
            hash: torrent_info.hash,
            hash_string: torrent_info.hash_string,
            is_private: torrent_info.is_private,
            is_folder: torrent_info.is_folder,
        }
    }
}

#[derive(SimpleObject)]
pub struct TorrentPiece {
    /// Last time the piece was checked
    pub time_checked: NaiveDateTime,
    pub hash: [u8; 20],
    /// Priority of the piece
    pub priority: i8,
    pub dnd: i8,
}

impl From<transmission::torrent::torrentinfo::TorrentPiece> for TorrentPiece {
    fn from(piece: transmission::torrent::torrentinfo::TorrentPiece) -> Self {
        Self {
            time_checked: piece.time_checked,
            hash: piece.hash,
            priority: piece.priority,
            dnd: piece.dnd,
        }
    }
}

#[derive(SimpleObject)]
pub struct TrackerInfo {
    pub tier: i32,
    pub announce: String,
    pub scrape: String,
    pub id: u32,
}

impl From<transmission::torrent::torrentinfo::TrackerInfo> for TrackerInfo {
    fn from(tracker_info: transmission::torrent::torrentinfo::TrackerInfo) -> Self {
        Self {
            tier: tracker_info.tier,
            announce: tracker_info.announce,
            scrape: tracker_info.scrape,
            id: tracker_info.id,
        }
    }
}
