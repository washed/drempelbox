use rppal::gpio::{Error, Gpio, OutputPin};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinSet;
use tracing::{debug, error, warn};

pub enum AmpControlMessage {
    On { responder: oneshot::Sender<()> },
    Off { responder: oneshot::Sender<()> },
}

#[derive(Clone)]
pub struct Amp {
    sender: Arc<Mutex<UnboundedSender<AmpControlMessage>>>,
}

impl Amp {
    const AMP_SD_GPIO_PIN: u8 = 21;

    pub async fn new(join_set: &mut JoinSet<()>) -> Result<Self, Error> {
        let mut pin = Self::get_pin().ok();

        let (sender, mut receiver) = unbounded_channel::<AmpControlMessage>();

        join_set.spawn(async move {
            loop {
                let message = receiver.recv().await;

                match message {
                    Some(message) => match message {
                        AmpControlMessage::On { responder } => {
                            match &mut pin {
                                Some(pin) => pin.set_high(),
                                None => warn!("couldn't set amp amp to high, no pin"),
                            }
                            match responder.send(()) {
                                Ok(_) => {}
                                Err(_) => error!("error sending amp on message response"),
                            };
                        }
                        AmpControlMessage::Off { responder } => {
                            match &mut pin {
                                Some(pin) => pin.set_low(),
                                None => warn!("couldn't set amp amp to low, no pin"),
                            }
                            match responder.send(()) {
                                Ok(_) => {}
                                Err(_) => error!("error sending amp off message response"),
                            };
                        }
                    },
                    None => error!("amp channel closed"),
                };
            }
        });

        let sender = Arc::new(Mutex::new(sender));

        Ok(Amp { sender })
    }

    fn get_pin() -> Result<OutputPin, Error> {
        let mut pin: OutputPin = Gpio::new()?.get(Amp::AMP_SD_GPIO_PIN)?.into_output();
        pin.set_low();
        Ok(pin)
    }

    pub async fn on(&self) -> Result<(), tokio::sync::oneshot::error::RecvError> {
        let sender = self.sender.lock().await;
        let (response_sender, response_receiver) = oneshot::channel::<()>();
        match sender.send(AmpControlMessage::On {
            responder: response_sender,
        }) {
            Ok(_) => debug!("submitted amp on request"),
            Err(e) => error!("error submitting amp on request: {e}"),
        };

        response_receiver.await
    }

    pub async fn off(&self) -> Result<(), tokio::sync::oneshot::error::RecvError> {
        let sender = self.sender.lock().await;
        let (response_sender, response_receiver) = oneshot::channel::<()>();
        match sender.send(AmpControlMessage::Off {
            responder: response_sender,
        }) {
            Ok(_) => debug!("submitted amp off request"),
            Err(e) => error!("error submitting amp off request: {e}"),
        };

        response_receiver.await
    }
}
