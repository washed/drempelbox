use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tracing::error;
use tracing_subscriber;

pub mod ndef;
pub mod ntag215;

pub mod spotify_player;
use crate::spotify_player::SpotifyPlayer;

pub mod file_player;
use crate::file_player::FilePlayer;

pub mod server;
use crate::server::{start_server_task, AppState};

pub mod ntag;
use crate::ntag::start_ntag_reader_task;

pub mod player;
use crate::player::{start_sink_handler, PlayerRequestMessage};

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let spotify_player = SpotifyPlayer::new().await?;
    let file_player = FilePlayer::new().await?;

    let (sender, receiver) = broadcast::channel::<PlayerRequestMessage>(16);
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
