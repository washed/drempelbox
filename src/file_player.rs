use async_std::prelude::*;
use async_std::sync::Arc;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use tokio::sync::Mutex; // this is more expensive than std::sync::Mutex but makes using it across awaits easier
use tracing::{error, info};

pub struct FilePlayer {
    sink: Arc<Mutex<Sink>>,
    _stream: OutputStream,
}

impl FilePlayer {
    pub async fn new() -> Result<FilePlayer, Box<dyn std::error::Error>> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        let sink = Arc::new(Mutex::new(sink));

        Ok(Self { sink, _stream })
    }

    pub async fn play(
        &self,
        file_path: String,
        play_immediately: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sink = self.sink.lock().await;
        let file = File::open(&file_path)?;
        let file = BufReader::new(file);
        let source = Decoder::new(file)?;

        sink.set_volume(1.0);
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
}

unsafe impl Send for FilePlayer {}
unsafe impl Sync for FilePlayer {}
