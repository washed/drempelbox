use std::sync::Arc;

use crate::file_player::FilePlayer;
use crate::spotify_player::SpotifyPlayer;
use itertools::Itertools;
use librespot::playback::config::VolumeCtrl;
use librespot::playback::mixer;
use librespot::playback::mixer::MixerConfig;
use percent_encoding::percent_decode_str;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinSet;
use tracing::{error, info};
use url::Url;

#[derive(Debug, Clone)]
pub enum PlayerRequestMessage {
    Stop,
    URL(Url),
    VolumeUp,
    VolumeDown,
}

pub async fn start_player_task(
    join_set: &mut JoinSet<()>,
    mut receiver: broadcast::Receiver<PlayerRequestMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mixer: Arc<Mutex<Box<dyn mixer::Mixer>>> = get_mixer()?;
    let mut spotify_player = SpotifyPlayer::new(mixer.clone()).await?;
    let file_player = FilePlayer::new(mixer.clone()).await?;

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

                    PlayerRequestMessage::VolumeUp => {
                        set_volume_delta(&mixer, 0.01).await;
                        file_player.volume_changed().await;
                    }
                    PlayerRequestMessage::VolumeDown => {
                        set_volume_delta(&mixer, -0.01).await;
                        file_player.volume_changed().await;
                    }
                },
                Err(e) => error!("Error receiving sink message {e}"),
            }
        }
    });
    Ok(())
}

async fn set_volume_delta(mixer: &Arc<Mutex<Box<dyn mixer::Mixer>>>, delta: f64) {
    let mixer = mixer.lock().await;

    // TODO: verify integer math here, make sure we don't explode
    let current_volume = mixer.volume() as i32;
    let volume_step = (VolumeCtrl::MAX_VOLUME as f64 * delta) as i32;
    let requested_volume = current_volume + volume_step;
    let requested_volume = requested_volume.clamp(0, u16::MAX as i32) as u16;
    mixer.set_volume(requested_volume);
    let new_volume = mixer.volume();
    info!(
        delta,
        current_volume, requested_volume, new_volume, "player volume change request"
    );
}

pub fn get_mixer() -> Result<Arc<Mutex<Box<dyn mixer::Mixer>>>, Box<dyn std::error::Error>> {
    let mixer_config = MixerConfig::default();
    let mixer = match mixer::find(Some("softvol")) {
        Some(mixer) => mixer(mixer_config),
        None => return Err(Box::<dyn std::error::Error>::from("Unable to find mixer!")),
    };
    let mixer = Arc::new(Mutex::new(mixer));
    Ok(mixer)
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
