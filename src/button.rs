#![allow(unused_imports)]

use rppal::gpio::{Error, Gpio, InputPin, Level, Trigger};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::time::Duration;
use tokio::spawn;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tracing::{debug, error, info};

pub struct Button {
    pub receiver: broadcast::Receiver<()>,
}

impl Button {
    const EVENT_CHANNEL_CAPACITY: usize = 16;

    pub fn new(pin_number: u8) -> Result<Self, Error> {
        let mut pin: InputPin = Gpio::new()?.get(pin_number)?.into_input_pullup();
        let trigger: Trigger = Trigger::RisingEdge;
        let (_sender, receiver) = broadcast::channel(Self::EVENT_CHANNEL_CAPACITY);
        spawn(async move {
            if let Err(e) = pin.set_async_interrupt(trigger, |level: Level| Self::callback(level)) {
                error!("Error setting interrupt: {}", e);
            }
            loop { sleep(Duration::from_secs(1)).await; } // This is stupid
        });
        Ok(Self { receiver })
    }

    fn callback(level: Level) {
        // Desired signature something along the lines of
        // callback(level: Level, pin: InputPin, sender: Sender)
        // to be called from the anonymous function in new()
        info!("Button pressed. Level: {}", level);
        // Here comes the polling and call to sender.send()
    }
}
