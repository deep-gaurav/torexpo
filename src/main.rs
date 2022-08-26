use std::sync::Arc;

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    Schema,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    response::{self, IntoResponse},
    routing::get,
    Extension, Router, Server,
};
use dashmap::DashMap;
use structures::{MainSchema, SubscriptionRoot};

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

    let transmission_config = transmission::ClientConfig::new()
        .app_name("torexpo")
        .download_dir(&std::env::var("TOREXPO_DOWNLOAD_DIR").unwrap_or_else(|_| "downloads".into()))
        .config_dir(&std::env::var("TOREXPO_CONFIG_DIR").unwrap_or_else(|_| "config".into()));
    let transmission_client = transmission::Client::new(transmission_config);
    let data = SharedData {
        client: transmission_client,
        torrents: Arc::new(DashMap::new()),
    };

    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(data)
        .finish();

    let app = Router::new()
        .route("/", get(graphql_playground).post(graphql_handler))
        .route("/ws", GraphQLSubscription::new(schema.clone()))
        .layer(Extension(schema));

    let port = std::env::var("TOREXPO_PORT").unwrap_or_else(|_| "8080".into());
    Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
