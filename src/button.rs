#![allow(unused_imports)]

use debounce::EventDebouncer;
use rppal::gpio::{Error, Gpio, InputPin, Level, Trigger};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::time::Duration;
use tokio::spawn;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

pub struct Button {
    pub receiver: broadcast::Receiver<Level>,
    _pin: InputPin,
}

impl Button {
    const EVENT_CHANNEL_CAPACITY: usize = 16;
    const BUTTON_PRESSED_DURATION_MS: u64 = 100;

    pub fn new(pin_number: u8) -> Result<Self, Error> {
        let mut pin: InputPin = Gpio::new()?.get(pin_number)?.into_input_pullup();
        let trigger: Trigger = Trigger::Both;
        let (sender, receiver) = broadcast::channel(Self::EVENT_CHANNEL_CAPACITY);
        let pin_name = pin.pin();

        let debounce_delay = Duration::from_millis(Button::BUTTON_PRESSED_DURATION_MS);
        let debouncer = EventDebouncer::new(debounce_delay, move |level: Level| {
            info!(
                pin_name=?pin_name,
                level=?level,
                "pin {pin_name} debounced level {level}"
            );
            if let Err(e) = sender.send(level) {
                error!("error sending button event message: {}", e)
            }
        });

        if let Err(e) = pin.set_async_interrupt(trigger, move |level: Level| {
            debug!(
                pin_name=?pin_name,
                trigger=?trigger, level=?level,
                "pin {pin_name} triggered on {trigger} level {level}"
            );
            debouncer.put(level);
        }) {
            error!("Error setting interrupt: {}", e);
        }

        Ok(Self {
            receiver,
            _pin: pin,
        })
    }
}
