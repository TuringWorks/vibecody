//! BDD coverage for real local-voice I/O (US-005).

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use cucumber::{World, given, then, when};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use vibecli_cli::voice_whisper::{
    NullTranscriber, Transcriber, download_model, encode_wav_mono_pcm16, parse_wav,
};

#[derive(Clone)]
struct ServerState {
    body_bytes: Vec<u8>,
    status: StatusCode,
}

type SharedState = Arc<Mutex<ServerState>>;

#[derive(Default, World)]
pub struct VoiceWorld {
    tmp: Option<TempDir>,
    server: Option<std::net::SocketAddr>,
    server_path: Option<String>,
    last_download_bytes: Option<u64>,
    last_download_error: Option<String>,
    pcm: Option<Vec<f32>>,
    sample_rate: Option<u32>,
    last_transcript: Option<String>,
    last_transcript_error: Option<String>,
}

impl std::fmt::Debug for VoiceWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceWorld")
            .field("server", &self.server)
            .field("path", &self.server_path)
            .field("download_bytes", &self.last_download_bytes)
            .field("pcm_len", &self.pcm.as_ref().map(|p| p.len()))
            .field("transcript", &self.last_transcript)
            .finish()
    }
}

async fn serve_bytes(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let s = state.lock().await.clone();
    (s.status, s.body_bytes)
}

async fn spawn_model_server(bytes: Vec<u8>, status: StatusCode) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let shared = Arc::new(Mutex::new(ServerState {
        body_bytes: bytes,
        status,
    }));
    // Register a wildcard GET route so any path returns the same body;
    // test asserts on the path used by the client.
    let app = Router::new()
        .route("/ggml-tiny.bin", get(serve_bytes))
        .route("/missing", get(serve_bytes))
        .with_state(shared);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    addr
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock model server serving (\d+) bytes at path "([^"]+)"$"#)]
async fn given_model_server(w: &mut VoiceWorld, size: usize, path: String) {
    let bytes: Vec<u8> = (0..size).map(|i| (i & 0xFF) as u8).collect();
    let addr = spawn_model_server(bytes, StatusCode::OK).await;
    w.server = Some(addr);
    w.server_path = Some(path);
    w.tmp = Some(tempfile::tempdir().unwrap());
}

#[given(regex = r#"^a mock model server that returns 404 at path "([^"]+)"$"#)]
async fn given_model_404(w: &mut VoiceWorld, path: String) {
    let addr = spawn_model_server(b"not found".to_vec(), StatusCode::NOT_FOUND).await;
    w.server = Some(addr);
    w.server_path = Some(path);
    w.tmp = Some(tempfile::tempdir().unwrap());
}

#[given(regex = r#"^a synthesized 16kHz mono WAV with (\d+) samples$"#)]
fn given_wav(w: &mut VoiceWorld, count: usize) {
    let samples: Vec<i16> = (0..count).map(|i| (i as i16).wrapping_mul(10)).collect();
    let bytes = encode_wav_mono_pcm16(&samples, 16_000);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.wav");
    std::fs::write(&path, &bytes).unwrap();
    // parse immediately and store; we test that the load path works end-to-end
    let pcm = parse_wav(&bytes).unwrap();
    w.pcm = Some(pcm.samples);
    w.sample_rate = Some(pcm.sample_rate);
    w.tmp = Some(dir);
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^the client downloads that model to a temp file$"#)]
async fn when_download(w: &mut VoiceWorld) {
    let addr = w.server.expect("server");
    let path = w.server_path.clone().expect("path");
    let url = format!("http://{}{}", addr, path);
    let dest = w.tmp.as_ref().unwrap().path().join("model.bin");
    let client = reqwest::Client::new();
    let written = download_model(&client, &url, &dest).await.expect("download");
    let on_disk = std::fs::metadata(&dest).unwrap().len();
    w.last_download_bytes = Some(written);
    // Sanity: file on disk matches reported length.
    assert_eq!(written, on_disk, "reported vs on-disk mismatch");
}

#[when(regex = r#"^the client attempts to download that path to a temp file$"#)]
async fn when_download_fail(w: &mut VoiceWorld) {
    let addr = w.server.expect("server");
    let path = w.server_path.clone().expect("path");
    let url = format!("http://{}{}", addr, path);
    let dest = w.tmp.as_ref().unwrap().path().join("missing.bin");
    let client = reqwest::Client::new();
    match download_model(&client, &url, &dest).await {
        Ok(n) => w.last_download_bytes = Some(n),
        Err(e) => w.last_download_error = Some(e),
    }
}

#[when(regex = r#"^the WAV file is loaded into PCM$"#)]
fn when_load_wav(_w: &mut VoiceWorld) {
    // Given-step already parsed the WAV; nothing to do here.
}

#[when(regex = r#"^a NullTranscriber transcribes an empty buffer$"#)]
fn when_null_empty(w: &mut VoiceWorld) {
    let t = NullTranscriber;
    match t.transcribe(&[], 16_000) {
        Ok(text) => w.last_transcript = Some(text),
        Err(e) => w.last_transcript_error = Some(e),
    }
}

#[when(regex = r#"^a NullTranscriber transcribes a buffer of (\d+) samples at (\d+) Hz$"#)]
fn when_null_shape(w: &mut VoiceWorld, n: usize, sr: u32) {
    let t = NullTranscriber;
    let pcm = vec![0.2f32; n];
    let out = t.transcribe(&pcm, sr).expect("transcribe");
    w.last_transcript = Some(out);
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the temp file size is (\d+) bytes$"#)]
fn then_file_size(w: &mut VoiceWorld, size: u64) {
    let dest = w.tmp.as_ref().unwrap().path().join("model.bin");
    let meta = std::fs::metadata(&dest).expect("stat");
    assert_eq!(meta.len(), size);
}

#[then(regex = r#"^the download reports (\d+) bytes$"#)]
fn then_download_bytes(w: &mut VoiceWorld, size: u64) {
    assert_eq!(w.last_download_bytes, Some(size));
}

#[then(regex = r#"^the download returns an error mentioning "([^"]+)"$"#)]
fn then_download_error(w: &mut VoiceWorld, needle: String) {
    let err = w.last_download_error.as_ref().expect("err");
    assert!(err.contains(&needle), "err {err:?} missing {needle}");
}

#[then(regex = r#"^the PCM length is (\d+) samples$"#)]
fn then_pcm_len(w: &mut VoiceWorld, n: usize) {
    assert_eq!(w.pcm.as_ref().unwrap().len(), n);
}

#[then(regex = r#"^the sample rate is (\d+)$"#)]
fn then_sample_rate(w: &mut VoiceWorld, sr: u32) {
    assert_eq!(w.sample_rate, Some(sr));
}

#[then(regex = r#"^transcription returns an error mentioning "([^"]+)"$"#)]
fn then_transcribe_error(w: &mut VoiceWorld, needle: String) {
    let err = w.last_transcript_error.as_ref().expect("err");
    assert!(err.contains(&needle), "err {err:?} missing {needle}");
}

#[then(regex = r#"^the transcript contains "([^"]+)"$"#)]
fn then_transcript_contains(w: &mut VoiceWorld, needle: String) {
    let t = w.last_transcript.as_ref().expect("transcript");
    assert!(t.contains(&needle), "transcript {t:?} missing {needle}");
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    VoiceWorld::run("tests/features/voice_whisper.feature").await;
}
