use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{debug_handler, extract::Query, extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::env;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::{error, info};
use url::Url;

use crate::amp::Amp;
use crate::player::PlayerRequestMessage;

#[derive(Clone)]
pub struct AppState {
    pub sender: mpsc::Sender<PlayerRequestMessage>,
    pub amp: Amp,
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
        .route("/volume/up", post(volume_up))
        .route("/volume/down", post(volume_down))
        .route("/volume/set", post(volume_set))
        .route("/amp/on", post(amp_on))
        .route("/amp/off", post(amp_off))
        .route("/amp/power-on", post(amp_power_on))
        .route("/amp/power-off", post(amp_power_off))
        .with_state(app_state);

    let bind_address: std::net::SocketAddr = env::var("BIND_ADDRESS")
        .unwrap_or(String::from("0.0.0.0:3000"))
        .parse()?;

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

    info!(url, "Got URL request");
    let url = Url::parse(&url).expect("couldn't parse this");

    match state.sender.send(PlayerRequestMessage::URL(url)).await {
        Ok(_) => info!("submitted URL request"),
        Err(e) => error!("error submitting URL request: {e}"),
    };
}

#[debug_handler]
async fn stop(State(state): State<AppState>) {
    info!("Got stop request");

    match state.sender.send(PlayerRequestMessage::Stop).await {
        Ok(_) => info!("submitted stop request"),
        Err(e) => error!("error submitting stop request: {e}"),
    };
}

#[derive(Serialize)]
struct Volume {
    volume: f64,
}

#[debug_handler]
async fn volume_up(State(state): State<AppState>) -> impl IntoResponse {
    info!("Got volume up request");

    let (sender, receiver) = oneshot::channel::<f64>();

    match state
        .sender
        .send(PlayerRequestMessage::VolumeUp { responder: sender })
        .await
    {
        Ok(_) => info!("submitted volume up request"),
        Err(e) => error!("error submitting volume up request: {e}"),
    };

    match receiver.await {
        Ok(response) => (StatusCode::OK, Json(Volume { volume: response })).into_response(),
        Err(_) => {
            error!("didn't receive player command response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("error receiving player command response"),
            )
                .into_response()
        }
    }
}

#[debug_handler]
async fn volume_down(State(state): State<AppState>) -> impl IntoResponse {
    info!("Got volume down request");

    let (sender, receiver) = oneshot::channel::<f64>();

    match state
        .sender
        .send(PlayerRequestMessage::VolumeDown { responder: sender })
        .await
    {
        Ok(_) => info!("submitted volume up request"),
        Err(e) => error!("error submitting volume up request: {e}"),
    };

    match receiver.await {
        Ok(response) => (StatusCode::OK, Json(Volume { volume: response })).into_response(),
        Err(_) => {
            error!("didn't receive player command response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("error receiving player command response"),
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
struct VolumeSetQuery {
    volume: f64,
}

#[debug_handler]
async fn volume_set(
    State(state): State<AppState>,
    volume_set_query: Query<VolumeSetQuery>,
) -> impl IntoResponse {
    info!("Got volume set request");

    let volume = volume_set_query.0.volume;

    let (sender, receiver) = oneshot::channel::<f64>();

    match state
        .sender
        .send(PlayerRequestMessage::VolumeSet {
            volume,
            responder: sender,
        })
        .await
    {
        Ok(_) => info!("submitted volume set request"),
        Err(e) => error!("error submitting volume set request: {e}"),
    };

    match receiver.await {
        Ok(response) => (StatusCode::OK, Json(Volume { volume: response })).into_response(),
        Err(_) => {
            error!("didn't receive player command response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("error receiving player command response"),
            )
                .into_response()
        }
    }
}

#[debug_handler]
async fn amp_on(State(state): State<AppState>) -> impl IntoResponse {
    info!("Got amp on request");

    match state.amp.on().await {
        Ok(_) => (StatusCode::OK).into_response(),
        Err(_) => {
            error!("didn't receive amp on command response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("error receiving amp on command response"),
            )
                .into_response()
        }
    }
}

#[debug_handler]
async fn amp_off(State(state): State<AppState>) -> impl IntoResponse {
    info!("Got amp off request");

    match state.amp.off().await {
        Ok(_) => (StatusCode::OK).into_response(),
        Err(_) => {
            error!("didn't receive amp off command response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("error receiving amp off command response"),
            )
                .into_response()
        }
    }
}

#[debug_handler]
async fn amp_power_on(State(state): State<AppState>) -> impl IntoResponse {
    info!("Got amp power on request");

    match state.amp.power_on().await {
        Ok(_) => (StatusCode::OK).into_response(),
        Err(_) => {
            error!("didn't receive amp power on command response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("error receiving amp power on command response"),
            )
                .into_response()
        }
    }
}

#[debug_handler]
async fn amp_power_off(State(state): State<AppState>) -> impl IntoResponse {
    info!("Got amp power off request");

    match state.amp.power_off().await {
        Ok(_) => (StatusCode::OK).into_response(),
        Err(_) => {
            error!("didn't receive amp power off command response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("error receiving amp power off command response"),
            )
                .into_response()
        }
    }
}
