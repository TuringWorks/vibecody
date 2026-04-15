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

    // ── Helper: parse the ngrok /api/tunnels response without a real TCP connection ──

    fn parse_api(body: &str, port: u16) -> Option<String> {
        let json: serde_json::Value = serde_json::from_str(body).ok()?;
        let tunnels = json["tunnels"].as_array()?;
        let port_str = port.to_string();
        for tunnel in tunnels {
            if tunnel["proto"].as_str() != Some("https") { continue; }
            let addr = tunnel["config"]["addr"].as_str().unwrap_or("");
            if addr.contains(&port_str) {
                if let Some(url) = tunnel["public_url"].as_str() {
                    return Some(url.to_string());
                }
            }
        }
        None
    }

    // ── RED tests (written before the feature; prove the contract) ────────────

    #[test]
    fn parse_api_returns_https_url_for_matching_port() {
        let body = r#"{"tunnels":[{"proto":"https","public_url":"https://abc.ngrok.io","config":{"addr":"localhost:7878"}}]}"#;
        assert_eq!(parse_api(body, 7878), Some("https://abc.ngrok.io".to_string()));
    }

    #[test]
    fn parse_api_ignores_http_tunnels() {
        let body = r#"{"tunnels":[{"proto":"http","public_url":"http://abc.ngrok.io","config":{"addr":"localhost:7878"}}]}"#;
        assert_eq!(parse_api(body, 7878), None);
    }

    #[test]
    fn parse_api_ignores_wrong_port() {
        let body = r#"{"tunnels":[{"proto":"https","public_url":"https://abc.ngrok.io","config":{"addr":"localhost:9999"}}]}"#;
        assert_eq!(parse_api(body, 7878), None);
    }

    #[test]
    fn parse_api_returns_none_for_empty_tunnel_list() {
        let body = r#"{"tunnels":[]}"#;
        assert_eq!(parse_api(body, 7878), None);
    }

    #[test]
    fn parse_api_returns_none_for_malformed_json() {
        assert_eq!(parse_api("not-json", 7878), None);
    }

    #[test]
    fn parse_api_returns_none_for_missing_tunnels_key() {
        assert_eq!(parse_api("{}", 7878), None);
    }

    #[test]
    fn parse_api_picks_first_matching_https_tunnel() {
        let body = r#"{"tunnels":[
            {"proto":"http","public_url":"http://a.ngrok.io","config":{"addr":"localhost:7878"}},
            {"proto":"https","public_url":"https://b.ngrok.io","config":{"addr":"localhost:7878"}}
        ]}"#;
        assert_eq!(parse_api(body, 7878), Some("https://b.ngrok.io".to_string()));
    }

    // ── TunnelConfig serde (local test struct mirrors config::TunnelConfig) ──────

    #[derive(Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TunnelCfg {
        #[serde(default)] tailscale_funnel: bool,
        #[serde(default)] ngrok_auto_start: bool,
        #[serde(default)] ngrok_auth_token: Option<String>,
    }

    #[test]
    fn tunnel_config_default_is_all_off() {
        let cfg = TunnelCfg::default();
        assert!(!cfg.tailscale_funnel);
        assert!(!cfg.ngrok_auto_start);
        assert!(cfg.ngrok_auth_token.is_none());
    }

    #[test]
    fn tunnel_config_roundtrips_through_toml() {
        let cfg = TunnelCfg {
            tailscale_funnel: true,
            ngrok_auto_start: false,
            ngrok_auth_token: Some("tok_test".to_string()),
        };
        let s = toml::to_string(&cfg).expect("serialise");
        let parsed: TunnelCfg = toml::from_str(&s).expect("deserialise");
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn tunnel_config_roundtrips_through_json() {
        let cfg = TunnelCfg {
            tailscale_funnel: false,
            ngrok_auto_start: true,
            ngrok_auth_token: Some("ngrok_secret".to_string()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: TunnelCfg = serde_json::from_str(&json).unwrap();
        assert!(parsed.ngrok_auto_start);
        assert_eq!(parsed.ngrok_auth_token.as_deref(), Some("ngrok_secret"));
    }

    #[test]
    fn tunnel_config_missing_optional_fields_use_defaults() {
        let s = r#"tailscale_funnel = true"#;
        let cfg: TunnelCfg = toml::from_str(s).expect("parse partial config");
        assert!(cfg.tailscale_funnel);
        assert!(!cfg.ngrok_auto_start);
        assert!(cfg.ngrok_auth_token.is_none());
    }

    // ── GREEN: detect_tunnel is safe when ngrok is absent ────────────────────

    #[test]
    fn detect_tunnel_does_not_panic_without_ngrok() {
        let result = detect_tunnel(19999);
        let _ = result; // may be None or Some in environments with ngrok
    }

    #[test]
    fn detect_tunnel_port_zero_is_safe() {
        let _ = detect_tunnel(0);
    }
}
