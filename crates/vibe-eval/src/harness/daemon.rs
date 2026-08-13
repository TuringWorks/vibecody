//! The daemon surface: `POST /agent`, then poll the job to completion.
//!
//! Ten of VibeCody's clients reach the agent loop through this daemon rather
//! than owning a copy of it, so a capability number measured here is the one
//! those clients inherit. What they do *not* inherit is the transport — that
//! is what the conformance probes in [`crate::harness::probe`] are for.

use std::path::Path;
use std::time::Duration;

use super::{
    daemon_port, health_is_vibecli, read_daemon_token, Harness, HarnessError, Preflight, RunOutcome,
};
use crate::task::{EvalTask, Surface};

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub base_url: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    /// How often to ask whether the job has finished.
    pub poll_interval: Duration,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{}", daemon_port()),
            provider: None,
            model: None,
            poll_interval: Duration::from_millis(750),
        }
    }
}

pub struct DaemonHarness {
    config: DaemonConfig,
    client: reqwest::Client,
}

impl DaemonHarness {
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Attach the bearer token, read fresh each call.
    ///
    /// Nearly every daemon route sits behind `require_auth`, and the token
    /// rotates on every daemon start. A token cached at construction is stale
    /// the moment anything restarts the daemon, and the symptom is a uniform
    /// 401 that reads like a capability collapse.
    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match read_daemon_token() {
            Some(token) => req.bearer_auth(token),
            None => req,
        }
    }
}

#[async_trait::async_trait]
impl Harness for DaemonHarness {
    fn surface(&self) -> Surface {
        Surface::Daemon
    }

    fn describe(&self) -> String {
        format!(
            "daemon {} · provider={} model={}",
            self.config.base_url,
            self.config
                .provider
                .as_deref()
                .unwrap_or("(daemon default)"),
            self.config.model.as_deref().unwrap_or("(daemon default)")
        )
    }

    async fn preflight(&self) -> Preflight {
        let url = format!("{}/health", self.config.base_url);
        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        match response {
            Err(e) => Preflight::unavailable(format!(
                "no daemon answering at {} ({}). Start one with `vibecli --serve --port {}`",
                self.config.base_url,
                e,
                daemon_port()
            )),
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Err(e) => Preflight::unavailable(format!(
                        "{} answered {} but not with JSON ({}) — something else is on this port",
                        self.config.base_url, status, e
                    )),
                    // Identity, not liveness: any process holding the port
                    // answers a connect, and calling that "the daemon is up"
                    // sends the operator to debug the wrong component.
                    Ok(body) if health_is_vibecli(&body) => {
                        if read_daemon_token().is_none() {
                            Preflight::unavailable(
                                "daemon is running but ~/.vibecli/daemon.token is missing or empty \
                                 — every protected route would 401"
                                    .to_string(),
                            )
                        } else {
                            Preflight::Ready
                        }
                    }
                    Ok(body) => Preflight::unavailable(format!(
                        "a non-VibeCLI service is listening on {} (health: {})",
                        self.config.base_url,
                        serde_json::to_string(&body).unwrap_or_default()
                    )),
                }
            }
        }
    }

    async fn run(
        &self,
        task: &EvalTask,
        workspace: &Path,
        timeout: Duration,
    ) -> Result<RunOutcome, HarnessError> {
        let mut body = serde_json::json!({
            "task": task.prompt,
            "approval": "full-auto",
            // The daemon runs wherever it was started unless told otherwise;
            // without this every task would grade an untouched fixture while
            // the agent edited the daemon's own cwd.
            "workspace_root": workspace.to_string_lossy(),
        });
        if let Some(p) = &self.config.provider {
            body["provider"] = serde_json::Value::String(p.clone());
        }
        if let Some(m) = &self.config.model {
            body["model"] = serde_json::Value::String(m.clone());
        }

        let started = std::time::Instant::now();
        let start_url = format!("{}/agent", self.config.base_url);
        let resp = self
            .authed(self.client.post(&start_url))
            .json(&body)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| HarnessError::Transport(e.to_string()))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| HarnessError::Transport(e.to_string()))?;
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(HarnessError::Transport(
                "401 from /agent — the bearer token is stale; the daemon rotates it on restart"
                    .to_string(),
            ));
        }
        if !status.is_success() {
            return Err(HarnessError::Protocol(format!(
                "/agent returned {}: {}",
                status,
                text.chars().take(400).collect::<String>()
            )));
        }
        let session_id = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| {
                v.get("session_id")
                    .and_then(|s| s.as_str())
                    .map(str::to_string)
            })
            .ok_or_else(|| {
                HarnessError::Protocol(format!("/agent returned no session_id: {}", text))
            })?;

        // Poll to a deadline rather than sleeping a guess. A cold daemon can
        // take many seconds before it even reports `running`, and a fixed wait
        // is how a healthy run gets recorded as a timeout.
        let deadline = std::time::Instant::now() + timeout;
        let job_url = format!("{}/jobs/{}", self.config.base_url, session_id);
        loop {
            if std::time::Instant::now() >= deadline {
                // Best-effort: stop the run so it does not keep burning tokens
                // against a task nobody is waiting for any more.
                let cancel = format!("{}/jobs/{}/cancel", self.config.base_url, session_id);
                let _ = self
                    .authed(self.client.post(&cancel))
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await;
                return Err(HarnessError::Timeout {
                    secs: timeout.as_secs(),
                    // The daemon runs out of process; its logs are not ours to
                    // capture, so this stays empty rather than fabricated.
                    tail: String::new(),
                });
            }

            tokio::time::sleep(self.config.poll_interval).await;

            let job = self
                .authed(self.client.get(&job_url))
                .timeout(Duration::from_secs(15))
                .send()
                .await;
            let Ok(resp) = job else { continue };
            if !resp.status().is_success() {
                continue;
            }
            let Ok(record) = resp.json::<serde_json::Value>().await else {
                continue;
            };
            let job_status = record
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            if !is_terminal(&job_status) {
                continue;
            }

            return Ok(RunOutcome {
                final_text: record
                    .get("summary")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                // The daemon's vocabulary differs from the CLI's; map it so a
                // task's `outcome_is` assertion means the same thing on both.
                // `partial` deliberately does not become `success`: the daemon
                // distinguishes them because folding them together is what let
                // unfinished runs be reported as wins.
                outcome: Some(normalise_outcome(&job_status)),
                // The job record carries counters, not a tool-by-tool
                // transcript. Leaving this empty is what makes transcript
                // graders report `error` instead of silently passing.
                steps: Vec::new(),
                duration_ms: started.elapsed().as_millis() as u64,
                exit_code: None,
                raw: Some(record),
            });
        }
    }
}

fn is_terminal(status: &str) -> bool {
    matches!(status, "complete" | "partial" | "failed" | "cancelled")
}

fn normalise_outcome(job_status: &str) -> String {
    match job_status {
        "complete" => "success".to_string(),
        "partial" => "partial".to_string(),
        "failed" => "failed".to_string(),
        "cancelled" => "cancelled".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_states_match_the_daemon_job_vocabulary() {
        for s in ["complete", "partial", "failed", "cancelled"] {
            assert!(is_terminal(s), "{} should be terminal", s);
        }
        for s in ["queued", "running", ""] {
            assert!(!is_terminal(s), "{} should not be terminal", s);
        }
    }

    #[test]
    fn partial_is_not_reported_as_success() {
        // The daemon keeps `partial` separate from `complete` precisely so an
        // unfinished run is not read as a win; the harness must preserve that.
        assert_eq!(normalise_outcome("complete"), "success");
        assert_eq!(normalise_outcome("partial"), "partial");
        assert_eq!(normalise_outcome("failed"), "failed");
    }

    #[tokio::test]
    async fn preflight_on_a_dead_port_names_the_port_and_the_fix() {
        let h = DaemonHarness::new(DaemonConfig {
            // Port 1 is reserved and will refuse instantly.
            base_url: "http://127.0.0.1:1".to_string(),
            ..DaemonConfig::default()
        });
        match h.preflight().await {
            Preflight::Unavailable { reason } => {
                assert!(reason.contains("vibecli --serve"), "{}", reason);
            }
            Preflight::Ready => panic!("nothing should be listening on port 1"),
        }
    }
}
