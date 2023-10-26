use bitflags::bitflags;
use std::str;

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
    data: &'a [u8],
}

impl<'a> ByteGetter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        ByteGetter {
            index: 0,
            data: data,
        }
    }

    pub fn get_byte(&mut self) -> u8 {
        let res = self.data[self.index];
        self.index += 1;
        res
    }

    pub fn get_bytes(&mut self, byte_count: usize) -> &'a [u8] {
        let res = &self.data[self.index..self.index + byte_count];
        self.index += byte_count;
        res
    }

    pub fn get_bytes_const<const N: usize>(&mut self) -> [u8; N] {
        let res: [u8; N] = self.data[self.index..self.index + N].try_into().unwrap();
        self.index += N;
        res
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

// this needs to be traitified so we can build a message with several different records
pub struct URIRecord {
    pub uri: String, // maybe parse directly into Url type?
}

pub struct Message {
    pub message_header: MessageHeader,
    pub records: Vec<URIRecord>,
}

impl Message {
    // TODO: This is a slightly less terrible ndef "parser" which is barely MVP ready!
    const MESSAGE_INIT_MARKER: u8 = 0x03;

    pub fn parse(data: &[u8]) -> Result<Message, Box<dyn std::error::Error>> {
        let mut bg = ByteGetter::new(data);

        // parse message header
        let message_init = bg.get_byte();
        if message_init != Self::MESSAGE_INIT_MARKER {
            return Err(Box::<dyn std::error::Error>::from(
                "NDEF Message init marker not found!",
            ));
        }

        let message_len = bg.get_byte();

        let message_header = MessageHeader {
            init: message_init,
            len: message_len,
        };

        // parse record header
        let flags_tnf = Flags::from_bits(bg.get_byte()).ok_or(
            Box::<dyn std::error::Error>::from("couldn't parse flags byte of ndef message"),
        )?;
        let type_length = bg.get_byte() as usize;

        let payload_length = match flags_tnf.contains(Flags::SHORT_RECORD) {
            true => u32::from(bg.get_byte()),
            false => u32::from_be_bytes(bg.get_bytes_const::<4>()),
        } as usize;

        let id_length = match flags_tnf.contains(Flags::ID_LENGTH_PRESENT) {
            true => Some(bg.get_byte()),
            false => None,
        };

        let payload_type = match type_length > 0 {
            true => {
                let payload_type = bg.get_bytes(type_length);
                Some(payload_type)
            }
            false => None,
        };

        let payload_id = match flags_tnf.contains(Flags::ID_LENGTH_PRESENT) && id_length > Some(0) {
            true => Some(bg.get_bytes(id_length.unwrap() as usize)),
            false => None,
        };

        let record_header = RecordHeader {
            flags_tnf,
            type_length,
            payload_length,
            id_length,
            payload_type,
            payload_id,
        };

        let payload = bg.get_bytes(payload_length);
        let record_raw = RecordRaw {
            header: record_header,
            payload,
        };

        // TODO: this is just parsing URI records right now!
        let prefix = PREFIX_STRINGS[usize::from(record_raw.payload[0])];
        let payload = str::from_utf8(&record_raw.payload[1..])?;
        let uri = [prefix, payload].join("");
        let uri_record = URIRecord { uri };

        // build records vector
        let records = Vec::from([uri_record]);

        Ok(Self {
            message_header,
            records,
        })
    }
}

#[cfg(test)]
mod tests {
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

        let message = crate::ndef::Message::parse(&NDEF_MESSAGE).unwrap();

        assert_eq!(message.records[0].uri, URI_DECODED);
    }
}
