use crate::file_player::FilePlayer;
use crate::spotify_player::SpotifyPlayer;
use itertools::Itertools;
use percent_encoding::percent_decode_str;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tracing::{error, info};
use url::Url;

#[derive(Debug, Clone)]
pub enum PlayerRequestMessage {
    Stop,
    URL(Url),
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
                    PlayerRequestMessage::Stop => {
                        info!("received stop request");
                        stop(&file_player, &spotify_player).await;
                    }
                    PlayerRequestMessage::URL(url) => {
                        let log_url = url.to_string();
                        info!(log_url, "received URL player request");

                        match url.scheme() {
                            "https" => match url.host_str() {
                                Some("open.spotify.com") => {
                                    info!(log_url, "playing spotify from url");
                                    play_spotify(&file_player, &mut spotify_player, url).await;
                                }
                                _ => error!(log_url, "unsupported URL"),
                            },
                            "file" => {
                                // TODO: we should sanitize the path here...
                                info!(log_url, "playing file from url");
                                play_file(&file_player, &spotify_player, url).await;
                            }
                            &_ => info!(log_url, "not sure what to do with this url"),
                        }
                    }
                },
                Err(e) => error!("Error receiving sink message {e}"),
            }
        }
    });
}

async fn stop(file_player: &FilePlayer, spotify_player: &SpotifyPlayer) {
    match file_player.stop().await {
        Ok(_) => {}
        Err(e) => error!(e, "Error stopping file playback!"),
    };
    match spotify_player.stop().await {
        Ok(_) => {}
        Err(e) => error!(e, "Error stopping spotify playback!"),
    };
}

async fn play_spotify(file_player: &FilePlayer, spotify_player: &mut SpotifyPlayer, url: Url) {
    match file_player.stop().await {
        Ok(_) => {}
        Err(e) => error!(e, "Error stopping file playback!"),
    };
    match spotify_player.play_from_url(url).await {
        Ok(_) => {}
        Err(e) => error!(e, "Error playing spotify!"),
    };
}

async fn play_file(file_player: &FilePlayer, spotify_player: &SpotifyPlayer, url: Url) {
    match spotify_player.stop().await {
        Ok(_) => {}
        Err(e) => error!(e, "Error stopping spotify playback!"),
    };
    // TODO: We need to think about the format of this path a bit.
    //       Relative paths aren't really a thing in URLs.
    let file_path = url.path().trim_matches('/');
    let file_path = String::from_utf8(percent_decode_str(file_path).collect_vec()).expect("oof");

    match file_player.play(file_path, true).await {
        Ok(_) => {}
        Err(e) => error!(e, "Error playing file!"),
    };
}
