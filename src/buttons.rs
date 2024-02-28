use async_trait::async_trait;
use rppal::gpio::{Error, Gpio, InputPin, Level};
use serde::Serialize;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::{player::PlayerRequestMessage, server::AppState};

#[derive(Clone)]
pub struct VolumeButton {}

#[derive(Debug)]
enum VolumePins {
    UP(u8),
    DOWN(u8),
}

#[derive(Serialize)]
struct Volume {
    volume: f64,
}

#[async_trait]
trait ButtonAction {
    async fn action(app_state: AppState);
}

struct VolumeUp {}
struct VolumeDown {}

#[async_trait]
impl ButtonAction for VolumeUp {
    async fn action(app_state: AppState) {
        info!("Volume up!");
        let (sender, receiver) = oneshot::channel::<f64>();
        let player_request_message = PlayerRequestMessage::VolumeUp { responder: sender };

        match app_state.sender.send(player_request_message).await {
            Ok(_) => debug!("Submitted volume up request."),
            Err(e) => error!("Error submitting volume up request: {e}"),
        };
        match receiver.await {
            Ok(response) => debug!("Player acknowledged volume up command: {response}."),
            Err(_) => error!("didn't receive player command response"),
        }
    }
}

#[async_trait]
impl ButtonAction for VolumeDown {
    async fn action(app_state: AppState) {
        info!("Volume down!");
        let (sender, receiver) = oneshot::channel::<f64>();
        let player_request_message = PlayerRequestMessage::VolumeDown { responder: sender };

        match app_state.sender.send(player_request_message).await {
            Ok(_) => debug!("Submitted volume down request."),
            Err(e) => error!("Error submitting volume down request: {e}"),
        };
        match receiver.await {
            Ok(response) => debug!("Player acknowledged volume down command: {response}."),
            Err(_) => error!("didn't receive player command response"),
        }
    }
}

impl VolumeButton {
    const VOLUME_UP_PIN: u8 = 27;
    const VOLUME_DOWN_PIN: u8 = 17;
    const BUTTON_POLLING_INTERVAL_MILLIS: u64 = 100;
    const BUTTON_POLLING_MAX_COUNT: u32 = 2;

    pub async fn new(join_set: &mut JoinSet<()>, app_state: AppState) -> Result<Self, Error> {
        let volume_up = VolumePins::UP(Self::VOLUME_UP_PIN);
        let volume_down = VolumePins::DOWN(Self::VOLUME_DOWN_PIN);
        Self::button_action(join_set, volume_up, app_state.clone()).await;
        Self::button_action(join_set, volume_down, app_state.clone()).await;
        Ok(Self {})
    }

    async fn button_action(
        join_set: &mut JoinSet<()>,
        volume_pin: VolumePins,
        app_state: AppState,
    ) {
        let (pin_nr, direction) = match volume_pin {
            VolumePins::UP(nr) => (nr, "Up"),
            VolumePins::DOWN(nr) => (nr, "Down"),
        };

        match Self::get_pin(pin_nr) {
            Ok(pin) => {
                join_set.spawn(async move {
                    info!(
                        "Volume {:?}  button task on pin {} started!",
                        direction, pin_nr
                    );
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
                                    match volume_pin {
                                        VolumePins::UP(_) => {
                                            VolumeUp::action(app_state.clone()).await
                                        }
                                        VolumePins::DOWN(_) => {
                                            VolumeDown::action(app_state.clone()).await
                                        }
                                    };
                                    change_count = 0;
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
        let pin: InputPin = Gpio::new()?.get(pin_nr)?.into_input_pullup();
        Ok(pin)
    }
}
