use crate::ndef::Record;
use crate::ntag215::NTAG215;
use crate::player::PlayerRequestMessage;
use crate::server::AppState;
use crate::tuple_windows::TupleWindowsExt;
use async_std::sync::Arc;
use mfrc522::comm::eh02::spi::SpiInterface;
use mfrc522::Mfrc522;
use rppal::gpio::Gpio;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tracing::{error, info};
use url::Url;

pub async fn start_ntag_reader_task(join_set: &mut JoinSet<()>, app_state: AppState) {
    match start_ntag_reader_task_impl(join_set, app_state).await {
        Ok(_) => {}
        Err(e) => error!(e, "error starting ntag reader task"),
    }
}

fn delay() {
    std::thread::sleep(Duration::from_nanos(50));
}

async fn start_ntag_reader_task_impl(
    join_set: &mut JoinSet<()>,
    app_state: AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting NTAG reader...");

    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)?;
    let itf = SpiInterface::new(spi).with_delay(delay);

    let gpio = Gpio::new()?;

    // Not used for now, but let's initialize it correctly
    let _irq_pin = gpio.get(24)?.into_input_pullup();

    // TODO: do we need to keep pin instances around to ensure their state remains stable?
    let mut reset_pin = gpio.get(25)?.into_output_low();
    sleep(Duration::from_micros(50)).await;
    reset_pin.set_high();
    sleep(Duration::from_micros(50)).await;

    let mfrc522 = Mfrc522::new(itf).init()?;
    sleep(Duration::from_micros(100)).await;

    let ntag = Arc::new(Mutex::new(NTAG215::new(mfrc522)));

    let (tx, rx) = tokio::sync::mpsc::channel::<Option<[u8; 7]>>(16);

    let stream = ReceiverStream::new(rx);
    let mut stream = stream.tuple_windows();

    let ntag_rx = ntag.clone();
    join_set.spawn(async move {
        while let Some(value) = stream.next().await {
            match value {
                (None, Some(uid)) => {
                    info!("new token: {:02x?}", uid);
                    let mut ntag = ntag_rx.lock().await;

                    let result = ntag.read();
                    match result {
                        Some(ndef) => {
                            // TODO: only the first record is used
                            let Record::URI { uri } = &ndef.records[0];
                            let url = match Url::parse(uri) {
                                Ok(url) => url,
                                Err(e) => {
                                    let e = e.to_string();
                                    error!(e, "error parsing url from token");
                                    continue;
                                }
                            };
                            match app_state.sender.send(PlayerRequestMessage::URL(url)).await {
                                Ok(_) => {}
                                Err(_) => error!("couldn't send spotify request from ntag"),
                            }
                        }
                        None => error!("error parsing ndef"),
                    };
                }
                (Some(uid), None) => {
                    info!("token removed: {:02x?}", uid);
                    match app_state.sender.send(PlayerRequestMessage::Stop).await {
                        Ok(_) => {}
                        Err(_) => error!("couldn't send spotify request from ntag"),
                    };
                }
                _ => {}
            };
        }
        error!("ntag task ended");
    });

    let ntag_tx = ntag.clone();
    join_set.spawn(async move {
        loop {
            // we can probably simplify this a bunch
            let mut ntag = ntag_tx.lock().await;
            let uid = ntag.is_token_present();
            let uid = match uid {
                Some(uid) => {
                    let uid: [u8; 7] = match uid.as_bytes().try_into() {
                        Ok(res) => res,
                        Err(_) => {
                            error!("error casting uid!");
                            let dummy: [u8; 7] = [0; 7];
                            dummy
                        }
                    };
                    Some(uid)
                }
                None => None,
            };

            match tx.send(uid).await {
                Ok(_) => {}
                Err(_) => error!("error sending into channel"),
            };
            sleep(Duration::from_millis(500)).await;
        }
    });

    Ok(())
}
