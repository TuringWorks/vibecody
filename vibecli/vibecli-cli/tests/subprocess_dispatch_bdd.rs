/*!
 * BDD tests for subprocess dispatch (M7).
 *
 * Exercises the JobManager → child `vibecli worker` round-trip through the
 * real binary. The full transport layer (handshake, framing, actor) is
 * unit-tested inside `subprocess_dispatch.rs`; this harness anchors the
 * end-to-end contract from the job-queue caller's perspective.
 *
 * Run with: cargo test --test subprocess_dispatch_bdd
 */

#![cfg(unix)]

use cucumber::{given, then, when, World};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::broadcast;
use vibecli_cli::job_manager::{
    AgentEventPayload, CreateJobReq, JobManager, JobStatus, JobsDb,
};
use vibecli_cli::subprocess_dispatch::{spawn_worker, DispatchFrame};

fn vibecli_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_vibecli"))
}

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct DispatchWorld {
    manager: Option<JobManager>,
    _tmp: Option<Arc<TempDir>>,
    last_job_id: Option<String>,
    last_task: Option<String>,
    broadcast_rx: Option<broadcast::Receiver<AgentEventPayload>>,
    saw_chunk: bool,
}

impl DispatchWorld {
    fn manager(&self) -> &JobManager {
        self.manager.as_ref().expect("manager not initialised")
    }
    fn last_sid(&self) -> String {
        self.last_job_id.clone().expect("no job id recorded")
    }
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given("a JobManager backed by a fresh encrypted db")]
async fn given_fresh_manager(w: &mut DispatchWorld) {
    let tmp = Arc::new(TempDir::new().unwrap());
    let db = JobsDb::open_with(&tmp.path().join("jobs.db"), [13u8; 32]).unwrap();
    w.manager = Some(JobManager::new_with(db));
    w._tmp = Some(tmp);
}

#[given(regex = r#"^a queued job with task "(.+)"$"#)]
async fn given_queued_job(w: &mut DispatchWorld, task: String) {
    let sid = w
        .manager()
        .create(CreateJobReq {
            task: task.clone(),
            provider: "stub".into(),
            approval: "auto".into(),
            workspace_root: "/tmp".into(),
            priority: 5,
            webhook_url: None,
            tags: vec![],
            quota_bucket: None,
        })
        .await
        .expect("create job");
    w.manager().mark_running(&sid).await.expect("mark running");
    w.last_job_id = Some(sid);
    w.last_task = Some(task);
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I dispatch the job to a child worker using the test binary")]
async fn when_dispatch(w: &mut DispatchWorld) {
    let sid = w.last_sid();
    let task = w.last_task.clone().expect("task not recorded");

    // Open the broadcast stream BEFORE the child starts emitting so we don't
    // miss events. The bridge task below also forwards to this tx.
    let bcast = w.manager().open_stream(&sid).await;
    let rx = bcast.subscribe();
    w.broadcast_rx = Some(rx);

    // JobManager::spawn_subprocess would call std::env::current_exe(), which
    // in a cargo test returns the test binary path — not `vibecli`. So this
    // step drives spawn_worker directly with CARGO_BIN_EXE_vibecli, mirroring
    // what spawn_subprocess does internally. This keeps the harness honest
    // about what it's testing: the daemon↔worker contract.
    let psk = [77u8; 32];
    let handle = spawn_worker(&vibecli_bin(), &sid, &psk)
        .await
        .expect("spawn worker");

    handle
        .outgoing
        .send(DispatchFrame::Run {
            job_id: sid.clone(),
            task: task.clone(),
            provider: "stub".into(),
            approval: "auto".into(),
            workspace_root: "/tmp".into(),
            max_turns: 3,
        })
        .await
        .expect("send Run");

    let jm = w.manager().clone();
    let sid_for_task = sid.clone();
    tokio::spawn(async move {
        let mut child = handle.child;
        let mut incoming = handle.incoming;
        let outgoing = handle.outgoing;
        while let Some(frame) = incoming.recv().await {
            match frame {
                DispatchFrame::Event(ev) => {
                    let _ = bcast.send(ev);
                }
                DispatchFrame::Complete { summary } => {
                    let _ = jm
                        .mark_terminal(
                            &sid_for_task,
                            JobStatus::Complete,
                            Some(summary),
                            None,
                        )
                        .await;
                    break;
                }
                DispatchFrame::Error { message } => {
                    let _ = jm
                        .mark_terminal(
                            &sid_for_task,
                            JobStatus::Failed,
                            None,
                            Some(message),
                        )
                        .await;
                    break;
                }
                _ => {}
            }
        }
        jm.close_stream(&sid_for_task).await;
        drop(outgoing);
        let _ = child.wait().await;
    });
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(regex = r#"^the job should eventually reach status "(.+)"$"#)]
async fn then_job_reaches_status(w: &mut DispatchWorld, expected: String) {
    let sid = w.last_sid();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let rec = w.manager().get(&sid).await;
        if let Some(r) = rec {
            if r.status == expected {
                return;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            let final_rec = w.manager().get(&sid).await;
            panic!(
                "timed out waiting for status {expected:?}; last = {:?}",
                final_rec.map(|r| r.status)
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[then(regex = r#"^the job summary should contain "(.+)"$"#)]
async fn then_summary_contains(w: &mut DispatchWorld, needle: String) {
    let sid = w.last_sid();
    let rec = w.manager().get(&sid).await.expect("record exists");
    let summary = rec.summary.unwrap_or_default();
    assert!(
        summary.contains(&needle),
        "summary {summary:?} does not contain {needle:?}"
    );
}

#[then("the broadcast stream should have delivered at least one chunk event")]
async fn then_chunk_delivered(w: &mut DispatchWorld) {
    if w.saw_chunk {
        return;
    }
    let mut rx = w
        .broadcast_rx
        .take()
        .expect("broadcast rx not initialised");
    // The bridge task may have already consumed the chunk into the broadcast
    // tx before we subscribed, so this is best-effort with a short window.
    let got = tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            match rx.recv().await {
                Ok(ev) if ev.kind == "chunk" => return true,
                Ok(_) => continue,
                Err(_) => return false,
            }
        }
    })
    .await
    .unwrap_or(false);
    assert!(got || w.saw_chunk, "no chunk event observed");
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // BDD exercises the transport contract only; the child uses the stub
    // loop so scenarios don't depend on a live AI provider.
    std::env::set_var("VIBECLI_WORKER_MODE", "stub");
    DispatchWorld::run("tests/features/subprocess_dispatch.feature").await;
}
