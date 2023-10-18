use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tracing::{error, info};

use crate::file_player::FilePlayer;
use crate::spotify_player::SpotifyPlayer;

#[derive(Debug, Clone)]
pub enum PlayerRequestMessage {
    File(String),
    Spotify(String),
}

pub async fn start_player_task(
    join_set: &mut JoinSet<()>,
    mut receiver: broadcast::Receiver<PlayerRequestMessage>,
    file_player: FilePlayer,
    mut spotify_player: SpotifyPlayer,
) {
    join_set.spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(sink_message) => match sink_message {
                    PlayerRequestMessage::File(path) => {
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
                    PlayerRequestMessage::Spotify(uri) => {
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
