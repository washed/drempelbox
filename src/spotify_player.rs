use futures::StreamExt;
use librespot::{
    core::{
        authentication::Credentials,
        cache::Cache,
        config::SessionConfig,
        session::Session,
        spotify_id::{SpotifyAudioType, SpotifyId},
    },
    metadata::Metadata,
    metadata::{Album, Artist, Playlist},
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer,
        player::{Player, PlayerEvent},
    },
};
use librespot_discovery::DeviceType;
use sha1::{Digest, Sha1};
use std::sync::Arc;
use std::{collections::VecDeque, env};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info};
use url::Url;

pub enum SpotifyPlayerCommand {
    PlayTracks(Vec<SpotifyId>),
    Stop,
}

pub struct SpotifyPlayer {
    session: Arc<Mutex<Session>>,
    player_tx: UnboundedSender<SpotifyPlayerCommand>,
}

impl SpotifyPlayer {
    pub async fn new(
        mixer: Arc<Mutex<Box<dyn mixer::Mixer>>>,
    ) -> Result<SpotifyPlayer, Box<dyn std::error::Error>> {
        let (player_tx, player_rx) = unbounded_channel::<SpotifyPlayerCommand>();

        let (session, player, player_event_receiver) =
            match SpotifyPlayer::connect(mixer.clone()).await {
                Ok(res) => res,
                Err(e) => {
                    error!(e, "Could not connect to spotify!");
                    return Err(e);
                }
            };

        let session = Arc::new(Mutex::new(session));
        let player = Arc::new(Mutex::new(player));

        // TODO: consider keeping this around to enable us to check up on it
        let _task = SpotifyPlayer::run(player, player_rx, player_event_receiver);

        let inst = Self { session, player_tx };

        Ok(inst)
    }

    async fn discovery() -> Option<Credentials> {
        info!("use authenticated spotify client to allow Drempelbox access");
        let name = "Drempelspot";
        let device_id = hex::encode(Sha1::digest(name.as_bytes()));

        let mut server = librespot_discovery::Discovery::builder(device_id)
            .name(name)
            .device_type(DeviceType::Computer)
            .launch()
            .unwrap();

        while let Some(x) = server.next().await {
            return Some(x);
        }
        None
    }

    fn run(
        player: Arc<Mutex<Player>>,
        mut player_rx: UnboundedReceiver<SpotifyPlayerCommand>,
        mut player_event_receiver: UnboundedReceiver<PlayerEvent>,
    ) -> (JoinHandle<()>, JoinHandle<()>) {
        let player_command_handler = player.clone();
        let player_event_handler = player.clone();

        let tracks: Arc<Mutex<VecDeque<SpotifyId>>> = Arc::new(Mutex::new(VecDeque::new()));
        let tracks_command_handler = tracks.clone();
        let tracks_event_handler = tracks.clone();

        (
            tokio::spawn(async move {
                loop {
                    match player_rx.recv().await {
                        Some(command) => {
                            let mut player = player_command_handler.lock().await;
                            let mut tracks = tracks_command_handler.lock().await;
                            match command {
                                SpotifyPlayerCommand::PlayTracks(new_tracks) => {
                                    let first_track = new_tracks[0];
                                    let rest_tracks = new_tracks[1..].to_vec();

                                    // start playing the first track immediately
                                    player.load(first_track, true, 0);

                                    // queue up the other tracks
                                    tracks.extend(rest_tracks);
                                }
                                SpotifyPlayerCommand::Stop => {
                                    info!("stopping spotify");
                                    tracks.clear();
                                    player.stop();
                                }
                            }
                        }
                        None => {}
                    }
                }
            }),
            tokio::spawn(async move {
                loop {
                    match player_event_receiver.recv().await {
                        Some(player_event) => {
                            let mut player = player_event_handler.lock().await;
                            let mut tracks = tracks_event_handler.lock().await;

                            match player_event {
                                PlayerEvent::TimeToPreloadNextTrack {
                                    play_request_id: _,
                                    track_id: _,
                                } => {
                                    info!("TimeToPreloadNextTrack!");
                                    if let Some(next_track) = tracks.front() {
                                        info!(next_track.id, "pre-loading");
                                        player.preload(next_track.to_owned());
                                    }
                                }
                                PlayerEvent::EndOfTrack {
                                    play_request_id: _,
                                    track_id: _,
                                } => {
                                    info!("EndOfTrack!");
                                    if let Some(next_track) = tracks.pop_front() {
                                        info!(next_track.id, "playing");
                                        player.load(next_track, true, 0);
                                    }
                                }
                                _ => {
                                    // TODO: implement more events?
                                }
                            }
                        }
                        None => {}
                    }
                }
            }),
        )
    }

    async fn connect(
        mixer: Arc<Mutex<Box<dyn mixer::Mixer>>>,
    ) -> Result<(Session, Player, UnboundedReceiver<PlayerEvent>), Box<dyn std::error::Error>> {
        let session_config = SessionConfig::default();
        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();

        // we shouldn't need the default here, as systemd should export CACHE_DIRECTORY,
        // but for some reason it is not seen by our process
        let cache_directory =
            env::var("CACHE_DIRECTORY").unwrap_or(String::from("/var/cache/drempelbox"));

        let cache = Cache::new(
            Some(cache_directory.clone() + "/credentials"),
            Some(cache_directory.clone() + "/volume"),
            Some(cache_directory.clone() + "/audio"),
            Some(1024 * 1024 * 1024),
        )?;

        let credentials = match cache.credentials() {
            Some(credentials) => {
                info!("using cached credentials");
                credentials
            }
            None => {
                info!("await auth via spotify connect");
                SpotifyPlayer::discovery().await.unwrap()
            }
        };

        let (session, _credentials) =
            Session::connect(session_config, credentials, Some(cache), true).await?;

        let backend: fn(Option<String>, AudioFormat) -> Box<dyn audio_backend::Sink> =
            audio_backend::find(None).unwrap();
        let mixer = mixer.lock().await;
        let (player, receiver) = Player::new(
            player_config,
            session.clone(),
            mixer.get_soft_volume(),
            move || backend(None, audio_format),
        );
        Ok((session, player, receiver))
    }

    pub async fn play_from_url(&mut self, url: Url) -> Result<(), Box<dyn std::error::Error>> {
        if let Some((context_type, spotify_id)) = url.path().trim_matches('/').split_once('/') {
            let mut spotify_id = SpotifyId::from_base62(&spotify_id).unwrap();
            let session = self.session.lock().await;

            match context_type {
                "track" => {
                    spotify_id.audio_type = SpotifyAudioType::Track;
                    self.play_tracks(Vec::from([spotify_id])).await?;
                    println!("Playing...");
                }
                "playlist" => {
                    let playlist: Playlist = Playlist::get(&session, spotify_id).await.unwrap();
                    self.play_tracks(playlist.tracks).await?;
                }
                "album" => {
                    let album: Album = Album::get(&session, spotify_id).await.unwrap();
                    self.play_tracks(album.tracks).await?;
                }
                "artist" => {
                    let artist: Artist = Artist::get(&session, spotify_id).await.unwrap();
                    self.play_tracks(artist.top_tracks).await?;
                }
                _ => info!("Unknown spotify context_type"),
            }
            return Ok(());
        }

        Err(Box::<dyn std::error::Error>::from("error splitting uri"))
    }

    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.player_tx.send(SpotifyPlayerCommand::Stop)?;
        Ok(())
    }

    async fn play_tracks(&self, tracks: Vec<SpotifyId>) -> Result<(), Box<dyn std::error::Error>> {
        self.player_tx
            .send(SpotifyPlayerCommand::PlayTracks(tracks))?;
        Ok(())
    }
}
