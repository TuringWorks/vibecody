//! `transcribe_audio` / `voice_status` — the desktop shells' bridge to the
//! daemon's voice routes.

use base64::Engine;

/// Default daemon address, matching `DEFAULT_DAEMON_URL` in the shells.
const DEFAULT_DAEMON_URL: &str = "http://127.0.0.1:7878";

/// Transcription can take a while on the local whisper path — a 60 s clip on a
/// cold CPU model is not unusual. The daemon's own Groq call already times out
/// at 60 s, so this has to exceed that or we'd abandon a request the daemon is
/// still working on and report a timeout the user can't act on.
const TRANSCRIBE_TIMEOUT_SECS: u64 = 180;

/// Resolve the daemon bearer token.
///
/// Same order as the shells' own `resolve_token`: an explicit token, then
/// `VIBECLI_TOKEN`, then `~/.vibecli/daemon.token` where `vibecli --serve`
/// writes it. `None` is a legitimate answer — a daemon may run without auth —
/// so this is not an error path.
fn resolve_token(explicit: Option<String>) -> Option<String> {
    explicit
        .filter(|t| !t.is_empty())
        .or_else(|| std::env::var("VIBECLI_TOKEN").ok().filter(|t| !t.is_empty()))
        .or_else(|| {
            let path = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)?
                .join(".vibecli")
                .join("daemon.token");
            std::fs::read_to_string(path)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
}

fn with_auth(req: reqwest::RequestBuilder, token: Option<String>) -> reqwest::RequestBuilder {
    match resolve_token(token) {
        Some(t) => req.header("Authorization", format!("Bearer {}", t)),
        None => req,
    }
}

fn base_url(url: Option<String>) -> String {
    url.filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_DAEMON_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Pull the daemon's `{"error": "..."}` message out of a failure response.
///
/// The daemon's voice errors are setup guidance — "run /voice download base",
/// "set GROQ_API_KEY" — so surfacing them verbatim is the point. Falling back
/// to the bare status code loses the only actionable part.
async fn error_message(resp: reqwest::Response) -> String {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
        .unwrap_or_else(|| {
            if body.is_empty() {
                format!("Transcription failed (HTTP {})", status.as_u16())
            } else {
                format!("Transcription failed (HTTP {}): {}", status.as_u16(), body)
            }
        })
}

/// Transcribe base64-encoded audio via the daemon.
///
/// Returns the recognised text. The daemon decides which engine runs (local
/// whisper first when configured, Groq otherwise); `prefer_local` forces the
/// local one so audio never leaves the machine.
#[tauri::command]
pub async fn transcribe_audio(
    url: Option<String>,
    audio_base64: String,
    mime_type: Option<String>,
    language: Option<String>,
    prefer_local: Option<bool>,
    token: Option<String>,
) -> Result<String, String> {
    if audio_base64.is_empty() {
        return Err("No audio to transcribe.".to_string());
    }
    // Validate here rather than shipping bad bytes across the wire: a base64
    // error from the daemon reads like a daemon problem.
    base64::engine::general_purpose::STANDARD
        .decode(&audio_base64)
        .map_err(|e| format!("Recorded audio could not be encoded: {}", e))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TRANSCRIBE_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let mut body = serde_json::json!({
        "audio_base64": audio_base64,
        "mime_type": mime_type.unwrap_or_else(|| "audio/webm".to_string()),
    });
    if let Some(lang) = language.filter(|l| !l.is_empty()) {
        body["language"] = serde_json::Value::String(lang);
    }
    if let Some(local) = prefer_local {
        body["prefer_local"] = serde_json::Value::Bool(local);
    }

    let endpoint = format!("{}/voice/transcribe", base_url(url));
    let resp = with_auth(client.post(&endpoint).json(&body), token)
        .send()
        .await
        .map_err(|e| format!("Cannot reach the daemon for transcription: {}", e))?;

    if !resp.status().is_success() {
        return Err(error_message(resp).await);
    }

    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Daemon returned an unreadable transcription response: {}", e))?;
    value
        .get("text")
        .and_then(|t| t.as_str())
        .map(str::to_string)
        .ok_or_else(|| "Daemon returned no transcript.".to_string())
}

/// Report what the daemon's voice stack can do, so a shell can explain a
/// disabled mic button instead of just failing on click.
#[tauri::command]
pub async fn voice_status(
    url: Option<String>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let endpoint = format!("{}/voice/status", base_url(url));
    let resp = with_auth(client.get(&endpoint), token)
        .send()
        .await
        .map_err(|e| format!("Cannot reach the daemon: {}", e))?;
    if !resp.status().is_success() {
        return Err(error_message(resp).await);
    }
    resp.json()
        .await
        .map_err(|e| format!("Daemon returned an unreadable voice status: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_defaults_and_strips_trailing_slash() {
        assert_eq!(base_url(None), DEFAULT_DAEMON_URL);
        assert_eq!(base_url(Some(String::new())), DEFAULT_DAEMON_URL);
        assert_eq!(base_url(Some("   ".into())), DEFAULT_DAEMON_URL);
        assert_eq!(
            base_url(Some("http://127.0.0.1:9999/".into())),
            "http://127.0.0.1:9999"
        );
    }

    #[test]
    fn explicit_token_wins_over_the_environment() {
        // An explicit token is the only source this test can assert without
        // mutating process-global env, which would race other tests.
        assert_eq!(
            resolve_token(Some("explicit".into())),
            Some("explicit".into())
        );
        // An empty explicit token must fall through, not be taken literally —
        // the frontend passes `""` when it has nothing.
        assert_ne!(resolve_token(Some(String::new())), Some(String::new()));
    }

    #[tokio::test]
    async fn transcribe_rejects_empty_and_invalid_audio_before_any_request() {
        // Both must fail locally: reaching the daemon to learn the payload was
        // malformed turns a client bug into a "daemon is broken" message.
        let empty =
            transcribe_audio(None, String::new(), None, None, None, Some("t".into())).await;
        assert!(empty.is_err());

        let bad = transcribe_audio(
            None,
            "not base64!!!".into(),
            None,
            None,
            None,
            Some("t".into()),
        )
        .await;
        assert!(bad.unwrap_err().contains("could not be encoded"));
    }
}
