//! Subprocess dispatch transport — M7 of the async-agents subsystem.
//!
//! Spawns a child `vibecli worker <job-id>` process, inherits one half
//! of a Unix socketpair as fd 3, performs a Noise_NNpsk0 handshake with
//! a per-job 32-byte PSK, then exchanges length-prefixed JSON frames
//! encrypted under the resulting transport keys.
//!
//! The parent is the Noise initiator and the child is the responder.
//! Both sides share the same PSK but have no static keys (NN pattern),
//! so a third party cannot inject frames even if they guess the socket —
//! they would need the PSK, which is delivered to the child via
//! `VIBECLI_WORKER_PSK` in the child's private environment.
//!
//! Platform: Unix only. Windows dispatch stays in-process (T1) for M7.

#![cfg(unix)]

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use snow::{params::NoiseParams, Builder, HandshakeState, TransportState};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, Mutex};

use crate::job_manager::AgentEventPayload;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Noise pattern used on the wire.
const NOISE_PATTERN: &str = "Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s";

/// Max single message size (plaintext + Noise tag). Noise caps at 65535;
/// we keep comfortably below to leave room for framing.
const MAX_FRAME_PAYLOAD: usize = 63 * 1024;

/// Env var names the child reads on startup.
pub const ENV_WORKER_FD: &str = "VIBECLI_WORKER_FD";
pub const ENV_WORKER_PSK: &str = "VIBECLI_WORKER_PSK";
pub const ENV_WORKER_JOB_ID: &str = "VIBECLI_WORKER_JOB_ID";

// ── Wire protocol ─────────────────────────────────────────────────────────────

/// Frames exchanged between parent and child over the Noise channel.
/// Parent → Child: `Run`, `Cancel`. Child → Parent: `Ready`, `Event`,
/// `Complete`, `Error`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum DispatchFrame {
    /// Parent → Child: run this task. Sent once, immediately after handshake.
    Run {
        job_id: String,
        task: String,
        provider: String,
        approval: String,
        workspace_root: String,
        #[serde(default = "default_max_turns")]
        max_turns: usize,
    },
    /// Parent → Child: cancel the running task.
    Cancel { reason: String },
    /// Child → Parent: posted once after the child has set up its agent loop
    /// and is ready to begin work. Useful for deterministic tests.
    Ready,
    /// Child → Parent: agent event (chunk, step, …). Mirrors the in-process
    /// `AgentEventPayload` so the JobManager SSE fan-out is transparent.
    Event(AgentEventPayload),
    /// Child → Parent: terminal success.
    Complete { summary: String },
    /// Child → Parent: terminal failure.
    Error { message: String },
}

fn default_max_turns() -> usize {
    25
}

// ── Noise helpers ─────────────────────────────────────────────────────────────

fn noise_params() -> NoiseParams {
    NOISE_PATTERN
        .parse()
        .expect("hard-coded Noise pattern parses")
}

/// Build an initiator handshake state for the given 32-byte PSK.
pub fn build_initiator(psk: &[u8; 32]) -> Result<HandshakeState> {
    Builder::new(noise_params())
        .psk(0, psk)
        .build_initiator()
        .map_err(|e| anyhow!("noise initiator: {e}"))
}

/// Build a responder handshake state for the given 32-byte PSK.
pub fn build_responder(psk: &[u8; 32]) -> Result<HandshakeState> {
    Builder::new(noise_params())
        .psk(0, psk)
        .build_responder()
        .map_err(|e| anyhow!("noise responder: {e}"))
}

/// Generate a fresh 32-byte PSK for a new job.
pub fn generate_psk() -> [u8; 32] {
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

// ── Frame I/O ─────────────────────────────────────────────────────────────────

async fn read_exact<S: AsyncReadExt + Unpin>(stream: &mut S, buf: &mut [u8]) -> Result<()> {
    stream
        .read_exact(buf)
        .await
        .context("dispatch read_exact")?;
    Ok(())
}

async fn write_all<S: AsyncWriteExt + Unpin>(stream: &mut S, buf: &[u8]) -> Result<()> {
    stream.write_all(buf).await.context("dispatch write_all")?;
    Ok(())
}

async fn read_frame<S: AsyncReadExt + Unpin>(stream: &mut S) -> Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    read_exact(stream, &mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > MAX_FRAME_PAYLOAD + 16 {
        bail!("dispatch: invalid frame length {len}");
    }
    let mut buf = vec![0u8; len];
    read_exact(stream, &mut buf).await?;
    Ok(buf)
}

async fn write_frame<S: AsyncWriteExt + Unpin>(stream: &mut S, payload: &[u8]) -> Result<()> {
    let len = payload.len() as u32;
    write_all(stream, &len.to_le_bytes()).await?;
    write_all(stream, payload).await?;
    stream.flush().await.context("dispatch flush")?;
    Ok(())
}

// ── Handshake drivers ─────────────────────────────────────────────────────────

/// Parent side: send msg 1 (-> e, psk), read msg 2 (<- e, ee), upgrade
/// to transport mode.
pub async fn run_initiator_handshake(
    stream: &mut UnixStream,
    psk: &[u8; 32],
) -> Result<TransportState> {
    let mut hs = build_initiator(psk)?;
    let mut out = vec![0u8; 1024];
    let n = hs.write_message(&[], &mut out).context("noise e")?;
    write_frame(stream, &out[..n]).await?;

    let frame = read_frame(stream).await?;
    let mut buf = vec![0u8; frame.len()];
    hs.read_message(&frame, &mut buf).context("noise ee")?;

    hs.into_transport_mode()
        .map_err(|e| anyhow!("noise into_transport: {e}"))
}

/// Child side: read msg 1 (-> e, psk), send msg 2 (<- e, ee), upgrade
/// to transport mode.
pub async fn run_responder_handshake(
    stream: &mut UnixStream,
    psk: &[u8; 32],
) -> Result<TransportState> {
    let mut hs = build_responder(psk)?;
    let frame = read_frame(stream).await?;
    let mut buf = vec![0u8; frame.len()];
    hs.read_message(&frame, &mut buf).context("noise e")?;

    let mut out = vec![0u8; 1024];
    let n = hs.write_message(&[], &mut out).context("noise ee")?;
    write_frame(stream, &out[..n]).await?;

    hs.into_transport_mode()
        .map_err(|e| anyhow!("noise into_transport: {e}"))
}

// ── Actor: drives socket ↔ mpsc channels ──────────────────────────────────────

/// Spawn a background task that owns the socket + Noise transport state
/// and bridges it to mpsc channels. Returns (outgoing_tx, incoming_rx).
pub fn spawn_actor(
    stream: UnixStream,
    noise: TransportState,
) -> (
    mpsc::Sender<DispatchFrame>,
    mpsc::Receiver<DispatchFrame>,
    tokio::task::JoinHandle<Result<()>>,
) {
    let (out_tx, out_rx) = mpsc::channel::<DispatchFrame>(64);
    let (in_tx, in_rx) = mpsc::channel::<DispatchFrame>(64);
    let handle = tokio::spawn(actor_loop(stream, noise, out_rx, in_tx));
    (out_tx, in_rx, handle)
}

async fn actor_loop(
    stream: UnixStream,
    noise: TransportState,
    mut out_rx: mpsc::Receiver<DispatchFrame>,
    in_tx: mpsc::Sender<DispatchFrame>,
) -> Result<()> {
    let (mut reader, mut writer) = stream.into_split();
    let noise = Arc::new(Mutex::new(noise));

    let r_noise = noise.clone();
    let in_tx_r = in_tx.clone();
    let read_task = tokio::spawn(async move {
        loop {
            let cipher = match read_frame(&mut reader).await {
                Ok(v) => v,
                Err(_) => break, // peer closed
            };
            let mut plain = vec![0u8; cipher.len()];
            let n = {
                let mut g = r_noise.lock().await;
                g.read_message(&cipher, &mut plain)
                    .context("noise read_message")?
            };
            plain.truncate(n);
            let frame: DispatchFrame = serde_json::from_slice(&plain)
                .context("dispatch: decode incoming frame")?;
            if in_tx_r.send(frame).await.is_err() {
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    let w_noise = noise.clone();
    let write_task = tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            let plain = serde_json::to_vec(&frame).context("dispatch: encode outgoing frame")?;
            if plain.len() > MAX_FRAME_PAYLOAD {
                bail!("dispatch: frame too large ({} > {MAX_FRAME_PAYLOAD})", plain.len());
            }
            let mut cipher = vec![0u8; plain.len() + 16];
            let n = {
                let mut g = w_noise.lock().await;
                g.write_message(&plain, &mut cipher)
                    .context("noise write_message")?
            };
            cipher.truncate(n);
            write_frame(&mut writer, &cipher).await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    let r = read_task.await.map_err(|e| anyhow!("read join: {e}"))?;
    let w = write_task.await.map_err(|e| anyhow!("write join: {e}"))?;
    r?;
    w?;
    drop(in_tx);
    Ok(())
}

// ── Socketpair ────────────────────────────────────────────────────────────────

/// Create a `AF_UNIX / SOCK_STREAM` socketpair suitable for parent↔child IPC.
/// Returns `(parent_end, child_end)`. Both sockets are blocking; the parent
/// end is converted to a tokio `UnixStream` by callers.
pub fn make_socketpair() -> Result<(std::os::unix::net::UnixStream, std::os::unix::net::UnixStream)> {
    use socket2::{Domain, Socket, Type};
    let (a, b) = Socket::pair(Domain::UNIX, Type::STREAM, None).context("socketpair")?;
    // Convert via raw fd → std UnixStream.
    let a_fd = a.into_raw_fd();
    let b_fd = b.into_raw_fd();
    // SAFETY: we own the fds from socket2 and nobody else holds them.
    let a_std = unsafe { std::os::unix::net::UnixStream::from_raw_fd(a_fd) };
    let b_std = unsafe { std::os::unix::net::UnixStream::from_raw_fd(b_fd) };
    Ok((a_std, b_std))
}

// ── Parent-side: spawn a child worker ────────────────────────────────────────

/// Handle to a running child worker. Dropping cancels nothing — callers
/// must send a `Cancel` frame or kill the child explicitly.
pub struct ChildHandle {
    pub child: tokio::process::Child,
    pub outgoing: mpsc::Sender<DispatchFrame>,
    pub incoming: mpsc::Receiver<DispatchFrame>,
    pub actor: tokio::task::JoinHandle<Result<()>>,
}

/// Spawn a child `vibecli worker` subprocess, perform the Noise_NNpsk0
/// handshake, and return a bidirectional channel.
///
/// `exe` is the path to the current binary (usually `std::env::current_exe()`).
/// `job_id` is passed to the child via env so it can tag log output.
pub async fn spawn_worker(
    exe: &std::path::Path,
    job_id: &str,
    psk: &[u8; 32],
) -> Result<ChildHandle> {
    use std::os::fd::AsFd;

    let (parent_end, child_end) = make_socketpair()?;

    // Parent: make it async.
    parent_end
        .set_nonblocking(true)
        .context("set_nonblocking parent end")?;
    let mut parent_stream = UnixStream::from_std(parent_end).context("tokio UnixStream")?;

    // Build the child command. The child_end fd gets dup2'd to fd 3 in
    // `pre_exec`, which drops FD_CLOEXEC and makes it inheritable.
    let mut cmd = tokio::process::Command::new(exe);
    cmd.arg("worker");
    cmd.env(ENV_WORKER_FD, "3");
    cmd.env(ENV_WORKER_PSK, hex::encode(psk));
    cmd.env(ENV_WORKER_JOB_ID, job_id);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    let child_raw_fd: RawFd = child_end.as_fd().as_raw_fd();
    // Keep the owned std UnixStream alive until after spawn so the fd stays
    // valid. dup2 in pre_exec creates an independent fd 3 in the child.
    unsafe {
        cmd.pre_exec(move || {
            if libc::dup2(child_raw_fd, 3) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let child = cmd.spawn().context("spawn worker")?;
    // Parent doesn't need its copy of the child end anymore.
    drop(child_end);

    // Perform the Noise handshake.
    let transport = run_initiator_handshake(&mut parent_stream, psk)
        .await
        .context("initiator handshake")?;

    let (outgoing, incoming, actor) = spawn_actor(parent_stream, transport);
    Ok(ChildHandle {
        child,
        outgoing,
        incoming,
        actor,
    })
}

// ── Child-side: worker entry point ───────────────────────────────────────────

/// Child-side entry point. Called by `main.rs` when invoked as `vibecli worker`.
/// Reads the inherited fd + PSK from env, performs the responder handshake,
/// and hands control to `run`.
pub async fn run_worker<F, Fut>(run: F) -> Result<()>
where
    F: FnOnce(WorkerSession) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let fd_str = std::env::var(ENV_WORKER_FD)
        .map_err(|_| anyhow!("worker: {} not set", ENV_WORKER_FD))?;
    let fd: RawFd = fd_str
        .parse()
        .map_err(|e| anyhow!("worker: invalid fd {fd_str}: {e}"))?;
    let psk_hex = std::env::var(ENV_WORKER_PSK)
        .map_err(|_| anyhow!("worker: {} not set", ENV_WORKER_PSK))?;
    let psk_bytes = hex::decode(&psk_hex).map_err(|e| anyhow!("worker: bad PSK hex: {e}"))?;
    let psk: [u8; 32] = psk_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("worker: PSK must be 32 bytes"))?;
    let job_id = std::env::var(ENV_WORKER_JOB_ID).unwrap_or_default();

    // SAFETY: the parent passed us fd 3 via dup2. We take ownership.
    let std_stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(fd) };
    std_stream
        .set_nonblocking(true)
        .context("worker: set_nonblocking")?;
    let mut stream = UnixStream::from_std(std_stream).context("worker: tokio UnixStream")?;

    let transport = run_responder_handshake(&mut stream, &psk)
        .await
        .context("responder handshake")?;
    let (outgoing, incoming, actor) = spawn_actor(stream, transport);

    let session = WorkerSession {
        job_id,
        outgoing,
        incoming,
    };
    let result = run(session).await;
    // Closing the outgoing channel flushes the writer task inside the actor.
    let _ = actor.await;
    result
}

/// Handles given to the worker's user-supplied `run` closure.
pub struct WorkerSession {
    pub job_id: String,
    pub outgoing: mpsc::Sender<DispatchFrame>,
    pub incoming: mpsc::Receiver<DispatchFrame>,
}

impl WorkerSession {
    /// Send a frame to the parent. Drops silently if the parent is gone.
    pub async fn send(&self, frame: DispatchFrame) {
        let _ = self.outgoing.send(frame).await;
    }

    /// Receive the next frame from the parent, or `None` if the channel closes.
    pub async fn recv(&mut self) -> Option<DispatchFrame> {
        self.incoming.recv().await
    }
}

// ── Stub worker loop ─────────────────────────────────────────────────────────

/// Minimal worker agent loop used by transport/integration tests. Reads
/// one `Run` frame, posts `Ready`, and then runs a short "work loop" that
/// emits a chunk Event per 50 ms tick, yielding to the incoming channel
/// between ticks so a `Cancel` frame can short-circuit with an `Error`.
/// A clean run terminates after one chunk with `Complete`.
pub async fn run_stub_agent_loop(mut session: WorkerSession) -> Result<()> {
    let task = match session.recv().await {
        Some(DispatchFrame::Run { task, .. }) => task,
        Some(DispatchFrame::Cancel { reason }) => {
            session
                .send(DispatchFrame::Error {
                    message: format!("cancelled before start: {reason}"),
                })
                .await;
            return Ok(());
        }
        Some(other) => {
            session
                .send(DispatchFrame::Error {
                    message: format!("expected Run, got {other:?}"),
                })
                .await;
            return Ok(());
        }
        None => return Ok(()),
    };

    session.send(DispatchFrame::Ready).await;

    // Emit one chunk, then pause briefly on an incoming-watch so tests
    // that cancel immediately after Run can observe the Cancel → Error
    // path. A cancel beats the Complete frame.
    session
        .send(DispatchFrame::Event(AgentEventPayload::chunk(format!(
            "worker received task: {task}"
        ))))
        .await;

    tokio::select! {
        _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
            session
                .send(DispatchFrame::Complete {
                    summary: format!("stub worker done: {task}"),
                })
                .await;
        }
        frame = session.recv() => {
            match frame {
                Some(DispatchFrame::Cancel { reason }) => {
                    session
                        .send(DispatchFrame::Error {
                            message: format!("cancelled: {reason}"),
                        })
                        .await;
                }
                _ => {
                    session
                        .send(DispatchFrame::Complete {
                            summary: format!("stub worker done: {task}"),
                        })
                        .await;
                }
            }
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive initiator + responder handshakes over an in-memory socketpair.
    #[tokio::test]
    async fn handshake_roundtrip_matching_psk() {
        let psk = [7u8; 32];
        let (a, b) = make_socketpair().unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        let mut a_tokio = UnixStream::from_std(a).unwrap();
        let mut b_tokio = UnixStream::from_std(b).unwrap();

        let psk_a = psk;
        let psk_b = psk;
        let init = tokio::spawn(async move {
            run_initiator_handshake(&mut a_tokio, &psk_a).await
        });
        let resp = tokio::spawn(async move {
            run_responder_handshake(&mut b_tokio, &psk_b).await
        });

        let (init_res, resp_res) = tokio::join!(init, resp);
        assert!(init_res.unwrap().is_ok(), "initiator must complete");
        assert!(resp_res.unwrap().is_ok(), "responder must complete");
    }

    #[tokio::test]
    async fn handshake_mismatched_psk_fails() {
        let (a, b) = make_socketpair().unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        let mut a_tokio = UnixStream::from_std(a).unwrap();
        let mut b_tokio = UnixStream::from_std(b).unwrap();

        let psk_a = [1u8; 32];
        let psk_b = [2u8; 32];
        let init = tokio::spawn(async move {
            run_initiator_handshake(&mut a_tokio, &psk_a).await
        });
        let resp = tokio::spawn(async move {
            run_responder_handshake(&mut b_tokio, &psk_b).await
        });

        let (init_res, resp_res) = tokio::join!(init, resp);
        // At least one side must fail; Noise_NNpsk0 rejects on the responder
        // because msg 1 contains a MAC keyed by the PSK.
        assert!(
            init_res.unwrap().is_err() || resp_res.unwrap().is_err(),
            "mismatched PSK must abort at least one side"
        );
    }

    /// Exchange real frames through the actor after a successful handshake.
    #[tokio::test]
    async fn frames_roundtrip_through_actor() {
        let psk = [9u8; 32];
        let (a, b) = make_socketpair().unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        let mut a_tokio = UnixStream::from_std(a).unwrap();
        let mut b_tokio = UnixStream::from_std(b).unwrap();

        let psk_a = psk;
        let psk_b = psk;
        let init = tokio::spawn(async move {
            let t = run_initiator_handshake(&mut a_tokio, &psk_a).await?;
            Ok::<_, anyhow::Error>((a_tokio, t))
        });
        let resp = tokio::spawn(async move {
            let t = run_responder_handshake(&mut b_tokio, &psk_b).await?;
            Ok::<_, anyhow::Error>((b_tokio, t))
        });
        let ((a_stream, a_ts), (b_stream, b_ts)) = (init.await.unwrap().unwrap(), resp.await.unwrap().unwrap());

        let (a_out, mut a_in, _a_actor) = spawn_actor(a_stream, a_ts);
        let (b_out, mut b_in, _b_actor) = spawn_actor(b_stream, b_ts);

        // A → B
        a_out
            .send(DispatchFrame::Run {
                job_id: "j1".into(),
                task: "hello".into(),
                provider: "anthropic".into(),
                approval: "auto".into(),
                workspace_root: "/tmp".into(),
                max_turns: 3,
            })
            .await
            .unwrap();
        let got = tokio::time::timeout(std::time::Duration::from_secs(2), b_in.recv())
            .await
            .unwrap()
            .unwrap();
        match got {
            DispatchFrame::Run { job_id, task, max_turns, .. } => {
                assert_eq!(job_id, "j1");
                assert_eq!(task, "hello");
                assert_eq!(max_turns, 3);
            }
            other => panic!("expected Run, got {other:?}"),
        }

        // B → A
        b_out
            .send(DispatchFrame::Complete { summary: "done".into() })
            .await
            .unwrap();
        let got = tokio::time::timeout(std::time::Duration::from_secs(2), a_in.recv())
            .await
            .unwrap()
            .unwrap();
        match got {
            DispatchFrame::Complete { summary } => assert_eq!(summary, "done"),
            other => panic!("expected Complete, got {other:?}"),
        }
    }

    #[test]
    fn psk_is_32_bytes() {
        let psk = generate_psk();
        assert_eq!(psk.len(), 32);
    }

    /// Regression: `AgentEventPayload` has its own `#[serde(rename = "type")]`
    /// field, so the outer `DispatchFrame` tag must not be `"type"` or
    /// serialization silently collides.
    #[test]
    fn event_frame_serializes_without_tag_collision() {
        let f = DispatchFrame::Event(AgentEventPayload::chunk("hi".into()));
        let s = serde_json::to_string(&f).expect("event frame must serialize");
        assert!(s.contains("\"frame\":\"event\""));
        assert!(s.contains("\"type\":\"chunk\""));
        let back: DispatchFrame =
            serde_json::from_str(&s).expect("event frame must deserialize");
        match back {
            DispatchFrame::Event(ev) => assert_eq!(ev.kind, "chunk"),
            other => panic!("expected Event, got {other:?}"),
        }
    }
}
