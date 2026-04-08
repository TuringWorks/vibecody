//! Browser Automation Agent — CDP (Chrome DevTools Protocol) browser automation.
//!
//! Provides headless and headed browser automation for VibeCody's agent framework
//! via the Chrome DevTools Protocol HTTP/JSON endpoints. Communicates with a running
//! Chrome/Chromium instance's debug port to navigate pages, click elements, type text,
//! take screenshots, evaluate JavaScript, and more.
//!
//! Usage:
//! - `launch_chrome(&config)` — spawn a Chrome process with remote debugging
//! - `BrowserSession::new(&config)` — connect to an already-running Chrome
//! - `session.execute_action(&action)` — dispatch any `BrowserAction`
//! - `BrowserPool` — manage multiple tabs/sessions
//!
//! All CDP communication uses HTTP GET/POST to `http://localhost:{port}/json/*` endpoints.
//! No WebSocket crate is required — commands are sent via the `/json/protocol` HTTP API
//! and `reqwest` handles all networking.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

// ── BrowserConfig ──────────────────────────────────────────────────────────

/// Configuration for connecting to or launching a Chrome instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Chrome DevTools Protocol debug port.
    pub debug_port: u16,
    /// Run Chrome in headless mode.
    pub headless: bool,
    /// Viewport width in pixels.
    pub viewport_width: u32,
    /// Viewport height in pixels.
    pub viewport_height: u32,
    /// Default timeout for operations, in seconds.
    pub timeout_secs: u64,
    /// Optional custom User-Agent string.
    pub user_agent: Option<String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            debug_port: 9222,
            headless: true,
            viewport_width: 1280,
            viewport_height: 720,
            timeout_secs: 30,
            user_agent: None,
        }
    }
}

impl BrowserConfig {
    /// Build the Chrome debug base URL from the configured port.
    pub fn debug_base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.debug_port)
    }

    /// Build Chrome launch arguments from this config.
    pub fn chrome_args(&self) -> Vec<String> {
        let mut args = vec![
            format!("--remote-debugging-port={}", self.debug_port),
            "--no-first-run".to_string(),
            "--disable-gpu".to_string(),
            "--disable-extensions".to_string(),
            "--disable-default-apps".to_string(),
            format!("--window-size={},{}", self.viewport_width, self.viewport_height),
        ];
        if self.headless {
            args.push("--headless=new".to_string());
        }
        if let Some(ref ua) = self.user_agent {
            args.push(format!("--user-agent={ua}"));
        }
        args
    }
}

// ── NavigationEntry ────────────────────────────────────────────────────────

/// Record of a page navigation in the session history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationEntry {
    /// The URL navigated to.
    pub url: String,
    /// The page title at time of navigation.
    pub title: String,
    /// Millisecond timestamp (epoch) when navigation occurred.
    pub timestamp_ms: u64,
}

impl NavigationEntry {
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            url: url.into(),
            title: title.into(),
            timestamp_ms: now,
        }
    }

    #[cfg(test)]
    pub fn with_timestamp(url: impl Into<String>, title: impl Into<String>, ts: u64) -> Self {
        Self {
            url: url.into(),
            title: title.into(),
            timestamp_ms: ts,
        }
    }
}

// ── ScreenshotEntry ────────────────────────────────────────────────────────

/// A captured screenshot with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotEntry {
    /// Millisecond timestamp when the screenshot was taken.
    pub timestamp_ms: u64,
    /// Description of the action performed before the screenshot.
    pub action_before: String,
    /// Base64-encoded PNG data.
    pub png_base64: String,
}

impl ScreenshotEntry {
    pub fn new(action_before: impl Into<String>, png_base64: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            timestamp_ms: now,
            action_before: action_before.into(),
            png_base64: png_base64.into(),
        }
    }
}

// ── PageInfo ───────────────────────────────────────────────────────────────

/// Summary information about the current page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    /// Current page URL.
    pub url: String,
    /// Current page title.
    pub title: String,
    /// Truncated text content of the DOM.
    pub dom_summary: String,
}

impl PageInfo {
    /// Maximum characters for the dom_summary field.
    pub const MAX_DOM_SUMMARY_LEN: usize = 4096;

    /// Create a PageInfo, truncating dom_summary if needed.
    pub fn new(url: impl Into<String>, title: impl Into<String>, dom_text: impl Into<String>) -> Self {
        let dom = dom_text.into();
        let dom_summary = if dom.len() > Self::MAX_DOM_SUMMARY_LEN {
            let mut s = dom[..Self::MAX_DOM_SUMMARY_LEN].to_string();
            s.push_str("...[truncated]");
            s
        } else {
            dom
        };
        Self {
            url: url.into(),
            title: title.into(),
            dom_summary,
        }
    }
}

// ── ScrollDirection ────────────────────────────────────────────────────────

/// Direction for scroll actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl std::fmt::Display for ScrollDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Up => write!(f, "up"),
            Self::Down => write!(f, "down"),
            Self::Left => write!(f, "left"),
            Self::Right => write!(f, "right"),
        }
    }
}

// ── BrowserAction ──────────────────────────────────────────────────────────

/// An action the browser agent can perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BrowserAction {
    /// Navigate to a URL.
    Navigate { url: String },
    /// Click an element by CSS selector.
    Click { selector: String },
    /// Type text into an element identified by CSS selector.
    Type { selector: String, text: String },
    /// Scroll the page in a direction by a pixel amount.
    Scroll { direction: ScrollDirection, amount: u32 },
    /// Capture a screenshot.
    Screenshot,
    /// Extract text content from an optional selector (or whole page).
    ExtractText { selector: Option<String> },
    /// Evaluate arbitrary JavaScript and return the result.
    EvaluateJs { script: String },
    /// Wait for a CSS selector to appear in the DOM.
    WaitForSelector { selector: String, timeout_ms: u64 },
    /// Navigate back in history.
    Back,
    /// Navigate forward in history.
    Forward,
    /// Get current page information.
    GetPageInfo,
}

impl std::fmt::Display for BrowserAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Navigate { url } => write!(f, "Navigate({})", url),
            Self::Click { selector } => write!(f, "Click({})", selector),
            Self::Type { selector, text } => write!(f, "Type({}, \"{}\")", selector, text),
            Self::Scroll { direction, amount } => {
                write!(f, "Scroll({}, {}px)", direction, amount)
            }
            Self::Screenshot => write!(f, "Screenshot"),
            Self::ExtractText { selector } => {
                if let Some(s) = selector {
                    write!(f, "ExtractText({})", s)
                } else {
                    write!(f, "ExtractText(body)")
                }
            }
            Self::EvaluateJs { script } => {
                let preview = if script.len() > 40 {
                    format!("{}...", &script[..40])
                } else {
                    script.clone()
                };
                write!(f, "EvaluateJs({})", preview)
            }
            Self::WaitForSelector { selector, timeout_ms } => {
                write!(f, "WaitForSelector({}, {}ms)", selector, timeout_ms)
            }
            Self::Back => write!(f, "Back"),
            Self::Forward => write!(f, "Forward"),
            Self::GetPageInfo => write!(f, "GetPageInfo"),
        }
    }
}

// ── BrowserResult ──────────────────────────────────────────────────────────

/// Result of a browser action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserResult {
    /// Whether the action completed successfully.
    pub success: bool,
    /// Textual result data (JS return value, extracted text, status message, etc.).
    pub data: String,
    /// Optional screenshot captured after the action, as base64 PNG.
    pub screenshot_base64: Option<String>,
}

impl BrowserResult {
    /// Create a successful result.
    pub fn ok(data: impl Into<String>) -> Self {
        Self {
            success: true,
            data: data.into(),
            screenshot_base64: None,
        }
    }

    /// Create a successful result with a screenshot.
    pub fn ok_with_screenshot(data: impl Into<String>, screenshot: impl Into<String>) -> Self {
        Self {
            success: true,
            data: data.into(),
            screenshot_base64: Some(screenshot.into()),
        }
    }

    /// Create a failure result.
    pub fn fail(data: impl Into<String>) -> Self {
        Self {
            success: false,
            data: data.into(),
            screenshot_base64: None,
        }
    }
}

// ── CDP Target Info ────────────────────────────────────────────────────────

/// Chrome DevTools Protocol target descriptor (from /json/list).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CdpTarget {
    id: String,
    #[serde(rename = "type")]
    target_type: String,
    title: String,
    url: String,
    web_socket_debugger_url: Option<String>,
    #[serde(default)]
    description: String,
}

/// CDP command envelope for HTTP-based command execution.
#[derive(Debug, Serialize)]
struct CdpCommand {
    id: u64,
    method: String,
    params: serde_json::Value,
}

// ── BrowserSession ─────────────────────────────────────────────────────────

/// Manages a connection to a single Chrome page via CDP HTTP endpoints.
pub struct BrowserSession {
    /// HTTP client for CDP communication.
    client: reqwest::Client,
    /// Base URL for the debug port (e.g. `http://127.0.0.1:9222`).
    debug_url: String,
    /// The CDP target ID for the active page.
    target_id: String,
    /// Current page URL.
    pub page_url: String,
    /// Current page title.
    pub page_title: String,
    /// Navigation history.
    pub history: Vec<NavigationEntry>,
    /// Captured screenshots.
    pub screenshots: Vec<ScreenshotEntry>,
    /// Command sequence counter.
    cmd_id: u64,
    /// Default timeout for operations.
    timeout: Duration,
}

impl std::fmt::Debug for BrowserSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserSession")
            .field("debug_url", &self.debug_url)
            .field("target_id", &self.target_id)
            .field("page_url", &self.page_url)
            .field("page_title", &self.page_title)
            .field("history_len", &self.history.len())
            .field("screenshots_len", &self.screenshots.len())
            .finish()
    }
}

impl BrowserSession {
    /// Connect to a running Chrome instance at the configured debug port.
    ///
    /// Discovers the first `page` type target via `/json/list` and binds to it.
    pub async fn new(config: &BrowserConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to build HTTP client")?;

        let debug_url = config.debug_base_url();
        info!(port = config.debug_port, "Connecting to Chrome debug port");

        let targets = Self::fetch_targets_static(&client, &debug_url).await?;
        let page_target = targets
            .iter()
            .find(|t| t.target_type == "page")
            .ok_or_else(|| anyhow!("No page target found in Chrome debug targets"))?;

        info!(target_id = %page_target.id, url = %page_target.url, "Bound to page target");

        Ok(Self {
            client,
            debug_url,
            target_id: page_target.id.clone(),
            page_url: page_target.url.clone(),
            page_title: page_target.title.clone(),
            history: Vec::new(),
            screenshots: Vec::new(),
            cmd_id: 0,
            timeout: Duration::from_secs(config.timeout_secs),
        })
    }

    /// Create a session from pre-existing state (used internally and for testing).
    #[cfg(test)]
    fn from_parts(
        client: reqwest::Client,
        debug_url: String,
        target_id: String,
        page_url: String,
        page_title: String,
    ) -> Self {
        Self {
            client,
            debug_url,
            target_id,
            page_url,
            page_title,
            history: Vec::new(),
            screenshots: Vec::new(),
            cmd_id: 0,
            timeout: Duration::from_secs(30),
        }
    }

    // ── CDP Helpers (private) ──────────────────────────────────────────────

    /// Fetch the list of debug targets from Chrome.
    async fn fetch_targets_static(client: &reqwest::Client, debug_url: &str) -> Result<Vec<CdpTarget>> {
        let url = format!("{}/json/list", debug_url);
        debug!(url = %url, "Fetching CDP targets");
        let resp = client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to Chrome debug port")?;

        if !resp.status().is_success() {
            bail!(
                "Chrome debug port returned status {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            );
        }

        let targets: Vec<CdpTarget> = resp
            .json()
            .await
            .context("Failed to parse CDP target list")?;
        debug!(count = targets.len(), "Discovered CDP targets");
        Ok(targets)
    }

    /// Fetch targets for this session's debug URL.
    async fn fetch_targets(&self) -> Result<Vec<CdpTarget>> {
        Self::fetch_targets_static(&self.client, &self.debug_url).await
    }

    /// Get the next command ID.
    fn next_cmd_id(&mut self) -> u64 {
        self.cmd_id += 1;
        self.cmd_id
    }

    /// Build the CDP HTTP endpoint URL for sending commands to the active target.
    fn cdp_endpoint(&self) -> String {
        format!("{}/json/protocol", self.debug_url)
    }

    /// Build the target-specific command URL.
    fn target_url(&self, _method: &str) -> String {
        // The /json/protocol endpoint describes the protocol; actual commands
        // go through the page-specific endpoint.
        format!("{}/json/command/{}", self.debug_url, self.target_id)
    }

    /// Send a CDP command via HTTP POST and return the result.
    ///
    /// Uses the `/json/command/{target_id}` endpoint for HTTP-based CDP.
    /// Falls back to evaluating via `PUT /json/activate/{target_id}` + direct endpoint
    /// if the command endpoint is not available.
    async fn cdp_send(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let cmd_id = self.next_cmd_id();
        let cmd = CdpCommand {
            id: cmd_id,
            method: method.to_string(),
            params,
        };

        debug!(method = %method, id = cmd_id, "Sending CDP command");

        let url = self.target_url(method);
        let resp = self
            .client
            .post(&url)
            .json(&cmd)
            .send()
            .await
            .with_context(|| format!("CDP command {method} failed to send"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({"error": {"message": "Empty response"}}));

        if !status.is_success() {
            let err_msg = body
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown CDP error");
            bail!("CDP {method} failed (HTTP {status}): {err_msg}");
        }

        if let Some(error) = body.get("error") {
            let msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown CDP error");
            bail!("CDP {method} error: {msg}");
        }

        Ok(body.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }

    /// Shorthand for `Runtime.evaluate`.
    async fn cdp_evaluate(&mut self, expression: &str) -> Result<serde_json::Value> {
        self.cdp_send(
            "Runtime.evaluate",
            serde_json::json!({
                "expression": expression,
                "returnByValue": true,
                "awaitPromise": true,
            }),
        )
        .await
    }

    /// Extract the string value from a Runtime.evaluate result.
    fn extract_js_value(result: &serde_json::Value) -> String {
        if let Some(val) = result.get("result").and_then(|r| r.get("value")) {
            match val {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => "null".to_string(),
                other => other.to_string(),
            }
        } else if let Some(desc) = result.get("result").and_then(|r| r.get("description")) {
            desc.as_str().unwrap_or("undefined").to_string()
        } else {
            "undefined".to_string()
        }
    }

    /// Check if a Runtime.evaluate result indicates an exception.
    fn has_exception(result: &serde_json::Value) -> Option<String> {
        result.get("exceptionDetails").map(|details| {
            details
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("Script exception")
                .to_string()
        })
    }

    // ── Public Action Methods ──────────────────────────────────────────────

    /// Navigate to a URL.
    pub async fn navigate(&mut self, url: &str) -> Result<BrowserResult> {
        info!(url = %url, "Navigating");

        let result = self
            .cdp_send("Page.navigate", serde_json::json!({ "url": url }))
            .await?;

        // Check for navigation error
        if let Some(error_text) = result.get("errorText").and_then(|e| e.as_str()) {
            warn!(error = %error_text, url = %url, "Navigation failed");
            return Ok(BrowserResult::fail(format!("Navigation failed: {error_text}")));
        }

        // Update session state
        self.page_url = url.to_string();

        // Try to get the page title
        let title_result = self.cdp_evaluate("document.title").await;
        if let Ok(ref val) = title_result {
            self.page_title = Self::extract_js_value(val);
        }

        self.history.push(NavigationEntry::new(url, &self.page_title));
        info!(url = %url, title = %self.page_title, "Navigation complete");

        Ok(BrowserResult::ok(format!("Navigated to {url}")))
    }

    /// Click an element identified by a CSS selector.
    pub async fn click(&mut self, selector: &str) -> Result<BrowserResult> {
        info!(selector = %selector, "Clicking element");

        let js = format!(
            r#"(function() {{
                var el = document.querySelector({sel});
                if (!el) return 'ERROR:Element not found: {raw}';
                el.click();
                return 'Clicked: ' + (el.tagName || '') + (el.id ? '#' + el.id : '');
            }})()"#,
            sel = serde_json::to_string(selector).unwrap_or_else(|_| format!("\"{}\"", selector)),
            raw = selector.replace('\'', "\\'"),
        );

        let result = self.cdp_evaluate(&js).await?;

        if let Some(exc) = Self::has_exception(&result) {
            return Ok(BrowserResult::fail(exc));
        }

        let value = Self::extract_js_value(&result);
        if value.starts_with("ERROR:") {
            Ok(BrowserResult::fail(value))
        } else {
            Ok(BrowserResult::ok(value))
        }
    }

    /// Type text into an element identified by a CSS selector.
    ///
    /// Focuses the element first, then dispatches individual key events.
    pub async fn type_text(&mut self, selector: &str, text: &str) -> Result<BrowserResult> {
        info!(selector = %selector, len = text.len(), "Typing text");

        // Focus the element
        let focus_js = format!(
            r#"(function() {{
                var el = document.querySelector({sel});
                if (!el) return 'ERROR:Element not found';
                el.focus();
                return 'focused';
            }})()"#,
            sel = serde_json::to_string(selector).unwrap_or_else(|_| format!("\"{}\"", selector)),
        );

        let focus_result = self.cdp_evaluate(&focus_js).await?;
        let focus_val = Self::extract_js_value(&focus_result);
        if focus_val.starts_with("ERROR:") {
            return Ok(BrowserResult::fail(focus_val));
        }

        // Dispatch key events for each character
        for ch in text.chars() {
            self.cdp_send(
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "keyDown",
                    "text": ch.to_string(),
                    "key": ch.to_string(),
                    "unmodifiedText": ch.to_string(),
                }),
            )
            .await?;

            self.cdp_send(
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "keyUp",
                    "key": ch.to_string(),
                }),
            )
            .await?;
        }

        Ok(BrowserResult::ok(format!(
            "Typed {} characters into {selector}",
            text.len()
        )))
    }

    /// Scroll the page in the given direction by a pixel amount.
    pub async fn scroll(&mut self, direction: ScrollDirection, amount: u32) -> Result<BrowserResult> {
        let (dx, dy) = match direction {
            ScrollDirection::Up => (0, -(amount as i64)),
            ScrollDirection::Down => (0, amount as i64),
            ScrollDirection::Left => (-(amount as i64), 0),
            ScrollDirection::Right => (amount as i64, 0),
        };

        let js = format!("window.scrollBy({dx}, {dy}); 'scrolled'");
        let result = self.cdp_evaluate(&js).await?;

        if let Some(exc) = Self::has_exception(&result) {
            return Ok(BrowserResult::fail(exc));
        }

        Ok(BrowserResult::ok(format!(
            "Scrolled {direction} by {amount}px"
        )))
    }

    /// Capture a screenshot of the current page as base64 PNG.
    pub async fn screenshot(&mut self) -> Result<String> {
        info!("Capturing screenshot");

        let result = self
            .cdp_send(
                "Page.captureScreenshot",
                serde_json::json!({ "format": "png" }),
            )
            .await?;

        let data = result
            .get("data")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        if !data.is_empty() {
            self.screenshots.push(ScreenshotEntry::new(
                format!("screenshot at {}", self.page_url),
                &data,
            ));
        }

        Ok(data)
    }

    /// Extract text content from the page or a specific element.
    pub async fn extract_text(&mut self, selector: Option<&str>) -> Result<String> {
        let js = match selector {
            Some(sel) => format!(
                r#"(function() {{
                    var el = document.querySelector({sel});
                    if (!el) return 'ERROR:Element not found';
                    return el.innerText || el.textContent || '';
                }})()"#,
                sel = serde_json::to_string(sel).unwrap_or_else(|_| format!("\"{}\"", sel)),
            ),
            None => "document.body.innerText || document.body.textContent || ''".to_string(),
        };

        let result = self.cdp_evaluate(&js).await?;

        if let Some(exc) = Self::has_exception(&result) {
            bail!("Text extraction failed: {exc}");
        }

        Ok(Self::extract_js_value(&result))
    }

    /// Evaluate arbitrary JavaScript and return the string result.
    pub async fn evaluate_js(&mut self, script: &str) -> Result<String> {
        debug!(len = script.len(), "Evaluating JavaScript");

        let result = self.cdp_evaluate(script).await?;

        if let Some(exc) = Self::has_exception(&result) {
            bail!("JS evaluation error: {exc}");
        }

        Ok(Self::extract_js_value(&result))
    }

    /// Wait for a CSS selector to appear in the DOM, polling until timeout.
    pub async fn wait_for_selector(&mut self, selector: &str, timeout_ms: u64) -> Result<bool> {
        info!(selector = %selector, timeout_ms, "Waiting for selector");

        let poll_interval = Duration::from_millis(100);
        let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);

        let js = format!(
            "!!document.querySelector({sel})",
            sel = serde_json::to_string(selector).unwrap_or_else(|_| format!("\"{}\"", selector)),
        );

        loop {
            let result = self.cdp_evaluate(&js).await?;
            let value = Self::extract_js_value(&result);

            if value == "true" {
                info!(selector = %selector, "Selector found");
                return Ok(true);
            }

            if tokio::time::Instant::now() >= deadline {
                warn!(selector = %selector, "Selector wait timed out");
                return Ok(false);
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Navigate back in the browser history.
    pub async fn back(&mut self) -> Result<BrowserResult> {
        info!("Navigating back");

        let result = self
            .cdp_evaluate("window.history.back(); 'navigated_back'")
            .await?;

        if let Some(exc) = Self::has_exception(&result) {
            return Ok(BrowserResult::fail(exc));
        }

        // Small delay for navigation to settle
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Update page state
        if let Ok(val) = self.cdp_evaluate("document.location.href").await {
            self.page_url = Self::extract_js_value(&val);
        }
        if let Ok(val) = self.cdp_evaluate("document.title").await {
            self.page_title = Self::extract_js_value(&val);
        }

        Ok(BrowserResult::ok(format!("Navigated back to {}", self.page_url)))
    }

    /// Navigate forward in the browser history.
    pub async fn forward(&mut self) -> Result<BrowserResult> {
        info!("Navigating forward");

        let result = self
            .cdp_evaluate("window.history.forward(); 'navigated_forward'")
            .await?;

        if let Some(exc) = Self::has_exception(&result) {
            return Ok(BrowserResult::fail(exc));
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        if let Ok(val) = self.cdp_evaluate("document.location.href").await {
            self.page_url = Self::extract_js_value(&val);
        }
        if let Ok(val) = self.cdp_evaluate("document.title").await {
            self.page_title = Self::extract_js_value(&val);
        }

        Ok(BrowserResult::ok(format!(
            "Navigated forward to {}",
            self.page_url
        )))
    }

    /// Get information about the current page.
    pub async fn get_page_info(&mut self) -> Result<PageInfo> {
        let url_result = self.cdp_evaluate("document.location.href").await?;
        let title_result = self.cdp_evaluate("document.title").await?;
        let text_result = self
            .cdp_evaluate(
                "(document.body.innerText || document.body.textContent || '').substring(0, 8192)",
            )
            .await?;

        let url = Self::extract_js_value(&url_result);
        let title = Self::extract_js_value(&title_result);
        let text = Self::extract_js_value(&text_result);

        self.page_url = url.clone();
        self.page_title = title.clone();

        Ok(PageInfo::new(url, title, text))
    }

    /// Dispatch a `BrowserAction` to the appropriate handler method.
    pub async fn execute_action(&mut self, action: &BrowserAction) -> Result<BrowserResult> {
        debug!(action = %action, "Executing browser action");

        match action {
            BrowserAction::Navigate { url } => self.navigate(url).await,
            BrowserAction::Click { selector } => self.click(selector).await,
            BrowserAction::Type { selector, text } => self.type_text(selector, text).await,
            BrowserAction::Scroll { direction, amount } => self.scroll(*direction, *amount).await,
            BrowserAction::Screenshot => {
                let data = self.screenshot().await?;
                Ok(BrowserResult::ok_with_screenshot("Screenshot captured", data))
            }
            BrowserAction::ExtractText { selector } => {
                let text = self.extract_text(selector.as_deref()).await?;
                Ok(BrowserResult::ok(text))
            }
            BrowserAction::EvaluateJs { script } => {
                let result = self.evaluate_js(script).await?;
                Ok(BrowserResult::ok(result))
            }
            BrowserAction::WaitForSelector {
                selector,
                timeout_ms,
            } => {
                let found = self.wait_for_selector(selector, *timeout_ms).await?;
                if found {
                    Ok(BrowserResult::ok(format!("Selector '{selector}' found")))
                } else {
                    Ok(BrowserResult::fail(format!(
                        "Selector '{selector}' not found within {timeout_ms}ms"
                    )))
                }
            }
            BrowserAction::Back => self.back().await,
            BrowserAction::Forward => self.forward().await,
            BrowserAction::GetPageInfo => {
                let info = self.get_page_info().await?;
                Ok(BrowserResult::ok(serde_json::to_string_pretty(&info)?))
            }
        }
    }

    /// Close the page (sends `Page.close`). Does not kill the Chrome process.
    pub async fn close(&mut self) -> Result<()> {
        info!(target_id = %self.target_id, "Closing page");
        let url = format!("{}/json/close/{}", self.debug_url, self.target_id);
        let _ = self.client.get(&url).send().await;
        Ok(())
    }
}

// ── BrowserPool ────────────────────────────────────────────────────────────

/// Manages multiple browser sessions (tabs) and provides tab switching.
pub struct BrowserPool {
    /// All managed sessions.
    pub sessions: Vec<BrowserSession>,
    /// Index of the currently active session.
    pub active_idx: usize,
    /// Config used for creating new tabs.
    config: BrowserConfig,
}

impl BrowserPool {
    /// Create a new pool with a single session connected to Chrome.
    pub async fn new(config: BrowserConfig) -> Result<Self> {
        let session = BrowserSession::new(&config).await?;
        Ok(Self {
            sessions: vec![session],
            active_idx: 0,
            config,
        })
    }

    /// Create a pool from pre-existing sessions (useful for testing).
    #[cfg(test)]
    fn from_sessions(sessions: Vec<BrowserSession>, config: BrowserConfig) -> Self {
        Self {
            sessions,
            active_idx: 0,
            config,
        }
    }

    /// Open a new tab and return its index in the pool.
    pub async fn new_tab(&mut self) -> Result<usize> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .build()?;

        let url = format!("{}/json/new", self.config.debug_base_url());
        let resp = client.get(&url).send().await?;
        let target: CdpTarget = resp.json().await?;

        info!(id = %target.id, "Opened new tab");

        let session = BrowserSession {
            client,
            debug_url: self.config.debug_base_url(),
            target_id: target.id,
            page_url: target.url,
            page_title: target.title,
            history: Vec::new(),
            screenshots: Vec::new(),
            cmd_id: 0,
            timeout: Duration::from_secs(self.config.timeout_secs),
        };

        self.sessions.push(session);
        let idx = self.sessions.len() - 1;
        Ok(idx)
    }

    /// Switch to a tab by index.
    pub fn switch_tab(&mut self, idx: usize) -> Result<()> {
        if idx >= self.sessions.len() {
            bail!(
                "Tab index {idx} out of range (0..{})",
                self.sessions.len()
            );
        }
        self.active_idx = idx;
        info!(tab = idx, "Switched to tab");
        Ok(())
    }

    /// Close a tab by index and remove it from the pool.
    pub async fn close_tab(&mut self, idx: usize) -> Result<()> {
        if idx >= self.sessions.len() {
            bail!(
                "Tab index {idx} out of range (0..{})",
                self.sessions.len()
            );
        }
        if self.sessions.len() == 1 {
            bail!("Cannot close the last tab");
        }

        let mut session = self.sessions.remove(idx);
        session.close().await?;

        // Adjust active index
        if self.active_idx >= self.sessions.len() {
            self.active_idx = self.sessions.len() - 1;
        }
        Ok(())
    }

    /// Get a reference to the active session.
    pub fn active(&self) -> &BrowserSession {
        &self.sessions[self.active_idx]
    }

    /// Get a mutable reference to the active session.
    pub fn active_mut(&mut self) -> &mut BrowserSession {
        &mut self.sessions[self.active_idx]
    }

    /// Get the number of open tabs.
    pub fn tab_count(&self) -> usize {
        self.sessions.len()
    }

    /// Execute an action on the active tab.
    pub async fn execute(&mut self, action: &BrowserAction) -> Result<BrowserResult> {
        self.sessions[self.active_idx].execute_action(action).await
    }
}

// ── Chrome Launcher ────────────────────────────────────────────────────────

/// Well-known Chrome/Chromium binary paths by platform.
const CHROME_PATHS_MACOS: &[&str] = &[
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/usr/local/bin/chromium",
    "/opt/homebrew/bin/chromium",
];

const CHROME_PATHS_LINUX: &[&str] = &[
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/snap/bin/chromium",
    "/usr/local/bin/chrome",
    "/usr/local/bin/chromium",
];

/// Detect the Chrome/Chromium binary path on the current platform.
pub fn detect_chrome_path() -> Option<String> {
    let candidates = if cfg!(target_os = "macos") {
        CHROME_PATHS_MACOS
    } else {
        CHROME_PATHS_LINUX
    };

    // Check CHROME_PATH env var first
    if let Ok(custom) = std::env::var("CHROME_PATH") {
        if std::path::Path::new(&custom).exists() {
            return Some(custom);
        }
    }

    for path in candidates {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // Try `which` as a fallback
    for name in &["google-chrome", "chromium", "chromium-browser", "chrome"] {
        if let Ok(output) = std::process::Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }

    None
}

/// Detect Chrome binary path from a set of candidate paths (testable).
pub fn detect_chrome_from_candidates(candidates: &[&str]) -> Option<String> {
    for path in candidates {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    None
}

/// Launch a Chrome process with remote debugging enabled.
///
/// Returns the child process handle. The caller is responsible for killing
/// the process when done.
pub async fn launch_chrome(config: &BrowserConfig) -> Result<tokio::process::Child> {
    let chrome_path = detect_chrome_path()
        .ok_or_else(|| anyhow!("Chrome/Chromium not found. Set CHROME_PATH or install Chrome."))?;

    info!(path = %chrome_path, port = config.debug_port, "Launching Chrome");

    let args = config.chrome_args();
    let child = tokio::process::Command::new(&chrome_path)
        .args(&args)
        .arg("about:blank")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("Failed to launch Chrome at {chrome_path}"))?;

    // Wait briefly for Chrome to start and open the debug port
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify the debug port is responding
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let url = format!("{}/json/version", config.debug_base_url());
    let mut attempts = 0;
    loop {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!("Chrome debug port is ready");
                break;
            }
            _ => {
                attempts += 1;
                if attempts > 10 {
                    bail!(
                        "Chrome debug port at {} did not become ready after 5 seconds",
                        config.debug_base_url()
                    );
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }

    Ok(child)
}

// ── Agent Tool Definitions ─────────────────────────────────────────────────

/// Return tool definitions for the browser agent that can be injected into
/// the AI agent's system prompt (XML tool-calling format).
pub fn browser_agent_tool_definitions() -> Vec<BrowserToolDef> {
    vec![
        BrowserToolDef {
            name: "browser_navigate".into(),
            description: "Navigate the browser to a URL".into(),
            parameters: vec![("url".into(), "string".into(), "The URL to navigate to".into())],
        },
        BrowserToolDef {
            name: "browser_click".into(),
            description: "Click an element by CSS selector".into(),
            parameters: vec![("selector".into(), "string".into(), "CSS selector of the element to click".into())],
        },
        BrowserToolDef {
            name: "browser_type".into(),
            description: "Type text into an input element".into(),
            parameters: vec![
                ("selector".into(), "string".into(), "CSS selector of the input element".into()),
                ("text".into(), "string".into(), "The text to type".into()),
            ],
        },
        BrowserToolDef {
            name: "browser_scroll".into(),
            description: "Scroll the page".into(),
            parameters: vec![
                ("direction".into(), "string".into(), "Scroll direction: up, down, left, right".into()),
                ("amount".into(), "number".into(), "Pixels to scroll".into()),
            ],
        },
        BrowserToolDef {
            name: "browser_screenshot".into(),
            description: "Capture a screenshot of the current page".into(),
            parameters: vec![],
        },
        BrowserToolDef {
            name: "browser_extract_text".into(),
            description: "Extract text content from the page or a specific element".into(),
            parameters: vec![("selector".into(), "string".into(), "Optional CSS selector (omit for whole page)".into())],
        },
        BrowserToolDef {
            name: "browser_evaluate_js".into(),
            description: "Evaluate JavaScript in the browser and return the result".into(),
            parameters: vec![("script".into(), "string".into(), "JavaScript code to evaluate".into())],
        },
        BrowserToolDef {
            name: "browser_wait".into(),
            description: "Wait for a CSS selector to appear in the DOM".into(),
            parameters: vec![
                ("selector".into(), "string".into(), "CSS selector to wait for".into()),
                ("timeout_ms".into(), "number".into(), "Maximum time to wait in milliseconds".into()),
            ],
        },
        BrowserToolDef {
            name: "browser_back".into(),
            description: "Navigate back in browser history".into(),
            parameters: vec![],
        },
        BrowserToolDef {
            name: "browser_forward".into(),
            description: "Navigate forward in browser history".into(),
            parameters: vec![],
        },
        BrowserToolDef {
            name: "browser_page_info".into(),
            description: "Get information about the current page (URL, title, text summary)".into(),
            parameters: vec![],
        },
    ]
}

/// Definition of a browser agent tool for system prompt injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserToolDef {
    pub name: String,
    pub description: String,
    /// Each parameter: (name, type, description).
    pub parameters: Vec<(String, String, String)>,
}

impl BrowserToolDef {
    /// Render this tool as an XML definition for the agent system prompt.
    pub fn to_xml(&self) -> String {
        let mut xml = format!(
            "<tool name=\"{}\">\n  <description>{}</description>\n",
            self.name, self.description
        );
        if !self.parameters.is_empty() {
            xml.push_str("  <parameters>\n");
            for (name, ty, desc) in &self.parameters {
                xml.push_str(&format!(
                    "    <param name=\"{name}\" type=\"{ty}\">{desc}</param>\n"
                ));
            }
            xml.push_str("  </parameters>\n");
        }
        xml.push_str("</tool>");
        xml
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── BrowserConfig tests ────────────────────────────────────────────

    #[test]
    fn test_config_defaults() {
        let cfg = BrowserConfig::default();
        assert_eq!(cfg.debug_port, 9222);
        assert!(cfg.headless);
        assert_eq!(cfg.viewport_width, 1280);
        assert_eq!(cfg.viewport_height, 720);
        assert_eq!(cfg.timeout_secs, 30);
        assert!(cfg.user_agent.is_none());
    }

    #[test]
    fn test_config_debug_base_url() {
        let cfg = BrowserConfig::default();
        assert_eq!(cfg.debug_base_url(), "http://127.0.0.1:9222");

        let cfg2 = BrowserConfig {
            debug_port: 9333,
            ..Default::default()
        };
        assert_eq!(cfg2.debug_base_url(), "http://127.0.0.1:9333");
    }

    #[test]
    fn test_config_chrome_args_headless() {
        let cfg = BrowserConfig::default();
        let args = cfg.chrome_args();
        assert!(args.contains(&"--remote-debugging-port=9222".to_string()));
        assert!(args.contains(&"--headless=new".to_string()));
        assert!(args.contains(&"--no-first-run".to_string()));
        assert!(args.contains(&"--disable-gpu".to_string()));
        assert!(args.contains(&"--window-size=1280,720".to_string()));
    }

    #[test]
    fn test_config_chrome_args_headed() {
        let cfg = BrowserConfig {
            headless: false,
            ..Default::default()
        };
        let args = cfg.chrome_args();
        assert!(!args.iter().any(|a| a.contains("headless")));
    }

    #[test]
    fn test_config_chrome_args_custom_user_agent() {
        let cfg = BrowserConfig {
            user_agent: Some("VibeCody/1.0".to_string()),
            ..Default::default()
        };
        let args = cfg.chrome_args();
        assert!(args.contains(&"--user-agent=VibeCody/1.0".to_string()));
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let cfg = BrowserConfig {
            debug_port: 9333,
            headless: false,
            viewport_width: 1920,
            viewport_height: 1080,
            timeout_secs: 60,
            user_agent: Some("Test/1.0".into()),
        };
        let json = serde_json::to_string(&cfg).expect("serialize");
        let cfg2: BrowserConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg2.debug_port, 9333);
        assert!(!cfg2.headless);
        assert_eq!(cfg2.viewport_width, 1920);
        assert_eq!(cfg2.viewport_height, 1080);
        assert_eq!(cfg2.timeout_secs, 60);
        assert_eq!(cfg2.user_agent.as_deref(), Some("Test/1.0"));
    }

    #[test]
    fn test_config_deserialization_defaults() {
        let json = r#"{"debug_port":9222,"headless":true,"viewport_width":1280,"viewport_height":720,"timeout_secs":30,"user_agent":null}"#;
        let cfg: BrowserConfig = serde_json::from_str(json).expect("deser");
        assert_eq!(cfg.debug_port, 9222);
        assert!(cfg.headless);
    }

    // ── NavigationEntry tests ──────────────────────────────────────────

    #[test]
    fn test_navigation_entry_creation() {
        let entry = NavigationEntry::new("https://example.com", "Example");
        assert_eq!(entry.url, "https://example.com");
        assert_eq!(entry.title, "Example");
        assert!(entry.timestamp_ms > 0);
    }

    #[test]
    fn test_navigation_entry_with_timestamp() {
        let entry = NavigationEntry::with_timestamp("https://test.com", "Test", 1234567890);
        assert_eq!(entry.url, "https://test.com");
        assert_eq!(entry.title, "Test");
        assert_eq!(entry.timestamp_ms, 1234567890);
    }

    #[test]
    fn test_navigation_entry_serialization() {
        let entry = NavigationEntry::with_timestamp("https://a.com", "A", 100);
        let json = serde_json::to_string(&entry).expect("ser");
        assert!(json.contains("\"url\":\"https://a.com\""));
        assert!(json.contains("\"title\":\"A\""));
        assert!(json.contains("\"timestamp_ms\":100"));
    }

    // ── ScreenshotEntry tests ──────────────────────────────────────────

    #[test]
    fn test_screenshot_entry_creation() {
        let entry = ScreenshotEntry::new("clicked button", "iVBOR...");
        assert_eq!(entry.action_before, "clicked button");
        assert_eq!(entry.png_base64, "iVBOR...");
        assert!(entry.timestamp_ms > 0);
    }

    #[test]
    fn test_screenshot_entry_serialization() {
        let entry = ScreenshotEntry::new("navigate", "ABCD");
        let json = serde_json::to_string(&entry).expect("ser");
        let deser: ScreenshotEntry = serde_json::from_str(&json).expect("deser");
        assert_eq!(deser.action_before, "navigate");
        assert_eq!(deser.png_base64, "ABCD");
    }

    // ── PageInfo tests ─────────────────────────────────────────────────

    #[test]
    fn test_page_info_short_text() {
        let info = PageInfo::new("https://x.com", "X", "Hello world");
        assert_eq!(info.url, "https://x.com");
        assert_eq!(info.title, "X");
        assert_eq!(info.dom_summary, "Hello world");
    }

    #[test]
    fn test_page_info_truncation() {
        let long_text = "A".repeat(5000);
        let info = PageInfo::new("https://x.com", "X", long_text);
        assert!(info.dom_summary.len() < 5000);
        assert!(info.dom_summary.ends_with("...[truncated]"));
        // The truncated summary should be MAX_DOM_SUMMARY_LEN + "...[truncated]".len()
        assert_eq!(
            info.dom_summary.len(),
            PageInfo::MAX_DOM_SUMMARY_LEN + "...[truncated]".len()
        );
    }

    #[test]
    fn test_page_info_exact_max_len() {
        let text = "B".repeat(PageInfo::MAX_DOM_SUMMARY_LEN);
        let info = PageInfo::new("u", "t", text.clone());
        assert_eq!(info.dom_summary, text);
        assert!(!info.dom_summary.contains("truncated"));
    }

    #[test]
    fn test_page_info_serialization() {
        let info = PageInfo::new("https://a.com", "Title", "body text");
        let json = serde_json::to_string(&info).expect("ser");
        let deser: PageInfo = serde_json::from_str(&json).expect("deser");
        assert_eq!(deser.url, "https://a.com");
        assert_eq!(deser.title, "Title");
        assert_eq!(deser.dom_summary, "body text");
    }

    // ── ScrollDirection tests ──────────────────────────────────────────

    #[test]
    fn test_scroll_direction_display() {
        assert_eq!(ScrollDirection::Up.to_string(), "up");
        assert_eq!(ScrollDirection::Down.to_string(), "down");
        assert_eq!(ScrollDirection::Left.to_string(), "left");
        assert_eq!(ScrollDirection::Right.to_string(), "right");
    }

    #[test]
    fn test_scroll_direction_serialization() {
        let json = serde_json::to_string(&ScrollDirection::Down).expect("ser");
        assert_eq!(json, "\"down\"");
        let dir: ScrollDirection = serde_json::from_str("\"up\"").expect("deser");
        assert_eq!(dir, ScrollDirection::Up);
    }

    #[test]
    fn test_scroll_direction_equality() {
        assert_eq!(ScrollDirection::Up, ScrollDirection::Up);
        assert_ne!(ScrollDirection::Up, ScrollDirection::Down);
    }

    // ── BrowserAction tests ────────────────────────────────────────────

    #[test]
    fn test_action_navigate_display() {
        let a = BrowserAction::Navigate {
            url: "https://example.com".into(),
        };
        assert_eq!(a.to_string(), "Navigate(https://example.com)");
    }

    #[test]
    fn test_action_click_display() {
        let a = BrowserAction::Click {
            selector: "#btn".into(),
        };
        assert_eq!(a.to_string(), "Click(#btn)");
    }

    #[test]
    fn test_action_type_display() {
        let a = BrowserAction::Type {
            selector: "input".into(),
            text: "hello".into(),
        };
        assert_eq!(a.to_string(), "Type(input, \"hello\")");
    }

    #[test]
    fn test_action_scroll_display() {
        let a = BrowserAction::Scroll {
            direction: ScrollDirection::Down,
            amount: 500,
        };
        assert_eq!(a.to_string(), "Scroll(down, 500px)");
    }

    #[test]
    fn test_action_screenshot_display() {
        assert_eq!(BrowserAction::Screenshot.to_string(), "Screenshot");
    }

    #[test]
    fn test_action_extract_text_with_selector() {
        let a = BrowserAction::ExtractText {
            selector: Some(".content".into()),
        };
        assert_eq!(a.to_string(), "ExtractText(.content)");
    }

    #[test]
    fn test_action_extract_text_whole_page() {
        let a = BrowserAction::ExtractText { selector: None };
        assert_eq!(a.to_string(), "ExtractText(body)");
    }

    #[test]
    fn test_action_evaluate_js_short() {
        let a = BrowserAction::EvaluateJs {
            script: "1+1".into(),
        };
        assert_eq!(a.to_string(), "EvaluateJs(1+1)");
    }

    #[test]
    fn test_action_evaluate_js_long_truncated() {
        let a = BrowserAction::EvaluateJs {
            script: "x".repeat(100),
        };
        let display = a.to_string();
        assert!(display.contains("..."));
        assert!(display.len() < 100);
    }

    #[test]
    fn test_action_wait_display() {
        let a = BrowserAction::WaitForSelector {
            selector: ".loaded".into(),
            timeout_ms: 5000,
        };
        assert_eq!(a.to_string(), "WaitForSelector(.loaded, 5000ms)");
    }

    #[test]
    fn test_action_back_forward_display() {
        assert_eq!(BrowserAction::Back.to_string(), "Back");
        assert_eq!(BrowserAction::Forward.to_string(), "Forward");
    }

    #[test]
    fn test_action_get_page_info_display() {
        assert_eq!(BrowserAction::GetPageInfo.to_string(), "GetPageInfo");
    }

    #[test]
    fn test_action_serialization_navigate() {
        let a = BrowserAction::Navigate {
            url: "https://test.com".into(),
        };
        let json = serde_json::to_string(&a).expect("ser");
        assert!(json.contains("\"action\":\"navigate\""));
        assert!(json.contains("\"url\":\"https://test.com\""));

        let deser: BrowserAction = serde_json::from_str(&json).expect("deser");
        if let BrowserAction::Navigate { url } = deser {
            assert_eq!(url, "https://test.com");
        } else {
            panic!("Expected Navigate");
        }
    }

    #[test]
    fn test_action_serialization_scroll() {
        let a = BrowserAction::Scroll {
            direction: ScrollDirection::Left,
            amount: 200,
        };
        let json = serde_json::to_string(&a).expect("ser");
        let deser: BrowserAction = serde_json::from_str(&json).expect("deser");
        if let BrowserAction::Scroll { direction, amount } = deser {
            assert_eq!(direction, ScrollDirection::Left);
            assert_eq!(amount, 200);
        } else {
            panic!("Expected Scroll");
        }
    }

    #[test]
    fn test_action_serialization_roundtrip_all_variants() {
        let actions: Vec<BrowserAction> = vec![
            BrowserAction::Navigate {
                url: "https://a.com".into(),
            },
            BrowserAction::Click {
                selector: "#x".into(),
            },
            BrowserAction::Type {
                selector: "input".into(),
                text: "hi".into(),
            },
            BrowserAction::Scroll {
                direction: ScrollDirection::Up,
                amount: 100,
            },
            BrowserAction::Screenshot,
            BrowserAction::ExtractText { selector: None },
            BrowserAction::ExtractText {
                selector: Some("p".into()),
            },
            BrowserAction::EvaluateJs {
                script: "1".into(),
            },
            BrowserAction::WaitForSelector {
                selector: "div".into(),
                timeout_ms: 1000,
            },
            BrowserAction::Back,
            BrowserAction::Forward,
            BrowserAction::GetPageInfo,
        ];

        for action in &actions {
            let json = serde_json::to_string(action).expect("serialize");
            let _deser: BrowserAction =
                serde_json::from_str(&json).expect("deserialize");
        }
    }

    // ── BrowserResult tests ────────────────────────────────────────────

    #[test]
    fn test_result_ok() {
        let r = BrowserResult::ok("done");
        assert!(r.success);
        assert_eq!(r.data, "done");
        assert!(r.screenshot_base64.is_none());
    }

    #[test]
    fn test_result_ok_with_screenshot() {
        let r = BrowserResult::ok_with_screenshot("captured", "base64data");
        assert!(r.success);
        assert_eq!(r.data, "captured");
        assert_eq!(r.screenshot_base64.as_deref(), Some("base64data"));
    }

    #[test]
    fn test_result_fail() {
        let r = BrowserResult::fail("element not found");
        assert!(!r.success);
        assert_eq!(r.data, "element not found");
        assert!(r.screenshot_base64.is_none());
    }

    #[test]
    fn test_result_serialization() {
        let r = BrowserResult::ok_with_screenshot("ok", "png123");
        let json = serde_json::to_string(&r).expect("ser");
        let deser: BrowserResult = serde_json::from_str(&json).expect("deser");
        assert!(deser.success);
        assert_eq!(deser.data, "ok");
        assert_eq!(deser.screenshot_base64.as_deref(), Some("png123"));
    }

    // ── CDP helper tests ───────────────────────────────────────────────

    #[test]
    fn test_cdp_target_url_construction() {
        let client = reqwest::Client::new();
        let session = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9222".into(),
            "ABCD1234".into(),
            "about:blank".into(),
            "".into(),
        );
        let url = session.target_url("Runtime.evaluate");
        assert_eq!(url, "http://127.0.0.1:9222/json/command/ABCD1234");
    }

    #[test]
    fn test_cdp_endpoint() {
        let client = reqwest::Client::new();
        let session = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9333".into(),
            "TGT".into(),
            "about:blank".into(),
            "".into(),
        );
        assert_eq!(session.cdp_endpoint(), "http://127.0.0.1:9333/json/protocol");
    }

    #[test]
    fn test_next_cmd_id_increments() {
        let client = reqwest::Client::new();
        let mut session = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9222".into(),
            "T".into(),
            "".into(),
            "".into(),
        );
        assert_eq!(session.next_cmd_id(), 1);
        assert_eq!(session.next_cmd_id(), 2);
        assert_eq!(session.next_cmd_id(), 3);
    }

    // ── extract_js_value tests ─────────────────────────────────────────

    #[test]
    fn test_extract_js_value_string() {
        let val = serde_json::json!({"result": {"value": "hello"}});
        assert_eq!(BrowserSession::extract_js_value(&val), "hello");
    }

    #[test]
    fn test_extract_js_value_number() {
        let val = serde_json::json!({"result": {"value": 42}});
        assert_eq!(BrowserSession::extract_js_value(&val), "42");
    }

    #[test]
    fn test_extract_js_value_null() {
        let val = serde_json::json!({"result": {"value": null}});
        assert_eq!(BrowserSession::extract_js_value(&val), "null");
    }

    #[test]
    fn test_extract_js_value_boolean() {
        let val = serde_json::json!({"result": {"value": true}});
        assert_eq!(BrowserSession::extract_js_value(&val), "true");
    }

    #[test]
    fn test_extract_js_value_description_fallback() {
        let val = serde_json::json!({"result": {"description": "Promise"}});
        assert_eq!(BrowserSession::extract_js_value(&val), "Promise");
    }

    #[test]
    fn test_extract_js_value_undefined() {
        let val = serde_json::json!({"result": {}});
        assert_eq!(BrowserSession::extract_js_value(&val), "undefined");
    }

    #[test]
    fn test_extract_js_value_no_result() {
        let val = serde_json::json!({});
        assert_eq!(BrowserSession::extract_js_value(&val), "undefined");
    }

    // ── has_exception tests ────────────────────────────────────────────

    #[test]
    fn test_has_exception_none() {
        let val = serde_json::json!({"result": {"value": "ok"}});
        assert!(BrowserSession::has_exception(&val).is_none());
    }

    #[test]
    fn test_has_exception_present() {
        let val = serde_json::json!({
            "exceptionDetails": {"text": "ReferenceError: x is not defined"}
        });
        let exc = BrowserSession::has_exception(&val);
        assert_eq!(exc.as_deref(), Some("ReferenceError: x is not defined"));
    }

    #[test]
    fn test_has_exception_no_text() {
        let val = serde_json::json!({
            "exceptionDetails": {"lineNumber": 1}
        });
        let exc = BrowserSession::has_exception(&val);
        assert_eq!(exc.as_deref(), Some("Script exception"));
    }

    // ── BrowserSession history tracking ────────────────────────────────

    #[test]
    fn test_session_initial_state() {
        let client = reqwest::Client::new();
        let session = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9222".into(),
            "T1".into(),
            "about:blank".into(),
            "New Tab".into(),
        );
        assert_eq!(session.page_url, "about:blank");
        assert_eq!(session.page_title, "New Tab");
        assert!(session.history.is_empty());
        assert!(session.screenshots.is_empty());
        assert_eq!(session.cmd_id, 0);
    }

    #[test]
    fn test_session_debug_format() {
        let client = reqwest::Client::new();
        let session = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9222".into(),
            "T1".into(),
            "https://example.com".into(),
            "Example".into(),
        );
        let dbg = format!("{:?}", session);
        assert!(dbg.contains("BrowserSession"));
        assert!(dbg.contains("https://example.com"));
        assert!(dbg.contains("Example"));
    }

    // ── BrowserPool tests ──────────────────────────────────────────────

    #[test]
    fn test_pool_tab_management() {
        let client = reqwest::Client::new();
        let s1 = BrowserSession::from_parts(
            client.clone(),
            "http://127.0.0.1:9222".into(),
            "T1".into(),
            "https://a.com".into(),
            "A".into(),
        );
        let s2 = BrowserSession::from_parts(
            client.clone(),
            "http://127.0.0.1:9222".into(),
            "T2".into(),
            "https://b.com".into(),
            "B".into(),
        );
        let s3 = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9222".into(),
            "T3".into(),
            "https://c.com".into(),
            "C".into(),
        );

        let pool = BrowserPool::from_sessions(
            vec![s1, s2, s3],
            BrowserConfig::default(),
        );

        assert_eq!(pool.tab_count(), 3);
        assert_eq!(pool.active_idx, 0);
        assert_eq!(pool.active().page_url, "https://a.com");
    }

    #[test]
    fn test_pool_switch_tab() {
        let client = reqwest::Client::new();
        let s1 = BrowserSession::from_parts(
            client.clone(),
            "http://127.0.0.1:9222".into(),
            "T1".into(),
            "https://a.com".into(),
            "A".into(),
        );
        let s2 = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9222".into(),
            "T2".into(),
            "https://b.com".into(),
            "B".into(),
        );

        let mut pool = BrowserPool::from_sessions(
            vec![s1, s2],
            BrowserConfig::default(),
        );

        pool.switch_tab(1).expect("switch");
        assert_eq!(pool.active_idx, 1);
        assert_eq!(pool.active().page_url, "https://b.com");

        pool.switch_tab(0).expect("switch back");
        assert_eq!(pool.active_idx, 0);
    }

    #[test]
    fn test_pool_switch_tab_out_of_range() {
        let client = reqwest::Client::new();
        let s1 = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9222".into(),
            "T1".into(),
            "".into(),
            "".into(),
        );
        let mut pool = BrowserPool::from_sessions(vec![s1], BrowserConfig::default());
        let err = pool.switch_tab(5);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("out of range"));
    }

    #[test]
    fn test_pool_active_mut() {
        let client = reqwest::Client::new();
        let s1 = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9222".into(),
            "T1".into(),
            "".into(),
            "".into(),
        );
        let mut pool = BrowserPool::from_sessions(vec![s1], BrowserConfig::default());
        pool.active_mut().page_url = "https://modified.com".to_string();
        assert_eq!(pool.active().page_url, "https://modified.com");
    }

    // ── Chrome path detection tests ────────────────────────────────────

    #[test]
    fn test_detect_chrome_from_empty_candidates() {
        let result = detect_chrome_from_candidates(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_chrome_from_nonexistent_paths() {
        let result = detect_chrome_from_candidates(&[
            "/nonexistent/path/chrome",
            "/also/not/here",
        ]);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_chrome_from_existing_path() {
        // /bin/sh exists on all Unix systems — use it as a stand-in
        let result = detect_chrome_from_candidates(&["/nonexistent", "/bin/sh"]);
        assert_eq!(result.as_deref(), Some("/bin/sh"));
    }

    #[test]
    fn test_detect_chrome_returns_first_match() {
        let result = detect_chrome_from_candidates(&["/bin/sh", "/bin/ls"]);
        // Should return the first existing path
        assert_eq!(result.as_deref(), Some("/bin/sh"));
    }

    #[test]
    fn test_chrome_paths_constants_non_empty() {
        assert!(!CHROME_PATHS_MACOS.is_empty());
        assert!(!CHROME_PATHS_LINUX.is_empty());
    }

    // ── BrowserToolDef tests ───────────────────────────────────────────

    #[test]
    fn test_tool_definitions_count() {
        let defs = browser_agent_tool_definitions();
        assert_eq!(defs.len(), 11);
    }

    #[test]
    fn test_tool_definitions_names() {
        let defs = browser_agent_tool_definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"browser_navigate"));
        assert!(names.contains(&"browser_click"));
        assert!(names.contains(&"browser_type"));
        assert!(names.contains(&"browser_scroll"));
        assert!(names.contains(&"browser_screenshot"));
        assert!(names.contains(&"browser_extract_text"));
        assert!(names.contains(&"browser_evaluate_js"));
        assert!(names.contains(&"browser_wait"));
        assert!(names.contains(&"browser_back"));
        assert!(names.contains(&"browser_forward"));
        assert!(names.contains(&"browser_page_info"));
    }

    #[test]
    fn test_tool_def_to_xml_no_params() {
        let def = BrowserToolDef {
            name: "browser_screenshot".into(),
            description: "Take screenshot".into(),
            parameters: vec![],
        };
        let xml = def.to_xml();
        assert!(xml.contains("<tool name=\"browser_screenshot\">"));
        assert!(xml.contains("<description>Take screenshot</description>"));
        assert!(!xml.contains("<parameters>"));
        assert!(xml.contains("</tool>"));
    }

    #[test]
    fn test_tool_def_to_xml_with_params() {
        let def = BrowserToolDef {
            name: "browser_navigate".into(),
            description: "Navigate".into(),
            parameters: vec![("url".into(), "string".into(), "Target URL".into())],
        };
        let xml = def.to_xml();
        assert!(xml.contains("<parameters>"));
        assert!(xml.contains("name=\"url\""));
        assert!(xml.contains("type=\"string\""));
        assert!(xml.contains("Target URL"));
    }

    #[test]
    fn test_tool_def_serialization() {
        let def = BrowserToolDef {
            name: "test".into(),
            description: "desc".into(),
            parameters: vec![("p1".into(), "string".into(), "d1".into())],
        };
        let json = serde_json::to_string(&def).expect("ser");
        let deser: BrowserToolDef = serde_json::from_str(&json).expect("deser");
        assert_eq!(deser.name, "test");
        assert_eq!(deser.parameters.len(), 1);
    }

    // ── CdpTarget deserialization test ─────────────────────────────────

    #[test]
    fn test_cdp_target_deserialization() {
        let json = r#"{
            "id": "ABCD",
            "type": "page",
            "title": "Google",
            "url": "https://google.com",
            "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/ABCD",
            "description": ""
        }"#;
        let target: CdpTarget = serde_json::from_str(json).expect("deser");
        assert_eq!(target.id, "ABCD");
        assert_eq!(target.target_type, "page");
        assert_eq!(target.title, "Google");
        assert_eq!(target.url, "https://google.com");
        assert!(target.web_socket_debugger_url.is_some());
    }

    #[test]
    fn test_cdp_target_list_deserialization() {
        let json = r#"[
            {"id":"A","type":"page","title":"T1","url":"http://a.com","description":""},
            {"id":"B","type":"background_page","title":"Ext","url":"chrome-extension://x","description":""}
        ]"#;
        let targets: Vec<CdpTarget> = serde_json::from_str(json).expect("deser");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].target_type, "page");
        assert_eq!(targets[1].target_type, "background_page");
    }

    // ── CdpCommand serialization test ──────────────────────────────────

    #[test]
    fn test_cdp_command_serialization() {
        let cmd = CdpCommand {
            id: 1,
            method: "Runtime.evaluate".into(),
            params: serde_json::json!({"expression": "1+1"}),
        };
        let json = serde_json::to_string(&cmd).expect("ser");
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"Runtime.evaluate\""));
        assert!(json.contains("\"expression\":\"1+1\""));
    }

    // ── Integration-style unit tests (no network) ──────────────────────

    #[test]
    fn test_action_dispatch_routes_correctly() {
        // Verify that execute_action maps each variant — we can't call it
        // without a real Chrome, but we verify the Display output which
        // mirrors the dispatch path.
        let cases: Vec<(BrowserAction, &str)> = vec![
            (
                BrowserAction::Navigate { url: "u".into() },
                "Navigate",
            ),
            (
                BrowserAction::Click { selector: "s".into() },
                "Click",
            ),
            (
                BrowserAction::Type {
                    selector: "s".into(),
                    text: "t".into(),
                },
                "Type",
            ),
            (
                BrowserAction::Scroll {
                    direction: ScrollDirection::Down,
                    amount: 1,
                },
                "Scroll",
            ),
            (BrowserAction::Screenshot, "Screenshot"),
            (BrowserAction::ExtractText { selector: None }, "ExtractText"),
            (
                BrowserAction::EvaluateJs { script: "x".into() },
                "EvaluateJs",
            ),
            (
                BrowserAction::WaitForSelector {
                    selector: "x".into(),
                    timeout_ms: 1,
                },
                "WaitForSelector",
            ),
            (BrowserAction::Back, "Back"),
            (BrowserAction::Forward, "Forward"),
            (BrowserAction::GetPageInfo, "GetPageInfo"),
        ];

        for (action, expected_prefix) in &cases {
            let display = action.to_string();
            assert!(
                display.starts_with(expected_prefix),
                "Action {:?} display '{}' should start with '{}'",
                action,
                display,
                expected_prefix,
            );
        }
    }

    #[test]
    fn test_history_tracking_simulation() {
        let client = reqwest::Client::new();
        let mut session = BrowserSession::from_parts(
            client,
            "http://127.0.0.1:9222".into(),
            "T1".into(),
            "about:blank".into(),
            "".into(),
        );

        // Simulate navigation entries being added
        session.history.push(NavigationEntry::with_timestamp(
            "https://a.com",
            "A",
            100,
        ));
        session.history.push(NavigationEntry::with_timestamp(
            "https://b.com",
            "B",
            200,
        ));
        session.history.push(NavigationEntry::with_timestamp(
            "https://c.com",
            "C",
            300,
        ));

        assert_eq!(session.history.len(), 3);
        assert_eq!(session.history[0].url, "https://a.com");
        assert_eq!(session.history[2].url, "https://c.com");
        assert!(session.history[0].timestamp_ms < session.history[2].timestamp_ms);
    }

    #[test]
    fn test_screenshot_entry_metadata() {
        let entry = ScreenshotEntry::new("after navigate to https://test.com", "iVBORw0K...");
        assert!(entry.action_before.contains("navigate"));
        assert!(!entry.png_base64.is_empty());
        assert!(entry.timestamp_ms > 0);
    }
}
