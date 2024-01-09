use rppal::gpio::{Error, Gpio, OutputPin};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinSet;
use tracing::{debug, error, warn};

pub enum AmpControlMessage {
    On { responder: oneshot::Sender<()> },
    Off { responder: oneshot::Sender<()> },
    PowerOn { responder: oneshot::Sender<()> },
    PowerOff { responder: oneshot::Sender<()> },
}

#[derive(Clone)]
pub struct Amp {
    sender: Arc<Mutex<UnboundedSender<AmpControlMessage>>>,
}

impl Amp {
    const AMP_POWER_GPIO_PIN: u8 = 20;
    const AMP_SD_GPIO_PIN: u8 = 21;

    pub async fn new(join_set: &mut JoinSet<()>) -> Result<Self, Error> {
        let mut pin_sd = Self::get_pin_sd().ok();
        let mut pin_power = Self::get_pin_power().ok();

        let (sender, mut receiver) = unbounded_channel::<AmpControlMessage>();

        join_set.spawn(async move {
            loop {
                let message = receiver.recv().await;

                match message {
                    Some(message) => match message {
                        AmpControlMessage::On { responder } => {
                            match &mut pin_sd {
                                Some(pin_sd) => pin_sd.set_high(),
                                None => warn!("couldn't set amp sd to high, no pin_sd"),
                            }
                            match responder.send(()) {
                                Ok(_) => {}
                                Err(_) => error!("error sending amp on message response"),
                            };
                        }
                        AmpControlMessage::Off { responder } => {
                            match &mut pin_sd {
                                Some(pin_sd) => pin_sd.set_low(),
                                None => warn!("couldn't set amp sd to low, no pin_sd"),
                            }
                            match responder.send(()) {
                                Ok(_) => {}
                                Err(_) => error!("error sending amp off message response"),
                            };
                        }
                        AmpControlMessage::PowerOn { responder } => {
                            match &mut pin_power {
                                Some(pin_power) => pin_power.set_high(),
                                None => warn!("couldn't set amp power to high, no pin_power"),
                            }
                            match responder.send(()) {
                                Ok(_) => {}
                                Err(_) => error!("error sending amp power on message response"),
                            };
                        }
                        AmpControlMessage::PowerOff { responder } => {
                            match &mut pin_power {
                                Some(pin_power) => pin_power.set_low(),
                                None => warn!("couldn't set amp power to low, no pin_sd"),
                            }
                            match responder.send(()) {
                                Ok(_) => {}
                                Err(_) => error!("error sending amp power off message response"),
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

    fn get_pin_power() -> Result<OutputPin, Error> {
        let mut pin_power: OutputPin = Gpio::new()?.get(Amp::AMP_POWER_GPIO_PIN)?.into_output();
        pin_power.set_low();
        Ok(pin_power)
    }

    fn get_pin_sd() -> Result<OutputPin, Error> {
        let mut pin_sd: OutputPin = Gpio::new()?.get(Amp::AMP_SD_GPIO_PIN)?.into_output();
        pin_sd.set_low();
        Ok(pin_sd)
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

    pub async fn power_on(&self) -> Result<(), tokio::sync::oneshot::error::RecvError> {
        let sender = self.sender.lock().await;
        let (response_sender, response_receiver) = oneshot::channel::<()>();
        match sender.send(AmpControlMessage::PowerOn {
            responder: response_sender,
        }) {
            Ok(_) => debug!("submitted amp power on request"),
            Err(e) => error!("error submitting amp power on request: {e}"),
        };

        response_receiver.await
    }

    pub async fn power_off(&self) -> Result<(), tokio::sync::oneshot::error::RecvError> {
        let sender = self.sender.lock().await;
        let (response_sender, response_receiver) = oneshot::channel::<()>();
        match sender.send(AmpControlMessage::PowerOff {
            responder: response_sender,
        }) {
            Ok(_) => debug!("submitted amp power off request"),
            Err(e) => error!("error submitting amp power off request: {e}"),
        };

        response_receiver.await
    }
}
