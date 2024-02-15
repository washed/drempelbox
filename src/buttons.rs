use rppal::gpio::{Error, Gpio, InputPin, Level};
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::{error, info, debug};

#[derive(Clone)]
pub struct VolumeButton {}

impl VolumeButton {
    const VOLUME_PIN_UP: u8 = 17;
    const VOLUME_PIN_DOWN: u8 = 27;

    pub async fn new(join_set: &mut JoinSet<()>) -> Result<Self, Error> {
         match Self::get_pin_up() {
            Ok(pin) => {
                join_set.spawn(async move {
                    debug!("Volume Button task started!");
                    loop {
                        match pin.read() {
                            Level::High => {
                                info!("Up button {} not pressed", pin.pin());
                            }
                            Level::Low => {
                                info!("Up button {} pressed", pin.pin());
                            }
                        }
                        sleep(Duration::from_secs(1)).await;
                    }   
                });
            }
            Err(e) => {
                error!(%e, "D'oh!");
            }
        }       
        match Self::get_pin_down() {
            Ok(pin) => {
                join_set.spawn(async move {
                    debug!("Volume Button Down task started!");
                    loop {
                        match pin.read() {
                            Level::High => {
                                info!("Down button {} not pressed", pin.pin());
                            }
                            Level::Low => {
                                info!("Down button {} pressed", pin.pin());
                            }
                        }
                        sleep(Duration::from_secs(1)).await;
                    }   
                });
            }
            Err(e) => {
                error!(%e, "D'oh!");
            }
        }
        Ok(Self {})
    }

    fn get_pin_up() -> Result<InputPin, Error> {
        let pin: InputPin = Gpio::new()?
            .get(Self::VOLUME_PIN_UP)?
            .into_input_pullup();
        Ok(pin)
    } 
    fn get_pin_down() -> Result<InputPin, Error> {
        let pin: InputPin = Gpio::new()?
            .get(Self::VOLUME_PIN_DOWN)?
            .into_input_pullup();
        Ok(pin)
    }
}
