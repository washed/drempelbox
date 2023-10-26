use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tracing::error;
use tracing_subscriber;

pub mod ndef;
pub mod ntag215;

pub mod file_player;
pub mod spotify_player;

pub mod server;
use crate::server::{start_server_task, AppState};

pub mod ntag;
use crate::ntag::start_ntag_reader_task;

pub mod player;
use crate::player::{start_player_task, PlayerRequestMessage};

pub mod tuple_windows;

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let (sender, receiver) = broadcast::channel::<PlayerRequestMessage>(16);
    let app_state = AppState { sender };

    let mut join_set = JoinSet::<()>::new();

    start_player_task(&mut join_set, receiver).await?;
    start_ntag_reader_task(&mut join_set, app_state.clone()).await;
    start_server_task(&mut join_set, app_state.clone()).await;

    while let Some(_res) = join_set.join_next().await {
        let err = _res.err().unwrap().to_string();
        error!(err, "Task finished unexpectedly!");
        // TODO: we should probably crash the app at this point
    }

    Ok(())
}
