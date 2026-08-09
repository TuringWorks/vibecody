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
//! ])
//! ```
//!
//! The frontend counterpart is `tauriTranscriber()` in
//! `packages/vibe-ui-shared/src/voice/transcribers.ts`, which calls
//! `transcribe_audio` by that exact name.

pub mod voice;

pub use voice::{transcribe_audio, voice_status};
