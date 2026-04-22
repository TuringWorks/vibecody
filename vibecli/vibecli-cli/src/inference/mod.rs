//! Pluggable local-inference backends exposed behind a single Ollama-compatible
//! HTTP surface.
//!
//! This module is the runtime counterpart to the static scaffolding in
//! `inference_server.rs` (CLI command builders, Docker / K8s manifests). The
//! types here wire into the Axum daemon: one [`Backend`] trait, one [`Router`]
//! that picks an implementation per request, and two implementations —
//! [`mistralrs::MistralRsBackend`] (in-process, TurboQuant-aware) and
//! [`ollama::OllamaProxyBackend`] (reverse-proxy to `ollama serve`).
//!
//! ## Architecture
//!
//! ```text
//!   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
//!   │ VibeUI       │    │ VibeCLI REPL │    │ curl / SDK   │
//!   │ (Tauri)      │    │ (TUI)        │    │ (any client) │
//!   └──────┬───────┘    └──────┬───────┘    └──────┬───────┘
//!          │ HTTPS+bearer      │                   │
//!          └───────────────────┴───────────────────┘
//!                              │
//!                              ▼
//!                ┌───────────────────────────────┐
//!                │  Axum daemon  (vibecli :7878) │
//!                │  /api/chat  /api/generate     │
//!                │  /api/tags  /api/pull /show   │
//!                └──────────────┬────────────────┘
//!                               │
//!                               ▼
//!                ┌───────────────────────────────┐
//!                │   inference::Router           │
//!                │   1. header X-VibeCLI-Backend │
//!                │   2. body.backend             │
//!                │   3. per-model pin (env)      │
//!                │   4. daemon default           │
//!                └──────┬─────────────────┬──────┘
//!                       │                 │
//!                       ▼                 ▼
//!         ┌────────────────────┐   ┌────────────────────┐
//!         │ MistralRsBackend   │   │ OllamaProxyBackend │
//!         │ (in-process,       │   │ (reverse-proxy to  │
//!         │  TurboQuant KV)    │   │  ollama serve)     │
//!         └─────────┬──────────┘   └─────────┬──────────┘
//!                   │                        │
//!                   ▼                        ▼
//!         ┌────────────────────┐   ┌────────────────────┐
//!         │ vibe-infer         │   │ http://localhost:  │
//!         │   ::Mistral        │   │   11434/api/*      │
//!         │   Generator        │   │ (separate process) │
//!         │ → mistral.rs ─→    │   │                    │
//!         │   CUDA/Metal       │   │                    │
//!         │   TurboQuant       │   │                    │
//!         │   kernels          │   │                    │
//!         └────────────────────┘   └────────────────────┘
//! ```
//!
//! Clients never see the split: every request arrives at `/api/*` and the
//! router resolves `model + request` to a backend via this precedence:
//!
//! 1. **Request header** — `X-VibeCLI-Backend: mistralrs|ollama`
//! 2. **Request body**   — `"backend": "mistralrs"` field
//! 3. **Per-model pin**  — `VIBECLI_BACKEND_PINS=Qwen/*=mistralrs,...`
//! 4. **Daemon default** — `VIBECLI_DEFAULT_BACKEND` env (fallback `ollama`)
//!
//! See `AGENTS.md` → "Explaining Changes" for the diagram-first guideline
//! that produced the diagram above.

pub mod backend;
pub mod mistralrs;
pub mod ollama;
pub mod router;

#[allow(unused_imports)]
pub use backend::{
    Backend, BackendKind, ChatChunk, ChatMessage, ChatRequest, GenerateChunk, GenerateRequest,
    ModelInfo, PullProgress, PullRequest,
};
#[allow(unused_imports)]
pub use router::Router;
