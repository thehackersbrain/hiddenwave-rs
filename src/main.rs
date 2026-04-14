use clap::{Parser, Subcommand};
use colored::Colorize;
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
    about = "Hide encrypted data in WAV/MP3 audio file",
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
    #[command(name = "check", visible_alias = "c")]
    Check {
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn banner() {
    let logo = r#"
  ___ ___ .__    .___  .___            __      __                     
 /   |   \|__| __| _/__| _/____   ____/  \    /  \________  __ ____  
/    ~    \  |/ __ |/ __ |/ __ \ /    \   \/\/   /\__  \  \/ // __ \ 
\    Y    /  / /_/ / /_/ \  ___/|   |  \        /  / __ \   /\  ___/ 
 \___|_  /|__\____ \____ |\___  >___|  /\__/\  /  (____ /\_/  \___ >
       \/         \/    \/    \/     \/      \/        \/         \/ "#;

    println!("{}", logo.bright_green().bold());
    println!(
        "           {}\n\n                     [by {}]                [v{}]\n",
        "Hide Your Secret Files in Audio Files.".bright_green(),
        "@thehackersbrain".bright_green().bold(),
        env!("CARGO_PKG_VERSION").bright_green().bold()
    );
}

fn main() -> Result<(), HiddenWaveError> {
    banner();
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
                    println!(
                        "[{}] {}",
                        "*".cyan().bold(),
                        "MP3 detected. Decoding to raw PCM...".cyan()
                    );
                    let (pcm_data, sample_rate, channels) =
                        hiddenwave_lib::mp3::decode_to_pcm(&input)?;
                    let header_bytes =
                        WavFile::generate_header(pcm_data.len(), sample_rate, channels)?;
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
                    "[{}] {}",
                    "!".yellow().bold(),
                    "Warning: Outputting as .wav to prevent payload destruction via lossy compression.".yellow()
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
                println!(
                    "[{}] {}",
                    "*".cyan().bold(),
                    "Encrypting payload with AES-256-GCM...".cyan()
                );
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
            println!(
                "[{}] {} {:?}",
                "+".green().bold(),
                "Data Hidden Successfully. Saved to".green(),
                final_output
            );
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

            if let Some(pwd) = password {
                println!("[{}] {}", "*".cyan().bold(), "Decrypting payload...".cyan());
                extracted.data = hiddenwave_lib::crypto::decrypt_payload(&extracted.data, &pwd)?;
            }

            match extracted.payload_type {
                hiddenwave_lib::stego::header::PayloadType::Text => {
                    let text = String::from_utf8(extracted.data).unwrap_or_else(|_| {
                        "Error: Decrypted data is not valid UTF-8 text. Wrong password?".to_string()
                    });
                    println!(
                        "[{}] {}\n{}",
                        "+".green().bold(),
                        "Extracted Message:".green(),
                        text.white()
                    );
                }
                hiddenwave_lib::stego::header::PayloadType::Binary => {
                    let mut out_path = output.unwrap_or_else(|| PathBuf::from("output"));
                    if !extracted.ext.is_empty() && out_path.extension().is_none() {
                        out_path.set_extension(extracted.ext);
                    }

                    fs::write(&out_path, extracted.data)?;
                    println!(
                        "[{}] {} {:?}",
                        "+".green().bold(),
                        "File Extracted Successfully. Saved to".green(),
                        out_path
                    );
                }
            }
        }

        Commands::Check { input } => {
            let input_ext = get_ext(&input).unwrap_or_default().to_lowercase();

            let pcm_len = match input_ext.as_str() {
                "mp3" => {
                    println!(
                        "[{}] {}",
                        "*".cyan().bold(),
                        "MP3 detected. Decoding to calculate true PCM capacity...".cyan()
                    );
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
                "\n[{}] {} {:?}",
                "*".green().bold(),
                "Capacity Analysis for:".green(),
                input
            );
            println!("    {} : {} bytes", "Raw PCM Size".cyan(), pcm_len);
            println!(
                "    {}  : {} bytes ({:.2} KB / {:.2} MB)\n",
                "Max Payload ".green().bold(),
                max_bytes,
                max_kb,
                max_mb
            );
        }
    }

    Ok(())
}

fn get_ext(path: &Path) -> Option<String> {
    path.extension().map(|e| e.to_string_lossy().to_string())
}
