use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::Spidev;
use mfrc522::comm::eh02::spi::SpiInterface;
use mfrc522::Mfrc522;
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

pub mod server;
use crate::server::{start_server_task, AppState, SinkRequestMessage};

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
