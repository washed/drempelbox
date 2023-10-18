use async_std::prelude::*;
use axum::{debug_handler, extract::Query, extract::State, routing::post, Router};
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::Spidev;
use mfrc522::comm::eh02::spi::SpiInterface;
use mfrc522::Mfrc522;
use serde::Deserialize;
use std::env;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tracing::{error, info};
use tracing_subscriber;

pub mod ntag215;
use crate::ntag215::NTAG215;

pub mod ndef;

pub mod spotify_player;
use crate::spotify_player::SpotifyPlayer;

pub mod file_player;
use crate::file_player::FilePlayer;

#[derive(Clone)]
struct AppState {
    sender: broadcast::Sender<SinkRequestMessage>,
}

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let spotify_player = SpotifyPlayer::new().await?;
    let file_player = FilePlayer::new().await?;

    let (sender, receiver) = broadcast::channel::<SinkRequestMessage>(16);
    let app_state = AppState { sender };

    let mut join_set = JoinSet::<()>::new();

    start_sink_handler(&mut join_set, receiver, file_player, spotify_player).await;
    start_ntag_reader_task(&mut join_set).await;
    start_server_task(&mut join_set, app_state).await;

    while let Some(_res) = join_set.join_next().await {
        let err = _res.err().unwrap().to_string();
        error!(err, "Task finished unexpectedly!");
    }

    Ok(())
}

#[derive(Debug, Clone)]
enum SinkRequestMessage {
    File(String),
    Spotify(String),
}

async fn start_sink_handler(
    join_set: &mut JoinSet<()>,
    mut receiver: broadcast::Receiver<SinkRequestMessage>,
    file_player: FilePlayer,
    mut spotify_player: SpotifyPlayer,
) {
    join_set.spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(sink_message) => match sink_message {
                    SinkRequestMessage::File(path) => {
                        info!(path, "received file sink request");

                        match spotify_player.stop().await {
                            Ok(_) => {}
                            Err(e) => error!(e, "Error stopping spotify playback!"),
                        };
                        match file_player.play(path, true).await {
                            Ok(_) => {}
                            Err(e) => error!(e, "Error playing file!"),
                        };
                    }
                    SinkRequestMessage::Spotify(uri) => {
                        info!(uri, "received spotify sink request");

                        match file_player.stop().await {
                            Ok(_) => {}
                            Err(e) => error!(e, "Error stopping file playback!"),
                        };

                        match spotify_player.play_from_url(uri).await {
                            Ok(_) => {}
                            Err(e) => error!(e, "Error playing spotify!"),
                        };
                    }
                },
                Err(e) => error!("Error receiving sink message {e}"),
            }
        }
    });
}

async fn start_ntag_reader_task(join_set: &mut JoinSet<()>) {
    join_set.spawn(async move {
        match start_ntag_reader().await {
            Ok(_) => {}
            Err(e) => {
                error!(e, "Error starting NTAG reader");
                panic!();
            }
        };
    });
}

async fn start_ntag_reader() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting NTAG reader...");
    let mut spi = Spidev::open("/dev/spidev0.0")?;
    let options = SpidevOptions::new()
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;

    let itf = SpiInterface::new(spi);
    let mfrc522 = Mfrc522::new(itf).init()?;
    let mut ntag = NTAG215::new(mfrc522);
    ntag.read();

    // this probably needs a loop and ndef parsing and then sending a message

    Ok(())
}

async fn start_server_task(join_set: &mut JoinSet<()>, app_state: AppState) {
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
