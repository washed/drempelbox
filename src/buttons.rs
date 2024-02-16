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
    pub async fn new(join_set: &mut JoinSet<()>) -> Result<Self, Error> {
        let volume_up = VolumePins::UP(17);
        let volume_down = VolumePins::DOWN(27);
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
                    debug!("Volume {:?}  button task on pin {} started!", direction, pin_nr);
                    loop {
                        match pin.read() {
                            Level::High => {
                                info!("{:?} button {} not pressed", direction, pin.pin());
                            }
                            Level::Low => {
                                info!("{:?} button {} pressed", direction, pin.pin());
                            }
                        }
                        sleep(Duration::from_secs(1)).await;
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
