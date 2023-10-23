use librespot::{
    core::{
        authentication::Credentials,
        config::ConnectConfig,
        config::SessionConfig,
        session::Session,
        spotify_id::{SpotifyAudioType, SpotifyId},
    },
    metadata::Metadata,
    metadata::{Album, Artist, Playlist},
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::softmixer::SoftMixer,
        mixer::{Mixer, MixerConfig, NoOpVolume},
        player::Player,
        player::PlayerEvent,
    },
    protocol::spirc::TrackRef,
};
use librespot_connect::spirc::{Spirc, SpircCommand};
use std::env;
use std::sync::Arc;
use tokio::join;
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};
use tracing::info;
use url::Url;

pub struct SpotifyPlayer {
    session: Arc<Mutex<Session>>,
    player: Arc<Mutex<Player>>,
    spirc: Arc<Mutex<Spirc>>,
}

impl SpotifyPlayer {
    pub async fn new() -> Result<SpotifyPlayer, Box<dyn std::error::Error>> {
        let session_config = SessionConfig::default();
        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();
        let connect_config = ConnectConfig::default();

        let spotify_username: String = env::var("SPOTIFY_USERNAME")?.parse()?;
        let spotify_password: String = env::var("SPOTIFY_PASSWORD")?.parse()?;
        let credentials = Credentials::with_password(&spotify_username, &spotify_password);

        let backend: fn(Option<String>, AudioFormat) -> Box<dyn audio_backend::Sink> =
            audio_backend::find(None).unwrap();

        let (session, _credentials) =
            Session::connect(session_config, credentials, None, false).await?;

        let (player, player_event_receiver) = Player::new(
            player_config.clone(),
            session.clone(),
            Box::new(NoOpVolume),
            move || backend(None, audio_format),
        );

        let (spirc_player, _spirc_player_event_receiver) = Player::new(
            player_config,
            session.clone(),
            Box::new(NoOpVolume),
            move || backend(None, audio_format),
        );

        let (spirc, spirc_task) = Spirc::new(
            connect_config,
            session.clone(),
            spirc_player,
            Box::new(SoftMixer::open(MixerConfig::default())),
        );

        // spirc_task?

        let session = Arc::new(Mutex::new(session));
        let player = Arc::new(Mutex::new(player));
        let spirc = Arc::new(Mutex::new(spirc));

        Ok(Self {
            session,
            player,
            spirc,
        })
    }

    fn parse_url<'a>(&'a self, url: &'a Url) -> Option<(&str, SpotifyId)> {
        let (context_type, spotify_id) = url.path().trim_matches('/').split_once('/')?;
        let spotify_id = SpotifyId::from_base62(&spotify_id).unwrap();
        Some((context_type, spotify_id))
    }

    pub async fn play_from_url(&mut self, url: Url) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: make the uri parsing more robust
        if let Some((context_type, mut spotify_id)) = self.parse_url(&url) {
            let mut player = self.player.lock().await;
            let session = self.session.lock().await;
            let spirc = self.spirc.lock().await;

            match context_type {
                "track" => {
                    spotify_id.audio_type = SpotifyAudioType::Track;
                    player.load(spotify_id, true, 0);
                    println!("Playing...");
                    // player.await_end_of_track().await; // to block or not to block (i think we don't need this)
                }
                "playlist" => {
                    let playlist: Playlist = Playlist::get(&session, spotify_id).await.unwrap();
                    println!("{:?}", playlist);
                    // TODO: do something with it!
                    // For now we just play the first song
                    player.load(playlist.tracks[0], true, 0);
                }
                "album" => {
                    let album: Album = Album::get(&session, spotify_id).await.unwrap();
                    // TODO: do something with it!
                    // For now we just play the first song
                    // player.load(album.tracks[0], true, 0);

                    /*
                    let tracks: Vec<TrackRef> = album
                        .tracks
                        .into_iter()
                        .map(|track_id| {
                            let mut track = TrackRef::new();
                            track.set_gid(Vec::from(track_id.to_raw()));
                            track
                        })
                        .collect();
                    */

                    info!("Playing album: {}", &album.name);
                    for track in album.tracks {
                        player.load(track, false, 0);
                    }
                    player.play();
                }
                "artist" => {
                    let album: Artist = Artist::get(&session, spotify_id).await.unwrap();
                    // TODO: do something with it!
                    // For now we just play the first song
                    player.load(album.top_tracks[0], true, 0);
                }
                _ => info!("Unknown spotify context_type"),
            }
            return Ok(());
        }

        Err(Box::<dyn std::error::Error>::from("error splitting uri"))
    }

    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let player = self.player.lock().await;
        player.stop();
        Ok(())
    }
}
