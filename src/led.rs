use std::sync::Arc;

use rppal::gpio::{Error, Gpio, OutputPin};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        oneshot, Mutex,
    },
    task::JoinSet,
};
use tracing::{debug, error, warn};

pub enum LedControlMessage {
    On { responder: oneshot::Sender<()> },
    Off { responder: oneshot::Sender<()> },
}

#[derive(Clone)]
pub struct Led {
    sender: Arc<Mutex<UnboundedSender<LedControlMessage>>>,
}

impl Led {
    const LED_GPIO_PIN: u8 = 12;

    pub async fn new(join_set: &mut JoinSet<()>) -> Result<Self, Error> {
        let mut pin_led = Self::get_pin_led().ok();

        let (sender, mut receiver) = unbounded_channel::<LedControlMessage>();

        join_set.spawn(async move {
            loop {
                let message = receiver.recv().await;

                match message {
                    Some(message) => match message {
                        LedControlMessage::On { responder } => {
                            match &mut pin_led {
                                Some(pin_led) => pin_led.set_high(),
                                None => warn!("couldn't set amp LED to high, no pin_led"),
                            }
                            match responder.send(()) {
                                Ok(_) => {}
                                Err(_) => error!("error sending amp LED on message response"),
                            };
                        }
                        LedControlMessage::Off { responder } => {
                            match &mut pin_led {
                                Some(pin_led) => pin_led.set_low(),
                                None => warn!("couldn't set amp LED to low, no pin_led"),
                            }
                            match responder.send(()) {
                                Ok(_) => {}
                                Err(_) => error!("error sending amp LED off message response"),
                            };
                        }
                    },
                    None => error!("led channel closed"),
                };
            }
        });

        let sender = Arc::new(Mutex::new(sender));

        Ok(Led { sender })
    }

    fn get_pin_led() -> Result<OutputPin, Error> {
        let mut pin_led: OutputPin = Gpio::new()?.get(Led::LED_GPIO_PIN)?.into_output();
        pin_led.set_low();
        Ok(pin_led)
    }

    pub async fn on(&self) -> Result<(), tokio::sync::oneshot::error::RecvError> {
        let sender = self.sender.lock().await;
        let (response_sender, response_receiver) = oneshot::channel::<()>();
        match sender.send(LedControlMessage::On {
            responder: response_sender,
        }) {
            Ok(_) => debug!("submitted amp LED on request"),
            Err(e) => error!("error submitting amp LED on request: {e}"),
        };

        response_receiver.await
    }

    pub async fn off(&self) -> Result<(), tokio::sync::oneshot::error::RecvError> {
        let sender = self.sender.lock().await;
        let (response_sender, response_receiver) = oneshot::channel::<()>();
        match sender.send(LedControlMessage::Off {
            responder: response_sender,
        }) {
            Ok(_) => debug!("submitted amp LED off request"),
            Err(e) => error!("error submitting amp LED off request: {e}"),
        };

        response_receiver.await
    }
}
