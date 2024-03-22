use crate::button::Button;
use crate::player::PlayerRequestMessage;
use rppal::gpio::Error;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::{select, spawn};
use tracing::{debug, error, info};

pub struct VolumeButtons {}

impl VolumeButtons {
    const VOLUME_UP_PIN: u8 = 27;
    const VOLUME_DOWN_PIN: u8 = 17;

    pub fn new(player_sender: mpsc::Sender<PlayerRequestMessage>) -> Result<Self, Error> {
        let mut button_up = Button::new(Self::VOLUME_UP_PIN)?;
        let mut button_down = Button::new(Self::VOLUME_DOWN_PIN)?;
        spawn(async move {
            loop {
                select! {
                    Ok(_) = button_up.receiver.recv() => Self::volume_up(&player_sender).await,
                    Ok(_) = button_down.receiver.recv() => Self::volume_down(&player_sender).await,
                }
            }
        });
        Ok(Self {})
    }

    async fn volume_up(player_sender: &mpsc::Sender<PlayerRequestMessage>) {
        info!("Volume up!");
        let (sender, receiver) = oneshot::channel::<f64>();
        let player_request_message = PlayerRequestMessage::VolumeUp { responder: sender };

        match player_sender.send(player_request_message).await {
            Ok(_) => debug!("Submitted volume up request."),
            Err(e) => error!("Error submitting volume up request: {e}"),
        };
        match receiver.await {
            Ok(response) => debug!("Player acknowledged volume up command: {response}."),
            Err(_) => error!("didn't receive player command response"),
        }
    }

    async fn volume_down(player_sender: &mpsc::Sender<PlayerRequestMessage>) {
        info!("Volume down!");
        let (sender, receiver) = oneshot::channel::<f64>();
        let player_request_message = PlayerRequestMessage::VolumeDown { responder: sender };

        match player_sender.send(player_request_message).await {
            Ok(_) => debug!("Submitted volume down request."),
            Err(e) => error!("Error submitting volume down request: {e}"),
        };
        match receiver.await {
            Ok(response) => debug!("Player acknowledged volume down command: {response}."),
            Err(_) => error!("didn't receive player command response"),
        }
    }
}
