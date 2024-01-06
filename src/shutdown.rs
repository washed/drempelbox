use std::time::Duration;

use rppal::gpio::{Error, Gpio, InputPin};
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct Shutdown {}

impl Shutdown {
    const POWEROFF_GPIO_PIN: u8 = 5;

    pub async fn new(join_set: &mut JoinSet<()>) -> Result<Self, Error> {
        let pin = Self::get_pin().ok();

        join_set.spawn(async move {
            // use interrupt instead of polling!
            debug!("Running shutdown task started");
            loop {
                match pin.as_ref() {
                    Some(pin) => {
                        let level = pin.read();
                        debug!(%level, "SHUTDOWN PIN LEVEL");
                        sleep(Duration::from_secs(1)).await;
                    }
                    None => {
                        warn!("shutdown pin not available!");
                        sleep(Duration::MAX).await;
                    }
                }
            }
        });

        Ok(Self {})
    }

    fn get_pin() -> Result<InputPin, Error> {
        let pin: InputPin = Gpio::new()?
            .get(Self::POWEROFF_GPIO_PIN)?
            .into_input_pullup(); //TODO: pullup ok?
        Ok(pin)
    }
}
