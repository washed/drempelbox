use rppal::gpio::{Error, Gpio};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinSet;
use tracing::error;

pub struct Amp {}

impl Amp {
    const AMP_SD_GPIO_PIN: u8 = 21;

    pub async fn new(join_set: &mut JoinSet<()>) -> Result<UnboundedSender<bool>, Error> {
        let mut pin = Gpio::new()?.get(Amp::AMP_SD_GPIO_PIN)?.into_output();

        pin.set_low();

        let (sender, mut receiver) = unbounded_channel::<bool>();

        join_set.spawn(async move {
            loop {
                let message = receiver.recv().await;

                match message {
                    Some(enable) => match enable {
                        true => pin.set_high(),
                        false => pin.set_low(),
                    },
                    None => error!("amp channel closed"),
                };
            }
        });

        Ok(sender)
    }
}
