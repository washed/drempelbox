use crate::ndef::Message;
use mfrc522::{
    comm::eh02::spi::{DummyNSS, SpiInterface},
    Initialized, Mfrc522, Uid,
};
use rppal::spi::Spi;
use tracing::{error, info};

#[allow(dead_code)]
mod constants {
    pub const PAGE_SIZE_BYTES: usize = 4;
    pub const BLOCK_SIZE_BYTES: usize = 16;
    pub const BLOCK_PAGE_OFFSET: usize = BLOCK_SIZE_BYTES / PAGE_SIZE_BYTES;
    pub const PAGE_COUNT: usize = 135;
    pub const TOTAL_BYTES_COUNT: usize = PAGE_COUNT * PAGE_SIZE_BYTES;
    pub const FULL_BLOCK_COUNT: usize = PAGE_COUNT * PAGE_SIZE_BYTES / BLOCK_SIZE_BYTES;
    pub const PARTIAL_BLOCKS_PAGE_COUNT: usize = (((PAGE_COUNT as f64 * PAGE_SIZE_BYTES as f64
        / BLOCK_SIZE_BYTES as f64)
        - FULL_BLOCK_COUNT as f64)
        * BLOCK_SIZE_BYTES as f64
        / PAGE_SIZE_BYTES as f64) as usize;

    // memory region offsets (ends are inclusive)
    pub const UID_START: usize = 0;
    pub const UID_END: usize = 8;

    pub const INTERNAL_START: usize = 9;
    pub const INTERNAL_END: usize = 9;

    pub const LOCK_BYTES_START: usize = 10;
    pub const LOCK_BYTES_END: usize = 11;

    pub const CAPABILITY_CONTAINER_START: usize = 12;
    pub const CAPABILITY_CONTAINER_END: usize = 15;

    pub const USER_MEMORY_START: usize = 16;
    pub const USER_MEMORY_END: usize = 515;

    pub const DYNAMIC_LOCK_BYTES_START: usize = 516;
    pub const DYNAMIC_LOCK_BYTES_END: usize = 518;

    pub const RFUI_0_START: usize = 519;
    pub const RFUI_0_END: usize = 519;

    pub const CFG_0_START: usize = 520;
    pub const CFG_0_END: usize = 523;

    pub const CFG_1_START: usize = 524;
    pub const CFG_1_END: usize = 527;

    pub const PWD_START: usize = 528;
    pub const PWD_END: usize = 531;

    pub const PACK_START: usize = 532;
    pub const PACK_END: usize = 533;

    pub const RFUI_1_START: usize = 534;
    pub const RFUI_1_END: usize = 535;
}

pub struct NTAG215<D: FnMut()> {
    pub mfrc522: Mfrc522<SpiInterface<Spi, DummyNSS, D>, Initialized>,
    pub memory: [u8; constants::TOTAL_BYTES_COUNT],
}

impl<D: FnMut()> NTAG215<D> {
    pub fn new(mut mfrc522: Mfrc522<SpiInterface<Spi, DummyNSS, D>, Initialized>) -> Self {
        let version = mfrc522.version().expect("Error getting MFRC522 version");
        info!(version, "MFRC522 version");

        assert!(version == 0x91 || version == 0x92);

        Self {
            mfrc522,
            memory: [0; constants::TOTAL_BYTES_COUNT],
        }
    }

    pub fn select(&mut self) -> Option<Uid> {
        let atqa = match self.mfrc522.reqa() {
            Ok(atqa) => Some(atqa),
            Err(e) => {
                error!("error in SPI comms: {:#?}", e);
                self.mfrc522.hlta().ok();
                self.mfrc522.wupa().ok()
            }
        };

        match atqa {
            Some(atqa) => match self.mfrc522.select(&atqa) {
                Ok(atqa) => Some(atqa),
                Err(_) => None,
            },
            None => None,
        }
    }

    pub fn is_token_present(&mut self) -> Option<Uid> {
        self.select()
    }

    pub fn read(&mut self) -> Option<Message> {
        self.select()?;
        self.read_blocks();

        let user_memory = &self.memory[constants::USER_MEMORY_START..constants::USER_MEMORY_END];
        let message = Message::parse(user_memory)?;
        Some(message)
    }

    fn read_blocks(&mut self) {
        for (block_num, chunk) in (0..constants::FULL_BLOCK_COUNT)
            .zip(self.memory.chunks_exact_mut(constants::BLOCK_SIZE_BYTES))
        {
            let page_addr = block_num * constants::BLOCK_PAGE_OFFSET;
            if let Ok(block) = self
                .mfrc522
                .mf_read(u8::try_from(page_addr).expect("Tried to read out of bound block!"))
            {
                for (dest, src) in chunk.iter_mut().zip(block.iter()) {
                    *dest = *src
                }
            }
        }

        if constants::PARTIAL_BLOCKS_PAGE_COUNT != 0 {
            let page_addr = constants::FULL_BLOCK_COUNT * constants::BLOCK_PAGE_OFFSET;
            if let Ok(partial_block) = self
                .mfrc522
                .mf_read(u8::try_from(page_addr).expect("Tried to read out of bound block!"))
            {
                for (dest, src) in self.memory
                    [(constants::FULL_BLOCK_COUNT * constants::BLOCK_SIZE_BYTES)..]
                    .iter_mut()
                    .zip(partial_block.iter())
                {
                    *dest = *src
                }
            }
        }
    }
}
