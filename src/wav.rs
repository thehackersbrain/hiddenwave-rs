use crate::error::HiddenWaveError;

pub struct WavFile {
    pub header_bytes: Vec<u8>,
    pub pcm_data: Vec<u8>,
}

impl WavFile {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, HiddenWaveError> {
        if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(HiddenWaveError::WavParse(
                "Not a valid RIFF/WAVE file".into(),
            ));
        }

        let mut offset = 12usize;
        let mut data_offset = None;

        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = u32::from_le_bytes(
                bytes[offset + 4..offset + 8]
                    .try_into()
                    .expect("slice is always 4 bytes"),
            ) as usize;

            if chunk_id == b"data" {
                data_offset = Some(offset + 8);
                break;
            }

            // Guard against malformed files with a chunk_size that would overflow
            // offset, which would wrap in release mode and cause an infinite loop
            // or mis-parse on the next iteration.
            let step = 8usize
                .checked_add(chunk_size)
                .and_then(|s| s.checked_add(chunk_size & 1))
                .ok_or_else(|| {
                    HiddenWaveError::WavParse("Chunk size overflow in WAV file".into())
                })?;
            offset = offset.checked_add(step).ok_or_else(|| {
                HiddenWaveError::WavParse("Chunk offset overflow in WAV file".into())
            })?;
        }

        let split_idx = data_offset
            .ok_or_else(|| HiddenWaveError::WavParse("Could not find 'data' chunk".into()))?;

        let (header, pcm) = bytes.split_at(split_idx);

        Ok(Self {
            header_bytes: header.to_vec(),
            pcm_data: pcm.to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.header_bytes.len() + self.pcm_data.len());
        out.extend_from_slice(&self.header_bytes);
        out.extend_from_slice(&self.pcm_data);
        out
    }

    pub fn generate_header(
        pcm_len: usize,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Vec<u8>, HiddenWaveError> {
        // WAV/RIFF sizes are stored as u32; files larger than ~4 GB cannot be
        // represented. Casting without checking silently truncates the size
        // fields, producing an unreadable file.
        let pcm_len_u32 = u32::try_from(pcm_len).map_err(|_| {
            HiddenWaveError::WavParse("PCM data exceeds maximum WAV file size (~4 GB)".into())
        })?;
        let riff_size = pcm_len_u32.checked_add(36).ok_or_else(|| {
            HiddenWaveError::WavParse("PCM data exceeds maximum WAV file size (~4 GB)".into())
        })?;

        let bits_per_sample: u16 = 16;
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * block_align as u32;

        let mut header = Vec::with_capacity(44);

        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&riff_size.to_le_bytes());
        header.extend_from_slice(b"WAVE");

        header.extend_from_slice(b"fmt ");
        header.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
        header.extend_from_slice(&1u16.to_le_bytes()); // PCM = 1
        header.extend_from_slice(&channels.to_le_bytes());
        header.extend_from_slice(&sample_rate.to_le_bytes());
        header.extend_from_slice(&byte_rate.to_le_bytes());
        header.extend_from_slice(&block_align.to_le_bytes());
        header.extend_from_slice(&bits_per_sample.to_le_bytes());

        header.extend_from_slice(b"data");
        header.extend_from_slice(&pcm_len_u32.to_le_bytes());

        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_wav(pcm: &[u8]) -> Vec<u8> {
        let mut out = WavFile::generate_header(pcm.len(), 44100, 1).unwrap();
        out.extend_from_slice(pcm);
        out
    }

    #[test]
    fn test_parse_round_trip() {
        let pcm = vec![0x01u8; 1000];
        let wav_bytes = make_minimal_wav(&pcm);
        let parsed = WavFile::parse(wav_bytes).unwrap();
        assert_eq!(parsed.pcm_data, pcm);
    }

    #[test]
    fn test_parse_finds_data_chunk_not_raw_scan() {
        let pcm = vec![0xBBu8; 100];
        let mut bytes = Vec::new();

        // RIFF header
        let fmt_chunk_size = 16u32;
        let fake_chunk_size = 8u32;
        let total_data_size = pcm.len() as u32;
        let riff_size = 4 + 8 + fmt_chunk_size + 8 + fake_chunk_size + 8 + total_data_size;

        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");

        // fmt chunk
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&fmt_chunk_size.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
        bytes.extend_from_slice(&44100u32.to_le_bytes());
        bytes.extend_from_slice(&88200u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());

        bytes.extend_from_slice(b"junk");
        bytes.extend_from_slice(&fake_chunk_size.to_le_bytes());
        bytes.extend_from_slice(b"datadata");

        // Real data chunk
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&total_data_size.to_le_bytes());
        bytes.extend_from_slice(&pcm);

        let parsed = WavFile::parse(bytes).unwrap();
        assert_eq!(parsed.pcm_data, pcm);
    }

    #[test]
    fn test_parse_rejects_non_wav() {
        let junk = vec![0xFFu8; 100];
        assert!(WavFile::parse(junk).is_err());
    }

    #[test]
    fn test_parse_rejects_missing_data_chunk() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&36u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
        // no data chunk
        assert!(WavFile::parse(bytes).is_err());
    }

    #[test]
    fn test_generate_header_is_44_bytes() {
        assert_eq!(WavFile::generate_header(1000, 44100, 2).unwrap().len(), 44);
    }

    #[test]
    fn test_to_bytes_reconstructs_correctly() {
        let pcm = vec![0x42u8; 200];
        let wav_bytes = make_minimal_wav(&pcm);
        let parsed = WavFile::parse(wav_bytes.clone()).unwrap();
        assert_eq!(parsed.to_bytes(), wav_bytes);
    }
}
