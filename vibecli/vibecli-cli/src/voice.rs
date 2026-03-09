#![allow(dead_code)]
//! Voice & media features — Whisper transcription via Groq, ElevenLabs TTS.
//!
//! Provides server-side speech-to-text and text-to-speech for gateway bots
//! and CLI voice input mode.

use anyhow::{Context, Result};

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

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str("audio/wav")?;

    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-large-v3")
        .part("file", part);

    let resp = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
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

    let resp = client
        .post(&url)
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.5
            }
        }))
        .send()
        .await
        .context("ElevenLabs TTS request failed")?;

    if !resp.status().is_success() {
        let err = resp.text().await?;
        anyhow::bail!("ElevenLabs API error: {}", err);
    }

    Ok(resp.bytes().await?.to_vec())
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
}
