//! C4 integration test — WebMCP discovery + invocation over a real CDP browser.
//!
//! Codifies the manual smoke test: launch Chrome, load a page that advertises a
//! WebMCP tool, and verify `discover_webmcp_tools` + `call_webmcp_tool` round-trip
//! through the live `BrowserSession`. `#[ignore]` because it requires a Chrome /
//! Chromium binary; run explicitly:
//!
//! ```text
//! cargo test -p vibecli --test webmcp_cdp_test -- --ignored
//! ```

use std::time::Duration;
use vibecli_cli::browser_agent::{launch_chrome, BrowserConfig, BrowserSession};
use vibecli_cli::webmcp::WebMcpFlag;

const TEST_PAGE: &str = r#"<!doctype html><html><head><title>WebMCP Test</title></head><body>
<script>
  window.agent = {
    tools: [
      { name: "search", description: "Search the catalog",
        params: [{ name: "q", required: true }, { name: "limit", required: false }] }
    ],
    callTool: async (name, args) => ({ called: name, args, result: "ok-" + (args.q || "") })
  };
</script></body></html>"#;

#[tokio::test]
#[ignore = "requires a Chrome/Chromium binary; run with --ignored"]
async fn webmcp_discover_and_call_over_cdp() {
    // 1. Write the advertising page to a temp file.
    let page = std::env::temp_dir().join("vibecli-webmcp-cdp-test.html");
    std::fs::write(&page, TEST_PAGE).expect("write test page");
    let page_url = format!("file://{}", page.display());

    // 2. Launch an isolated headless Chrome on a non-default debug port.
    let config = BrowserConfig {
        debug_port: 9456,
        headless: true,
        ..Default::default()
    };
    let mut child = launch_chrome(&config)
        .await
        .expect("launch Chrome (set CHROME_PATH if not auto-detected)");

    // 3. Bind a session, navigate, let the page's <script> register the tools.
    let result = async {
        let mut session = BrowserSession::new(&config).await?;
        session.navigate(&page_url).await?;
        tokio::time::sleep(Duration::from_millis(400)).await;

        // Consumer: discover the advertised tool.
        let tools = session.discover_webmcp_tools(WebMcpFlag(true)).await?;
        assert_eq!(tools.len(), 1, "expected exactly one advertised tool");
        assert_eq!(tools[0].name, "search");
        assert!(tools[0].params.iter().any(|p| p.name == "q" && p.required));

        // Disabled flag → no discovery (origin-trial gate).
        let none = session.discover_webmcp_tools(WebMcpFlag(false)).await?;
        assert!(none.is_empty(), "disabled flag must discover nothing");

        // Consumer: call the tool; the page's callTool echoes the args.
        let out = session
            .call_webmcp_tool(
                WebMcpFlag(true),
                &tools,
                "search",
                &[("q".into(), "rust".into())],
            )
            .await?;
        assert!(out.contains("ok-rust"), "call result was: {out}");

        // Missing required param is rejected before reaching the page.
        let bad = session
            .call_webmcp_tool(WebMcpFlag(true), &tools, "search", &[])
            .await;
        assert!(bad.is_err(), "missing required 'q' must be rejected");

        anyhow::Ok(())
    }
    .await;

    // 4. Always tear down Chrome and the temp page.
    let _ = child.kill().await;
    let _ = std::fs::remove_file(&page);

    result.expect("WebMCP CDP round-trip");
}
