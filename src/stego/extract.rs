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

    if header.modulus == 0 {
        return Err(HiddenWaveError::NoHeaderFound);
    }

    let raw_with_sentinel: Vec<u8> = pcm
        .iter()
        .skip(header_end)
        .step_by(header.modulus as usize)
        .copied()
        .collect();

    // The sentinel is appended at the very end of the payload, so the *last*
    // window matching it is always the real one. Using `.position()` (first
    // match) would stop early if the payload data itself happens to contain
    // the sentinel bytes, corrupting the extracted result silently.
    let sentinel_pos = raw_with_sentinel
        .windows(4)
        .enumerate()
        .filter(|(_, w)| *w == b"@<;;")
        .last()
        .map(|(i, _)| i)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stego::embed::embed;

    fn make_pcm(size: usize) -> Vec<u8> {
        vec![0u8; size]
    }

    #[test]
    fn test_embed_extract_text_round_trip() {
        let mut pcm = make_pcm(44100 * 2);
        let payload = b"hiddenwave rust rewrite";
        embed(&mut pcm, payload, "", false).unwrap();
        let result = extract(&pcm).unwrap();
        assert_eq!(result.data, payload);
        assert_eq!(result.payload_type, crate::stego::header::PayloadType::Text);
        assert_eq!(result.ext, "");
    }

    #[test]
    fn test_embed_extract_binary_round_trip() {
        let mut pcm = make_pcm(44100 * 4);
        let payload: Vec<u8> = (0u8..=255).cycle().take(512).collect();
        embed(&mut pcm, &payload, "bin", true).unwrap();
        let result = extract(&pcm).unwrap();
        assert_eq!(result.data, payload);
        assert_eq!(
            result.payload_type,
            crate::stego::header::PayloadType::Binary
        );
        assert_eq!(result.ext, "bin");
    }

    #[test]
    fn test_embed_extract_preserves_extension() {
        let mut pcm = make_pcm(88200);
        embed(&mut pcm, b"data", "pdf", true).unwrap();
        let result = extract(&pcm).unwrap();
        assert_eq!(result.ext, "pdf");
    }

    #[test]
    fn test_extract_zero_modulus_no_panic() {
        let pcm = make_pcm(10000);
        let result = extract(&pcm);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_too_short_errors() {
        let pcm = make_pcm(10);
        assert!(extract(&pcm).is_err());
    }
}
