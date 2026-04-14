use super::header::{HEADER_SIZE, MY_HEADER_MODULE, START_SPACE, StegHeader};
use crate::error::HiddenWaveError;
use crate::payload::ExtractedPayload;

pub fn extract(pcm: &[u8]) -> Result<ExtractedPayload, HiddenWaveError> {
    let header_end = START_SPACE + (MY_HEADER_MODULE * HEADER_SIZE);

    if pcm.len() < header_end {
        return Err(HiddenWaveError::NoHeaderFound);
    }

    let header_bytes: Vec<u8> = pcm
        .iter()
        .skip(START_SPACE)
        .step_by(MY_HEADER_MODULE)
        .take(HEADER_SIZE)
        .copied()
        .collect();

    let header = StegHeader::try_from(header_bytes.as_slice())?;

    let raw_with_sentinel: Vec<u8> = pcm
        .iter()
        .skip(header_end)
        .step_by(header.modulus as usize)
        .copied()
        .collect();

    let sentinel_pos = raw_with_sentinel
        .windows(4)
        .position(|w| w == b"@<;;")
        .ok_or(HiddenWaveError::SentinelMissing)?;

    let payload_bytes = raw_with_sentinel[..sentinel_pos].to_vec();
    let ext = String::from_utf8_lossy(&header.extension)
        .trim()
        .to_string();

    Ok(ExtractedPayload {
        payload_type: header.payload_type,
        data: payload_bytes,
        ext,
    })
}
