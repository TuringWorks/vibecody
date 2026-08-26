//! ASR, LLM and TTS — the three stages between end-of-speech and first audio.
//!
//! Each one reports the latency it actually cost. A voice feature whose budget
//! is asserted rather than measured is the success-assuming-fallback family
//! with a microphone attached.

use std::io::Write;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc;

/// Bumped whenever the user starts speaking. Every stage checks it and drops
/// work belonging to a superseded turn — that is what barge-in *is*.
#[derive(Clone, Default)]
pub struct Generation(Arc<AtomicU64>);

impl Generation {
    pub fn current(&self) -> u64 { self.0.load(Ordering::SeqCst) }
    pub fn bump(&self) -> u64 { self.0.fetch_add(1, Ordering::SeqCst) + 1 }
    pub fn is_stale(&self, gen: u64) -> bool { self.current() != gen }
}

// ── ASR ─────────────────────────────────────────────────────────────────────

fn write_wav(path: &std::path::Path, pcm: &[i16], rate: u32) -> anyhow::Result<()> {
    let mut f = std::fs::File::create(path)?;
    let data_len = (pcm.len() * 2) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;      // PCM
    f.write_all(&1u16.to_le_bytes())?;      // mono
    f.write_all(&rate.to_le_bytes())?;
    f.write_all(&(rate * 2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in pcm { f.write_all(&s.to_le_bytes())?; }
    Ok(())
}

/// Streaming, on-device ASR through the Swift sidecar.
///
/// The win is not model speed — it is that recognition overlaps the speech, so
/// end of utterance only has to *finalise*. Measured: **33 ms** to final,
/// against **978 ms** for whole-utterance Whisper, which could not begin any of
/// its work until the turn was already over.
///
/// macOS only. Linux and Windows need Moonshine v2 or Parakeet TDT to reach the
/// same shape; `Asr` below stays as the portable fallback.
pub struct StreamAsr {
    _child: Child,
    stdin: Arc<tokio::sync::Mutex<ChildStdin>>,
    finals: tokio::sync::Mutex<mpsc::UnboundedReceiver<(String, u128)>>,
}

impl StreamAsr {
    /// `on_partial` is called for every interim hypothesis, so the UI can show
    /// words appearing as they are spoken.
    pub async fn spawn<F>(bin: &str, mut on_partial: F) -> anyhow::Result<Self>
    where F: FnMut(String) + Send + 'static {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
            .spawn()?;
        let stdin = Arc::new(tokio::sync::Mutex::new(child.stdin.take().unwrap()));
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
                if let Some(f) = v.get("final").and_then(|x| x.as_str()) {
                    let ms = v.get("ms").and_then(|x| x.as_u64()).unwrap_or(0) as u128;
                    if tx.send((f.to_string(), ms)).is_err() { break; }
                } else if let Some(p) = v.get("partial").and_then(|x| x.as_str()) {
                    on_partial(p.to_string());
                }
            }
        });
        Ok(Self { _child: child, stdin, finals: tokio::sync::Mutex::new(rx) })
    }

    async fn frame(&self, tag: &[u8; 3], body: &[u8]) -> anyhow::Result<()> {
        let mut g = self.stdin.lock().await;
        g.write_all(tag).await?;
        g.write_all(&(body.len() as u32).to_le_bytes()).await?;
        if !body.is_empty() { g.write_all(body).await?; }
        g.flush().await?;
        Ok(())
    }

    /// Feed audio as it arrives — this is the part that buys the 945 ms.
    pub async fn feed(&self, pcm: &[i16]) -> anyhow::Result<()> {
        let mut bytes = Vec::with_capacity(pcm.len() * 2);
        for s in pcm { bytes.extend_from_slice(&s.to_le_bytes()); }
        self.frame(b"PCM", &bytes).await
    }

    pub async fn end_of_utterance(&self) -> anyhow::Result<()> { self.frame(b"EOU", &[]).await }

    /// Abandon the current utterance — barge-in.
    pub async fn reset(&self) -> anyhow::Result<()> { self.frame(b"RST", &[]).await }

    /// Bounded: a recogniser that never returns must not strand the turn.
    pub async fn wait_final(&self) -> Option<(String, u128)> {
        let mut rx = self.finals.lock().await;
        tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv()).await.ok().flatten()
    }
}

pub struct Asr {
    pub bin: String,
    pub model: String,
    /// Resident `whisper-server`, when one is up.
    ///
    /// Spawning `whisper-cli` per utterance pays model load *and* Metal/backend
    /// init every turn: `small` measured 1433 ms total against 570 ms of actual
    /// encode. Resident, the same model and the same audio answer in ~455 ms.
    pub server: Option<String>,
}

/// Whisper reports a language name ("hindi"); the rest of the pipeline speaks
/// ISO codes. Anything unmapped falls through as-is rather than being guessed at.
fn code_for(name: &str) -> String {
    match name {
        "english" => "en", "spanish" => "es", "french" => "fr", "german" => "de",
        "italian" => "it", "portuguese" => "pt", "dutch" => "nl", "russian" => "ru",
        "polish" => "pl", "ukrainian" => "uk", "turkish" => "tr", "arabic" => "ar",
        "hebrew" => "he", "hindi" => "hi", "bengali" => "bn", "tamil" => "ta",
        "telugu" => "te", "kannada" => "kn", "japanese" => "ja", "korean" => "ko",
        "chinese" => "zh", "vietnamese" => "vi", "thai" => "th", "indonesian" => "id",
        "malay" => "ms", "swedish" => "sv", "danish" => "da", "norwegian" => "nb",
        "finnish" => "fi", "czech" => "cs", "slovak" => "sk", "hungarian" => "hu",
        "romanian" => "ro", "greek" => "el", "bulgarian" => "bg", "croatian" => "hr",
        "catalan" => "ca", "slovenian" => "sl", "urdu" => "ur", "marathi" => "mr",
        other => other,
    }.to_string()
}

/// Human-readable name for a Whisper language code, for the reply instruction.
/// A code in the prompt ("reply in hi") is markedly less reliable than a name.
pub fn language_name(code: &str) -> &'static str {
    match code {
        "en" => "English", "es" => "Spanish", "fr" => "French", "de" => "German",
        "it" => "Italian", "pt" => "Portuguese", "nl" => "Dutch", "ru" => "Russian",
        "pl" => "Polish", "uk" => "Ukrainian", "tr" => "Turkish", "ar" => "Arabic",
        "he" => "Hebrew", "hi" => "Hindi", "bn" => "Bengali", "ta" => "Tamil",
        "te" => "Telugu", "kn" => "Kannada", "ja" => "Japanese", "ko" => "Korean",
        "zh" => "Chinese", "vi" => "Vietnamese", "th" => "Thai", "id" => "Indonesian",
        "ms" => "Malay", "sv" => "Swedish", "da" => "Danish", "nb" | "no" => "Norwegian",
        "fi" => "Finnish", "cs" => "Czech", "sk" => "Slovak", "hu" => "Hungarian",
        "ro" => "Romanian", "el" => "Greek", "bg" => "Bulgarian", "hr" => "Croatian",
        "ca" => "Catalan", "sl" => "Slovenian",
        _ => "the same language the user spoke",
    }
}

impl Asr {
    /// Transcribe, and report which language it was.
    ///
    /// `lang` of `None` means auto-detect — Whisper carries language ID for 99
    /// languages, which is why the multilingual path goes through it rather
    /// than the (much faster, but here English-only) streaming recogniser.
    ///
    /// **Never pin the language across turns in auto mode.** Forcing `-l xx`
    /// suppresses Whisper's `auto-detected language:` line, so there is nothing
    /// to report and the caller has to assume the language it asked for. A user
    /// who says one Hindi sentence and then an English one gets the English
    /// turn labelled Hindi, and the model is duly instructed to answer it in
    /// Hindi. Code-switching is the normal case for multilingual speakers, so
    /// detection runs every turn.
    pub async fn transcribe_lang(&self, pcm: &[i16], rate: u32, lang: Option<&str>)
        -> anyhow::Result<(String, Option<String>)>
    {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("vd-{}-l.wav", std::process::id()));
        write_wav(&path, pcm, rate)?;

        if let Some(url) = &self.server {
            let r = Command::new("curl")
                .args(["-sS", "-m", "60", &format!("{url}/inference"),
                       "-F", &format!("file=@{}", path.display()),
                       "-F", &format!("language={}", lang.unwrap_or("auto")),
                       "-F", "response_format=verbose_json", "-F", "temperature=0"])
                .stdout(Stdio::piped()).stderr(Stdio::null())
                .output().await;
            if let Ok(out) = r {
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    let _ = std::fs::remove_file(&path);
                    let text = v.get("text").and_then(|t| t.as_str()).unwrap_or("").trim().to_string();
                    // Only report a language when auto-detect actually ran.
                    let detected = if lang.is_none() {
                        v.get("detected_language").or_else(|| v.get("language"))
                            .and_then(|l| l.as_str()).map(code_for)
                    } else { lang.map(str::to_string) };
                    return Ok((clean_transcript(&text), detected));
                }
            }
            // Server unreachable or malformed: fall through to the CLI rather
            // than dropping the turn.
        }

        let out = Command::new(&self.bin)
            .args(["-m", &self.model, "-f", path.to_str().unwrap(),
                   "-l", lang.unwrap_or("auto"), "-nt", "-t", "4"])
            .stdout(Stdio::piped()).stderr(Stdio::piped())
            .output().await?;
        let _ = std::fs::remove_file(&path);
        let err = String::from_utf8_lossy(&out.stderr);
        // Only report a language Whisper actually detected. Falling back to the
        // requested one manufactures a fact nobody measured.
        let detected = err.split("auto-detected language: ").nth(1)
            .and_then(|r| r.split_whitespace().next())
            .map(str::to_string);
        Ok((clean_transcript(&String::from_utf8_lossy(&out.stdout)), detected))
    }

    pub async fn transcribe(&self, pcm: &[i16], rate: u32) -> anyhow::Result<String> {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("vd-{}.wav", std::process::id()));
        write_wav(&path, pcm, rate)?;
        let out = Command::new(&self.bin)
            .args(["-m", &self.model, "-f", path.to_str().unwrap(), "-nt", "-np", "-t", "4"])
            .stdout(Stdio::piped()).stderr(Stdio::null())
            .output().await?;
        let _ = std::fs::remove_file(&path);
        Ok(clean_transcript(&String::from_utf8_lossy(&out.stdout)))
    }
}

/// Whisper emits bracketed non-speech markers ("[BLANK_AUDIO]", "(music)").
/// They are not words and must not be sent to the model as if they were.
fn clean_transcript(raw: &str) -> String {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty()
            && !(l.starts_with('[') && l.ends_with(']'))
            && !(l.starts_with('(') && l.ends_with(')')))
        .collect::<Vec<_>>()
        .join(" ")
}

// ── LLM ─────────────────────────────────────────────────────────────────────

/// Split a token stream into speakable sentences.
///
/// TTS must start before the model finishes, or first-audio inherits the whole
/// generation time. A sentence is the smallest unit that still sounds natural.
pub struct SentenceSplitter { buf: String }

impl SentenceSplitter {
    pub fn new() -> Self { Self { buf: String::new() } }
    pub fn push(&mut self, tok: &str) -> Option<String> {
        self.buf.push_str(tok);
        // Split on terminal punctuation, but not on a decimal point or an
        // abbreviation, which would chop a sentence mid-breath.
        let bytes = self.buf.as_bytes();
        if let Some(i) = self.buf.rfind(['.', '!', '?', '\n']) {
            let next_ok = bytes.get(i + 1).map_or(true, |c| c.is_ascii_whitespace());
            let prev_digit = i > 0 && bytes[i - 1].is_ascii_digit();
            let next_digit = bytes.get(i + 1).is_some_and(|c| c.is_ascii_digit());
            if next_ok && !(prev_digit && next_digit) && i + 1 >= self.buf.trim_end().len() {
                let s = self.buf.trim().to_string();
                self.buf.clear();
                if s.len() > 1 { return Some(s); }
            }
        }
        None
    }
    pub fn flush(&mut self) -> Option<String> {
        let s = self.buf.trim().to_string();
        self.buf.clear();
        if s.is_empty() { None } else { Some(s) }
    }
}

pub struct Llm { pub url: String, pub model: String }

impl Llm {
    /// Force the model resident before the first real turn.
    ///
    /// A cold ollama load measured **3796 ms** to first token against **54–151 ms**
    /// warm. Without this the opening exchange of every session is four seconds
    /// of silence, which reads as "it's broken", not "it's loading".
    pub async fn warm(&self) -> anyhow::Result<u128> {
        let t = std::time::Instant::now();
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{"role":"user","content":"hi"}],
            "stream": false, "options": {"num_predict": 1}
        });
        let _ = Command::new("curl")
            .args(["-sS", "-m", "180", "-X", "POST", &format!("{}/api/chat", self.url),
                   "-H", "Content-Type: application/json", "-d", &body.to_string()])
            .stdout(Stdio::null()).stderr(Stdio::null())
            .status().await?;
        Ok(t.elapsed().as_millis())
    }

    /// Stream a reply, handing each completed sentence to `on_sentence`.
    /// Returns (full_text, ttft_ms).
    pub async fn stream<F>(
        &self, history: &[(String, String)], user: &str, lang: Option<&str>,
        gen: Generation, my_gen: u64, mut on_sentence: F,
    ) -> anyhow::Result<(String, u128)>
    where F: FnMut(String) {
        let lang_rule = match lang {
            Some(c) if c != "en" => format!(
                " The user is speaking {0}. Reply ONLY in {0}. Do not translate to English.",
                language_name(c)),
            _ => String::new(),
        };
        let mut messages = vec![serde_json::json!({
            "role": "system",
            "content": format!("You are a voice assistant. Reply in one or two short spoken sentences. \
                        No markdown, no lists, no code blocks — this is read aloud.{lang_rule}")
        })];
        for (u, a) in history {
            messages.push(serde_json::json!({"role":"user","content":u}));
            messages.push(serde_json::json!({"role":"assistant","content":a}));
        }
        messages.push(serde_json::json!({"role":"user","content":user}));

        let body = serde_json::json!({
            "model": self.model, "messages": messages, "stream": true,
            "options": {"num_predict": 120, "temperature": 0.6}
        });
        let t0 = std::time::Instant::now();
        let mut child = Command::new("curl")
            .args(["-sS", "-N", "-X", "POST", &format!("{}/api/chat", self.url),
                   "-H", "Content-Type: application/json", "-d", &body.to_string()])
            .stdout(Stdio::piped()).stderr(Stdio::null())
            .spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
        let mut lines = BufReader::new(stdout).lines();
        let mut full = String::new();
        let mut split = SentenceSplitter::new();
        let mut ttft = 0u128;

        while let Some(line) = lines.next_line().await? {
            if gen.is_stale(my_gen) { let _ = child.kill().await; break; }
            if line.trim().is_empty() { continue; }
            let v: serde_json::Value = match serde_json::from_str(&line) { Ok(v) => v, Err(_) => continue };
            let tok = v.pointer("/message/content").and_then(|c| c.as_str()).unwrap_or("");
            if !tok.is_empty() {
                if ttft == 0 { ttft = t0.elapsed().as_millis(); }
                full.push_str(tok);
                if let Some(s) = split.push(tok) { on_sentence(s); }
            }
            if v.get("done").and_then(|d| d.as_bool()).unwrap_or(false) { break; }
        }
        if !gen.is_stale(my_gen) {
            if let Some(s) = split.flush() { on_sentence(s); }
        }
        let _ = child.kill().await;
        Ok((full, ttft))
    }
}

// ── TTS ─────────────────────────────────────────────────────────────────────

/// The resident `AVSpeechSynthesizer` sidecar.
///
/// `say` costs ~690 ms per utterance — almost all process spawn — and cannot
/// write to a pipe. A process that stays alive answers in 21–30 ms once warm,
/// which is the difference between a demo that feels live and one that does not.
pub struct Tts {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    pub sample_rate: u32,
    pub voice: Option<String>,
    pub rate: f32,
}

impl Tts {
    /// Installed voices, as reported by the sidecar's `--list`.
    pub async fn list_voices(bin: &str) -> anyhow::Result<serde_json::Value> {
        let out = Command::new(bin).arg("--list").stdout(Stdio::piped()).stderr(Stdio::null()).output().await?;
        Ok(serde_json::from_slice(&out.stdout)?)
    }

    pub async fn spawn(bin: &str) -> anyhow::Result<Self> {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut t = Self { _child: child, stdin, stdout, sample_rate: 22050, voice: None, rate: 0.52 };
        // Warm the voice engine: the first utterance in a fresh process costs
        // ~290 ms, every later one ~25 ms. Pay it before the user is listening.
        // It must contain something speakable — a blank one produces no
        // callback at all.
        t.say("ok").await?;
        while t.next_chunk().await?.is_some() {}
        Ok(t)
    }

    pub async fn say(&mut self, text: &str) -> anyhow::Result<()> {
        self.say_as(text, None, self.rate).await
    }

    pub async fn say_as(&mut self, text: &str, voice: Option<&str>, rate: f32) -> anyhow::Result<()> {
        let mut o = serde_json::json!({"text": text, "rate": rate});
        let voice = voice.map(str::to_string).or_else(|| self.voice.clone());
        if let Some(v) = voice { o["voice"] = serde_json::json!(v); }
        let line = format!("{o}\n");
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub async fn cancel(&mut self) -> anyhow::Result<()> {
        self.stdin.write_all(b"{\"cmd\":\"cancel\"}\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// `Some(pcm)` for an audio frame, `None` at end of utterance.
    ///
    /// Bounded: a stage that can block forever is the worst failure mode in a
    /// real-time loop, and this one already did once — the sidecar returns no
    /// callback whatsoever for an unspeakable utterance. Treat a stall as end
    /// of utterance rather than deadlocking the turn.
    pub async fn next_chunk(&mut self) -> anyhow::Result<Option<Vec<f32>>> {
        match tokio::time::timeout(std::time::Duration::from_secs(5), self.read_frame()).await {
            Ok(r) => r,
            Err(_) => Ok(None),
        }
    }

    async fn read_frame(&mut self) -> anyhow::Result<Option<Vec<f32>>> {
        let mut tag = [0u8; 3];
        self.stdout.read_exact(&mut tag).await?;
        let mut len = [0u8; 4];
        self.stdout.read_exact(&mut len).await?;
        let n = u32::from_le_bytes(len) as usize;
        if std::env::var("VD_TRACE").is_ok() {
            eprintln!("  [frame {:?} len={}]", String::from_utf8_lossy(&tag), n);
        }
        if &tag == b"END" { return Ok(None); }
        let mut buf = vec![0u8; n];
        self.stdout.read_exact(&mut buf).await?;
        Ok(Some(buf.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()))
    }
}
