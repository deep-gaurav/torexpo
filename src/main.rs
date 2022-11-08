use std::sync::Arc;

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    Schema,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    body::{boxed, Body, BoxBody},
    extract::Path,
    http::{Method, Request, Response, StatusCode},
    response::{self, IntoResponse},
    routing::get,
    Extension, Router, Server,
};
use dashmap::DashMap;
use magic_crypt::{new_magic_crypt, MagicCrypt256, MagicCryptTrait};
use seed_buster::seed_buster;
use structures::{DownloadLinkStructure, MainSchema, SubscriptionRoot};
use tower::ServiceExt;
use tower_http::cors::{Any, CorsLayer};
use transmission::{Client, Torrent};

use crate::{
    context::SharedData,
    structures::{MutationRoot, QueryRoot},
};

pub mod context;
pub mod seed_buster;
pub mod structures;
pub mod torrent_struc;

lazy_static::lazy_static! {
    pub static ref DOWNLOAD_DIR: String = std::env::var("TOREXPO_DOWNLOAD_DIR").unwrap_or_else(|_| "downloads".into());
    pub static ref MCRYPT:MagicCrypt256 = new_magic_crypt!(std::env::var("TOREXPO_DOWNLOAD_ENCRYPT_KEY").unwrap_or_else(|_| "download key".into()), 256);
}

async fn graphql_handler(schema: Extension<MainSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    response::Html(playground_source(
        GraphQLPlaygroundConfig::new("/").subscription_endpoint("/ws"),
    ))
}

async fn load_torrents(client: &Client, config_dir: &str) -> Vec<Torrent> {
    let torrents_path = std::path::Path::new(config_dir).join("torrents");
    let config_dir = tokio::fs::read_dir(&torrents_path).await;
    let mut torrents_loaded = vec![];
    if let Ok(mut config_dir) = config_dir {
        while let Ok(entry) = config_dir.next_entry().await {
            match entry {
                Some(entry) => {
                    let torrent_path = entry.path();
                    if let Some(path) = torrent_path.as_os_str().to_str() {
                        log::info!("Loading torrent {path}");
                        let res = client.add_torrent_file(path);
                        match res {
                            Ok(torrent) => torrents_loaded.push(torrent),
                            Err(err) => {
                                log::warn!("Load error {:#?}", err);
                            }
                        }
                    }
                }
                None => break,
            }
        }
    }
    torrents_loaded
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    tokio_uring::start(async {
        let torrents = Arc::new(DashMap::new());
        let download_dir = DOWNLOAD_DIR.clone();
        let config_dir = std::env::var("TOREXPO_CONFIG_DIR").unwrap_or_else(|_| "config".into());
        let transmission_config = transmission::ClientConfig::new()
            .app_name("torexpo")
            .download_dir(&download_dir)
            .config_dir(&config_dir);
        let transmission_client = transmission::Client::new(transmission_config);

        let loaded_torrents = load_torrents(&transmission_client, &config_dir).await;
        for torrent in loaded_torrents.into_iter() {
            torrents.insert(torrent.id(), torrent);
        }

        let data = SharedData {
            client: transmission_client,
            torrents: torrents.clone(),
        };

        let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
            .data(data)
            .finish();
        let cors = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            // allow requests from any origin
            .allow_origin(Any);

        let app = Router::new()
            .route("/", get(graphql_playground).post(graphql_handler))
            .route("/ws", GraphQLSubscription::new(schema.clone()))
            .route("/download/:download_link", get(serve_file))
            .layer(Extension(schema))
            .layer(cors);

        let port = std::env::var("TOREXPO_PORT").unwrap_or_else(|_| "8080".into());
        let torrent_buster_proc = seed_buster(torrents);
        let server_proc = Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
            .serve(app.into_make_service());
        futures_util::future::select(Box::pin(torrent_buster_proc), server_proc).await;
    });
    Ok(())
}

async fn serve_file(
    Path(download_link): Path<String>,
    req: Request<Body>,
) -> Result<Response<BoxBody>, (StatusCode, String)> {
    log::info!("Requested download link {download_link}");
    match MCRYPT.decrypt_base64_to_bytes(download_link) {
        Ok(data) => match bincode::deserialize::<DownloadLinkStructure>(&data) {
            Ok(structure) => {
                if let Some(expiry) = &structure.expiry {
                    if &chrono::Utc::now() > expiry {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            "Expired download link".to_string(),
                        ));
                    }
                }
                let path = std::path::Path::new(structure.file.as_ref());
                let servefile = tower_http::services::ServeFile::new(&path);
                let filename = path.file_name();
                log::info!("Downloading {}", path.to_string_lossy());
                match servefile.oneshot(req).await {
                    Ok(mut res) => {
                        if let Some(filename) = filename {
                            if let Ok(value) =
                                format!("filename=\"{}\"", filename.to_string_lossy()).try_into()
                            {
                                res.headers_mut().insert("content-disposition", value);
                            }
                        }
                        Ok(res.map(boxed))
                    }
                    Err(err) => Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Something went wrong: {}", err),
                    )),
                }
            }
            Err(_) => Err((StatusCode::BAD_REQUEST, "invalid download link".to_string())),
        },
        Err(_err) => Err((StatusCode::BAD_REQUEST, "invalid download link".to_string())),
    }
}
