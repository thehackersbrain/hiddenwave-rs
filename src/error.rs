use thiserror::Error;

#[derive(Debug, Error)]
pub enum HiddenWaveError {
    #[error("Payload too large: need {needed} bytes of PCM, have {available}")]
    PayloadTooLarge { needed: usize, available: usize },

    #[error("No steg header found in audio")]
    NoHeaderFound,

    #[error("Sentinel not found — file may be corrupt or not a hiddenwave file")]
    SentinelMissing,

    #[error("WAV parse error: {0}")]
    WavParse(String),

    #[error("Cryptography error: {0}")]
    Crypto(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
