use axum::{debug_handler, extract::Query, extract::State, routing::post, Router};
use serde::Deserialize;
use std::env;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tracing::{error, info};
use url::Url;

use crate::player::PlayerRequestMessage;

#[derive(Clone)]
pub struct AppState {
    pub sender: broadcast::Sender<PlayerRequestMessage>,
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
        .route("/url", post(url))
        .route("/stop", post(stop))
        .with_state(app_state);

    let bind_address: std::net::SocketAddr = env::var("BIND_ADDRESS")?.parse()?;

    axum::Server::bind(&bind_address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[derive(Deserialize)]
struct SpotifyQuery {
    url: String,
}

#[debug_handler]
async fn url(State(state): State<AppState>, spotify_query: Query<SpotifyQuery>) {
    let spotify_query: SpotifyQuery = spotify_query.0;
    let url = spotify_query.url;

    info!(url, "Got spotify request");
    let url = Url::parse(&url).expect("couldn't parse this");

    match state.sender.send(PlayerRequestMessage::URL(url)) {
        Ok(res) => info!(res, "submitted spotify request"),
        Err(e) => error!("error submitting spotify request: {e}"),
    };
}

#[debug_handler]
async fn stop(State(state): State<AppState>) {
    info!("Got stop request");

    match state.sender.send(PlayerRequestMessage::Stop) {
        Ok(res) => info!(res, "submitted stop request"),
        Err(e) => error!("error submitting stop request: {e}"),
    };
}
