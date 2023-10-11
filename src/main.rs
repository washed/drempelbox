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

mod file;
use file::play_file;

pub mod ntag215;
use crate::ntag215::NTAG215;

pub mod ndef;

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    start_server().await?;
    // start_ntag_reader().await?;

    Ok(())
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

async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/spotify", get(spotify_url))
        .route("/file", get(file));

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
async fn file(file_query: Query<FileQuery>) {
    let file_query: FileQuery = file_query.0;
    let path = file_query.path;

    println!("Filepath: {path}");
    play_file();
}
