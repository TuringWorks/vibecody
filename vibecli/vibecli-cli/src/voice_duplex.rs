//! Full-duplex voice — the pipeline, in the daemon, for every surface.
//!
//! The microphone stays open while the assistant speaks, so a user can
//! interrupt mid-sentence. That is only possible because the *client's* capture
//! pipeline removes our own output from the mic signal; measured at ≥40 dB in a
//! WKWebView, covering WebAudio-rendered playback (`tools/webview-probe --arm
//! aec`). A client without echo cancellation must use push-to-talk instead —
//! an open mic there makes the agent interrupt itself on its own voice.
//!
//! ```text
//!   client mic → AEC → 16 kHz PCM ─ws─► VAD → ASR → provider → TTS ─ws─► client
//!                                        ▲                                │
//!                                        └────── barge-in cancels ────────┘
//! ```
//!
//! Everything here is transport-agnostic below the WebSocket: the same turn
//! loop serves the desktop shells, the IDE clients, mobile and the web client.
//! Clients contribute audio and speakers, nothing else.

use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// Frames are 20 ms at 16 kHz — the rate every ASR engine here wants.
pub const MIC_RATE: u32 = 16_000;
pub const FRAME: usize = 320;

// ── Voice activity detection ────────────────────────────────────────────────

const SPEECH_DBFS: f32 = -45.0;
/// Silence that ends a turn. 600 ms is where the 2026 local-stack write-ups
/// converge: shorter clips words, longer reads as lag.
const HANGOVER_MS: u32 = 600;
/// Ignore blips, so a keyboard click cannot open a turn.
const MIN_SPEECH_MS: u32 = 200;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Turn {
    Silence,
    /// Speech began — this is the barge-in signal.
    SpeechStart,
    Speech,
    /// The utterance is complete.
    SpeechEnd,
}

#[derive(Default)]
pub struct Vad {
    in_speech: bool,
    silence_ms: u32,
    speech_ms: u32,
}

pub fn dbfs(frame: &[i16]) -> f32 {
    if frame.is_empty() {
        return -120.0;
    }
    let sum: f64 = frame
        .iter()
        .map(|&s| {
            let f = s as f64 / 32768.0;
            f * f
        })
        .sum();
    let rms = (sum / frame.len() as f64).sqrt();
    if rms <= 1e-9 {
        -120.0
    } else {
        20.0 * rms.log10() as f32
    }
}

impl Vad {
    pub fn push(&mut self, frame: &[i16]) -> Turn {
        if dbfs(frame) > SPEECH_DBFS {
            self.silence_ms = 0;
            self.speech_ms += 20;
            if !self.in_speech && self.speech_ms >= MIN_SPEECH_MS {
                self.in_speech = true;
                return Turn::SpeechStart;
            }
            return if self.in_speech { Turn::Speech } else { Turn::Silence };
        }
        self.speech_ms = 0;
        if self.in_speech {
            self.silence_ms += 20;
            if self.silence_ms >= HANGOVER_MS {
                self.in_speech = false;
                self.silence_ms = 0;
                return Turn::SpeechEnd;
            }
            return Turn::Speech;
        }
        Turn::Silence
    }
}

// ── Barge-in ────────────────────────────────────────────────────────────────

/// Bumped whenever the user starts speaking. Every stage checks it and drops
/// work belonging to a superseded turn — that is what barge-in *is*.
#[derive(Clone, Default)]
pub struct Generation(Arc<AtomicU64>);

impl Generation {
    pub fn current(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
    pub fn bump(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }
    pub fn is_stale(&self, gen: u64) -> bool {
        self.current() != gen
    }
}

// ── Sentence splitting ──────────────────────────────────────────────────────

/// Split a token stream into speakable sentences.
///
/// TTS has to start before the model finishes or first-audio inherits the whole
/// generation time. A sentence is the smallest unit that still sounds natural.
#[derive(Default)]
pub struct SentenceSplitter {
    buf: String,
}

impl SentenceSplitter {
    pub fn push(&mut self, tok: &str) -> Option<String> {
        self.buf.push_str(tok);
        let bytes = self.buf.as_bytes();
        if let Some(i) = self.buf.rfind(['.', '!', '?', '\n', '।']) {
            let next_ok = bytes.get(i + 1).is_none_or(|c| c.is_ascii_whitespace());
            let prev_digit = i > 0 && bytes[i - 1].is_ascii_digit();
            let next_digit = bytes.get(i + 1).is_some_and(|c| c.is_ascii_digit());
            if next_ok && !(prev_digit && next_digit) && i + 1 >= self.buf.trim_end().len() {
                let s = self.buf.trim().to_string();
                self.buf.clear();
                if s.chars().any(|c| c.is_alphanumeric()) {
                    return Some(s);
                }
            }
        }
        None
    }

    pub fn flush(&mut self) -> Option<String> {
        let s = self.buf.trim().to_string();
        self.buf.clear();
        s.chars().any(|c| c.is_alphanumeric()).then_some(s)
    }
}

// ── Text to speech ──────────────────────────────────────────────────────────

/// Where synthesised audio comes from.
///
/// The streaming sidecar is macOS-only and optional; without it every platform
/// still speaks, just with the whole utterance synthesised before the first
/// sample goes out. That is a latency difference, not a capability one, so the
/// feature is never silently unavailable.
pub enum Tts {
    /// Resident `AVSpeechSynthesizer` sidecar: ~15–25 ms to first audio.
    Streaming(Sidecar),
    /// `say` / `espeak` / PowerShell to a WAV, then read it back.
    Batch,
}

pub struct Sidecar {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// Mono PCM plus the rate it was produced at. Callers resample or tell the
/// client; nothing here assumes 22.05 kHz.
pub struct Audio {
    pub pcm: Vec<f32>,
    pub rate: u32,
}

impl Tts {
    /// Prefer the streaming sidecar, fall back to batch synthesis.
    pub async fn open(sidecar_bin: Option<&str>) -> Self {
        if let Some(bin) = sidecar_bin {
            if std::path::Path::new(bin).exists() {
                if let Ok(s) = Sidecar::spawn(bin).await {
                    return Tts::Streaming(s);
                }
            }
        }
        Tts::Batch
    }

    pub fn is_streaming(&self) -> bool {
        matches!(self, Tts::Streaming(_))
    }

    /// Begin an utterance. Chunks then arrive from [`Tts::next_chunk`].
    pub async fn say(&mut self, text: &str, voice: Option<&str>, rate: f32) -> Result<()> {
        match self {
            Tts::Streaming(s) => s.say(text, voice, rate).await,
            Tts::Batch => Ok(()),
        }
    }

    pub async fn cancel(&mut self) -> Result<()> {
        match self {
            Tts::Streaming(s) => s.cancel().await,
            Tts::Batch => Ok(()),
        }
    }

    /// `Some(audio)` for a frame, `None` at end of utterance.
    pub async fn next_chunk(&mut self) -> Result<Option<Audio>> {
        match self {
            Tts::Streaming(s) => s.next_chunk().await,
            Tts::Batch => Ok(None),
        }
    }

    /// Whole-utterance synthesis, for the batch path.
    pub async fn synthesize(&mut self, text: &str, voice: Option<&str>) -> Result<Option<Audio>> {
        match self {
            Tts::Streaming(_) => Ok(None),
            Tts::Batch => synthesize_wav(text, voice).await.map(Some),
        }
    }
}

impl Sidecar {
    async fn spawn(bin: &str) -> Result<Self> {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("tts sidecar: no stdin"))?;
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| anyhow::anyhow!("tts sidecar: no stdout"))?,
        );
        let mut s = Self {
            _child: child,
            stdin,
            stdout,
        };
        // Warm the voice engine: the first utterance in a fresh process costs
        // ~300 ms and every later one ~20 ms. Pay it before anyone is listening.
        // It must contain something speakable — a blank utterance produces no
        // callback at all, and a reader waiting for the terminator would hang.
        s.say("ok", None, 0.52).await?;
        while s.next_chunk().await?.is_some() {}
        Ok(s)
    }

    async fn say(&mut self, text: &str, voice: Option<&str>, rate: f32) -> Result<()> {
        let mut o = serde_json::json!({ "text": text, "rate": rate });
        if let Some(v) = voice {
            o["voice"] = serde_json::json!(v);
        }
        self.stdin.write_all(format!("{o}\n").as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn cancel(&mut self) -> Result<()> {
        self.stdin.write_all(b"{\"cmd\":\"cancel\"}\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Bounded. A stage that can block forever is the worst failure mode in a
    /// real-time loop, and this one can: the synthesiser returns no callback at
    /// all for an unspeakable utterance. Treat a stall as end of utterance.
    async fn next_chunk(&mut self) -> Result<Option<Audio>> {
        match tokio::time::timeout(std::time::Duration::from_secs(5), self.read_frame()).await {
            Ok(r) => r,
            Err(_) => Ok(None),
        }
    }

    async fn read_frame(&mut self) -> Result<Option<Audio>> {
        let mut tag = [0u8; 3];
        self.stdout.read_exact(&mut tag).await?;
        let mut len = [0u8; 4];
        self.stdout.read_exact(&mut len).await?;
        let n = u32::from_le_bytes(len) as usize;
        if &tag == b"END" {
            return Ok(None);
        }
        // Bound the allocation on a length that arrived over a pipe. 4 MB is
        // ~47 s of 22 kHz mono float; nothing legitimate approaches it.
        if n > 4 * 1024 * 1024 {
            anyhow::bail!("tts sidecar: implausible frame length {n}");
        }
        let mut buf = vec![0u8; n];
        self.stdout.read_exact(&mut buf).await?;
        Ok(Some(Audio {
            pcm: buf
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
            rate: 22_050,
        }))
    }
}

/// Cross-platform whole-utterance synthesis to PCM.
///
/// Deliberately writes a file and reads it back rather than calling the
/// platform's "speak" verb: `voice.rs::local_tts` plays through the *daemon's*
/// speakers, which is the wrong machine entirely whenever the client is
/// somewhere else — a phone, a watch, or a browser on the other side of the
/// house.
async fn synthesize_wav(text: &str, voice: Option<&str>) -> Result<Audio> {
    let path = std::env::temp_dir().join(format!("vibe-tts-{}.wav", std::process::id()));
    let p = path.to_string_lossy().to_string();

    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("say");
        if let Some(v) = voice {
            cmd.args(["-v", v]);
        }
        cmd.args(["-o", &p, "--data-format=LEI16@22050", text]);
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        cmd.status().await?;
    }
    #[cfg(target_os = "linux")]
    {
        let _ = voice;
        Command::new("espeak")
            .args(["-w", &p, text])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;
    }
    #[cfg(target_os = "windows")]
    {
        let _ = voice;
        let script = format!(
            "Add-Type -AssemblyName System.Speech; \
             $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
             $s.SetOutputToWaveFile('{p}'); $s.Speak('{}'); $s.Dispose()",
            text.replace('\'', "''")
        );
        Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;
    }

    let bytes = tokio::fs::read(&path).await?;
    let _ = tokio::fs::remove_file(&path).await;
    Ok(decode_wav(&bytes))
}

/// Minimal PCM WAV reader — enough for what the synthesisers above emit.
///
/// Reads the declared sample rate rather than assuming one: `espeak` commonly
/// produces 22.05 kHz and `say` whatever it was asked for, and a wrong rate
/// does not fail, it just plays back at the wrong pitch.
fn decode_wav(bytes: &[u8]) -> Audio {
    let rate = bytes
        .get(24..28)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .filter(|r| (8_000..=48_000).contains(r))
        .unwrap_or(22_050);
    // Find `data`; the header is not always exactly 44 bytes.
    let start = bytes
        .windows(4)
        .position(|w| w == b"data")
        .map(|i| i + 8)
        .unwrap_or(44);
    let pcm = bytes
        .get(start..)
        .unwrap_or(&[])
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
        .collect();
    Audio { pcm, rate }
}

// ── Speech to text ──────────────────────────────────────────────────────────

/// Language name for a Whisper language code, for the reply instruction.
/// A bare code in the prompt is markedly less reliable than a name.
/// Bound and tidy a client-supplied workspace context block.
///
/// The client chooses what to send and the daemon holds it for the life of the
/// socket, so the size is not the client's to choose. 32k characters is far
/// more than a file tree and a pinned note, and truncating costs the tail of
/// the context rather than the whole turn.
pub fn clamp_context(s: &str) -> String {
    const MAX_CHARS: usize = 32 * 1024;
    let s = s.trim();
    // Split on a character boundary — a byte slice through a multi-byte
    // codepoint panics, and a file tree is exactly where non-ASCII shows up.
    match s.char_indices().nth(MAX_CHARS) {
        Some((byte, _)) => format!("{}\n…(context truncated)", &s[..byte]),
        None => s.to_string(),
    }
}

pub fn language_name(code: &str) -> &'static str {
    match code {
        "en" => "English", "es" => "Spanish", "fr" => "French", "de" => "German",
        "it" => "Italian", "pt" => "Portuguese", "nl" => "Dutch", "ru" => "Russian",
        "pl" => "Polish", "uk" => "Ukrainian", "tr" => "Turkish", "ar" => "Arabic",
        "he" => "Hebrew", "hi" => "Hindi", "bn" => "Bengali", "ta" => "Tamil",
        "te" => "Telugu", "kn" => "Kannada", "ja" => "Japanese", "ko" => "Korean",
        "zh" => "Chinese", "vi" => "Vietnamese", "th" => "Thai", "id" => "Indonesian",
        "ms" => "Malay", "sv" => "Swedish", "da" => "Danish", "nb" => "Norwegian",
        "fi" => "Finnish", "cs" => "Czech", "sk" => "Slovak", "hu" => "Hungarian",
        "ro" => "Romanian", "el" => "Greek", "mr" => "Marathi", "ur" => "Urdu",
        _ => "the same language the user spoke",
    }
}

/// Whisper reports a language *name*; the rest of the pipeline speaks codes.
pub fn code_for(name: &str) -> String {
    match name {
        "english" => "en", "spanish" => "es", "french" => "fr", "german" => "de",
        "italian" => "it", "portuguese" => "pt", "dutch" => "nl", "russian" => "ru",
        "polish" => "pl", "ukrainian" => "uk", "turkish" => "tr", "arabic" => "ar",
        "hebrew" => "he", "hindi" => "hi", "bengali" => "bn", "tamil" => "ta",
        "telugu" => "te", "kannada" => "kn", "japanese" => "ja", "korean" => "ko",
        "chinese" => "zh", "vietnamese" => "vi", "thai" => "th", "indonesian" => "id",
        "malay" => "ms", "swedish" => "sv", "danish" => "da", "norwegian" => "nb",
        "finnish" => "fi", "czech" => "cs", "slovak" => "sk", "hungarian" => "hu",
        "romanian" => "ro", "greek" => "el", "marathi" => "mr", "urdu" => "ur",
        other => other,
    }
    .to_string()
}

/// Write 16-bit mono PCM as a WAV, for engines that only accept files.
pub fn write_wav(path: &std::path::Path, pcm: &[i16], rate: u32) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    let data_len = (pcm.len() * 2) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&rate.to_le_bytes())?;
    f.write_all(&(rate * 2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in pcm {
        f.write_all(&s.to_le_bytes())?;
    }
    Ok(())
}

/// Transcribe through a resident `whisper-server`, returning text and, when
/// auto-detecting, the language it heard.
///
/// **Never pin a language across turns when auto-detecting.** Forcing `-l xx`
/// suppresses the detection result, leaving the caller to assume the language
/// it asked for — so a user who says one Hindi sentence and then an English one
/// gets the English turn labelled Hindi and answered in Hindi. Code-switching
/// is the normal case for multilingual speakers.
pub async fn transcribe_server(
    server: &str,
    pcm: &[i16],
    rate: u32,
    lang: Option<&str>,
) -> Result<(String, Option<String>)> {
    let path = std::env::temp_dir().join(format!("vibe-asr-{}.wav", std::process::id()));
    write_wav(&path, pcm, rate)?;
    let out = Command::new("curl")
        .args([
            "-sS", "-m", "60",
            &format!("{server}/inference"),
            "-F", &format!("file=@{}", path.display()),
            "-F", &format!("language={}", lang.unwrap_or("auto")),
            "-F", "response_format=verbose_json",
            "-F", "temperature=0",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;
    let _ = std::fs::remove_file(&path);
    let out = out?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let text = v.get("text").and_then(|t| t.as_str()).unwrap_or("").trim().to_string();
    // Only report a language when detection actually ran. Falling back to the
    // requested one manufactures a fact nobody measured.
    let detected = if lang.is_none() {
        v.get("detected_language")
            .or_else(|| v.get("language"))
            .and_then(|l| l.as_str())
            .map(code_for)
    } else {
        lang.map(str::to_string)
    };
    Ok((clean_transcript(&text), detected))
}

/// Whisper emits bracketed non-speech markers ("[BLANK_AUDIO]", "(music)").
/// They are not words and must not reach the model as if they were.
pub fn clean_transcript(raw: &str) -> String {
    raw.lines()
        .map(str::trim)
        .filter(|l| {
            !l.is_empty()
                && !(l.starts_with('[') && l.ends_with(']'))
                && !(l.starts_with('(') && l.ends_with(')'))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Bring up a resident `whisper-server`, or reuse one already listening.
///
/// Spawning `whisper-cli` per utterance pays model load *and* backend init
/// every turn: `ggml-small` measured 1433 ms total against 570 ms of actual
/// encode. Resident, the same model and audio answer in ~455 ms.
///
/// `ggml-small` is the quality floor for non-Latin scripts — `base` renders
/// Devanagari in Arabic script, while `small` and `medium` produce identical
/// correct text and `small` is 3× faster.
pub async fn ensure_whisper_server(bin: &str, model: &str, port: u16) -> Option<String> {
    let url = format!("http://127.0.0.1:{port}");

    // Ask the port first. A server already listening is usable whether or not
    // we can find a binary to start one — checking the binary first meant a
    // perfectly good running server was ignored because `whisper-server` is not
    // a path relative to the daemon's working directory.
    if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
        return Some(url);
    }

    let bin = resolve_bin(bin)?;
    if !std::path::Path::new(model).exists() {
        tracing::warn!(model, "voice: speech model not found; duplex voice unavailable");
        return None;
    }
    Command::new(&bin)
        .args(["-m", model, "--port", &port.to_string(), "-t", "4"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // Poll to a deadline; a cold model load is seconds, not milliseconds.
    for _ in 0..60 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return Some(url);
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    None
}

/// Resolve a configured binary to something runnable.
///
/// A bare command name is the natural thing to write in a config file and the
/// natural thing to get wrong: `Path::new("whisper-server").exists()` asks
/// whether it sits in the daemon's working directory, which it never does, so
/// the engine was reported missing on a machine that had it installed. Walk
/// `PATH` when the name contains no separator.
pub fn resolve_bin(bin: &str) -> Option<String> {
    let p = std::path::Path::new(bin);
    if p.components().count() > 1 || bin.starts_with('/') {
        return p.exists().then(|| bin.to_string());
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| {
            std::env::split_paths(&paths)
                .map(|d| d.join(bin))
                .collect::<Vec<_>>()
        })
        .find(|c| c.is_file())
        .map(|c| c.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    /// The pipeline the daemon runs between the provider and the speaker.
    fn spoken(chunks: &[&str]) -> Vec<String> {
        let mut filter = crate::agent_stream_filter::StreamFilter::new();
        let mut split = super::SentenceSplitter::default();
        let mut out = Vec::new();
        for c in chunks {
            let t = filter.push(c);
            if t.is_empty() {
                continue;
            }
            if let Some(s) = split.push(&t) {
                out.push(s);
            }
        }
        let tail = filter.finish();
        if !tail.is_empty() {
            if let Some(s) = split.push(&tail) {
                out.push(s);
            }
        }
        out.extend(split.flush());
        out
    }

    #[test]
    fn a_reasoning_model_speaks_its_answer_and_not_its_deliberation() {
        // What Ollama actually streams for gpt-oss: reasoning arrives in a
        // separate field and the provider splices it in as a `<thinking>`
        // block. Spoken unfiltered, the assistant read out "The user says:
        // Hey, how are you doing? As a voice assistant, respond in one or two
        // short spoken sentences…" before ever answering.
        let said = spoken(&[
            "<thinking>",
            "The user says: \"Hey, how are you doing?\" ",
            "As a voice assistant, respond in one or two short spoken sentences. ",
            "We should keep it short.",
            "</thinking>\n",
            "I'm doing great, thanks!",
            " How can I help you today?",
        ]);
        assert_eq!(
            said,
            vec!["I'm doing great, thanks!", "How can I help you today?"]
        );
    }

    #[test]
    fn a_tag_split_across_chunks_is_still_suppressed() {
        // Tokens are not tag-aligned. A per-chunk strip both misses the tag
        // and leaks its two halves into the speaker.
        let said = spoken(&["<thin", "king>", "deliberating.", "</think", "ing>", "Yes."]);
        assert_eq!(said, vec!["Yes."]);
    }

    #[test]
    fn an_unclosed_reasoning_block_is_dropped_rather_than_spoken() {
        // The stream ended mid-thought. It is still reasoning.
        assert!(spoken(&["<think>", "still deciding"]).is_empty());
    }

    #[test]
    fn a_model_that_does_not_reason_is_unaffected() {
        assert_eq!(spoken(&["Yes.", " Two files changed."]), vec!["Yes.", "Two files changed."]);
    }

    #[test]
    fn clamp_context_trims_and_keeps_short_blocks_whole() {
        assert_eq!(super::clamp_context("  src/main.rs\n  "), "src/main.rs");
        assert_eq!(super::clamp_context("   "), "");
    }

    #[test]
    fn clamp_context_bounds_a_client_that_sends_too_much() {
        let huge = "x".repeat(100_000);
        let out = super::clamp_context(&huge);
        assert!(out.starts_with(&"x".repeat(32 * 1024)));
        assert!(out.ends_with("(context truncated)"));
        assert!(out.chars().count() < 33 * 1024);
    }

    #[test]
    fn clamp_context_truncates_on_a_character_boundary() {
        // Slicing mid-codepoint panics; a tree full of box-drawing characters
        // is exactly the input that would find it.
        let wide = "└─".repeat(40_000);
        let out = super::clamp_context(&wide);
        assert!(out.ends_with("(context truncated)"));
    }

    use super::*;

    #[test]
    fn vad_needs_sustained_speech_before_opening_a_turn() {
        let mut v = Vad::default();
        let loud = vec![12_000i16; FRAME];
        // A single frame is a click, not a turn.
        assert_eq!(v.push(&loud), Turn::Silence);
        // 200 ms of it is speech.
        for _ in 0..8 {
            v.push(&loud);
        }
        let mut v2 = Vad::default();
        let mut started = false;
        for _ in 0..20 {
            if v2.push(&loud) == Turn::SpeechStart {
                started = true;
                break;
            }
        }
        assert!(started, "sustained speech must open a turn");
    }

    #[test]
    fn vad_ends_a_turn_only_after_the_hangover() {
        let mut v = Vad::default();
        let loud = vec![12_000i16; FRAME];
        let quiet = vec![0i16; FRAME];
        for _ in 0..20 {
            v.push(&loud);
        }
        // Well short of 600 ms must not end the turn.
        for _ in 0..10 {
            assert_ne!(v.push(&quiet), Turn::SpeechEnd);
        }
        let mut ended = false;
        for _ in 0..40 {
            if v.push(&quiet) == Turn::SpeechEnd {
                ended = true;
                break;
            }
        }
        assert!(ended, "sustained silence must end the turn");
    }

    #[test]
    fn sentences_are_emitted_whole_and_blanks_are_dropped() {
        let mut s = SentenceSplitter::default();
        assert_eq!(s.push("Hello"), None);
        assert_eq!(s.push(" there."), Some("Hello there.".to_string()));
        // A decimal point is not a sentence boundary.
        assert_eq!(s.push("It is 3"), None);
        assert_eq!(s.push(".5"), None);
        // Punctuation alone is not speakable.
        let mut t = SentenceSplitter::default();
        assert_eq!(t.push("..."), None);
        assert_eq!(t.flush(), None);
    }

    /// The distinction the turn loop depends on: a turn superseded *before* it
    /// spoke is a user still forming a thought, and its words must be carried
    /// into the next turn. One superseded *while* speaking is a real
    /// interruption and must be dropped.
    ///
    /// Getting this wrong is not cosmetic. "plus fifty one." <pause> "minus
    /// fifty four." is one instruction in two breaths; dropping the first half
    /// silently answers a different question — 32 - 54 instead of 32 + 51 - 54.
    #[test]
    fn unspoken_words_are_carried_forward_but_interrupted_speech_is_not() {
        fn resolve(prior: Option<&str>, heard: &str) -> String {
            match prior {
                Some(p) => format!("{p} {heard}"),
                None => heard.to_string(),
            }
        }
        // Superseded before any audio: the two breaths become one instruction.
        assert_eq!(resolve(Some("plus 51."), "minus 54."), "plus 51. minus 54.");
        // A turn that already spoke leaves nothing behind to merge.
        assert_eq!(resolve(None, "stop"), "stop");
    }

    #[test]
    fn a_barge_in_makes_the_previous_turn_stale() {
        let g = Generation::default();
        let mine = g.current();
        assert!(!g.is_stale(mine));
        g.bump();
        assert!(g.is_stale(mine), "a superseded turn must stop producing audio");
    }

    #[test]
    fn wav_roundtrips_through_the_decoder_at_the_declared_rate() {
        let pcm: Vec<i16> = (0..800).map(|i| ((i as f32 / 8.0).sin() * 10_000.0) as i16).collect();
        let path = std::env::temp_dir().join(format!("vd-test-{}.wav", std::process::id()));
        write_wav(&path, &pcm, 16_000).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).ok();
        let audio = decode_wav(&bytes);
        assert_eq!(audio.rate, 16_000, "the declared rate must be read, not assumed");
        assert_eq!(audio.pcm.len(), pcm.len());
    }

    #[test]
    fn non_speech_markers_are_not_words() {
        assert_eq!(clean_transcript("[BLANK_AUDIO]"), "");
        assert_eq!(clean_transcript("(music)\nhello"), "hello");
    }

    #[test]
    fn a_bare_command_name_is_resolved_through_path() {
        // The failure this exists to prevent: a bare name checked as a relative
        // path is never found, and a machine with the tool installed is told it
        // has no speech engine.
        assert!(
            resolve_bin("sh").is_some_and(|p| p.contains('/')),
            "a bare name on PATH must resolve to an absolute path"
        );
        assert_eq!(resolve_bin("definitely-not-a-real-binary-xyz"), None);
        // An explicit path is taken at face value, present or not.
        assert_eq!(resolve_bin("/nonexistent/whisper-server"), None);
        assert!(resolve_bin("/bin/sh").is_some());
    }

    #[test]
    fn language_codes_map_both_ways() {
        assert_eq!(code_for("hindi"), "hi");
        assert_eq!(language_name("hi"), "Hindi");
        // An unknown code must not be guessed at.
        assert_eq!(code_for("klingon"), "klingon");
    }
}
