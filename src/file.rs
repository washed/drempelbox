use rodio::source::{SineWave, Source};
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

pub fn play_file() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    // Add a dummy source of the sake of the example.
    /*
    let source = SineWave::new(440.0)
        .take_duration(Duration::from_secs_f32(0.25))
        .amplify(0.20);
    sink.append(source);
    */

    // Load a sound from a file, using a path relative to Cargo.toml
    let file = BufReader::new(File::open("Duel of the Fates.mp3").unwrap());

    // Decode that sound file into a source
    let source = Decoder::new(file).unwrap();

    sink.append(source);
    sink.detach();

    // TODO: probably a better idea is to keep a sink around in the main thread instead of detaching!

    // The sound plays in a separate thread. This call will block the current thread until the sink
    // has finished playing all its queued sounds.
    // sink.sleep_until_end();
}
