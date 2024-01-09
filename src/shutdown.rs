use rppal::gpio::{Error, Gpio, InputPin, Level};
use std::time::Duration;
use system_shutdown::shutdown;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::{debug, error, warn};

#[derive(Clone)]
pub struct Shutdown {}

impl Shutdown {
    const POWEROFF_GPIO_PIN: u8 = 5;
    const POWEROFF_HOLD_ITERATIONS: u8 = 4;

    pub async fn new(join_set: &mut JoinSet<()>) -> Result<Self, Error> {
        let mut low_counter = 0;
        match Self::get_pin() {
            Ok(pin) => {
                join_set.spawn(async move {
                    // use interrupt instead of polling!
                    debug!("shutdown task started");
                    loop {
                        match pin.read() {
                            Level::High => {}
                            Level::Low => {
                                low_counter += 1;
                                debug!(low_counter, "shutdown pin low");
                                if low_counter >= Self::POWEROFF_HOLD_ITERATIONS {
                                    warn!("attempting shutdown");
                                    match shutdown() {
                                        Ok(_) => println!("Shutting down, bye!"),
                                        Err(error) => eprintln!("Failed to shut down: {}", error),
                                    }
                                }
                            }
                        }
                        sleep(Duration::from_secs(1)).await;
                    }
                });
            }
            Err(e) => {
                error!(%e, "shutdown pin not available!");
            }
        }
        Ok(Self {})
    }

    fn get_pin() -> Result<InputPin, Error> {
        let pin: InputPin = Gpio::new()?
            .get(Self::POWEROFF_GPIO_PIN)?
            .into_input_pullup(); //TODO: pullup ok?
        Ok(pin)
    }
}
