use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::Spidev;
use mfrc522::comm::eh02::spi::SpiInterface;
use mfrc522::Mfrc522;

pub mod ntag215;
use crate::ntag215::NTAG215;

pub mod ndef;
use crate::ndef::NDEF;

fn main() {
    let mut spi = Spidev::open("/dev/spidev0.0").unwrap();
    let options = SpidevOptions::new()
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options).unwrap();

    let itf = SpiInterface::new(spi);
    let mfrc522 = Mfrc522::new(itf).init().expect("Error initing mfrc522");

    let mut ntag = NTAG215::new(mfrc522);
    ntag.read();

    let _ndef = NDEF::parse(&ntag.memory);
}
