use librespot::{
    core::{
        authentication::Credentials,
        config::SessionConfig,
        session::Session,
        spotify_id::{SpotifyAudioType, SpotifyId},
    },
    metadata::Metadata,
    metadata::{Album, Artist, Playlist},
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::NoOpVolume,
        player::Player,
    },
};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use url::Url;

pub struct SpotifyPlayer {
    session: Arc<Mutex<Session>>,
    player: Arc<Mutex<Player>>,
}

impl SpotifyPlayer {
    pub async fn new() -> Result<SpotifyPlayer, Box<dyn std::error::Error>> {
        let session_config = SessionConfig::default();
        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();

        let spotify_username: String = env::var("SPOTIFY_USERNAME")?.parse()?;
        let spotify_password: String = env::var("SPOTIFY_PASSWORD")?.parse()?;
        let credentials = Credentials::with_password(&spotify_username, &spotify_password);

        let backend: fn(Option<String>, AudioFormat) -> Box<dyn audio_backend::Sink> =
            audio_backend::find(None).unwrap();

        let (session, _credentials) =
            Session::connect(session_config, credentials, None, false).await?;

        let (player, _receiver) = Player::new(
            player_config,
            session.clone(),
            Box::new(NoOpVolume),
            move || backend(None, audio_format),
        );

        let session = Arc::new(Mutex::new(session));
        let player = Arc::new(Mutex::new(player));

        Ok(Self { session, player })
    }

    pub async fn play_from_url(&mut self, uri: String) -> Result<(), Box<dyn std::error::Error>> {
        let uri = Url::parse(&uri)?;

        // TODO: make the uri parsing more robust
        if let Some((context_type, spotify_id)) = uri.path().trim_matches('/').split_once('/') {
            let mut spotify_id = SpotifyId::from_base62(&spotify_id).unwrap();
            let mut player = self.player.lock().await;
            let session = self.session.lock().await;

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
                    player.load(album.tracks[0], true, 0);
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
