//! Voice-input Tauri commands shared by the VibeCody desktop shells.
//!
//! Every shell needs the same thing: take audio the webview recorded and turn
//! it into text. The transcription itself belongs to the daemon
//! (`POST /voice/transcribe`), which owns the Groq key and the local whisper
//! model — so this crate is a bridge, not an implementation. Registering it in
//! a shell is two lines; re-implementing it per shell is how VibeCoder ended up
//! with its own Groq call that ignores the local model entirely.
//!
//! # Registering
//!
//! ```ignore
//! .invoke_handler(tauri::generate_handler![
//!     vibe_desktop_voice::transcribe_audio,
//!     vibe_desktop_voice::voice_status,
//!     vibe_desktop_voice::daemon_token_effective,
//! ])
//! ```
//!
//! The frontend counterparts are `tauriTranscriber()` and `useVoiceDuplex()` in
//! `packages/vibe-ui-shared/src/voice/`, which call `transcribe_audio` and
//! `daemon_token_effective` by those exact names.
//!
//! `daemon_token_effective` exists because a WebSocket cannot set an
//! Authorization header, so `/ws/voice/duplex` takes `?token=` — and the token
//! a *local* daemon uses is minted fresh on every start and stored nowhere the
//! frontend can see, so asking the settings store returns null and the socket
//! 401s against the user's own daemon.

pub mod voice;

pub use voice::{daemon_token_effective, transcribe_audio, voice_status};
