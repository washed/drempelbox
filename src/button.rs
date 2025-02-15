use rppal::gpio::{Error, Gpio, InputPin, Level};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::time::Duration;
use tokio::spawn;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tracing::{error, info, trace};

pub struct Button {
    pub receiver: broadcast::Receiver<()>,
}

impl Button {
    const BUTTON_POLLING_MAX_COUNT: u64 = 10;
    const BUTTON_POLLING_INTERVAL_MILLIS: u64 = 10;
    const EVENT_CHANNEL_CAPACITY: usize = 16;

    pub fn new(pin_number: u8) -> Result<Self, Error> {
        let pin: InputPin = Gpio::new()?.get(pin_number)?.into_input_pullup();
        let (sender, receiver) = broadcast::channel(Self::EVENT_CHANNEL_CAPACITY);
        spawn(async move {
            info!(pin=?pin, "button poll task on pin {:?} started", pin);
            let mut change_count = 0;
            loop {
                match pin.read() {
                    Level::High => {
                        trace!("button at pin {:?} not pressed", pin);
                        change_count = 0;
                    }
                    Level::Low => {
                        trace!("button at pin {:?} pressed", pin);

                        match change_count.cmp(&Self::BUTTON_POLLING_MAX_COUNT) {
                            Less => change_count += 1,
                            Equal | Greater => {
                                if let Err(e) = sender.send(()) {
                                    error!("error sending button event message: {}", e)
                                }
                                change_count = 0;
                            }
                        }
                    }
                }
                sleep(Duration::from_millis(Self::BUTTON_POLLING_INTERVAL_MILLIS)).await;
            }
        });

        Ok(Self { receiver })
    }
}
