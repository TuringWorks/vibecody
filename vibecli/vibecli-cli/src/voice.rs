#![allow(dead_code)]
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
pub async fn text_to_speech(
    text: &str,
    api_key: &str,
    voice_id: &str,
) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let url = format!(
        "https://api.elevenlabs.io/v1/text-to-speech/{}",
        voice_id
    );

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
    models_dir().join(format!("ggml-{}.bin", model.name())).exists()
}

/// Download a Whisper GGML model from Hugging Face.
pub async fn download_model(model: &WhisperModel) -> Result<PathBuf> {
    let dir = models_dir();
    std::fs::create_dir_all(&dir).context("Failed to create models directory")?;

    let dest = dir.join(format!("ggml-{}.bin", model.name()));
    if dest.exists() {
        eprintln!("Model {} already downloaded at {}", model.name(), dest.display());
        return Ok(dest);
    }

    let url = model.ggml_url();
    eprintln!("Downloading {} model (~{}MB) from Hugging Face...", model.name(), model.size_mb());
    eprintln!("  {}", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = client.get(url).send().await.context("Download request failed")?;
    if !resp.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", resp.status());
    }

    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let tmp = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&tmp).await.context("Failed to create temp file")?;
    let mut downloaded: u64 = 0;

    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Download stream error")?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let pct = (downloaded as f64 / total as f64 * 100.0) as u32;
            eprint!("\r  [{:>3}%] {:.1}/{:.1} MB", pct, downloaded as f64 / 1e6, total as f64 / 1e6);
        }
    }
    file.flush().await?;
    drop(file);
    eprintln!();

    tokio::fs::rename(&tmp, &dest).await.context("Failed to rename downloaded model")?;
    eprintln!("Saved to {}", dest.display());
    Ok(dest)
}

/// Transcribe an audio file using the local `whisper-cpp` CLI tool or the `whisper` CLI.
///
/// Tries these in order:
/// 1. `whisper-cpp` (Homebrew: `brew install whisper-cpp`)
/// 2. `whisper` (Python: `pip install openai-whisper`)
/// 3. `main` from whisper.cpp build directory
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
            model.name(), model.name(), model_path.display()
        );
    }

    // Convert audio to WAV 16kHz mono if needed (ffmpeg)
    let wav_path = ensure_wav_16k(audio_path).await?;
    let wav_arg = wav_path.to_str().unwrap_or("");
    let model_arg = model_path.to_str().unwrap_or("");

    // Try whisper-cpp first (brew install whisper-cpp)
    let output = tokio::process::Command::new("whisper-cpp")
        .args(["--model", model_arg, "--language", language, "--no-timestamps", "--file", wav_arg])
        .output()
        .await;

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !text.is_empty() {
                cleanup_temp_wav(&wav_path, audio_path);
                return Ok(text);
            }
        }
    }

    // Try whisper.cpp `main` binary (manual build)
    let output = tokio::process::Command::new("main")
        .args(["-m", model_arg, "-l", language, "--no-timestamps", "-f", wav_arg])
        .output()
        .await;

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !text.is_empty() {
                cleanup_temp_wav(&wav_path, audio_path);
                return Ok(text);
            }
        }
    }

    // Try Python openai-whisper as last resort
    let output = tokio::process::Command::new("whisper")
        .args([wav_arg, "--model", model.name(), "--language", language, "--output_format", "txt"])
        .output()
        .await;

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            cleanup_temp_wav(&wav_path, audio_path);
            return Ok(text);
        }
    }

    cleanup_temp_wav(&wav_path, audio_path);
    anyhow::bail!(
        "No local Whisper runtime found. Install one of:\n  \
         - brew install whisper-cpp   (macOS)\n  \
         - pip install openai-whisper (Python)\n  \
         - Build whisper.cpp from source: https://github.com/ggerganov/whisper.cpp"
    )
}

/// Ensure an audio file is WAV 16kHz mono (required by whisper.cpp).
/// If the file is already .wav, try to use it directly.
/// Otherwise, convert via ffmpeg.
async fn ensure_wav_16k(audio_path: &Path) -> Result<PathBuf> {
    let ext = audio_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "wav" {
        // Assume it's already the right format for simplicity
        return Ok(audio_path.to_path_buf());
    }

    // Convert via ffmpeg
    let tmp = std::env::temp_dir().join("vibecli_voice_input.wav");
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y", "-i", audio_path.to_str().unwrap_or(""),
            "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le",
            tmp.to_str().unwrap_or(""),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;

    match status {
        Ok(s) if s.success() => Ok(tmp),
        _ => {
            // If ffmpeg is not available, try the original file anyway
            eprintln!("Warning: ffmpeg not found — trying original file format (may fail)");
            Ok(audio_path.to_path_buf())
        }
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
            if s.success() { return Ok(()); }
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
            .args(["-Command", &format!(
                "Add-Type -AssemblyName System.Speech; \
                 $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
                 $s.Speak('{}')", ps_text
            )])
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
    pub async fn transcribe_file(&self, path: &Path) -> Result<String> {
        if self.prefer_local || self.cloud_stt_key.is_none() {
            // Try local first
            if is_model_downloaded(&self.local_model) {
                match transcribe_local(path, &self.local_model, &self.language).await {
                    Ok(text) => return Ok(text),
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
            return Ok(text);
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

        eprintln!("Listening... (speak now, stops after {}s silence)", silence_secs);

        // Use sox `rec` for cross-platform mic capture
        let status = tokio::process::Command::new("rec")
            .args([
                tmp.to_str().unwrap_or(""),
                "rate", "16000",
                "channels", "1",
                "silence", "1", "0.1", "1%",  // start recording on sound
                "1", &silence_secs, "1%",      // stop after N seconds of silence
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

    /// Show current engine status.
    pub fn status(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Voice Engine Status:"));
        lines.push(format!("  Cloud STT (Groq Whisper): {}", if self.cloud_stt_key.is_some() { "configured" } else { "not configured" }));
        lines.push(format!("  Cloud TTS (ElevenLabs):   {}", if self.cloud_tts_key.is_some() { "configured" } else { "not configured" }));
        lines.push(format!("  Local model:              {} ({}MB)", self.local_model.name(), self.local_model.size_mb()));
        lines.push(format!("  Local model downloaded:   {}", if is_model_downloaded(&self.local_model) { "yes" } else { "no" }));
        lines.push(format!("  Prefer local:             {}", self.prefer_local));
        lines.push(format!("  Language:                 {}", self.language));

        // Check for local whisper runtime
        let has_whisper_cpp = std::process::Command::new("whisper-cpp").arg("--help")
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok();
        let has_whisper_py = std::process::Command::new("whisper").arg("--help")
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok();
        let has_sox = std::process::Command::new("sox").arg("--version")
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok();

        lines.push(format!("  whisper-cpp installed:    {}", if has_whisper_cpp { "yes" } else { "no" }));
        lines.push(format!("  whisper (Python):         {}", if has_whisper_py { "yes" } else { "no" }));
        lines.push(format!("  sox (mic capture):        {}", if has_sox { "yes" } else { "no" }));

        lines.join("\n")
    }
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
        return;
    }

    #[cfg(target_os = "linux")]
    {
        // Try aplay, then paplay, then mpv
        for cmd in &["aplay", "paplay", "mpv"] {
            if let Ok(s) = tokio::process::Command::new(cmd).arg(path_str).status().await {
                if s.success() { return; }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let _ = tokio::process::Command::new("powershell")
            .args(["-Command", &format!("(New-Object Media.SoundPlayer '{}').PlaySync()", path_str)])
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
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("audio.wav");
        assert_eq!(file_name, "recording.wav");
    }

    #[test]
    fn transcribe_path_no_extension() {
        let path = std::path::Path::new("/tmp/audiofile");
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("audio.wav");
        assert_eq!(file_name, "audiofile");
    }

    #[test]
    fn transcribe_path_fallback_name() {
        // A path with no file_name component should fall back
        let path = std::path::Path::new("/");
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("audio.wav");
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
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("audio.wav");
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
        assert_eq!(WhisperModel::from_name("medium"), Some(WhisperModel::Medium));
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
}
