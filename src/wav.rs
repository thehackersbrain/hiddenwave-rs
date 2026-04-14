use crate::error::HiddenWaveError;

pub struct WavFile {
    pub header_bytes: Vec<u8>,
    pub pcm_data: Vec<u8>,
}

impl WavFile {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, HiddenWaveError> {
        let data_marker = b"data";
        let mut data_offset = None;

        for i in 0..bytes.len().saturating_sub(8) {
            if &bytes[i..i + 4] == data_marker {
                data_offset = Some(i + 8);
                break;
            }
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

    pub fn generate_header(pcm_len: usize, sample_rate: u32, channels: u16) -> Vec<u8> {
        let mut header = Vec::with_capacity(44);
        let byte_rate = sample_rate * (channels as u32) * 2;
        let block_align = channels * 2;

        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&((36 + pcm_len) as u32).to_le_bytes());
        header.extend_from_slice(b"WAVE");

        header.extend_from_slice(b"fmt ");
        header.extend_from_slice(&16u32.to_le_bytes());
        header.extend_from_slice(&1u16.to_le_bytes());
        header.extend_from_slice(&channels.to_le_bytes());
        header.extend_from_slice(&sample_rate.to_le_bytes());
        header.extend_from_slice(&byte_rate.to_le_bytes());
        header.extend_from_slice(&block_align.to_le_bytes());
        header.extend_from_slice(&16u16.to_le_bytes());

        header.extend_from_slice(b"data");
        header.extend_from_slice(&(pcm_len as u32).to_le_bytes());

        header
    }
}
