use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};

use hiddenwave_lib::{
    error::HiddenWaveError,
    stego::{embed::embed, extract::extract},
    wav::WavFile,
};

#[derive(Parser)]
#[command(
    name = "hiddenwave",
    about = "Hide encrypted data in WAV/MP3 audio via striding",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(name = "hide", visible_alias = "h")]
    Hide {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value = "output.wav")]
        output: PathBuf,
        #[arg(short, long, conflicts_with = "file")]
        message: Option<String>,
        #[arg(short, long, conflicts_with = "message")]
        file: Option<PathBuf>,
        #[arg(short, long)]
        password: Option<String>,
    },
    #[command(name = "extract", visible_alias = "e")]
    Extract {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short, long)]
        password: Option<String>,
    },
    #[command(name = "capacity", visible_alias = "c")]
    Capacity {
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn main() -> Result<(), HiddenWaveError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Hide {
            input,
            output,
            message,
            file,
            password,
        } => {
            let input_ext = get_ext(&input).unwrap_or_default().to_lowercase();

            let mut wav = match input_ext.as_str() {
                "mp3" => {
                    println!("[*] MP3 detected. Decoding to raw PCM...");
                    let (pcm_data, sample_rate, channels) =
                        hiddenwave_lib::mp3::decode_to_pcm(&input)?;
                    let header_bytes =
                        WavFile::generate_header(pcm_data.len(), sample_rate, channels);
                    WavFile {
                        header_bytes,
                        pcm_data,
                    }
                }
                "wav" => {
                    let wav_bytes = fs::read(&input)?;
                    WavFile::parse(wav_bytes)?
                }
                _ => {
                    return Err(HiddenWaveError::WavParse(format!(
                        "Unsupported input file extension: '.{}'. Only .wav and .mp3 are supported.",
                        input_ext
                    )));
                }
            };

            let mut final_output = output;
            if input_ext == "mp3" && final_output.extension().unwrap_or_default() != "wav" {
                println!(
                    "[!] Warning: Outputting as .wav to prevent payload destruction via lossy compression."
                );
                final_output.set_extension("wav");
            }

            let mut raw_payload = if let Some(msg) = message {
                msg.into_bytes()
            } else if let Some(f_path) = file.clone() {
                fs::read(&f_path)?
            } else {
                return Err(HiddenWaveError::WavParse(
                    "Provide a message (-m) or file (-f)".into(),
                ));
            };

            if let Some(pwd) = password {
                println!("[*] Encrypting payload with AES-256-GCM...");
                raw_payload = hiddenwave_lib::crypto::encrypt_payload(&raw_payload, &pwd)?;
            }

            let ext = if let Some(path) = &file {
                get_ext(path).unwrap_or_default()
            } else {
                "".to_string()
            };
            let is_binary = file.is_some();

            embed(&mut wav.pcm_data, &raw_payload, &ext, is_binary)?;

            fs::write(&final_output, wav.to_bytes())?;
            println!("[+] Data Hidden Successfully. Saved to {:?}", final_output);
        }

        Commands::Extract {
            input,
            output,
            password,
        } => {
            let input_ext = get_ext(&input).unwrap_or_default().to_lowercase();
            if input_ext != "wav" {
                return Err(HiddenWaveError::WavParse(format!(
                    "Extraction failed. Input must be a lossless .wav file, not '.{}'. (Lossy formats destroy hidden payloads).",
                    input_ext
                )));
            }

            let wav_bytes = fs::read(&input)?;
            let wav = WavFile::parse(wav_bytes)?;

            let mut extracted = extract(&wav.pcm_data)?;

            // Decryption is now a default feature
            if let Some(pwd) = password {
                println!("[*] Decrypting payload...");
                extracted.data = hiddenwave_lib::crypto::decrypt_payload(&extracted.data, &pwd)?;
            }

            match extracted.payload_type {
                hiddenwave_lib::stego::header::PayloadType::Text => {
                    let text = String::from_utf8(extracted.data).unwrap_or_else(|_| {
                        "Error: Decrypted data is not valid UTF-8 text. Wrong password?".to_string()
                    });
                    println!("[+] Extracted Message:\n{}", text);
                }
                hiddenwave_lib::stego::header::PayloadType::Binary => {
                    let mut out_path = output.unwrap_or_else(|| PathBuf::from("output"));
                    if !extracted.ext.is_empty() && out_path.extension().is_none() {
                        out_path.set_extension(extracted.ext);
                    }

                    fs::write(&out_path, extracted.data)?;
                    println!("[+] File Extracted Successfully. Saved to {:?}", out_path);
                }
            }
        }

        Commands::Capacity { input } => {
            let input_ext = get_ext(&input).unwrap_or_default().to_lowercase();

            let pcm_len = match input_ext.as_str() {
                "mp3" => {
                    println!("[*] MP3 detected. Decoding to calculate true PCM capacity...");
                    let (pcm_data, _, _) = hiddenwave_lib::mp3::decode_to_pcm(&input)?;
                    pcm_data.len()
                }
                "wav" => {
                    let wav_bytes = fs::read(&input)?;
                    let wav = WavFile::parse(wav_bytes)?;
                    wav.pcm_data.len()
                }
                _ => {
                    return Err(HiddenWaveError::WavParse(format!(
                        "Unsupported input file extension: '.{}'. Only .wav and .mp3 are supported.",
                        input_ext
                    )));
                }
            };

            let max_bytes = hiddenwave_lib::stego::embed::max_capacity(pcm_len);
            let max_kb = max_bytes as f64 / 1024.0;
            let max_mb = max_kb / 1024.0;

            println!(
                "\n[\033[0;92m*\033[0;0m] Capacity Analysis for: {:?}",
                input
            );
            println!("    Raw PCM Size : {} bytes", pcm_len);
            println!(
                "    Max Payload  : {} bytes ({:.2} KB / {:.2} MB)\n",
                max_bytes, max_kb, max_mb
            );
        }
    }

    Ok(())
}

fn get_ext(path: &Path) -> Option<String> {
    path.extension().map(|e| e.to_string_lossy().to_string())
}
