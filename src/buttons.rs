use rppal::gpio::{Error, Gpio, InputPin, Level};
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::{error, info, debug};

#[derive(Clone)]
pub struct VolumeButton {}

#[derive(Debug)]
enum VolumePins {
    UP(u8),
    DOWN(u8),
}

impl VolumeButton {
    const VOLUME_UP_PIN: u8 = 17;
    const VOLUME_DOWN_PIN: u8 = 27;
    const BUTTON_POLLING_INTERVAL_MILLIS: u64 = 100;
    const BUTTON_POLLING_MAX_COUNT: u32 = 2;
    
    pub async fn new(join_set: &mut JoinSet<()>) -> Result<Self, Error> {
        let volume_up = VolumePins::UP(Self::VOLUME_UP_PIN);
        let volume_down = VolumePins::DOWN(Self::VOLUME_DOWN_PIN);
        Self::button_action(join_set, volume_up);
        Self::button_action(join_set, volume_down);
        Ok(Self {})
    }

    fn button_action(join_set: &mut JoinSet<()>, volume_pin: VolumePins) {
        let (pin_nr, direction) = match volume_pin {
            VolumePins::UP(nr) => (nr, "Up"),
            VolumePins::DOWN(nr) => (nr, "Down"),
        };

        match Self::get_pin(pin_nr) {
            Ok(pin) => {
                join_set.spawn(async move {
                    info!("Volume {:?}  button task on pin {} started!", direction, pin_nr);
                    let mut change_count = 0;
                    loop {
                        match pin.read() {
                            Level::High => {
                                debug!("{:?} button {} not pressed", direction, pin.pin());
                                change_count = 0;
                            }
                            Level::Low => {
                               debug!("{:?} button {} pressed", direction, pin.pin());
                               if change_count < Self::BUTTON_POLLING_MAX_COUNT {
                                    change_count += 1;
                                } else if change_count == Self::BUTTON_POLLING_MAX_COUNT {
                                    info!("Volume {}!", direction);
                                    // TODO: Send volume change request here
                                    change_count = u32::MAX;  // Indicates that button is pressed 
                                }
                            }
                        }
                        sleep(Duration::from_millis(Self::BUTTON_POLLING_INTERVAL_MILLIS)).await;
                    }   
                });
            }
            Err(e) => {
                error!(%e, "D'oh! on {:?}", volume_pin);
            }
        }
    }

    fn get_pin(pin_nr: u8) -> Result<InputPin, Error> {
        let pin: InputPin = Gpio::new()?
            .get(pin_nr)?
            .into_input_pullup();
        Ok(pin)
    } 
}
