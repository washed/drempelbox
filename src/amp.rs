use async_std::sync::Arc;
use linux_embedded_hal::sysfs_gpio::{Direction, Error};
use linux_embedded_hal::Pin;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};
use tracing::error;

pub struct Amp {
    pin: Arc<Mutex<Pin>>,
    receiver: UnboundedReceiver<bool>,
}

impl Amp {
    const AMP_SD_GPIO_PIN: u64 = 21;
    const DELAY: Duration = Duration::from_millis(1);

    pub async fn new(join_set: &mut JoinSet<()>) -> Result<UnboundedSender<bool>, Error> {
        let pin = Pin::new(Amp::AMP_SD_GPIO_PIN);
        pin.export()?;
        while !pin.is_exported() {
            sleep(Amp::DELAY).await;
        }
        // delay sometimes necessary because `is_exported()` returns too early?
        sleep(Amp::DELAY).await;

        pin.set_value(1)?;
        pin.set_direction(Direction::Out)?;

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
        match {
            match enable {
                true => self.on().await,
                false => self.off().await,
            }
        } {
            Ok(_) => {}
            Err(_) => error!(enable, "error switching amp"),
        }
    }

    async fn on(&self) -> Result<(), Error> {
        // main amp turn on!
        let pin = self.pin.lock().await;
        pin.set_value(0)?;
        Ok(())
    }

    async fn off(&self) -> Result<(), Error> {
        let pin = self.pin.lock().await;
        pin.set_value(1)?;
        Ok(())
    }
}
