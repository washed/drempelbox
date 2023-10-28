use crate::ndef::Message;
use linux_embedded_hal::Spidev;
use mfrc522::{
    comm::eh02::spi::{DummyDelay, DummyNSS, SpiInterface},
    Initialized, Mfrc522, Uid,
};

pub struct NTAG215 {
    pub mfrc522: Mfrc522<SpiInterface<Spidev, DummyNSS, DummyDelay>, Initialized>,
    pub memory: [u8; NTAG215::TOTAL_BYTES_COUNT],
}

impl NTAG215 {
    pub const PAGE_SIZE_BYTES: usize = 4;
    pub const BLOCK_SIZE_BYTES: usize = 16;
    pub const BLOCK_PAGE_OFFSET: usize = NTAG215::BLOCK_SIZE_BYTES / NTAG215::PAGE_SIZE_BYTES;
    pub const PAGE_COUNT: usize = 135;
    pub const TOTAL_BYTES_COUNT: usize = NTAG215::PAGE_COUNT * NTAG215::PAGE_SIZE_BYTES;
    pub const FULL_BLOCK_COUNT: usize =
        NTAG215::PAGE_COUNT * NTAG215::PAGE_SIZE_BYTES / NTAG215::BLOCK_SIZE_BYTES;
    pub const PARTIAL_BLOCKS_PAGE_COUNT: usize =
        (((NTAG215::PAGE_COUNT as f64 * NTAG215::PAGE_SIZE_BYTES as f64
            / NTAG215::BLOCK_SIZE_BYTES as f64)
            - NTAG215::FULL_BLOCK_COUNT as f64)
            * NTAG215::BLOCK_SIZE_BYTES as f64
            / NTAG215::PAGE_SIZE_BYTES as f64) as usize;

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

    pub fn select(&mut self) -> Result<Uid, Box<dyn std::error::Error>> {
        let atqa = self.mfrc522.reqa();
        let atqa = match atqa {
            Ok(atqa) => atqa,
            Err(_) => {
                self.mfrc522.hlta()?;
                self.mfrc522.wupa()?
            }
        };

        Ok(self.mfrc522.select(&atqa)?)
    }

    pub fn is_token_present(&mut self) -> Option<Uid> {
        match self.select() {
            Ok(res) => Some(res),
            Err(_) => None,
        }
    }

    pub fn read(&mut self) -> Result<Message, Box<dyn std::error::Error>> {
        self.select()?;
        self.read_blocks();

        let user_memory = &self.memory[NTAG215::USER_MEMORY_START..NTAG215::USER_MEMORY_END];
        let message = Message::parse(user_memory)?;
        Ok(message)
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
    }
}
