use async_std::sync::Arc;
use librespot::playback::mixer::{self, VolumeGetter};
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use tokio::sync::Mutex; // this is more expensive than std::sync::Mutex but makes using it across awaits easier
use tracing::info;

pub struct FilePlayer {
    sink: Arc<Mutex<Sink>>,
    _stream: OutputStream,
    volume_getter: Box<dyn VolumeGetter>,
}

impl FilePlayer {
    pub async fn new(
        mixer: Arc<Mutex<Box<dyn mixer::Mixer>>>,
    ) -> Result<FilePlayer, Box<dyn std::error::Error>> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        let sink = Arc::new(Mutex::new(sink));

        let mixer = mixer.lock().await;
        let volume_getter = mixer.get_soft_volume();

        Ok(Self {
            sink,
            _stream,
            volume_getter,
        })
    }

    pub async fn play(
        &self,
        file_path: String,
        play_immediately: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!(file_path, "attempting to open file");

        let file = File::open(&file_path)?;
        let file = BufReader::new(file);
        let source = Decoder::new(file)?;

        self.volume_changed().await;

        let sink = self.sink.lock().await;
        sink.play();

        if play_immediately == true && !sink.empty() {
            sink.clear();
        }

        info!(file_path, "Appending to sink queue");
        sink.append(source);
        sink.play();

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sink = self.sink.lock().await;
        sink.stop();
        Ok(())
    }

    pub async fn volume_changed(&self) {
        // TODO: we could use some observer pattern here instead
        let sink = self.sink.lock().await;
        let attenuation_factor = self.volume_getter.attenuation_factor() as f32;
        info!(attenuation_factor, "changing file player volume");
        sink.set_volume(attenuation_factor);
    }
}

unsafe impl Send for FilePlayer {}
unsafe impl Sync for FilePlayer {}
