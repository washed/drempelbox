use axum::http::Uri;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::Spidev;
use mfrc522::comm::eh02::spi::SpiInterface;
use mfrc522::Mfrc522;

use axum::headers;
use axum::{
    debug_handler,
    extract::Query,
    extract::State,
    response::sse::{Event, Sse},
    routing::get,
    Router, TypedHeader,
};
use futures::stream::Stream;
use serde::Deserialize;
use std::env;
use std::path::Path;
use std::{convert::Infallible, time::Duration};
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio::time;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;

use rodio::source::{SineWave, Source};
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

use tracing::{error, info};
use tracing_subscriber;

pub mod ntag215;
use crate::ntag215::NTAG215;

pub mod ndef;

#[derive(Clone)]
struct AppState {
    sender: broadcast::Sender<String>,
}

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let (sender, receiver) = broadcast::channel::<String>(16);

    let app_state = AppState { sender };

    let mut join_set = JoinSet::<()>::new();

    start_sink_handler(&mut join_set, receiver).await;

    start_server(app_state).await?;
    // start_ntag_reader().await?;


    while let Some(res) = join_set.join_next().await {
        println!("Task finished unexpectedly!");
    }

    Ok(())
}

async fn start_sink_handler(join_set: &mut JoinSet<()>, mut receiver: broadcast::Receiver<String>) {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    join_set.spawn(async move {
        let sink = Sink::try_new(&stream_handle).unwrap();

        loop {
            match receiver.recv().await {
                Ok(sink_message) => {
                    info!(sink_message, "received sink message");
                    let file = BufReader::new(File::open(&sink_message).unwrap());
                    let source = Decoder::new(file).unwrap();
                    sink.append(source);
                    sink.sleep_until_end();
                    info!(sink_message, "done playing");
                }
                Err(e) => error!("Error receiving sink message {e}"),
            }
        }
    });
}

async fn start_ntag_reader() -> Result<(), Box<dyn std::error::Error>> {
    let mut spi = Spidev::open("/dev/spidev0.0").unwrap();
    let options = SpidevOptions::new()
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;

    let itf = SpiInterface::new(spi);
    let mfrc522 = Mfrc522::new(itf).init().expect("Error initing mfrc522");

    let mut ntag = NTAG215::new(mfrc522);
    ntag.read();

    Ok(())
}

async fn start_server(app_state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/spotify", get(spotify_url))
        .route("/file", get(file))
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
async fn spotify_url(spotify_query: Query<SpotifyQuery>) {
    let spotify_query: SpotifyQuery = spotify_query.0;
    let uri = spotify_query.uri;

    println!("Spotify URI: {uri}");
}

#[derive(Deserialize)]
struct FileQuery {
    path: String,
}

#[debug_handler]
async fn file(State(state): State<AppState>, file_query: Query<FileQuery>) {
    let file_query: FileQuery = file_query.0;
    let path = file_query.path;
    info!(path, "Got file play request");
    match state.sender.send(path) {
        Ok(res) => info!(res, "submitted file request"),
        Err(e) => info!("error submitting file request: {e}")
    };
}
