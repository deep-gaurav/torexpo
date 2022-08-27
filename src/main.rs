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
use structures::{MainSchema, SubscriptionRoot};
use tower::ServiceExt;
use transmission::Torrent;

use crate::{
    context::SharedData,
    structures::{MutationRoot, QueryRoot},
};

pub mod context;
pub mod structures;
pub mod torrent_struc;

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
    let download_dir = std::env::var("TOREXPO_DOWNLOAD_DIR").unwrap_or_else(|_| "downloads".into());
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
        .route(
            "/download/:torrent_id/:file_id",
            get(move |path, req| serve_file(torrents, download_dir, path, req)),
        )
        .layer(Extension(schema));

    let port = std::env::var("TOREXPO_PORT").unwrap_or_else(|_| "8080".into());
    Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn serve_file(
    torrents: Arc<DashMap<i32, Torrent>>,
    download_dir: String,
    Path((torrent_id, file_id)): Path<(i32, usize)>,
    req: Request<Body>,
) -> Result<Response<BoxBody>, (StatusCode, String)> {
    match torrents
        .get(&torrent_id)
        .map(|torrent| torrent.info().files)
    {
        Some(files) => match files.get(file_id) {
            Some(file) => {
                let path = std::path::Path::new(&download_dir).join(&file.name);
                let servefile = tower_http::services::ServeFile::new(path);
                match servefile.oneshot(req).await {
                    Ok(res) => Ok(res.map(boxed)),
                    Err(err) => Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Something went wrong: {}", err),
                    )),
                }
            }
            None => Err((StatusCode::NOT_FOUND, "File to download Not found".into())),
        },
        None => Err((StatusCode::NOT_FOUND, "Torrent not found".to_string())),
    }
}
