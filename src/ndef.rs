use bitflags::bitflags;
use std::str;
use tracing::{debug, error};

bitflags! {
    pub struct Flags: u8 {
        const MESSAGE_BEGIN = 0b10000000;
        const MESSAGE_END = 0b01000000;
        const CHUNK = 0b00100000;
        const SHORT_RECORD = 0b00010000;
        const ID_LENGTH_PRESENT = 0b00001000;
        const TNF_EMPTY = 0b00000000;
        const TNF_NFC_WELL_KNOWN = 0b00000001;
        const TNF_MEDIA = 0b00000010;
        const TNF_ABSOLUTE_URI = 0b00000011;
        const TNF_NFC_EXTERNAL = 0b00000100;
        const TNF_UNKNOWN = 0b00000101;
        const TNF_UNCHANGED = 0b00000110;
        const TNF_RESERVED = 0b00000111;
    }
}

const PREFIX_STRINGS: &[&str] = &[
    "",
    "http://www.",
    "https://www.",
    "http://",
    "https://",
    "tel:",
    "mailto:",
    "ftp://anonymous:anonymous@",
    "ftp://ftp.",
    "ftps://",
    "sftp://",
    "smb://",
    "nfs://",
    "ftp://",
    "dav://",
    "news:",
    "telnet://",
    "imap:",
    "rtsp://",
    "urn:",
    "pop:",
    "sip:",
    "sips:",
    "tftp:",
    "btspp://",
    "btl2cap://",
    "btgoep://",
    "tcpobex://",
    "irdaobex://",
    "file://",
    "urn:epc:id:",
    "urn:epc:tag:",
    "urn:epc:pat:",
    "urn:epc:raw:",
    "urn:epc:",
    "urn:nfc:",
];

pub enum WellKnownType {
    URI,
}

struct ByteGetter<'a> {
    index: usize,
    len: Option<usize>,
    data: &'a [u8],
}

impl<'a> ByteGetter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        ByteGetter {
            index: 0,
            len: None,
            data,
        }
    }

    fn check_index(&self) -> Option<()> {
        match self.len.unwrap_or(usize::MAX) > self.index {
            true => Some(()),
            false => {
                error!("Trying to overread buffer!");
                None
            }
        }
    }

    pub fn get_byte(&mut self) -> Option<u8> {
        self.check_index()?;

        let res = self.data[self.index];
        debug!("got byte {:02x?}", res);
        self.index += 1;
        Some(res)
    }

    pub fn get_bytes(&mut self, byte_count: usize) -> Option<&'a [u8]> {
        self.check_index()?;

        let res = &self.data[self.index..self.index + byte_count];
        debug!("got bytes {:02x?}", res);
        self.index += byte_count;
        Some(res)
    }

    pub fn get_bytes_const<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.check_index()?;
        let res: [u8; N] = self.data[self.index..self.index + N].try_into().unwrap();
        self.index += N;
        Some(res)
    }

    pub fn set_len(&mut self, len: usize) {
        self.len = Some(len);
    }
}

pub struct MessageHeader {
    pub init: u8,
    pub len: u8,
}

pub struct RecordHeader<'a> {
    pub flags_tnf: Flags,
    pub type_length: usize,
    pub payload_length: usize,
    pub id_length: Option<u8>,
    pub payload_type: Option<&'a [u8]>,
    pub payload_id: Option<&'a [u8]>,
}

pub struct RecordRaw<'a> {
    pub header: RecordHeader<'a>,
    pub payload: &'a [u8],
}

pub enum Record {
    URI { uri: String },
}

pub struct Message {
    pub message_header: MessageHeader,
    pub records: Vec<Record>,
}

impl Message {
    // TODO: This is a slightly less terrible ndef "parser" which is barely MVP ready!
    const MESSAGE_INIT_MARKER: u8 = 0x03;

    fn parse_message_header(bg: &mut ByteGetter) -> Option<MessageHeader> {
        let message_init = bg.get_byte()?;
        debug!(message_init);
        if message_init != Self::MESSAGE_INIT_MARKER {
            error!("NDEF Message init marker not found!");
            return None;
        }

        let message_len = bg.get_byte()?;
        debug!(message_len);
        bg.set_len(message_len as usize);

        let message_header = MessageHeader {
            init: message_init,
            len: message_len,
        };

        Some(message_header)
    }

    fn parse_record_raw<'a>(bg: &'a mut ByteGetter<'a>) -> Option<RecordRaw<'a>> {
        let flags_tnf = Flags::from_bits(bg.get_byte()?)?;
        let type_length = bg.get_byte()? as usize;

        let payload_length = match flags_tnf.contains(Flags::SHORT_RECORD) {
            true => u32::from(bg.get_byte()?),
            false => u32::from_be_bytes(bg.get_bytes_const::<4>()?),
        } as usize;

        let id_length = match flags_tnf.contains(Flags::ID_LENGTH_PRESENT) {
            true => Some(bg.get_byte()?),
            false => None,
        };

        let payload_type = match type_length > 0 {
            true => {
                let payload_type = bg.get_bytes(type_length)?;
                Some(payload_type)
            }
            false => None,
        };

        let payload_id = match flags_tnf.contains(Flags::ID_LENGTH_PRESENT) && id_length > Some(0) {
            true => Some(bg.get_bytes(id_length.unwrap() as usize)?),
            false => None,
        };

        let header = RecordHeader {
            flags_tnf,
            type_length,
            payload_length,
            id_length,
            payload_type,
            payload_id,
        };

        let payload = bg.get_bytes(header.payload_length)?;

        let record_raw = RecordRaw { header, payload };
        Some(record_raw)
    }

    fn parse_uri_record(record_raw: RecordRaw) -> Option<Record> {
        let prefix = PREFIX_STRINGS[usize::from(record_raw.payload[0])];
        let payload = str::from_utf8(&record_raw.payload[1..]).ok()?;
        let uri = [prefix, payload].join("");
        Some(Record::URI { uri })
    }

    fn parse_records<'a>(bg: &'a mut ByteGetter<'a>) -> Option<Vec<Record>> {
        let mut records = Vec::<Record>::new();

        // loop {
        let record_raw = Message::parse_record_raw(bg)?;
        // let last_record = record_raw.header.flags_tnf.contains(Flags::MESSAGE_END);

        // TODO: this is just parsing URI records right now!
        let uri_record = Message::parse_uri_record(record_raw)?;
        records.push(uri_record);

        //     if last_record {
        //         break;
        //     }
        // }

        Some(records)
    }

    pub fn parse(data: &[u8]) -> Option<Message> {
        let mut bg = ByteGetter::new(data);

        let message_header = Message::parse_message_header(&mut bg)?;
        let records = Message::parse_records(&mut bg)?;

        Some(Self {
            message_header,
            records,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ndef::*;

    #[test]
    fn parse_uri() {
        const NDEF_MESSAGE: [u8; 82] = [
            // header
            0x03, 0x4f, 0xd1, 0x01, 0x4b, 0x55, //
            // payload
            0x04, 0x6f, 0x70, 0x65, 0x6e, 0x2e, 0x73, 0x70, //
            0x6f, 0x74, 0x69, 0x66, 0x79, 0x2e, 0x63, 0x6f, //
            0x6d, 0x2f, 0x70, 0x6c, 0x61, 0x79, 0x6c, 0x69, //
            0x73, 0x74, 0x2f, 0x36, 0x32, 0x51, 0x39, 0x4a, //
            0x75, 0x67, 0x79, 0x74, 0x52, 0x45, 0x44, 0x74, //
            0x6c, 0x34, 0x69, 0x34, 0x66, 0x63, 0x48, 0x66, //
            0x58, 0x3f, 0x73, 0x69, 0x3d, 0x50, 0x57, 0x32, //
            0x6b, 0x4c, 0x77, 0x54, 0x47, 0x51, 0x36, 0x36, //
            0x5f, 0x4e, 0x55, 0x45, 0x46, 0x4a, 0x44, 0x36, //
            0x57, 0x59, 0x67, 0xfe,
        ];
        const URI_DECODED: &str =
            "https://open.spotify.com/playlist/62Q9JugytREDtl4i4fcHfX?si=PW2kLwTGQ66_NUEFJD6WYg";

        let message = Message::parse(&NDEF_MESSAGE).unwrap();

        match &message.records[0] {
            Record::URI { uri } => {
                assert_eq!(uri, URI_DECODED);
            }
        }
    }
}
