use crate::error::HiddenWaveError;

pub const HEADER_SIZE: usize = 9;
pub const MY_HEADER_MODULE: usize = 64;
pub const START_SPACE: usize = 0;

#[derive(Debug, Clone)]
pub struct StegHeader {
    pub modulus: u32,
    pub extension: [u8; 4],
    pub payload_type: PayloadType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PayloadType {
    Text = b't',
    Binary = b'b',
}

impl TryFrom<&[u8]> for StegHeader {
    type Error = HiddenWaveError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < HEADER_SIZE {
            return Err(HiddenWaveError::NoHeaderFound);
        }

        let mut mod_bytes = [0u8; 4];
        mod_bytes.copy_from_slice(&bytes[0..4]);
        let modulus = u32::from_le_bytes(mod_bytes);

        let mut extension = [0u8; 4];
        extension.copy_from_slice(&bytes[4..8]);

        let payload_type = match bytes[8] {
            b't' => PayloadType::Text,
            b'b' => PayloadType::Binary,
            _ => return Err(HiddenWaveError::NoHeaderFound),
        };

        Ok(Self {
            modulus,
            extension,
            payload_type,
        })
    }
}

impl From<StegHeader> for [u8; 9] {
    fn from(h: StegHeader) -> [u8; 9] {
        let mut out = [0u8; 9];
        out[0..4].copy_from_slice(&h.modulus.to_le_bytes());
        out[4..8].copy_from_slice(&h.extension);
        out[8] = h.payload_type as u8;
        out
    }
}

pub fn ext_to_bytes(ext: &str) -> [u8; 4] {
    let mut out = [b' '; 4];
    for (i, b) in ext.bytes().take(4).enumerate() {
        out[i] = b;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_round_trip_text() {
        let h = StegHeader {
            modulus: 42,
            extension: *b"txt ",
            payload_type: PayloadType::Text,
        };
        let bytes: [u8; 9] = h.into();
        let h2 = StegHeader::try_from(bytes.as_slice()).unwrap();
        assert_eq!(h2.modulus, 42);
        assert_eq!(h2.extension, *b"txt ");
        assert_eq!(h2.payload_type, PayloadType::Text);
    }

    #[test]
    fn test_header_round_trip_binary() {
        let h = StegHeader {
            modulus: 999999,
            extension: *b"pdf ",
            payload_type: PayloadType::Binary,
        };
        let bytes: [u8; 9] = h.into();
        let h2 = StegHeader::try_from(bytes.as_slice()).unwrap();
        assert_eq!(h2.modulus, 999999);
        assert_eq!(h2.payload_type, PayloadType::Binary);
    }

    #[test]
    fn test_header_le_byte_order() {
        let h = StegHeader {
            modulus: 256,
            extension: *b"    ",
            payload_type: PayloadType::Text,
        };
        let bytes: [u8; 9] = h.into();
        assert_eq!(&bytes[0..4], &[0x00, 0x01, 0x00, 0x00]);
    }

    #[test]
    fn test_invalid_type_byte_errors() {
        let bytes = [0u8, 0, 0, 0, b' ', b' ', b' ', b' ', b'x'];
        assert!(StegHeader::try_from(bytes.as_slice()).is_err());
    }

    #[test]
    fn test_too_short_errors() {
        let bytes = [0u8; 5];
        assert!(StegHeader::try_from(bytes.as_slice()).is_err());
    }

    #[test]
    fn test_ext_to_bytes_short() {
        assert_eq!(ext_to_bytes("rs"), *b"rs  ");
    }

    #[test]
    fn test_ext_to_bytes_exact() {
        assert_eq!(ext_to_bytes("jpeg"), *b"jpeg");
    }

    #[test]
    fn test_ext_to_bytes_truncates() {
        assert_eq!(ext_to_bytes("abcdefgh"), *b"abcd");
    }
}
