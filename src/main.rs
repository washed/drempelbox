use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::Spidev;
use mfrc522::comm::eh02::spi::SpiInterface;
use mfrc522::Mfrc522;

use axum::{debug_handler, extract::Query, extract::State, routing::post, Router};
use serde::Deserialize;
use std::env;
use tokio::sync::broadcast;
use tokio::task::JoinSet;

use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::sync::Mutex; // this is more expensive than std::sync::Mutex but makes using it across awaits easier
use tracing::{error, info};
use tracing_subscriber;
use url::Url;

use librespot::{
    core::{
        authentication::Credentials,
        config::SessionConfig,
        session::Session,
        spotify_id::{SpotifyAudioType, SpotifyId},
    },
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::NoOpVolume,
        player::Player,
    },
};

pub mod ntag215;
use crate::ntag215::NTAG215;

pub mod ndef;

type RodioSink = Arc<Mutex<Sink>>;
type SpotifyPlayer = Arc<Mutex<Player>>;

#[derive(Clone)]
struct AppState {
    sender: broadcast::Sender<SinkRequestMessage>,
}

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let player = get_spotify_player().await?;
    let player = Arc::new(Mutex::new(player));

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let sink = Arc::new(Mutex::new(sink));

    let (sender, receiver) = broadcast::channel::<SinkRequestMessage>(16);
    let app_state = AppState { sender };

    let mut join_set = JoinSet::<()>::new();

    start_sink_handler(&mut join_set, receiver, sink.clone(), player.clone()).await;
    start_ntag_reader_task(&mut join_set).await;
    start_server_task(&mut join_set, app_state).await;

    while let Some(_res) = join_set.join_next().await {
        let err = _res.err().unwrap().to_string();
        error!(err, "Task finished unexpectedly!");
    }

    Ok(())
}

async fn get_spotify_player() -> Result<Player, Box<dyn std::error::Error>> {
    let session_config = SessionConfig::default();
    let player_config = PlayerConfig::default();
    let audio_format = AudioFormat::default();

    let spotify_username: String = env::var("SPOTIFY_USERNAME")?.parse()?;
    let spotify_password: String = env::var("SPOTIFY_PASSWORD")?.parse()?;

    let credentials = Credentials::with_password(&spotify_username, &spotify_password);
    let backend: fn(Option<String>, AudioFormat) -> Box<dyn audio_backend::Sink> =
        audio_backend::find(None).unwrap();

    println!("Connecting...");
    let (session, _credentials) =
        Session::connect(session_config, credentials, None, false).await?;

    let (player, _receiver) =
        Player::new(player_config, session, Box::new(NoOpVolume), move || {
            backend(None, audio_format)
        });

    Ok(player)
}

#[derive(Debug, Clone)]
enum SinkRequestMessage {
    File(String),
    Spotify(String),
}

async fn start_sink_handler(
    join_set: &mut JoinSet<()>,
    mut receiver: broadcast::Receiver<SinkRequestMessage>,
    sink: RodioSink,
    player: SpotifyPlayer,
) {
    join_set.spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(sink_message) => match sink_message {
                    SinkRequestMessage::File(path) => {
                        info!(path, "received file sink request");

                        match try_stop_spotify(&player).await {
                            Ok(_) => {}
                            Err(e) => error!(e, "Error stopping spotify playback!"),
                        };

                        match try_play_file(&sink, path, true).await {
                            Ok(_) => {}
                            Err(e) => error!(e, "Error playing file!"),
                        };
                    }
                    SinkRequestMessage::Spotify(uri) => {
                        info!(uri, "received spotify sink request");

                        match try_stop_file(&sink).await {
                            Ok(_) => {}
                            Err(e) => error!(e, "Error stopping file playback!"),
                        };

                        match try_play_spotify(&player, uri).await {
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

async fn try_play_spotify(
    player: &SpotifyPlayer,
    uri: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut player = player.lock().await;

    let uri = Url::parse(&uri)?;

    // TODO: make the uri parsing more robust
    if let Some((context_type, context_id)) = uri.path().trim_matches('/').split_once('/') {
        match context_type {
            "track" => {
                let mut track = SpotifyId::from_base62(&context_id).unwrap();
                track.audio_type = SpotifyAudioType::Track;
                player.load(track, true, 0);
                println!("Playing...");
                // player.await_end_of_track().await; // to block or not to block
            }
            _ => info!("Unknown spotify context_type"),
        }
        return Ok(());
    }

    Err(Box::<dyn std::error::Error>::from("error splitting uri"))
}

async fn try_stop_spotify(player: &SpotifyPlayer) -> Result<(), Box<dyn std::error::Error>> {
    let player = player.lock().await;
    player.stop();
    Ok(())
}

async fn try_play_file(
    sink: &RodioSink,
    file_path: String,
    play_immediately: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let sink = sink.lock().await;
    let file = File::open(&file_path)?;
    let file = BufReader::new(file);
    let source = Decoder::new(file)?;

    if play_immediately == true && !sink.empty() {
        sink.clear();
    }

    info!(file_path, "Appending to sink queue");
    sink.append(source);
    sink.play();

    // sink.sleep_until_end(); // to block or not to block

    Ok(())
}

async fn try_stop_file(sink: &RodioSink) -> Result<(), Box<dyn std::error::Error>> {
    let sink = sink.lock().await;
    sink.stop();
    Ok(())
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
