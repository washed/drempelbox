use axum::{debug_handler, extract::Query, extract::State, routing::post, Router};
use serde::Deserialize;
use std::env;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub enum SinkRequestMessage {
    File(String),
    Spotify(String),
}

#[derive(Clone)]
pub struct AppState {
    pub sender: broadcast::Sender<SinkRequestMessage>,
}

pub async fn start_server_task(join_set: &mut JoinSet<()>, app_state: AppState) {
    join_set.spawn(async {
        match start_server(app_state).await {
            Ok(_) => {}
            Err(e) => error!(e, "Error starting http server"),
        }
    });
}

async fn start_server(app_state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting http server...");

    let app = Router::new()
        .route("/spotify", post(spotify_url))
        .route("/file", post(file))
        .with_state(app_state);

    let bind_address: std::net::SocketAddr = env::var("BIND_ADDRESS")?.parse()?;

    axum::Server::bind(&bind_address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[derive(Deserialize)]
struct SpotifyQuery {
    uri: String,
}

#[debug_handler]
async fn spotify_url(State(state): State<AppState>, spotify_query: Query<SpotifyQuery>) {
    let spotify_query: SpotifyQuery = spotify_query.0;
    let uri = spotify_query.uri;

    info!(uri, "Got spotify request");

    match state.sender.send(SinkRequestMessage::Spotify(uri)) {
        Ok(res) => info!(res, "submitted spotify request"),
        Err(e) => error!("error submitting spotify request: {e}"),
    };
}

#[derive(Deserialize)]
struct FileQuery {
    path: String,
}

#[debug_handler]
async fn file(State(state): State<AppState>, file_query: Query<FileQuery>) {
    let file_query: FileQuery = file_query.0;
    let path = file_query.path;

    info!(path, "got file play request");

    match state.sender.send(SinkRequestMessage::File(path)) {
        Ok(res) => info!(res, "submitted file request"),
        Err(e) => error!("error submitting file request: {e}"),
    };
}
