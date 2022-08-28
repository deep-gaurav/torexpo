use std::sync::Arc;

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    Schema,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    body::{boxed, Body, BoxBody},
    extract::Path,
    http::{Request, Response, StatusCode},
    response::{self, IntoResponse},
    routing::get,
    Extension, Router, Server,
};
use dashmap::DashMap;
use magic_crypt::{new_magic_crypt, MagicCrypt256, MagicCryptTrait};
use structures::{DownloadLinkStructure, MainSchema, SubscriptionRoot};
use tower::ServiceExt;

use crate::{
    context::SharedData,
    structures::{MutationRoot, QueryRoot},
};

pub mod context;
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

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let torrents = Arc::new(DashMap::new());
    let download_dir = DOWNLOAD_DIR.clone();
    let transmission_config = transmission::ClientConfig::new()
        .app_name("torexpo")
        .download_dir(&download_dir)
        .config_dir(&std::env::var("TOREXPO_CONFIG_DIR").unwrap_or_else(|_| "config".into()));
    let transmission_client = transmission::Client::new(transmission_config);
    let data = SharedData {
        client: transmission_client,
        torrents: torrents.clone(),
    };

    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(data)
        .finish();

    let app = Router::new()
        .route("/", get(graphql_playground).post(graphql_handler))
        .route("/ws", GraphQLSubscription::new(schema.clone()))
        .route("/download/:torrent_id/:file_id", get(serve_file))
        .layer(Extension(schema));

    let port = std::env::var("TOREXPO_PORT").unwrap_or_else(|_| "8080".into());
    Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn serve_file(
    Path(download_link): Path<String>,
    req: Request<Body>,
) -> Result<Response<BoxBody>, (StatusCode, String)> {
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
