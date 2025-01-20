use alsa::mixer::SelemId;
use alsa::{Error, Mixer};
use tokio::task::JoinSet;
use tracing::info;

#[derive(Clone)]
pub struct Alsa {}

impl Alsa {
    const TARGET_VOLUME_PERCENT: i64 = 80;

    pub async fn new(_join_set: &mut JoinSet<()>) -> Result<Self, Error> {
        let mixer = Mixer::new("default", false).expect("Failed to open mixer.");
        let selem_id = SelemId::new("Master", 0);
        let selem = mixer
            .find_selem(&selem_id)
            .expect("Failed to find mixer control.");
        let (min, max) = selem.get_playback_volume_range();
        let target_volume = (max - min) * Self::TARGET_VOLUME_PERCENT / 100 + min;
        selem
            .set_playback_volume_all(target_volume)
            .expect("Failed to set volume.");
        info!(
            "Setting volume to {} ({}% of range {} - {}).",
            target_volume,
            Self::TARGET_VOLUME_PERCENT,
            min,
            max
        );
        Ok(Self {})
    }
}
