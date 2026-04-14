use crate::stego::header::PayloadType;

pub struct ExtractedPayload {
    pub payload_type: PayloadType,
    pub data: Vec<u8>,
    pub ext: String,
}
