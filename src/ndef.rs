use bitflags::bitflags;
use itertools::Itertools;
use std::str;

enum State {
    None,
    MessageInit,
    RecordHeader,
    RecordData,
}

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

pub enum WellKnownType {
    URI,
}

pub struct NDEF {
    // pub records: Vec<WellKnownType>,
    pub uri: String,
    // pub uri_2: str,
}

impl NDEF {
    const MESSAGE_INIT_MARKER: u8 = 0x03;

    pub fn parse(buffer: &[u8]) -> Self {
        let mut state = State::None;
        let mut message_len: u8 = 0;
        let mut payload_length: u32 = 0;
        let mut i: usize = 0;

        // let mut records = Vec::new();

        let mut uri: String = String::new();

        while i < buffer.len() {
            match state {
                State::None => {
                    let byte = buffer[i];
                    if byte == NDEF::MESSAGE_INIT_MARKER {
                        state = State::MessageInit;
                    }
                    // else error?
                    i += 1;
                }
                State::MessageInit => {
                    message_len = buffer[i];
                    state = State::RecordHeader;
                    i += 1;
                }
                State::RecordHeader => {
                    let flags_tnf =
                        Flags::from_bits(buffer[i]).expect("Couldn't parse Flags and TNF byte :(");
                    i += 1;

                    let type_length = buffer[i] as usize;
                    println!("type_length: {type_length}");
                    i += 1;

                    if flags_tnf.contains(Flags::SHORT_RECORD) {
                        payload_length = u32::from(buffer[i]);
                        i += 1;
                    } else {
                        payload_length =
                            u32::from_be_bytes(buffer[i..i + 4].try_into().expect("oof"));
                        i += 4;
                    }
                    println!("payload_length: {payload_length}");

                    let mut id_length: usize = 0;
                    if flags_tnf.contains(Flags::ID_LENGTH_PRESENT) {
                        id_length = usize::from(buffer[i]);
                        i += 1;
                    }
                    println!("id_length: {id_length}");

                    // TODO: some of these need some length checks
                    let mut payload_type = 0;
                    if type_length > 0 {
                        // payload_type =
                        //     u32::from_be_bytes(buffer[i..i + type_length].try_into().expect("oof"));
                        i += type_length;
                    }

                    let mut payload_id = 0;
                    if flags_tnf.contains(Flags::ID_LENGTH_PRESENT) && id_length > 0 {
                        payload_id =
                            u32::from_be_bytes(buffer[i..i + id_length].try_into().expect("oof"));
                        i += id_length;
                    }

                    // D1 01 4B 55
                    state = State::RecordData;
                }
                State::RecordData => {
                    let data = buffer.get(i..(i + payload_length as usize)).expect("oh no");

                    for block in data.chunks(8) {
                        println!("{:02x}", block.iter().format(""));
                    }
                    uri = String::from_utf8(Vec::from(data)).expect("crap");
                    println!("uri: {uri}");
                    break;
                }
            }
        }

        Self { uri }
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
            "\u{4}open.spotify.com/playlist/62Q9JugytREDtl4i4fcHfX?si=PW2kLwTGQ66_NUEFJD6WYg";

        let ndef = crate::ndef::NDEF::parse(&NDEF_MESSAGE);

        assert_eq!(ndef.uri, URI_DECODED);
    }
}
