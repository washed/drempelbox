use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::Spidev;
use mfrc522::comm::eh02::spi::SpiInterface;
use mfrc522::Mfrc522;
use tokio::task::JoinSet;
use tracing::{error, info};

use crate::ntag215::NTAG215;

pub async fn start_ntag_reader_task(join_set: &mut JoinSet<()>) {
    join_set.spawn(async move {
        match start_ntag_reader().await {
            Ok(_) => {}
            Err(e) => {
                error!(e, "Error starting NTAG reader");
                panic!();
            }
        };
    });
}

async fn start_ntag_reader() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting NTAG reader...");
    let mut spi = Spidev::open("/dev/spidev0.0")?;
    let options = SpidevOptions::new()
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;

    let itf = SpiInterface::new(spi);
    let mfrc522 = Mfrc522::new(itf).init()?;
    let mut ntag = NTAG215::new(mfrc522);
    ntag.read();

    // this probably needs a loop and ndef parsing and then sending a message

    Ok(())
}
