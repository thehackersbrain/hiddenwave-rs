pub mod error;
pub mod payload;
pub mod stego;
pub mod wav;

pub mod crypto;
pub mod mp3;

// Convenience re-exports for library consumers
pub use error::HiddenWaveError;
pub use payload::ExtractedPayload;
pub use stego::embed::embed;
pub use stego::extract::extract;
pub use wav::WavFile;
