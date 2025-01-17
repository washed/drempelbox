use std::sync::Arc;

use crate::amp::Amp;
use crate::file_player::FilePlayer;
use crate::spotify_player::SpotifyPlayer;
use itertools::Itertools;
use librespot::playback::config::VolumeCtrl;
use librespot::playback::mixer;
use librespot::playback::mixer::MixerConfig;
use percent_encoding::percent_decode_str;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::{error, info};
use url::Url;

#[derive(Debug)]
pub enum PlayerRequestMessage {
    Stop,
    URL(Url),
    VolumeUp {
        responder: oneshot::Sender<f64>,
    },
    VolumeDown {
        responder: oneshot::Sender<f64>,
    },
    VolumeSet {
        volume: f64,
        responder: oneshot::Sender<f64>,
    },
}

pub type Mixer = Arc<dyn mixer::Mixer>;

pub async fn start_player_task(
    join_set: &mut JoinSet<()>,
    mut receiver: mpsc::Receiver<PlayerRequestMessage>,
    amp: Amp,
) -> Result<(), Box<dyn std::error::Error>> {
    let mixer: Mixer = get_mixer()?;
    let mut spotify_player = SpotifyPlayer::new(mixer.clone()).await?;
    let file_player = FilePlayer::new(mixer.clone()).await?;

    join_set.spawn(async move {
        loop {
            let command = receiver.recv().await;
            match command {
                Some(sink_message) => match sink_message {
                    PlayerRequestMessage::Stop => {
                        info!("received stop request");
                        stop(&file_player, &spotify_player, &amp).await;
                    }
                    PlayerRequestMessage::URL(url) => {
                        let log_url = url.to_string();
                        info!(log_url, "received URL player request");

                        match url.scheme() {
                            "https" => match url.host_str() {
                                Some("open.spotify.com") => {
                                    info!(log_url, "playing spotify from url");
                                    play_spotify(&file_player, &mut spotify_player, url, &amp)
                                        .await;
                                }
                                _ => error!(log_url, "unsupported URL"),
                            },
                            "file" => {
                                // TODO: we should sanitize the path here...
                                info!(log_url, "playing file from url");
                                play_file(&file_player, &spotify_player, url, &amp).await;
                            }
                            &_ => info!(log_url, "not sure what to do with this url"),
                        }
                    }
                    PlayerRequestMessage::VolumeUp { responder } => {
                        let new_volume = set_volume_delta(&mixer, 0.01).await;
                        file_player.volume_changed().await;
                        match responder.send(new_volume) {
                            Ok(_) => {}
                            Err(_) => error!("error sending volume up command response"),
                        };
                    }
                    PlayerRequestMessage::VolumeDown { responder } => {
                        let new_volume = set_volume_delta(&mixer, -0.01).await;
                        file_player.volume_changed().await;
                        match responder.send(new_volume) {
                            Ok(_) => {}
                            Err(_) => error!("error sending volume up command response"),
                        };
                    }
                    PlayerRequestMessage::VolumeSet { volume, responder } => {
                        let new_volume = set_volume_absolute(&mixer, volume).await;
                        file_player.volume_changed().await;
                        match responder.send(new_volume) {
                            Ok(_) => {}
                            Err(_) => error!("error sending volume up command response"),
                        };
                    }
                },
                None => error!("PlayerRequestMessage channel has been closed!"),
            }
        }
    });
    Ok(())
}

async fn set_volume_delta(mixer: &Mixer, delta: f64) -> f64 {
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
    new_volume as f64 / VolumeCtrl::MAX_VOLUME as f64
}

async fn set_volume_absolute(mixer: &Mixer, volume: f64) -> f64 {
    // TODO: verify integer math here, make sure we don't explode
    let requested_volume = volume * VolumeCtrl::MAX_VOLUME as f64;
    let requested_volume = requested_volume.clamp(0.0, u16::MAX as f64) as u16;
    mixer.set_volume(requested_volume);
    let new_volume = mixer.volume();
    info!(
        volume,
        requested_volume, new_volume, "player volume change request"
    );
    new_volume as f64 / VolumeCtrl::MAX_VOLUME as f64
}

pub fn get_mixer() -> Result<Mixer, Box<dyn std::error::Error>> {
    let mixer_config = MixerConfig::default();
    let mixer = match mixer::find(Some("softvol")) {
        Some(mixer) => mixer(mixer_config),
        None => return Err(Box::<dyn std::error::Error>::from("Unable to find mixer!")),
    };
    // let mixer = Arc::new(Mutex::new(mixer));
    Ok(mixer)
}

async fn stop(file_player: &FilePlayer, spotify_player: &SpotifyPlayer, amp: &Amp) {
    match file_player.stop().await {
        Ok(_) => {}
        Err(e) => error!(e, "Error stopping file playback!"),
    };
    match spotify_player.stop().await {
        Ok(_) => {}
        Err(e) => error!(e, "Error stopping spotify playback!"),
    };
    match amp.off().await {
        Ok(_) => {}
        Err(e) => {
            let error_msg = e.to_string();
            error!(
                error_msg,
                "Error switching off amp after stopping playback!"
            )
        }
    };
}

async fn play_spotify(
    file_player: &FilePlayer,
    spotify_player: &mut SpotifyPlayer,
    url: Url,
    amp: &Amp,
) {
    match amp.on().await {
        Ok(_) => {}
        Err(e) => {
            let error_msg = e.to_string();
            error!(
                error_msg,
                "Error switching on amp before starting playback!"
            )
        }
    };
    // TODO: do we want to wait for this to actually happen?

    match file_player.stop().await {
        Ok(_) => {}
        Err(e) => error!(e, "Error stopping file playback!"),
    };
    match spotify_player.play_from_url(url).await {
        Ok(_) => {}
        Err(e) => error!(e, "Error playing spotify!"),
    };
}

async fn play_file(file_player: &FilePlayer, spotify_player: &SpotifyPlayer, url: Url, amp: &Amp) {
    match amp.on().await {
        Ok(_) => {}
        Err(e) => {
            let error_msg = e.to_string();
            error!(
                error_msg,
                "Error switching on amp before starting playback!"
            )
        }
    };
    // TODO: do we want to wait for this to actually happen?

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
