# HiddenWave 🌊

[![Crates.io](https://img.shields.io/crates/v/hiddenwave.svg)](https://crates.io/crates/hiddenwave)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Portfolio](https://img.shields.io/badge/Portfolio-Live-black?style=for-the-badge)](https://thehackersbrain.dev)

**Secure Audio Steganography & Cryptography in Rust**

HiddenWave is a fast, memory-safe CLI tool and library that hides files and text messages inside audio files (WAV/MP3) using a striding byte-injection algorithm. It combines **steganography** (hiding the existence of data) with **AES-256-GCM cryptography** (protecting the contents).

## Features

- **WAV & MP3 Support:** Natively parse WAV files or decode MP3s on the fly via `symphonia`.
- **Encryption:** Payloads are encrypted and authenticated using AES-256-GCM with a PBKDF2-derived key.
- **Striding Algorithm:** Distributes the payload evenly across the audio file to minimize distortion.
- **Format Preservation:** Automatically outputs safe lossless `.wav` formats to prevent lossy compression from destroying payloads.
- **Library API:** Exposes `hiddenwave_lib` for easy integration into other Rust projects.

## Installation

### As a CLI Tool (Cargo Install)

```bash
cargo install hiddenwave
```

### Build from Source

```bash
git clone https://github.com/thehackersbrain/hiddenwave-rs.git
cd hiddenwave-rs
cargo build --release
```

The compiled binary will be at `./target/release/hiddenwave-rs`.

## CLI Usage

### 1. Check Capacity (`check` or `c`)

Analyze an audio file to see exactly how much data it can hold.

```bash
hiddenwave c -i cover_audio.mp3
```

### 2. Hide Data (`hide` or `h`)

Hide a message or file. If no output `-o` is specified, it defaults to `output.wav`.

```bash
# Hide a text message with a password
hiddenwave h -i song.wav -m "Meeting at midnight" -p "SuperSecret123"

# Hide an entire file (e.g., PDF, ZIP) inside an MP3
hiddenwave h -i podcast.mp3 -f secret_document.pdf -o secure_audio.wav -p "SuperSecret123"
```

### 3. Extract Data (`extract` or `e`)

Extract and decrypt your hidden payloads.

```bash
# Extract a hidden text message
hiddenwave e -i secure_audio.wav -p "SuperSecret123"

# Extract a hidden file
hiddenwave e -i secure_audio.wav -o recovered.pdf -p "SuperSecret123"
```

## Using as a Library

Add `hiddenwave` to your `Cargo.toml`:

```toml
[dependencies]
hiddenwave = "x.x.x"
```

**Example: Embedding and Extracting Data**

```rust
use hiddenwave_lib::stego::{embed::embed, extract::extract};

// Note: To use the library, you provide raw PCM audio bytes and the payload.
```

## How It Works

1. **Encryption:** Your payload is encrypted using AES-256-GCM. The key is derived via PBKDF2 (100,000 iterations).
2. **Analysis:** The tool calculates the available raw PCM audio samples and determines a "stride" (interval).
3. **Injection:** The encrypted bytes are injected at the calculated intervals, replacing bits to make the change imperceptible to the human ear.
4. **Sentinels:** A magic byte sequence (`@<;;`) marks the end of the payload for safe extraction.

> **MP3 Disclaimer:** MP3 is a _lossy_ format. If you compress an audio file containing steganography into an MP3, the compression algorithm will destroy the hidden bytes. HiddenWave accepts MP3s as _input_, but must _output_ a lossless `.wav` file to preserve your data.
