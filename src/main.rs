use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::error;

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

pub mod amp;
use crate::amp::Amp;

pub mod shutdown;
use crate::shutdown::Shutdown;

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let mut join_set = JoinSet::<()>::new();

    let amp = Amp::new(&mut join_set).await?;
    let amp_player = amp.clone();

    let _shutdown = Shutdown::new(&mut join_set).await?;

    let (sender, receiver) = mpsc::channel::<PlayerRequestMessage>(16);
    let app_state = AppState { sender, amp };

    start_player_task(&mut join_set, receiver, amp_player).await?;
    start_ntag_reader_task(&mut join_set, app_state.clone()).await;
    start_server_task(&mut join_set, app_state.clone()).await;

    app_state.amp.power_on().await?;
    sleep(Duration::from_millis(250)).await;

    while let Some(_res) = join_set.join_next().await {
        let err = _res.err().unwrap().to_string();
        error!(err, "Task finished unexpectedly!");
        // TODO: we should probably crash the app at this point
    }

    Ok(())
}
