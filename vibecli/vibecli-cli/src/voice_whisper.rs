//! Real I/O for local voice transcription (US-005).
//!
//! The existing [`voice_local`] module holds the in-memory data model
//! (transcription history, VAD config, model registry, metrics). This module
//! adds the real-I/O primitives the stub was missing:
//!
//! - [`download_model`] — streams a GGML whisper model over HTTP to disk
//! - [`load_wav_pcm`] — parses a 16-bit PCM / 16 kHz / mono WAV file into
//!   the `Vec<f32>` shape whisper.cpp expects
//! - [`Transcriber`] — a trait the rest of the app can use without caring
//!   whether the backend is NullTranscriber (tests), whisper.cpp (desktop),
//!   or a future cloud backend
//! - [`NullTranscriber`] — deterministic backend for environments without
//!   whisper.cpp; still validates buffer shape and reports meaningful text
//! - [`WhisperTranscriber`] — real whisper.cpp FFI, gated behind the
//!   `voice-whisper` cargo feature so the default build stays portable
//!
//! The stub in `voice_local.rs` keeps its pure business logic (model
//! catalog, VAD, history). Callers that need real transcription construct a
//! [`Transcriber`] and hand it the PCM samples.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

// ── Transcriber trait ───────────────────────────────────────────────────────

/// Backend-agnostic speech-to-text interface.
pub trait Transcriber: Send + Sync {
    /// Run inference over a buffer of mono float32 PCM samples.
    fn transcribe(&self, pcm: &[f32], sample_rate: u32) -> Result<String, String>;
}

/// Deterministic transcriber used in tests and as a fallback when whisper.cpp
/// isn't compiled in. It reports the shape of the input so callers can tell
/// they got real PCM through the pipeline even without a real model.
pub struct NullTranscriber;

impl Transcriber for NullTranscriber {
    fn transcribe(&self, pcm: &[f32], sample_rate: u32) -> Result<String, String> {
        if pcm.is_empty() {
            return Err("empty audio buffer".to_string());
        }
        if sample_rate == 0 {
            return Err("sample_rate must be > 0".to_string());
        }
        let duration = pcm.len() as f64 / sample_rate as f64;
        Ok(format!(
            "[null-transcriber] {} samples at {} Hz ({:.2}s)",
            pcm.len(),
            sample_rate,
            duration
        ))
    }
}

// ── Model download ──────────────────────────────────────────────────────────

/// Stream a whisper GGML model from `url` to `dest`, returning the number of
/// bytes written. Uses reqwest's chunked `bytes_stream` so large model files
/// don't need to fit in memory.
pub async fn download_model(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<u64, String> {
    use futures::StreamExt;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("http error: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!(
            "download failed: HTTP {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        ));
    }
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir: {e}"))?;
        }
    }
    let mut file = File::create(dest).map_err(|e| format!("create: {e}"))?;
    let mut stream = resp.bytes_stream();
    let mut total: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("read chunk: {e}"))?;
        file.write_all(&chunk).map_err(|e| format!("write: {e}"))?;
        total += chunk.len() as u64;
    }
    Ok(total)
}

// ── WAV parsing ─────────────────────────────────────────────────────────────

/// Parsed PCM buffer from a WAV file.
#[derive(Debug, Clone)]
pub struct WavPcm {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Read a RIFF WAV file and return float32-normalized PCM samples.
///
/// Supports PCM16 (common whisper.cpp input) mono files. Multi-channel files
/// are downmixed by averaging channels. Returns an error for compressed
/// WAVs or sample rates other than what the caller can handle.
pub fn load_wav_pcm(path: &Path) -> Result<WavPcm, String> {
    let mut f = File::open(path).map_err(|e| format!("open: {e}"))?;
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).map_err(|e| format!("read: {e}"))?;
    parse_wav(&bytes)
}

/// Parse a WAV blob. Extracted so tests can synthesize bytes without hitting
/// disk.
pub fn parse_wav(bytes: &[u8]) -> Result<WavPcm, String> {
    if bytes.len() < 44 {
        return Err(format!("wav too short: {} bytes", bytes.len()));
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".to_string());
    }

    let mut pos = 12usize;
    let mut fmt_chunk: Option<(u16, u16, u32, u16)> = None; // (audio_format, channels, sample_rate, bits_per_sample)
    let mut data_chunk: Option<&[u8]> = None;

    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let chunk_size =
            u32::from_le_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]])
                as usize;
        let chunk_start = pos + 8;
        let chunk_end = chunk_start.saturating_add(chunk_size);
        if chunk_end > bytes.len() {
            return Err(format!(
                "chunk {:?} at {pos} overflows (len={})",
                std::str::from_utf8(chunk_id).unwrap_or("??"),
                chunk_size
            ));
        }
        match chunk_id {
            b"fmt " => {
                if chunk_size < 16 {
                    return Err("fmt chunk too small".to_string());
                }
                let audio_format = u16::from_le_bytes([bytes[chunk_start], bytes[chunk_start + 1]]);
                let channels =
                    u16::from_le_bytes([bytes[chunk_start + 2], bytes[chunk_start + 3]]);
                let sample_rate = u32::from_le_bytes([
                    bytes[chunk_start + 4],
                    bytes[chunk_start + 5],
                    bytes[chunk_start + 6],
                    bytes[chunk_start + 7],
                ]);
                let bits_per_sample =
                    u16::from_le_bytes([bytes[chunk_start + 14], bytes[chunk_start + 15]]);
                fmt_chunk = Some((audio_format, channels, sample_rate, bits_per_sample));
            }
            b"data" => {
                data_chunk = Some(&bytes[chunk_start..chunk_end]);
            }
            _ => {}
        }
        pos = chunk_end;
        // WAV chunks are padded to even size.
        if chunk_size % 2 == 1 && pos < bytes.len() {
            pos += 1;
        }
    }

    let (fmt, channels, sr, bits) =
        fmt_chunk.ok_or_else(|| "missing fmt chunk".to_string())?;
    let data = data_chunk.ok_or_else(|| "missing data chunk".to_string())?;
    if fmt != 1 {
        return Err(format!(
            "unsupported WAV format tag {fmt} (only PCM/1 supported)"
        ));
    }
    if bits != 16 {
        return Err(format!(
            "unsupported bits_per_sample {bits} (only 16 supported)"
        ));
    }
    if channels == 0 {
        return Err("WAV reports 0 channels".to_string());
    }

    let bytes_per_frame = 2usize * channels as usize;
    if data.len() % bytes_per_frame != 0 {
        return Err(format!(
            "data length {} not a multiple of frame size {}",
            data.len(),
            bytes_per_frame
        ));
    }
    let frame_count = data.len() / bytes_per_frame;
    let mut out = Vec::with_capacity(frame_count);
    for frame_idx in 0..frame_count {
        let base = frame_idx * bytes_per_frame;
        let mut acc: f32 = 0.0;
        for c in 0..channels as usize {
            let lo = data[base + 2 * c];
            let hi = data[base + 2 * c + 1];
            let v = i16::from_le_bytes([lo, hi]) as f32 / 32768.0;
            acc += v;
        }
        out.push(acc / channels as f32);
    }
    Ok(WavPcm {
        samples: out,
        sample_rate: sr,
        channels,
    })
}

/// Build a minimal RIFF/WAVE/PCM16 blob for the given mono samples.  Used by
/// tests that want a real WAV header but don't want to depend on `hound`.
pub fn encode_wav_mono_pcm16(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let byte_rate = sample_rate * 2;
    let data_bytes = samples.len() as u32 * 2;
    let riff_size = 36 + data_bytes;
    let mut out = Vec::with_capacity(44 + data_bytes as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

// ── whisper.cpp FFI backend (opt-in) ────────────────────────────────────────

#[cfg(feature = "voice-whisper")]
pub use whisper_backend::WhisperTranscriber;

#[cfg(feature = "voice-whisper")]
mod whisper_backend {
    use super::Transcriber;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    /// whisper.cpp-backed transcriber. Holds a single [`WhisperContext`] so
    /// the model is loaded once and reused across calls.
    pub struct WhisperTranscriber {
        ctx: Mutex<WhisperContext>,
        #[allow(dead_code)]
        model_path: PathBuf,
    }

    impl WhisperTranscriber {
        pub fn load(model_path: &Path) -> Result<Self, String> {
            let params = WhisperContextParameters::default();
            let ctx = WhisperContext::new_with_params(
                &model_path.to_string_lossy(),
                params,
            )
            .map_err(|e| format!("load whisper model {model_path:?}: {e}"))?;
            Ok(Self {
                ctx: Mutex::new(ctx),
                model_path: model_path.to_path_buf(),
            })
        }
    }

    impl Transcriber for WhisperTranscriber {
        fn transcribe(&self, pcm: &[f32], sample_rate: u32) -> Result<String, String> {
            if pcm.is_empty() {
                return Err("empty audio buffer".to_string());
            }
            // whisper.cpp requires 16 kHz mono.
            if sample_rate != 16_000 {
                return Err(format!(
                    "whisper.cpp expects 16 kHz audio, got {sample_rate}"
                ));
            }
            let mut ctx = self.ctx.lock().map_err(|_| "ctx lock poisoned")?;
            let mut state = ctx
                .create_state()
                .map_err(|e| format!("create state: {e}"))?;
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_translate(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_special(false);
            state
                .full(params, pcm)
                .map_err(|e| format!("whisper.full: {e}"))?;
            let n = state
                .full_n_segments()
                .map_err(|e| format!("n_segments: {e}"))?;
            let mut text = String::new();
            for i in 0..n {
                let seg = state
                    .full_get_segment_text(i)
                    .map_err(|e| format!("segment {i}: {e}"))?;
                text.push_str(&seg);
            }
            Ok(text.trim().to_string())
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_transcriber_rejects_empty() {
        let t = NullTranscriber;
        let err = t.transcribe(&[], 16_000).unwrap_err();
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn null_transcriber_reports_shape() {
        let t = NullTranscriber;
        let pcm = vec![0.1_f32; 1600];
        let out = t.transcribe(&pcm, 16_000).unwrap();
        assert!(out.contains("1600 samples"), "{out}");
        assert!(out.contains("16000 Hz"), "{out}");
        assert!(out.contains("0.10s"), "{out}");
    }

    #[test]
    fn encode_and_decode_wav_round_trip() {
        let input: Vec<i16> = (0..800).map(|i| (i as i16).wrapping_mul(10)).collect();
        let bytes = encode_wav_mono_pcm16(&input, 16_000);
        let parsed = parse_wav(&bytes).unwrap();
        assert_eq!(parsed.sample_rate, 16_000);
        assert_eq!(parsed.channels, 1);
        assert_eq!(parsed.samples.len(), 800);
        // first non-zero sample should be positive
        assert!(parsed.samples[1] > 0.0);
    }

    #[test]
    fn parse_wav_rejects_garbage() {
        let err = parse_wav(b"not a wav file at all").unwrap_err();
        assert!(err.contains("RIFF") || err.contains("short"), "{err}");
    }
}
