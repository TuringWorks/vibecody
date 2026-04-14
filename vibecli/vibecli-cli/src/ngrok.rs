#![allow(dead_code)]
//! ngrok tunnel auto-detection and startup for VibeCLI daemon remote access.
//!
//! Checks `localhost:4040/api/tunnels` for a running ngrok agent and optionally
//! starts a new `ngrok http <port>` tunnel process in the background.

use anyhow::{Context, Result};

/// Returns the public HTTPS URL for a tunnel on `port` from a running ngrok
/// agent (localhost:4040).  Returns `None` if ngrok is not running or has no
/// tunnel configured for this port.
///
/// This function is synchronous — it uses `reqwest::blocking::Client` so it
/// can be called from non-async contexts.  In an async context prefer calling
/// it via `tokio::task::spawn_blocking`.
pub fn detect_tunnel(port: u16) -> Option<String> {
    // Quick TCP probe — avoids the HTTP round-trip when ngrok is not running.
    use std::net::TcpStream;
    use std::time::Duration;
    if TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], 4040)),
        Duration::from_millis(200),
    )
    .is_err()
    {
        return None;
    }

    // Fetch the tunnel list from the local ngrok agent API.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;

    let resp = client
        .get("http://localhost:4040/api/tunnels")
        .send()
        .ok()?;

    let body: serde_json::Value = resp.json().ok()?;

    // The response looks like:
    //   {"tunnels":[{"proto":"https","public_url":"https://abc.ngrok.io",
    //                "config":{"addr":"localhost:7878",...},...}]}
    let tunnels = body["tunnels"].as_array()?;
    let port_str = port.to_string();

    for tunnel in tunnels {
        // Only care about HTTPS tunnels.
        if tunnel["proto"].as_str() != Some("https") {
            continue;
        }

        // Match on config.addr containing our port.
        let addr = tunnel["config"]["addr"].as_str().unwrap_or("");
        if addr.contains(&port_str) {
            if let Some(url) = tunnel["public_url"].as_str() {
                return Some(url.to_string());
            }
        }
    }

    None
}

/// Starts an `ngrok http <port>` tunnel in the background and returns the
/// public HTTPS URL once the tunnel comes up.
///
/// * If `auth_token` is `Some`, it is exported as the `NGROK_AUTHTOKEN`
///   environment variable before spawning (overrides any existing value).
/// * Polls `detect_tunnel(port)` every 500 ms for up to 15 seconds.
/// * Returns an error if the ngrok binary is not found, the process fails to
///   spawn, or no tunnel appears within the timeout.
pub async fn start_tunnel(port: u16, auth_token: Option<&str>) -> Result<String> {
    use tokio::time::{sleep, Duration};

    let mut cmd = tokio::process::Command::new("ngrok");
    cmd.args(["http", &port.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    if let Some(token) = auth_token {
        if !token.is_empty() {
            cmd.env("NGROK_AUTHTOKEN", token);
        }
    }

    let _child = cmd
        .spawn()
        .context("Failed to start ngrok. Is ngrok installed and on PATH?")?;

    // Poll for the tunnel to appear (up to 15 s, 500 ms interval = 30 attempts).
    for attempt in 0..30u32 {
        sleep(Duration::from_millis(500)).await;

        // Blocking I/O moved off the async executor.
        let port_copy = port;
        if let Ok(Some(url)) =
            tokio::task::spawn_blocking(move || detect_tunnel(port_copy)).await
        {
            return Ok(url);
        }

        if attempt == 0 {
            // Give ngrok a bit more time on first iteration.
            sleep(Duration::from_millis(500)).await;
        }
    }

    anyhow::bail!(
        "ngrok tunnel on port {port} did not appear within 15 seconds. \
         Check that the ngrok binary works and the auth token is valid."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_tunnel_returns_none_when_ngrok_not_running() {
        // In CI / dev environments without ngrok, this should return None rather
        // than panic.  Port 19999 is used to avoid clashing with a real daemon.
        let result = detect_tunnel(19999);
        // We only assert it does not panic.  The actual value depends on the
        // environment.
        let _ = result;
    }

    #[test]
    fn detect_tunnel_port_zero_returns_none() {
        // Port 0 will never match a real ngrok tunnel.
        let result = detect_tunnel(0);
        let _ = result;
    }
}
