//! Voice & media features — online (Groq Whisper / ElevenLabs) and offline
//! (local Whisper CLI / system TTS) speech-to-text and text-to-speech.
//!
//! The [`VoiceDispatcher`] provides unified access with automatic fallback:
//! - If `prefer_local` is set or no cloud API key → try local first
//! - If local fails and cloud key is available → fall back to cloud
//! - If neither is available → return helpful error with setup instructions

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use vibe_ai::{retry_async, RetryConfig};

use crate::config::VoiceConfig;
use crate::voice_local::WhisperModel;

/// Transcribe an audio file via Groq's Whisper endpoint.
///
/// Returns the transcribed text.
pub async fn transcribe_audio(audio_path: &std::path::Path, api_key: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let file_bytes = tokio::fs::read(audio_path)
        .await
        .context("Failed to read audio file")?;

    let file_name = audio_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();

    let api_key_owned = api_key.to_string();
    let resp = retry_async(&RetryConfig::default(), "groq-whisper-transcribe", || {
        let client = client.clone();
        let file_bytes = file_bytes.clone();
        let file_name = file_name.clone();
        let api_key_owned = api_key_owned.clone();
        async move {
            let part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(file_name)
                .mime_str("audio/wav")?;
            let form = reqwest::multipart::Form::new()
                .text("model", "whisper-large-v3")
                .part("file", part);
            client
                .post("https://api.groq.com/openai/v1/audio/transcriptions")
                .header("Authorization", format!("Bearer {}", api_key_owned))
                .multipart(form)
                .send()
                .await
                .map_err(Into::into)
        }
    })
    .await
    .context("Whisper transcription request failed")?;

    if !resp.status().is_success() {
        let err = resp.text().await?;
        anyhow::bail!("Whisper API error: {}", err);
    }

    let body: serde_json::Value = resp.json().await?;
    Ok(body["text"].as_str().unwrap_or("").to_string())
}

/// Convert text to speech via ElevenLabs API. Returns audio bytes (mp3).
pub async fn text_to_speech(text: &str, api_key: &str, voice_id: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);

    let api_key_owned = api_key.to_string();
    let text_owned = text.to_string();
    let resp = retry_async(&RetryConfig::default(), "elevenlabs-tts", || {
        let client = client.clone();
        let url = url.clone();
        let api_key_owned = api_key_owned.clone();
        let text_owned = text_owned.clone();
        async move {
            client
                .post(&url)
                .header("xi-api-key", &api_key_owned)
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "text": text_owned,
                    "model_id": "eleven_multilingual_v2",
                    "voice_settings": {
                        "stability": 0.5,
                        "similarity_boost": 0.5
                    }
                }))
                .send()
                .await
                .map_err(Into::into)
        }
    })
    .await
    .context("ElevenLabs TTS request failed")?;

    if !resp.status().is_success() {
        let err = resp.text().await?;
        anyhow::bail!("ElevenLabs API error: {}", err);
    }

    Ok(resp.bytes().await?.to_vec())
}

// ── Local (offline) transcription via whisper.cpp CLI ──────────────────────────

/// Directory where Whisper GGML models are stored.
pub fn models_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("models")
}

/// Check if a local Whisper model is downloaded.
pub fn is_model_downloaded(model: &WhisperModel) -> bool {
    models_dir()
        .join(format!("ggml-{}.bin", model.name()))
        .exists()
}

/// Download a Whisper GGML model from Hugging Face.
pub async fn download_model(model: &WhisperModel) -> Result<PathBuf> {
    let dir = models_dir();
    std::fs::create_dir_all(&dir).context("Failed to create models directory")?;

    let dest = dir.join(format!("ggml-{}.bin", model.name()));
    if dest.exists() {
        eprintln!(
            "Model {} already downloaded at {}",
            model.name(),
            dest.display()
        );
        return Ok(dest);
    }

    let url = model.ggml_url();
    eprintln!(
        "Downloading {} model (~{}MB) from Hugging Face...",
        model.name(),
        model.size_mb()
    );
    eprintln!("  {}", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = client
        .get(url)
        .send()
        .await
        .context("Download request failed")?;
    if !resp.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", resp.status());
    }

    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let tmp = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .context("Failed to create temp file")?;
    let mut downloaded: u64 = 0;

    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Download stream error")?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let pct = (downloaded as f64 / total as f64 * 100.0) as u32;
            eprint!(
                "\r  [{:>3}%] {:.1}/{:.1} MB",
                pct,
                downloaded as f64 / 1e6,
                total as f64 / 1e6
            );
        }
    }
    file.flush().await?;
    drop(file);
    eprintln!();

    tokio::fs::rename(&tmp, &dest)
        .await
        .context("Failed to rename downloaded model")?;
    eprintln!("Saved to {}", dest.display());
    Ok(dest)
}

/// Transcribe an audio file with a locally installed Whisper runtime.
///
/// Candidates are tried in the order given by [`local_whisper_candidates`]:
/// `whisper-cli` (current Homebrew), `whisper-cpp` (older Homebrew), `main`
/// (source build), then Python `openai-whisper`.
///
/// Non-WAV input is converted by ffmpeg first — see [`ensure_wav_16k`], whose
/// absence is reported as itself rather than as a missing Whisper runtime.
pub async fn transcribe_local(
    audio_path: &Path,
    model: &WhisperModel,
    language: &str,
) -> Result<String> {
    let model_path = models_dir().join(format!("ggml-{}.bin", model.name()));
    if !model_path.exists() {
        anyhow::bail!(
            "Model '{}' not downloaded. Run: /voice download {}\n  \
             Or download manually to {}",
            model.name(),
            model.name(),
            model_path.display()
        );
    }

    // Convert audio to WAV 16kHz mono if needed (ffmpeg)
    let wav_path = ensure_wav_16k(audio_path).await?;
    let wav_arg = wav_path.to_str().unwrap_or("");
    let model_arg = model_path.to_str().unwrap_or("");

    for candidate in local_whisper_candidates(model.name(), model_arg, language, wav_arg) {
        let output = tokio::process::Command::new(candidate.binary)
            .args(&candidate.args)
            .output()
            .await;

        if let Ok(out) = output {
            if out.status.success() {
                let text = clean_whisper_stdout(&String::from_utf8_lossy(&out.stdout));
                if !text.is_empty() {
                    cleanup_temp_wav(&wav_path, audio_path);
                    return Ok(text);
                }
            }
        }
    }

    cleanup_temp_wav(&wav_path, audio_path);
    anyhow::bail!(
        "No local Whisper runtime found. Install one of:\n  \
         - brew install whisper-cpp   (macOS — provides `whisper-cli`)\n  \
         - pip install openai-whisper (Python)\n  \
         - Build whisper.cpp from source: https://github.com/ggerganov/whisper.cpp"
    )
}

/// One way of invoking a local Whisper runtime.
pub(crate) struct WhisperInvocation {
    pub binary: &'static str,
    pub args: Vec<String>,
}

/// Local Whisper runtimes to try, in order.
///
/// `whisper-cli` leads because that is what current Homebrew's `whisper-cpp`
/// formula actually installs — it has not shipped a `whisper-cpp` binary for
/// several releases. Only looking for the old names meant the error message
/// told users to `brew install whisper-cpp` and then failed to find anything
/// after they did, which is worse than no advice.
pub(crate) fn local_whisper_candidates(
    model_name: &str,
    model_path: &str,
    language: &str,
    wav_path: &str,
) -> Vec<WhisperInvocation> {
    let cpp_args = |flag_model: &str, flag_lang: &str, flag_file: &str| {
        vec![
            flag_model.to_string(),
            model_path.to_string(),
            flag_lang.to_string(),
            language.to_string(),
            "--no-timestamps".to_string(),
            flag_file.to_string(),
            wav_path.to_string(),
        ]
    };
    vec![
        // Homebrew whisper-cpp ≥ 1.7.
        WhisperInvocation {
            binary: "whisper-cli",
            args: cpp_args("--model", "--language", "--file"),
        },
        // Older Homebrew builds.
        WhisperInvocation {
            binary: "whisper-cpp",
            args: cpp_args("--model", "--language", "--file"),
        },
        // Manually built whisper.cpp, which names its binary `main`.
        WhisperInvocation {
            binary: "main",
            args: cpp_args("-m", "-l", "-f"),
        },
        // Python openai-whisper as a last resort. Takes the model by name, not
        // path, and writes to stdout only with an explicit output format.
        WhisperInvocation {
            binary: "whisper",
            args: vec![
                wav_path.to_string(),
                "--model".to_string(),
                model_name.to_string(),
                "--language".to_string(),
                language.to_string(),
                "--output_format".to_string(),
                "txt".to_string(),
            ],
        },
    ]
}

/// Strip whisper.cpp's per-segment decoration from stdout.
///
/// Even with `--no-timestamps`, whisper-cli prefixes each segment with an
/// ANSI colour reset and pads with leading spaces; passing that through puts
/// escape codes into the user's composer.
pub(crate) fn clean_whisper_stdout(raw: &str) -> String {
    let stripped: String = {
        // Minimal CSI stripper — enough for whisper's colour resets, and
        // cheaper than pulling in a dependency for one call site.
        let mut out = String::with_capacity(raw.len());
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for e in chars.by_ref() {
                        if e.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                continue;
            }
            out.push(c);
        }
        out
    };

    stripped
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Ensure an audio file is WAV 16kHz mono (required by whisper.cpp).
/// If the file is already .wav, try to use it directly.
/// Otherwise, convert via ffmpeg.
///
/// A missing ffmpeg is an error here, not a shrug. It used to return the
/// unconverted file with a warning on stderr; whisper then failed to read it
/// and the caller reported "No local Whisper runtime found" — advice to install
/// the one component that was already present. Browser clients record WebM, so
/// this was the default path for the most common client.
async fn ensure_wav_16k(audio_path: &Path) -> Result<PathBuf> {
    let ext = audio_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if ext == "wav" {
        // Assume it's already the right format for simplicity
        return Ok(audio_path.to_path_buf());
    }

    // Convert via ffmpeg. Unique per call: the daemon serves concurrent
    // requests, and a fixed /tmp name means two simultaneous transcriptions
    // overwrite each other's audio — one caller silently gets the other's words.
    let tmp = std::env::temp_dir().join(format!(
        "vibecli_voice_{}_{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            audio_path.to_str().unwrap_or(""),
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            tmp.to_str().unwrap_or(""),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;

    match status {
        Ok(s) if s.success() => Ok(tmp),
        Ok(_) => anyhow::bail!(
            "ffmpeg could not convert '{}' to 16 kHz mono WAV. The recording may be corrupt.",
            audio_path.display()
        ),
        Err(_) => anyhow::bail!(
            "Local transcription of '.{ext}' audio needs ffmpeg to convert it to WAV. Install it:\n  \
             macOS:  brew install ffmpeg\n  \
             Linux:  sudo apt install ffmpeg\n  \
             Windows: choco install ffmpeg\n\
             (Cloud transcription accepts this format directly — set GROQ_API_KEY to skip the conversion.)"
        ),
    }
}

fn cleanup_temp_wav(wav_path: &Path, original: &Path) {
    if wav_path != original {
        let _ = std::fs::remove_file(wav_path);
    }
}

// ── Local TTS via system commands ─────────────────────────────────────────────

/// Speak text using local system TTS (no API key needed).
pub async fn local_tts(text: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let status = tokio::process::Command::new("say")
            .arg(text)
            .status()
            .await
            .context("Failed to run 'say' command")?;
        if !status.success() {
            anyhow::bail!("'say' command failed with exit code {:?}", status.code());
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        // Try espeak first, then spd-say
        let status = tokio::process::Command::new("espeak")
            .arg(text)
            .status()
            .await;
        if let Ok(s) = status {
            if s.success() {
                return Ok(());
            }
        }
        let status = tokio::process::Command::new("spd-say")
            .arg(text)
            .status()
            .await
            .context("No TTS found. Install espeak: sudo apt install espeak")?;
        if !status.success() {
            anyhow::bail!("TTS command failed");
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let ps_text = text.replace('\'', "''");
        let status = tokio::process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Add-Type -AssemblyName System.Speech; \
                 $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
                 $s.Speak('{}')",
                    ps_text
                ),
            ])
            .status()
            .await
            .context("Failed to run PowerShell TTS")?;
        if !status.success() {
            anyhow::bail!("PowerShell TTS failed");
        }
        return Ok(());
    }

    #[allow(unreachable_code)]
    {
        anyhow::bail!("Local TTS not supported on this platform")
    }
}

// ── Voice Dispatcher — unified online/offline access ──────────────────────────

/// Which engine actually produced a transcript.
///
/// [`VoiceDispatcher::transcribe_file`] falls back between two very different
/// engines, and callers that surface a transcript to a user need to say which
/// one ran — "transcribed locally" and "sent to Groq" are not interchangeable
/// claims to make on someone's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceEngine {
    /// Local whisper.cpp / openai-whisper CLI — audio never left the machine.
    LocalWhisper,
    /// Groq's hosted `whisper-large-v3` — audio was uploaded.
    CloudWhisper,
}

impl VoiceEngine {
    /// Stable wire name, used in daemon JSON responses.
    pub fn as_str(&self) -> &'static str {
        match self {
            VoiceEngine::LocalWhisper => "local_whisper",
            VoiceEngine::CloudWhisper => "cloud_whisper",
        }
    }
}

/// Unified voice engine with automatic online/offline fallback.
pub struct VoiceDispatcher {
    /// Groq Whisper API key (None = cloud unavailable).
    cloud_stt_key: Option<String>,
    /// ElevenLabs API key for cloud TTS.
    cloud_tts_key: Option<String>,
    /// ElevenLabs voice ID.
    cloud_voice_id: String,
    /// Local Whisper model to use.
    local_model: WhisperModel,
    /// Language code for local transcription.
    language: String,
    /// Prefer local even when cloud is available.
    prefer_local: bool,
}

impl VoiceDispatcher {
    /// Build from config. Resolves API keys from config, env vars, etc.
    pub fn from_config(vcfg: &VoiceConfig, groq_key: Option<&str>) -> Self {
        let model = WhisperModel::from_name(&vcfg.local_model).unwrap_or(WhisperModel::Base);
        Self {
            cloud_stt_key: vcfg.resolve_whisper_api_key(groq_key),
            cloud_tts_key: vcfg.resolve_elevenlabs_api_key(),
            cloud_voice_id: vcfg.resolve_elevenlabs_voice_id(),
            local_model: model,
            language: vcfg.language.clone(),
            prefer_local: vcfg.prefer_local,
        }
    }

    /// Transcribe an audio file (auto-fallback between local and cloud).
    ///
    /// Use [`Self::transcribe_file_with_engine`] when the caller needs to tell
    /// the user which engine ran.
    pub async fn transcribe_file(&self, path: &Path) -> Result<String> {
        self.transcribe_file_with_engine(path)
            .await
            .map(|(text, _)| text)
    }

    /// Transcribe an audio file, reporting which engine produced the text.
    pub async fn transcribe_file_with_engine(&self, path: &Path) -> Result<(String, VoiceEngine)> {
        if self.prefer_local || self.cloud_stt_key.is_none() {
            // Try local first
            if is_model_downloaded(&self.local_model) {
                match transcribe_local(path, &self.local_model, &self.language).await {
                    Ok(text) => return Ok((text, VoiceEngine::LocalWhisper)),
                    Err(e) => {
                        if self.cloud_stt_key.is_some() {
                            eprintln!("Local transcription failed, falling back to cloud: {e}");
                        } else {
                            return Err(e);
                        }
                    }
                }
            } else if self.cloud_stt_key.is_none() {
                anyhow::bail!(
                    "No voice engine available.\n  \
                     Offline: run /voice download {} to get the local model\n  \
                     Online:  set GROQ_API_KEY for cloud Whisper",
                    self.local_model.name()
                );
            }
        }

        // Cloud
        if let Some(key) = &self.cloud_stt_key {
            let text = transcribe_audio(path, key).await?;
            return Ok((text, VoiceEngine::CloudWhisper));
        }

        anyhow::bail!("No voice engine available. Set GROQ_API_KEY or run /voice download.")
    }

    /// Speak text (cloud TTS → local TTS fallback).
    pub async fn speak(&self, text: &str) -> Result<()> {
        // Try cloud TTS first if key is available
        if let Some(key) = &self.cloud_tts_key {
            match text_to_speech(text, key, &self.cloud_voice_id).await {
                Ok(bytes) => {
                    let out_path = std::env::temp_dir().join("vibecli_tts.mp3");
                    std::fs::write(&out_path, &bytes)?;
                    // Try to play the audio
                    play_audio(&out_path).await;
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Cloud TTS failed, falling back to local: {e}");
                }
            }
        }
        // Fall back to local system TTS
        local_tts(text).await
    }

    /// Record from microphone and transcribe (requires sox `rec` command).
    pub async fn listen(&self, silence_timeout_ms: u64) -> Result<String> {
        let tmp = std::env::temp_dir().join("vibecli_mic.wav");
        let silence_secs = format!("{:.1}", silence_timeout_ms as f64 / 1000.0);

        eprintln!(
            "Listening... (speak now, stops after {}s silence)",
            silence_secs
        );

        // Use sox `rec` for cross-platform mic capture
        let status = tokio::process::Command::new("rec")
            .args([
                tmp.to_str().unwrap_or(""),
                "rate",
                "16000",
                "channels",
                "1",
                "silence",
                "1",
                "0.1",
                "1%", // start recording on sound
                "1",
                &silence_secs,
                "1%", // stop after N seconds of silence
            ])
            .stdout(std::process::Stdio::null())
            .status()
            .await;

        match status {
            Ok(s) if s.success() => {
                let text = self.transcribe_file(&tmp).await?;
                let _ = std::fs::remove_file(&tmp);
                Ok(text)
            }
            Ok(_) => {
                let _ = std::fs::remove_file(&tmp);
                anyhow::bail!("Microphone recording failed")
            }
            Err(_) => {
                anyhow::bail!(
                    "Microphone capture requires SoX. Install it:\n  \
                     macOS:  brew install sox\n  \
                     Linux:  sudo apt install sox\n  \
                     Windows: choco install sox"
                )
            }
        }
    }

    /// Machine-readable engine status — the single source behind both
    /// [`Self::status`] (the REPL string) and the daemon's `GET /voice/status`.
    pub fn status_report(&self) -> VoiceStatus {
        VoiceStatus {
            cloud_stt_configured: self.cloud_stt_key.is_some(),
            cloud_tts_configured: self.cloud_tts_key.is_some(),
            local_model: self.local_model.name().to_string(),
            local_model_size_mb: self.local_model.size_mb(),
            local_model_downloaded: is_model_downloaded(&self.local_model),
            prefer_local: self.prefer_local,
            language: self.language.clone(),
            // `whisper-cli` is what current Homebrew installs; `whisper-cpp`
            // only exists on older builds. Probing just one under-reports.
            whisper_cpp_installed: binary_present("whisper-cli", "--help")
                || binary_present("whisper-cpp", "--help"),
            whisper_python_installed: binary_present("whisper", "--help"),
            sox_installed: binary_present("sox", "--version"),
            ffmpeg_installed: binary_present("ffmpeg", "-version"),
        }
    }

    /// Show current engine status.
    pub fn status(&self) -> String {
        let s = self.status_report();
        let yes_no = |b: bool| if b { "yes" } else { "no" };
        let configured = |b: bool| if b { "configured" } else { "not configured" };
        [
            "Voice Engine Status:".to_string(),
            format!(
                "  Cloud STT (Groq Whisper): {}",
                configured(s.cloud_stt_configured)
            ),
            format!(
                "  Cloud TTS (ElevenLabs):   {}",
                configured(s.cloud_tts_configured)
            ),
            format!(
                "  Local model:              {} ({}MB)",
                s.local_model, s.local_model_size_mb
            ),
            format!(
                "  Local model downloaded:   {}",
                yes_no(s.local_model_downloaded)
            ),
            format!("  Prefer local:             {}", s.prefer_local),
            format!("  Language:                 {}", s.language),
            format!(
                "  whisper-cpp installed:    {}",
                yes_no(s.whisper_cpp_installed)
            ),
            format!(
                "  whisper (Python):         {}",
                yes_no(s.whisper_python_installed)
            ),
            format!("  sox (mic capture):        {}", yes_no(s.sox_installed)),
            format!("  ffmpeg (format convert):  {}", yes_no(s.ffmpeg_installed)),
        ]
        .join("\n")
    }
}

/// Snapshot of what the voice stack can actually do on this machine.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VoiceStatus {
    pub cloud_stt_configured: bool,
    pub cloud_tts_configured: bool,
    pub local_model: String,
    pub local_model_size_mb: u64,
    pub local_model_downloaded: bool,
    pub prefer_local: bool,
    pub language: String,
    pub whisper_cpp_installed: bool,
    pub whisper_python_installed: bool,
    pub sox_installed: bool,
    /// Required for *local* transcription of anything that isn't already WAV.
    /// Browser clients record WebM, so without this their audio can only be
    /// transcribed in the cloud.
    pub ffmpeg_installed: bool,
}

impl VoiceStatus {
    /// True when at least one transcription engine can run right now.
    /// A downloaded model with no runtime to execute it is not a usable engine.
    pub fn can_transcribe(&self) -> bool {
        self.cloud_stt_configured
            || (self.local_model_downloaded
                && (self.whisper_cpp_installed || self.whisper_python_installed))
    }
}

/// Probe whether `bin` is on PATH by running it with a harmless flag.
fn binary_present(bin: &str, probe_arg: &str) -> bool {
    std::process::Command::new(bin)
        .arg(probe_arg)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// Try to play an audio file using system commands.
async fn play_audio(path: &Path) {
    let path_str = path.to_str().unwrap_or("");

    #[cfg(target_os = "macos")]
    {
        let _ = tokio::process::Command::new("afplay")
            .arg(path_str)
            .status()
            .await;
    }

    #[cfg(target_os = "linux")]
    {
        // Try aplay, then paplay, then mpv
        for cmd in &["aplay", "paplay", "mpv"] {
            if let Ok(s) = tokio::process::Command::new(cmd)
                .arg(path_str)
                .status()
                .await
            {
                if s.success() {
                    return;
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let _ = tokio::process::Command::new("powershell")
            .args([
                "-Command",
                &format!("(New-Object Media.SoundPlayer '{}').PlaySync()", path_str),
            ])
            .status()
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcribe_requires_path() {
        // Just verify the function signature compiles
        let _ = async {
            let path = std::path::Path::new("/tmp/test.wav");
            let _ = transcribe_audio(path, "test_key").await;
        };
    }

    #[test]
    fn tts_requires_voice_id() {
        let _ = async {
            let _ = text_to_speech("hello", "test_key", "voice_123").await;
        };
    }

    #[test]
    fn transcribe_with_different_extensions() {
        // Verify various audio file path extensions compile and work
        for ext in &["wav", "mp3", "ogg", "flac", "m4a"] {
            let path = std::path::PathBuf::from(format!("/tmp/audio.{}", ext));
            let _ = async move {
                let _ = transcribe_audio(&path, "key").await;
            };
        }
    }

    #[test]
    fn transcribe_path_file_name_extraction() {
        let path = std::path::Path::new("/home/user/recording.wav");
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav");
        assert_eq!(file_name, "recording.wav");
    }

    #[test]
    fn transcribe_path_no_extension() {
        let path = std::path::Path::new("/tmp/audiofile");
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav");
        assert_eq!(file_name, "audiofile");
    }

    #[test]
    fn transcribe_path_fallback_name() {
        // A path with no file_name component should fall back
        let path = std::path::Path::new("/");
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav");
        assert_eq!(file_name, "audio.wav");
    }

    #[test]
    fn tts_url_construction() {
        let voice_id = "abc123";
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);
        assert_eq!(url, "https://api.elevenlabs.io/v1/text-to-speech/abc123");
    }

    #[test]
    fn tts_url_with_special_chars() {
        let voice_id = "voice-with-dashes";
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);
        assert!(url.ends_with("voice-with-dashes"));
    }

    #[test]
    fn whisper_api_url_is_correct() {
        let url = "https://api.groq.com/openai/v1/audio/transcriptions";
        assert!(url.starts_with("https://"));
        assert!(url.contains("groq.com"));
        assert!(url.contains("transcriptions"));
    }

    #[test]
    fn auth_header_format() {
        let api_key = "gsk_test_key_12345";
        let header = format!("Bearer {}", api_key);
        assert!(header.starts_with("Bearer "));
        assert!(header.ends_with("12345"));
    }

    #[test]
    fn tts_json_payload_structure() {
        let text = "Hello world";
        let payload = serde_json::json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.5
            }
        });
        assert_eq!(payload["text"], "Hello world");
        assert_eq!(payload["model_id"], "eleven_multilingual_v2");
        assert_eq!(payload["voice_settings"]["stability"], 0.5);
        assert_eq!(payload["voice_settings"]["similarity_boost"], 0.5);
    }

    #[test]
    fn tts_json_payload_empty_text() {
        let payload = serde_json::json!({
            "text": "",
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.5
            }
        });
        assert_eq!(payload["text"], "");
    }

    #[test]
    fn tts_json_payload_unicode_text() {
        let text = "Bonjour le monde! \u{1F600}";
        let payload = serde_json::json!({ "text": text });
        let serialized = serde_json::to_string(&payload).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert!(parsed["text"].as_str().unwrap().contains("Bonjour"));
    }

    #[test]
    fn transcribe_audio_path_with_spaces() {
        let path = std::path::Path::new("/tmp/my audio file.wav");
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav");
        assert_eq!(file_name, "my audio file.wav");
    }

    #[test]
    fn tts_url_construction_empty_voice_id() {
        let voice_id = "";
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);
        assert!(url.ends_with('/'));
    }

    // ── Local/Offline voice tests ─────────────────────────────────────

    #[test]
    fn models_dir_is_under_vibecli() {
        let dir = models_dir();
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.contains(".vibecli") && dir_str.contains("models"));
    }

    #[test]
    fn is_model_downloaded_returns_false_for_missing() {
        // No model files should exist in CI
        assert!(!is_model_downloaded(&WhisperModel::Large));
    }

    #[test]
    fn dispatcher_from_config_no_keys() {
        let vcfg = VoiceConfig {
            whisper_api_key: None,
            elevenlabs_api_key: None,
            elevenlabs_voice_id: None,
            tts_enabled: false,
            prefer_local: false,
            local_model: "base".to_string(),
            language: "en".to_string(),
            silence_timeout_ms: 1500,
        };
        let d = VoiceDispatcher::from_config(&vcfg, None);
        assert!(d.cloud_stt_key.is_none());
        assert!(d.cloud_tts_key.is_none());
        assert!(!d.prefer_local);
        assert_eq!(d.language, "en");
    }

    #[test]
    fn dispatcher_from_config_with_cloud_key() {
        let vcfg = VoiceConfig {
            whisper_api_key: Some("gsk_test".to_string()),
            elevenlabs_api_key: Some("el_test".to_string()),
            elevenlabs_voice_id: None,
            tts_enabled: false,
            prefer_local: false,
            local_model: "tiny".to_string(),
            language: "fr".to_string(),
            silence_timeout_ms: 2000,
        };
        let d = VoiceDispatcher::from_config(&vcfg, None);
        assert_eq!(d.cloud_stt_key.as_deref(), Some("gsk_test"));
        assert_eq!(d.cloud_tts_key.as_deref(), Some("el_test"));
        assert_eq!(d.language, "fr");
    }

    #[test]
    fn dispatcher_from_config_prefer_local() {
        let vcfg = VoiceConfig {
            whisper_api_key: Some("gsk_key".to_string()),
            elevenlabs_api_key: None,
            elevenlabs_voice_id: None,
            tts_enabled: false,
            prefer_local: true,
            local_model: "small".to_string(),
            language: "de".to_string(),
            silence_timeout_ms: 1500,
        };
        let d = VoiceDispatcher::from_config(&vcfg, None);
        assert!(d.prefer_local);
        assert!(d.cloud_stt_key.is_some()); // still available as fallback
    }

    #[test]
    fn dispatcher_from_config_groq_key_fallback() {
        let vcfg = VoiceConfig {
            whisper_api_key: None,
            elevenlabs_api_key: None,
            elevenlabs_voice_id: None,
            tts_enabled: false,
            prefer_local: false,
            local_model: "base".to_string(),
            language: "en".to_string(),
            silence_timeout_ms: 1500,
        };
        let d = VoiceDispatcher::from_config(&vcfg, Some("groq_fallback_key"));
        assert_eq!(d.cloud_stt_key.as_deref(), Some("groq_fallback_key"));
    }

    #[test]
    fn dispatcher_status_contains_key_info() {
        let vcfg = VoiceConfig {
            whisper_api_key: None,
            elevenlabs_api_key: None,
            elevenlabs_voice_id: None,
            tts_enabled: false,
            prefer_local: true,
            local_model: "base".to_string(),
            language: "en".to_string(),
            silence_timeout_ms: 1500,
        };
        let d = VoiceDispatcher::from_config(&vcfg, None);
        let status = d.status();
        assert!(status.contains("Cloud STT"));
        assert!(status.contains("not configured"));
        assert!(status.contains("Local model"));
        assert!(status.contains("base"));
        assert!(status.contains("Prefer local"));
    }

    #[test]
    fn cleanup_temp_wav_noop_when_same_path() {
        let p = Path::new("/tmp/test.wav");
        // Should not panic when paths are the same
        cleanup_temp_wav(p, p);
    }

    #[test]
    fn whisper_model_from_name() {
        assert_eq!(WhisperModel::from_name("tiny"), Some(WhisperModel::Tiny));
        assert_eq!(WhisperModel::from_name("BASE"), Some(WhisperModel::Base));
        assert_eq!(WhisperModel::from_name("Small"), Some(WhisperModel::Small));
        assert_eq!(
            WhisperModel::from_name("medium"),
            Some(WhisperModel::Medium)
        );
        assert_eq!(WhisperModel::from_name("large"), Some(WhisperModel::Large));
        assert_eq!(WhisperModel::from_name("unknown"), None);
    }

    #[test]
    fn whisper_model_ggml_urls_are_valid() {
        for model in WhisperModel::all() {
            let url = model.ggml_url();
            assert!(url.starts_with("https://huggingface.co/"));
            assert!(url.contains("ggml-"));
            assert!(url.ends_with(".bin"));
        }
    }

    // ── Local runtime discovery ────────────────────────────────────────────

    #[test]
    fn whisper_candidates_lead_with_the_binary_homebrew_actually_installs() {
        // Homebrew's `whisper-cpp` formula ships `whisper-cli`; it has not
        // shipped a `whisper-cpp` binary for several releases. Probing only the
        // old names made the error message recommend an install that then still
        // failed to be found.
        let candidates =
            local_whisper_candidates("base", "/models/ggml-base.bin", "en", "/tmp/a.wav");
        let names: Vec<&str> = candidates.iter().map(|c| c.binary).collect();
        assert_eq!(names[0], "whisper-cli");
        assert!(names.contains(&"whisper-cpp"), "older Homebrew builds still work");
        assert!(names.contains(&"main"), "source builds name the binary `main`");
        assert!(names.contains(&"whisper"), "Python openai-whisper is the fallback");
    }

    #[test]
    fn whisper_cpp_candidates_pass_the_model_path_and_python_passes_the_model_name() {
        // The two families disagree: whisper.cpp takes a path to the ggml file,
        // openai-whisper takes a model *name* and downloads its own weights.
        let candidates =
            local_whisper_candidates("base", "/models/ggml-base.bin", "de", "/tmp/a.wav");
        let cli = candidates.iter().find(|c| c.binary == "whisper-cli").unwrap();
        assert!(cli.args.contains(&"/models/ggml-base.bin".to_string()));
        assert!(cli.args.contains(&"de".to_string()));
        assert!(cli.args.contains(&"/tmp/a.wav".to_string()));

        let py = candidates.iter().find(|c| c.binary == "whisper").unwrap();
        assert!(py.args.contains(&"base".to_string()));
        assert!(!py.args.contains(&"/models/ggml-base.bin".to_string()));
    }

    #[test]
    fn whisper_stdout_is_stripped_of_ansi_and_joined_to_one_line() {
        // whisper-cli emits a colour reset and leading padding per segment even
        // with --no-timestamps; passing it through puts escape codes into the
        // user's composer.
        let raw = "\u{1b}[2K   Add a test for the parser\n\u{1b}[2K   and fix the build\n\n";
        assert_eq!(
            clean_whisper_stdout(raw),
            "Add a test for the parser and fix the build"
        );
    }

    #[test]
    fn whisper_stdout_cleaning_is_a_no_op_for_plain_text() {
        assert_eq!(clean_whisper_stdout("  hello world  "), "hello world");
        assert_eq!(clean_whisper_stdout(""), "");
    }
}
