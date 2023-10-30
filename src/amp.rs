use async_std::sync::Arc;
use rppal::gpio::{Error, Gpio, OutputPin};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::error;

pub struct Amp {
    pin: Arc<Mutex<OutputPin>>,
    receiver: UnboundedReceiver<bool>,
}

impl Amp {
    const AMP_SD_GPIO_PIN: u8 = 21;

    pub async fn new(join_set: &mut JoinSet<()>) -> Result<UnboundedSender<bool>, Error> {
        let mut pin = Gpio::new()?.get(Amp::AMP_SD_GPIO_PIN)?.into_output();

        pin.set_high();

        let pin = Arc::new(Mutex::new(pin));
        let (sender, receiver) = unbounded_channel::<bool>();
        let mut amp = Self { pin, receiver };

        join_set.spawn(async move {
            loop {
                let message = amp.receiver.recv().await;

                match message {
                    Some(message) => amp.enable(message).await,
                    None => error!("amp channel closed"),
                };
            }
        });

        Ok(sender)
    }

    async fn enable(&self, enable: bool) {
        match enable {
            true => self.on().await,
            false => self.off().await,
        };
    }

    async fn on(&self) {
        // main amp turn on!
        let mut pin = self.pin.lock().await;
        pin.set_low();
    }

    async fn off(&self) {
        let mut pin = self.pin.lock().await;
        pin.set_high();
    }
}
