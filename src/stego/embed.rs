use super::header::{
    HEADER_SIZE, MY_HEADER_MODULE, PayloadType, START_SPACE, StegHeader, ext_to_bytes,
};
use crate::error::HiddenWaveError;

pub fn max_capacity(pcm_len: usize) -> usize {
    let header_end_offset = START_SPACE + (MY_HEADER_MODULE * HEADER_SIZE);

    if pcm_len <= header_end_offset {
        return 0;
    }

    let available_space = pcm_len - header_end_offset;
    let sentinel_size = 4;

    let max_payload_plus_sentinel = available_space / 4;

    max_payload_plus_sentinel.saturating_sub(sentinel_size)
}

pub fn embed(
    pcm: &mut [u8],
    payload: &[u8],
    ext: &str,
    is_binary: bool,
) -> Result<(), HiddenWaveError> {
    let payload_with_sentinel: Vec<u8> = payload
        .iter()
        .copied()
        .chain(b"@<;;".iter().copied())
        .collect();

    let header_end_offset = START_SPACE + (MY_HEADER_MODULE * HEADER_SIZE);

    if pcm.len() <= header_end_offset {
        return Err(HiddenWaveError::PayloadTooLarge {
            needed: header_end_offset,
            available: pcm.len(),
        });
    }

    let available_payload_space = pcm.len() - header_end_offset;
    let modulus = (available_payload_space / payload_with_sentinel.len()) as u32;

    if modulus <= 3 {
        return Err(HiddenWaveError::PayloadTooLarge {
            needed: payload_with_sentinel.len() * 4,
            available: available_payload_space,
        });
    }

    let payload_type = if is_binary {
        PayloadType::Binary
    } else {
        PayloadType::Text
    };
    let header = StegHeader {
        modulus,
        extension: ext_to_bytes(ext),
        payload_type,
    };
    let header_bytes: [u8; 9] = header.into();

    pcm.iter_mut()
        .skip(START_SPACE)
        .step_by(MY_HEADER_MODULE)
        .zip(header_bytes.iter())
        .for_each(|(sample, &h)| *sample = h);

    pcm.iter_mut()
        .skip(header_end_offset)
        .step_by(modulus as usize)
        .zip(payload_with_sentinel.iter())
        .for_each(|(sample, &p)| *sample = p);

    Ok(())
}
