//! voice-duplex — a full-duplex voice loop against a local stack.
//!
//! The mic stays open the whole time, including while the assistant is
//! speaking. That is the whole point, and it is only possible because the
//! webview's echo canceller removes our own output from the capture stream
//! (measured: ≥40 dB, and it covers WebAudio-rendered output — see the
//! webview-probe `aec` arm). Without AEC an open mic makes the agent interrupt
//! itself on its own voice.
//!
//! Transport is a WebSocket carrying raw PCM rather than WebRTC. On loopback
//! WebRTC's loss concealment buys nothing, and AEC lives in the *capture*
//! pipeline so it applies either way. WebRTC remains the answer for the remote
//! case (phone/watch → daemon over Tailscale), which is a different problem.
//!
//!   mic → AEC → 16 kHz PCM → ws → VAD → whisper → ollama → AVSpeech → ws → speakers
//!                                  ↑                                        |
//!                                  └──────── barge-in cancels ──────────────┘

mod stages;
mod vad;

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use stages::{Asr, Generation, Llm, StreamAsr, Tts};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;
use vad::{Turn, Vad, FRAME};
use wry::WebViewBuilder;

const PAGE: &str = include_str!("../web/index.html");
const MIC_RATE: u32 = 16_000;
/// Keep this much audio from *before* the VAD fired, so the first word of a
/// turn is not clipped off the front of the utterance.
const PREROLL_FRAMES: usize = 15; // 300 ms

struct Cfg {
    language: String,
    asr_engine: String,
    asr_bin: String,
    whisper_bin: String,
    whisper_model: String,
    whisper_server_bin: String,
    whisper_server_model: String,
    whisper_port: u16,
    tts_bin: String,
    ollama: String,
    model: String,
    http_port: u16,
    ws_port: u16,
}

fn cfg() -> Cfg {
    let a: Vec<String> = std::env::args().collect();
    let g = |k: &str, d: &str| {
        a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).cloned().unwrap_or_else(|| d.into())
    };
    let home = std::env::var("HOME").unwrap_or_default();
    Cfg {
        // `apple` streams and finalises in ~33 ms; `whisper` is the portable
        // fallback and costs ~978 ms because it cannot start until the turn ends.
        // "en" = fast streaming path; "auto" = Whisper language ID over 99 languages.
        language: g("--language", "en"),
        asr_engine: g("--asr", if cfg!(target_os = "macos") { "apple" } else { "whisper" }),
        asr_bin: g("--asr-bin", "./sidecar/asr"),
        whisper_bin: g("--whisper", "/opt/homebrew/bin/whisper-cli"),
        whisper_model: g("--whisper-model", &format!("{home}/.vibecli/models/ggml-base.bin")),
        // `small` is the quality floor for non-Latin scripts: `base` renders
        // Devanagari as Arabic script, `small` and `medium` produce identical
        // correct text, and `small` is 3x faster than `medium`.
        whisper_server_bin: g("--whisper-server", "/opt/homebrew/bin/whisper-server"),
        whisper_server_model: g("--whisper-server-model", &format!("{home}/.vibecli/models/ggml-small.bin")),
        whisper_port: g("--whisper-port", "8923").parse().unwrap_or(8923),
        tts_bin: g("--tts", "./sidecar/tts"),
        ollama: g("--ollama", "http://127.0.0.1:11434"),
        model: g("--model", "granite4.1:3b"),
        http_port: g("--http-port", "8921").parse().unwrap_or(8921),
        ws_port: g("--ws-port", "8922").parse().unwrap_or(8922),
    }
}

type Out = mpsc::UnboundedSender<Message>;

/// Bring up a resident `whisper-server`, if the binary and model are present.
///
/// Returns its base URL, or `None` — in which case transcription falls back to
/// spawning `whisper-cli` per utterance, which works but costs ~1 s more.
async fn spawn_whisper_server(cfg: &Cfg) -> Option<String> {
    if !std::path::Path::new(&cfg.whisper_server_bin).exists()
        || !std::path::Path::new(&cfg.whisper_server_model).exists() { return None; }
    let url = format!("http://127.0.0.1:{}", cfg.whisper_port);
    // Reuse one that is already up rather than fighting it for the port.
    if tokio::net::TcpStream::connect(("127.0.0.1", cfg.whisper_port)).await.is_ok() {
        eprintln!("voice-duplex: reusing whisper-server on {url}");
        return Some(url);
    }
    let spawned = tokio::process::Command::new(&cfg.whisper_server_bin)
        .args(["-m", &cfg.whisper_server_model, "--port", &cfg.whisper_port.to_string(), "-t", "4"])
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .kill_on_drop(false)
        .spawn();
    if spawned.is_err() { return None; }
    // Poll to a deadline; a cold model load is seconds, not milliseconds.
    for _ in 0..60 {
        if tokio::net::TcpStream::connect(("127.0.0.1", cfg.whisper_port)).await.is_ok() {
            eprintln!("voice-duplex: whisper-server ready on {url}");
            return Some(url);
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    eprintln!("voice-duplex: whisper-server did not come up; using whisper-cli per utterance");
    None
}

/// Best installed voice for a language: neural tiers first, then any match.
fn best_voice_for(voices: &serde_json::Value, lang: &str) -> Option<String> {
    let list = voices.get("voices")?.as_array()?;
    let rank = |q: &str| match q { "premium" => 0, "enhanced" => 1, _ => 2 };
    list.iter()
        .filter(|v| v.get("novelty").and_then(|n| n.as_bool()) != Some(true))
        .filter(|v| v.get("lang").and_then(|l| l.as_str()).is_some_and(|l| l.starts_with(lang)))
        .min_by_key(|v| rank(v.get("quality").and_then(|q| q.as_str()).unwrap_or("default")))
        .and_then(|v| v.get("id").and_then(|i| i.as_str()).map(str::to_string))
}

fn send_json(tx: &Out, v: serde_json::Value) {
    let _ = tx.send(Message::Text(v.to_string()));
}

/// One user turn: transcribe, answer, speak — checking `gen` at every stage so
/// a barge-in abandons the turn instead of talking over the user.
#[allow(clippy::too_many_arguments)]
async fn run_turn(
    text: String, asr_ms: u128, lang: Option<String>, t_end_of_speech: std::time::Instant,
    tx: Out, gen: Generation, my_gen: u64,
    llm: Arc<Llm>, tts: Arc<Mutex<Tts>>,
    history: Arc<Mutex<Vec<(String, String)>>>,
    unanswered: Arc<Mutex<Option<String>>>,
) {
    // A turn superseded before it spoke is not an interruption — it is a user
    // still forming their thought. "plus fifty one." <pause> "minus fifty
    // four." is one instruction in two breaths, and dropping the first half
    // silently answers a different question. Carry it forward.
    //
    // Only before any audio has gone out; once the assistant is speaking, the
    // user talking over it *is* an interruption.
    if gen.is_stale(my_gen) {
        let mut c = unanswered.lock().await;
        *c = Some(match c.take() {
            Some(prev) => format!("{prev} {text}"),
            None => text.clone(),
        });
        send_json(&tx, serde_json::json!({"type": "carried", "text": text}));
        return;
    }
    // Anything said while we were still thinking belongs to this turn.
    let text = match unanswered.lock().await.take() {
        Some(prev) => format!("{prev} {text}"),
        None => text,
    };
    let hist = history.lock().await.clone();
    let (tx_s, mut rx_s) = mpsc::unbounded_channel::<String>();

    // Speak sentences as they complete rather than after the model finishes —
    // otherwise first-audio inherits the whole generation time.
    let speak = {
        let tx = tx.clone();
        let tts = Arc::clone(&tts);
        let gen = gen.clone();
        tokio::spawn(async move {
            let mut first_audio_ms: Option<u128> = None;
            while let Some(sentence) = rx_s.recv().await {
                if gen.is_stale(my_gen) { break; }
                if !sentence.chars().any(|c| c.is_alphanumeric()) { continue; }
                let mut t = tts.lock().await;
                if t.say(&sentence).await.is_err() { break; }
                send_json(&tx, serde_json::json!({"type":"speaking","text":sentence}));
                loop {
                    if gen.is_stale(my_gen) { let _ = t.cancel().await; break; }
                    match t.next_chunk().await {
                        Ok(Some(chunk)) => {
                            if first_audio_ms.is_none() {
                                first_audio_ms = Some(t_end_of_speech.elapsed().as_millis());
                                send_json(&tx, serde_json::json!({
                                    "type":"latency","first_audio_ms":first_audio_ms}));
                            }
                            let mut bytes = Vec::with_capacity(chunk.len() * 4);
                            for s in &chunk { bytes.extend_from_slice(&s.to_le_bytes()); }
                            let _ = tx.send(Message::Binary(bytes));
                        }
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }
            }
        })
    };

    let (full, ttft) = match llm.stream(&hist, &text, lang.as_deref(), gen.clone(), my_gen, |s| { let _ = tx_s.send(s); }).await {
        Ok(v) => v,
        Err(e) => { send_json(&tx, serde_json::json!({"type":"error","message":format!("llm: {e}")})); (String::new(), 0) }
    };
    drop(tx_s);
    let _ = speak.await;

    if !gen.is_stale(my_gen) {
        send_json(&tx, serde_json::json!({
            "type":"reply","text":full,"asr_ms":asr_ms,"llm_ttft_ms":ttft,
            "total_ms":t_end_of_speech.elapsed().as_millis()}));
        let mut h = history.lock().await;
        h.push((text, full));
        if h.len() > 6 { h.remove(0); }
        send_json(&tx, serde_json::json!({"type":"state","state":"listening"}));
    }
}

async fn serve_ws(cfg: Arc<Cfg>) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", cfg.ws_port)).await?;
    eprintln!("voice-duplex: ws on 127.0.0.1:{}", cfg.ws_port);

    let server = spawn_whisper_server(&cfg).await;
    let asr = Arc::new(Asr {
        bin: cfg.whisper_bin.clone(),
        model: cfg.whisper_model.clone(),
        server,
    });
    let llm = Arc::new(Llm { url: cfg.ollama.clone(), model: cfg.model.clone() });

    while let Ok((stream, _)) = listener.accept().await {
        let (asr, llm, cfg) = (Arc::clone(&asr), Arc::clone(&llm), Arc::clone(&cfg));
        tokio::spawn(async move {
            let ws = match tokio_tungstenite::accept_async(stream).await { Ok(w) => w, Err(_) => return };
            let (mut sink, mut source) = ws.split();
            let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
            tokio::spawn(async move { while let Some(m) = rx.recv().await { if sink.send(m).await.is_err() { break; } } });

            let tts = match Tts::spawn(&cfg.tts_bin).await {
                Ok(t) => Arc::new(Mutex::new(t)),
                Err(e) => { send_json(&tx, serde_json::json!({"type":"error","message":format!("tts sidecar: {e}")})); return; }
            };
            let voices = Tts::list_voices(&cfg.tts_bin).await.unwrap_or(serde_json::json!({}));
            send_json(&tx, serde_json::json!({"type":"voices","list":voices}));
            let tts_rate = tts.lock().await.sample_rate;
            send_json(&tx, serde_json::json!({"type":"ready","model":cfg.model,"tts_rate":tts_rate}));
            // Both engines warm before we invite anyone to speak.
            send_json(&tx, serde_json::json!({"type":"state","state":"thinking"}));
            match llm.warm().await {
                Ok(ms) => send_json(&tx, serde_json::json!({"type":"warm","llm_ms":ms})),
                Err(e) => send_json(&tx, serde_json::json!({"type":"error","message":format!("llm warm: {e}")})),
            }
            send_json(&tx, serde_json::json!({"type":"state","state":"listening"}));

            let gen = Generation::default();
            let history = Arc::new(Mutex::new(Vec::new()));
            // "auto" until the first turn establishes it; then pinned, because
            // a known language is both faster and more accurate than `auto`.
            let session_lang: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            // Words from a turn the user superseded before it could answer.
            let unanswered: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            // Language mode: "en" keeps the fast streaming path; anything else
            // routes through Whisper, which is the only multilingual engine here.
            let lang_mode = Arc::new(Mutex::new(cfg.language.clone()));
            let user_pinned_voice = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let stream_asr = if cfg.asr_engine == "apple" {
                let tx_p = tx.clone();
                match StreamAsr::spawn(&cfg.asr_bin, move |p| {
                    send_json(&tx_p, serde_json::json!({"type":"partial","text":p}));
                }).await {
                    Ok(a) => Some(Arc::new(a)),
                    Err(e) => { send_json(&tx, serde_json::json!({"type":"error",
                        "message":format!("asr sidecar: {e} — falling back to whisper")})); None }
                }
            } else { None };
            send_json(&tx, serde_json::json!({"type":"asr","engine":
                if stream_asr.is_some() { "apple-streaming" } else { "whisper-batch" }}));
            let mut v = Vad::default();
            let mut pending: Vec<i16> = Vec::new();
            let mut preroll: std::collections::VecDeque<Vec<i16>> = std::collections::VecDeque::new();
            let mut capturing = false;
            let mut carry: Vec<i16> = Vec::new();

            while let Some(Ok(msg)) = source.next().await {
                if let Message::Text(t) = &msg {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
                        match v.get("type").and_then(|x| x.as_str()) {
                            Some("set_language") => {
                                let l = v.get("lang").and_then(|x| x.as_str()).unwrap_or("auto").to_string();
                                *session_lang.lock().await = None;
                                *lang_mode.lock().await = l.clone();
                                send_json(&tx, serde_json::json!({"type":"language","mode":l}));
                            }
                            Some("set_voice") => {
                                if v.get("sample").is_some() {
                                    user_pinned_voice.store(true, std::sync::atomic::Ordering::SeqCst);
                                }
                                let id = v.get("id").and_then(|x| x.as_str()).map(str::to_string);
                                let mut g = tts.lock().await;
                                g.voice = id.clone();
                                // Audition immediately: picking a voice you cannot
                                // hear is not choosing, it is guessing.
                                if let Some(sample) = v.get("sample").and_then(|x| x.as_str()) {
                                    gen.bump();
                                    let rate_now = g.rate;
                                    let _ = g.say_as(sample, id.as_deref(), rate_now).await;
                                    while let Ok(Some(chunk)) = g.next_chunk().await {
                                        let mut b = Vec::with_capacity(chunk.len() * 4);
                                        for s in &chunk { b.extend_from_slice(&s.to_le_bytes()); }
                                        let _ = tx.send(Message::Binary(b));
                                    }
                                }
                            }
                            Some("set_rate") => {
                                if let Some(r) = v.get("rate").and_then(|x| x.as_f64()) {
                                    tts.lock().await.rate = r as f32;
                                }
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
                let Message::Binary(buf) = msg else { continue };
                carry.extend(buf.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])));
                while carry.len() >= FRAME {
                    let frame: Vec<i16> = carry.drain(..FRAME).collect();
                    match v.push(&frame) {
                        Turn::SpeechStart => {
                            // Barge-in: whatever is speaking belongs to a turn
                            // the user just superseded.
                            let g = gen.bump();
                            let _ = tts.lock().await.cancel().await;
                            if let Some(a) = &stream_asr { let _ = a.reset().await; }
                            send_json(&tx, serde_json::json!({"type":"flush"}));
                            send_json(&tx, serde_json::json!({"type":"state","state":"hearing","gen":g}));
                            capturing = true;
                            pending = preroll.iter().flatten().copied().collect();
                            pending.extend_from_slice(&frame);
                        }
                        Turn::Speech => {
                            if capturing {
                                pending.extend_from_slice(&frame);
                                // Feed as we go: this is what makes end-of-speech
                                // a finalise rather than a transcribe.
                                if let Some(a) = &stream_asr { let _ = a.feed(&frame).await; }
                            }
                        }
                        Turn::SpeechEnd => {
                            capturing = false;
                            let audio = std::mem::take(&mut pending);
                            let my_gen = gen.current();
                            if audio.len() <= MIC_RATE as usize / 5 {
                                if let Some(a) = &stream_asr { let _ = a.reset().await; }
                                send_json(&tx, serde_json::json!({"type":"state","state":"listening"}));
                                preroll.push_back(frame);
                                if preroll.len() > PREROLL_FRAMES { preroll.pop_front(); }
                                continue;
                            }
                            send_json(&tx, serde_json::json!({"type":"state","state":"thinking"}));
                            let t_eos = std::time::Instant::now();
                            let mode = lang_mode.lock().await.clone();
                            // English keeps the fast streaming recogniser. Everything
                            // else goes through Whisper: on this machine only 4 of
                            // SFSpeechRecognizer's 63 locales have on-device assets
                            // and all four are English, so the streaming path simply
                            // cannot hear another language without sending audio away.
                            let use_stream = mode == "en" && stream_asr.is_some();
                            let (text, asr_ms, detected) = if use_stream {
                                let a = stream_asr.as_ref().unwrap();
                                for f in audio.chunks(FRAME) { let _ = a.feed(f).await; }
                                let _ = a.end_of_utterance().await;
                                let t = a.wait_final().await.map(|(t, _)| t).unwrap_or_default();
                                (t, t_eos.elapsed().as_millis(), Some("en".to_string()))
                            } else {
                                // auto => detect every turn (see transcribe_lang).
                                let want = if mode == "auto" { None } else { Some(mode.clone()) };
                                match asr.transcribe_lang(&audio, MIC_RATE, want.as_deref()).await {
                                    Ok((t, d)) => {
                                        let lang = d.or_else(|| if mode == "auto" { None } else { Some(mode.clone()) });
                                        (t, t_eos.elapsed().as_millis(), lang)
                                    }
                                    Err(_) => (String::new(), t_eos.elapsed().as_millis(), None),
                                }
                            };
                            if let Some(d) = &detected {
                                let mut sl = session_lang.lock().await;
                                if sl.as_deref() != Some(d.as_str()) {
                                    *sl = Some(d.clone());
                                    send_json(&tx, serde_json::json!({"type":"detected","lang":d}));
                                    // The voice follows the language unless the user
                                    // has chosen one — answering Hindi in an English
                                    // voice is worse than answering slowly.
                                    if !user_pinned_voice.load(std::sync::atomic::Ordering::SeqCst) {
                                        if let Some(id) = best_voice_for(&voices, d) {
                                            tts.lock().await.voice = Some(id.clone());
                                            send_json(&tx, serde_json::json!({"type":"voice_auto","id":id,"lang":d}));
                                        }
                                    }
                                }
                            }
                            if text.trim().is_empty() {
                                send_json(&tx, serde_json::json!({"type":"state","state":"listening"}));
                            } else {
                                send_json(&tx, serde_json::json!({"type":"transcript","text":text,"asr_ms":asr_ms,"lang":detected}));
                                tokio::spawn(run_turn(
                                    text, asr_ms, detected.clone(), t_eos, tx.clone(), gen.clone(), my_gen,
                                    Arc::clone(&llm), Arc::clone(&tts), Arc::clone(&history),
                                    Arc::clone(&unanswered)));
                            }
                        }
                        Turn::Silence => {}
                    }
                    preroll.push_back(frame);
                    if preroll.len() > PREROLL_FRAMES { preroll.pop_front(); }
                }
            }
        });
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_linux_media_fix(webview: &wry::WebView) {
    // `is::<T>()` is a glib trait method, not a webkit2gtk one. Without this
    // import the function does not compile — which nothing here noticed,
    // because no workflow builds this tool on Linux.
    use webkit2gtk::glib::ObjectExt;
    use webkit2gtk::{PermissionRequestExt, SettingsExt, UserMediaPermissionRequest, WebViewExt};
    use wry::WebViewExtUnix;
    let wv = webview.webview();
    // Three switches, all off by default: capture, WebAudio, and WebRTC. The
    // third is why `RTCPeerConnection` is undefined on this engine rather than
    // merely blocked — measured by `tools/webview-probe` on a CI runner.
    if let Some(s) = WebViewExt::settings(&wv) {
        s.set_enable_media_stream(true);
        s.set_enable_webaudio(true);
        s.set_enable_webrtc(true);
    }
    wv.connect_permission_request(|_, req| {
        if req.is::<UserMediaPermissionRequest>() { req.allow(); return true; }
        false
    });
}
#[cfg(not(target_os = "linux"))]
fn apply_linux_media_fix(_w: &wry::WebView) {}

/// Exercise ASR → LLM → TTS with no microphone and no window, reporting what
/// each stage actually cost. Proves the pipeline before a human talks to it,
/// and gives the latency budget a number instead of a claim.
async fn selftest(cfg: Arc<Cfg>) -> anyhow::Result<()> {
    println!("voice-duplex selftest");
    let asr = Asr { bin: cfg.whisper_bin.clone(), model: cfg.whisper_model.clone(),
                    server: spawn_whisper_server(&cfg).await };
    let llm = Llm { url: cfg.ollama.clone(), model: cfg.model.clone() };
    println!("  warming  : {:>5} ms (ollama model load)", llm.warm().await?);

    // Synthesise a question with the sidecar, then feed it back through ASR —
    // a closed loop that needs no audio hardware at all.
    let mut tts = Tts::spawn(&cfg.tts_bin).await?;
    let phrase = "What is the capital of France?";
    tts.say(phrase).await?;
    let mut spoken: Vec<f32> = Vec::new();
    while let Some(c) = tts.next_chunk().await? { spoken.extend(c); }
    println!("  tts      : {} samples @ {} Hz", spoken.len(), tts.sample_rate);

    // 22.05 kHz -> 16 kHz, nearest-neighbour. Good enough to prove wiring;
    // the live path never does this because capture is already 16 kHz.
    let ratio = tts.sample_rate as f32 / MIC_RATE as f32;
    let n = (spoken.len() as f32 / ratio) as usize;
    let pcm: Vec<i16> = (0..n)
        .map(|i| {
            let s = spoken[((i as f32) * ratio) as usize].clamp(-1.0, 1.0);
            (s * 32767.0) as i16
        })
        .collect();

    // Both ASR paths, on identical audio, so the comparison is like-for-like.
    let t = std::time::Instant::now();
    let batch = asr.transcribe(&pcm, MIC_RATE).await?;
    let batch_ms = t.elapsed().as_millis();
    println!("  asr batch: {batch_ms:>5} ms -> {batch:?}   (whisper, whole utterance)");

    let (heard, asr_ms) = if std::path::Path::new(&cfg.asr_bin).exists() {
        let sa = StreamAsr::spawn(&cfg.asr_bin, |_p| {}).await?;
        // Feed at real-time pace: recognition must overlap the speech, which is
        // the entire point. Feeding instantly would flatter the number.
        let mut fed = 0usize;
        for f in pcm.chunks(FRAME) {
            sa.feed(f).await?;
            fed += f.len();
            tokio::time::sleep(std::time::Duration::from_millis(18)).await;
        }
        let _ = fed;
        let t = std::time::Instant::now();
        sa.end_of_utterance().await?;
        match sa.wait_final().await {
            Some((text, _)) => { let ms = t.elapsed().as_millis();
                println!("  asr strm : {ms:>5} ms -> {text:?}   (apple, finalise only)"); (text, ms) }
            None => { println!("  asr strm : no final returned; using batch"); (batch.clone(), batch_ms) }
        }
    } else { (batch.clone(), batch_ms) };
    if heard.is_empty() { anyhow::bail!("asr returned nothing"); }

    let gen = Generation::default();
    let g = gen.current();
    let mut first_sentence_ms = None;
    let t = std::time::Instant::now();
    let (full, ttft) = llm.stream(&[], &heard, None, gen.clone(), g, |_s| {
        if first_sentence_ms.is_none() { first_sentence_ms = Some(t.elapsed().as_millis()); }
    }).await?;
    println!("  llm ttft : {ttft:>5} ms");
    println!("  1st sent : {:>5} ms", first_sentence_ms.unwrap_or(0));
    println!("  reply    : {:?}", full.trim());

    let t = std::time::Instant::now();
    tts.say("The capital of France is Paris.").await?;
    let mut first_audio_ms = None;
    while let Some(_c) = tts.next_chunk().await? {
        if first_audio_ms.is_none() { first_audio_ms = Some(t.elapsed().as_millis()); }
    }
    println!("  tts warm : {:>5} ms to first audio", first_audio_ms.unwrap_or(0));

    let tts_ms = first_audio_ms.unwrap_or(0);
    let budget = asr_ms + ttft + tts_ms;
    let batch_budget = batch_ms + ttft + tts_ms;
    println!("\n  end-of-speech -> first audio: ~{budget} ms  (asr {asr_ms} + llm {ttft} + tts {tts_ms})");
    println!("  same loop on batch whisper  : ~{batch_budget} ms  ({}x slower)",
             if budget > 0 { batch_budget / budget.max(1) } else { 0 });
    println!("  + {} ms of VAD hangover before any of it starts.", 600);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cfg = Arc::new(cfg());
    for (what, path) in [("whisper", &cfg.whisper_bin), ("whisper model", &cfg.whisper_model), ("tts sidecar", &cfg.tts_bin)] {
        if !std::path::Path::new(path).exists() {
            eprintln!("voice-duplex: {what} not found at {path}");
            eprintln!("  build the sidecar with: swiftc -O -o sidecar/tts sidecar/tts.swift");
            std::process::exit(2);
        }
    }

    let rt = tokio::runtime::Runtime::new()?;
    if std::env::args().any(|a| a == "--selftest") {
        return rt.block_on(selftest(cfg));
    }
    { let cfg = Arc::clone(&cfg); rt.spawn(async move { let _ = serve_ws(cfg).await; }); }

    let ws_port = cfg.ws_port;
    let page = PAGE.replace("__WS_PORT__", &ws_port.to_string());
    let server = tiny_http::Server::http(("127.0.0.1", cfg.http_port)).map_err(|e| anyhow::anyhow!("{e}"))?;
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let ct = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
            let _ = req.respond(tiny_http::Response::from_string(page.clone()).with_header(ct));
        }
    });

    let event_loop = EventLoopBuilder::new().build();
    let window = WindowBuilder::new()
        .with_title("VibeCody — full-duplex voice")
        .with_inner_size(tao::dpi::LogicalSize::new(820.0, 620.0))
        .build(&event_loop)?;
    // Built empty, *then* navigated: WebKitGTK settles which globals a page
    // gets when its JS context is created, so flipping the switches after
    // `with_url` has already loaded it leaves the page without them.
    let webview = WebViewBuilder::new().build(&window)?;
    apply_linux_media_fix(&webview);
    webview.load_url(&format!("http://127.0.0.1:{}/", cfg.http_port))?;

    eprintln!("voice-duplex: model={} — talk when it says Listening", cfg.model);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
            std::process::exit(0);
        }
    });
}
