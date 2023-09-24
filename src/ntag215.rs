use embedded_hal::blocking::delay::DelayMs;
use itertools::Itertools;
use linux_embedded_hal::Delay;
use linux_embedded_hal::Spidev;
use mfrc522::{
    comm::eh02::spi::{DummyDelay, DummyNSS, SpiInterface},
    Initialized, Mfrc522,
};

use crate::ndef::NDEF;

pub struct NTAG215 {
    pub mfrc522: Mfrc522<SpiInterface<Spidev, DummyNSS, DummyDelay>, Initialized>,
    pub memory: [u8; NTAG215::TOTAL_BYTES_COUNT],
}

impl NTAG215 {
    const PAGE_SIZE_BYTES: usize = 4;
    const BLOCK_SIZE_BYTES: usize = 16;
    const BLOCK_PAGE_OFFSET: usize = NTAG215::BLOCK_SIZE_BYTES / NTAG215::PAGE_SIZE_BYTES;
    const PAGE_COUNT: usize = 135;
    const TOTAL_BYTES_COUNT: usize = NTAG215::PAGE_COUNT * NTAG215::PAGE_SIZE_BYTES;
    const FULL_BLOCK_COUNT: usize =
        NTAG215::PAGE_COUNT * NTAG215::PAGE_SIZE_BYTES / NTAG215::BLOCK_SIZE_BYTES;
    const PARTIAL_BLOCKS_PAGE_COUNT: usize = (((NTAG215::PAGE_COUNT as f64
        * NTAG215::PAGE_SIZE_BYTES as f64
        / NTAG215::BLOCK_SIZE_BYTES as f64)
        - NTAG215::FULL_BLOCK_COUNT as f64)
        * NTAG215::BLOCK_SIZE_BYTES as f64
        / NTAG215::PAGE_SIZE_BYTES as f64) as usize;

    // memory region offsets (ends are inclusive)
    const UID_START: u32 = 0;
    const UID_END: u32 = 8;

    const INTERNAL_START: u32 = 9;
    const INTERNAL_END: u32 = 9;

    const LOCK_BYTES_START: u32 = 10;
    const LOCK_BYTES_END: u32 = 11;

    const CAPABILITY_CONTAINER_START: u32 = 12;
    const CAPABILITY_CONTAINER_END: u32 = 15;

    const USER_MEMORY_START: u32 = 16;
    const USER_MEMORY_END: u32 = 515;

    const DYNAMIC_LOCK_BYTES_START: u32 = 516;
    const DYNAMIC_LOCK_BYTES_END: u32 = 518;

    const RFUI_0_START: u32 = 519;
    const RFUI_0_END: u32 = 519;

    const CFG_0_START: u32 = 520;
    const CFG_0_END: u32 = 523;

    const CFG_1_START: u32 = 524;
    const CFG_1_END: u32 = 527;

    const PWD_START: u32 = 528;
    const PWD_END: u32 = 531;

    const PACK_START: u32 = 532;
    const PACK_END: u32 = 533;

    const RFUI_1_START: u32 = 534;
    const RFUI_1_END: u32 = 535;

    pub fn new(
        mut mfrc522: Mfrc522<SpiInterface<Spidev, DummyNSS, DummyDelay>, Initialized>,
    ) -> Self {
        let vers = mfrc522.version().expect("Error getting MFRC522 version");
        assert!(vers == 0x91 || vers == 0x92);

        Self {
            mfrc522,
            memory: [0; NTAG215::TOTAL_BYTES_COUNT],
        }
    }

    pub fn read(&mut self) {
        let mut delay = Delay;

        loop {
            if let Ok(atqa) = self.mfrc522.reqa() {
                if let Ok(uid) = self.mfrc522.select(&atqa) {
                    println!("UID: {:02x}", uid.as_bytes().iter().format(""));

                    self.read_blocks();

                    let ndef = NDEF::parse(&self.memory);
                }
            }
            delay.delay_ms(100u32);
        }
    }

    fn read_blocks(&mut self) {
        for (block_num, chunk) in (0..NTAG215::FULL_BLOCK_COUNT).zip(
            self.memory
                .chunks_exact_mut(NTAG215::BLOCK_SIZE_BYTES as usize),
        ) {
            let page_addr = block_num * NTAG215::BLOCK_PAGE_OFFSET;
            if let Ok(block) = self
                .mfrc522
                .mf_read(u8::try_from(page_addr).expect("Tried to read out of bound block!"))
            {
                for (dest, src) in chunk.iter_mut().zip(block.iter()) {
                    *dest = *src
                }
            }
        }

        if NTAG215::PARTIAL_BLOCKS_PAGE_COUNT != 0 {
            let page_addr = NTAG215::FULL_BLOCK_COUNT * NTAG215::BLOCK_PAGE_OFFSET;
            if let Ok(partial_block) = self
                .mfrc522
                .mf_read(u8::try_from(page_addr).expect("Tried to read out of bound block!"))
            {
                for (dest, src) in self.memory
                    [(NTAG215::FULL_BLOCK_COUNT as usize * NTAG215::BLOCK_SIZE_BYTES as usize)..]
                    .iter_mut()
                    .zip(partial_block.iter())
                {
                    *dest = *src
                }
            }
        }

        for block in self.memory.chunks(NTAG215::BLOCK_SIZE_BYTES as usize) {
            println!("{:02x}", block.iter().format(""));
        }
        println!()
    }
}
