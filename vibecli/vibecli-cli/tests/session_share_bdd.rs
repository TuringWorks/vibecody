/*!
 * BDD tests for session HTML export + GitHub Gist sharing.
 * Run with: cargo test --test session_share_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::session_share::{
    GistClient, GistOptions, GistResult, HtmlExportOptions, HtmlExporter,
    ShareMessage, ShareRole,
};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct SsWorld {
    /// Messages accumulated for the current scenario.
    messages: Vec<ShareMessage>,
    /// Raw content string for fence-highlight / escape tests.
    raw_content: String,
    /// Rendered HTML output.
    html_output: String,
    /// Highlighted/escaped text output.
    text_output: String,
    /// GistOptions for payload / parse scenarios.
    gist_opts: Option<GistOptions>,
    /// Built gist payload JSON.
    gist_payload: String,
    /// Session title used when building the gist payload.
    session_title: String,
    /// HTML content used when building the gist payload.
    html_content_for_gist: String,
    /// Parsed gist result (scenario 5).
    parsed_gist: Option<GistResult>,
    /// Whether parse_response returned an error.
    parse_error: bool,
}

// ---------------------------------------------------------------------------
// Scenario 1 — HTML export with messages
// ---------------------------------------------------------------------------

#[given(
    expr = "a session with a user message {string} and an assistant message {string}"
)]
fn given_session_with_messages(world: &mut SsWorld, user_msg: String, asst_msg: String) {
    world.messages.push(ShareMessage::user(user_msg));
    world.messages.push(ShareMessage::assistant(asst_msg));
}

#[when("I export the session as HTML with default options")]
fn when_export_html(world: &mut SsWorld) {
    let opts = HtmlExportOptions::default();
    world.html_output = HtmlExporter::export(&world.messages, &opts);
}

#[then(expr = "the output starts with {string}")]
fn then_starts_with(world: &mut SsWorld, prefix: String) {
    assert!(
        world.html_output.starts_with(&prefix),
        "Expected output to start with {prefix:?}, got: {}",
        &world.html_output[..prefix.len().min(world.html_output.len())]
    );
}

#[then(expr = "the output contains the CSS class {string}")]
fn then_contains_css_class(world: &mut SsWorld, css_class: String) {
    assert!(
        world.html_output.contains(&css_class),
        "Expected HTML to contain CSS class {css_class:?}"
    );
}

#[then("the output contains the session title in a <title> element")]
fn then_title_element(world: &mut SsWorld) {
    // Default title is "VibeCody Session"
    assert!(
        world.html_output.contains("<title>VibeCody Session</title>"),
        "Expected <title>VibeCody Session</title> in output"
    );
}

// ---------------------------------------------------------------------------
// Scenario 2 — Code fence highlighting
// ---------------------------------------------------------------------------

#[given("a markdown content block with a fenced Rust code block")]
fn given_fenced_rust_block(world: &mut SsWorld) {
    world.raw_content =
        "Here is some code:\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```\nEnd."
            .to_string();
}

#[when("I apply highlight_fences to the content")]
fn when_highlight_fences(world: &mut SsWorld) {
    world.text_output = HtmlExporter::highlight_fences(&world.raw_content);
}

#[then(expr = "the output contains a pre element with class {string}")]
fn then_pre_with_lang_class(world: &mut SsWorld, lang_class: String) {
    let expected = format!(r#"class="{lang_class}""#);
    assert!(
        world.text_output.contains(&expected),
        "Expected class {lang_class:?} in output. Got:\n{}",
        world.text_output
    );
}

#[then("the output does not contain the raw triple-backtick fence markers")]
fn then_no_backticks(world: &mut SsWorld) {
    assert!(
        !world.text_output.contains("```"),
        "Output should not contain raw ``` markers"
    );
}

// ---------------------------------------------------------------------------
// Scenario 3 — HTML escape
// ---------------------------------------------------------------------------

#[given(
    "a raw string containing HTML special characters '<', '>', '&', '\"', and \"'\""
)]
fn given_html_special_chars(world: &mut SsWorld) {
    world.raw_content = r#"<tag> & "quote" & 'apos'"#.to_string();
}

#[when("I call escape_html on the string")]
fn when_escape_html(world: &mut SsWorld) {
    world.text_output = HtmlExporter::escape_html(&world.raw_content);
}

#[then(expr = "the output contains {string} instead of {string}")]
fn then_contains_instead_of(world: &mut SsWorld, expected: String, _original: String) {
    assert!(
        world.text_output.contains(&expected),
        "Expected {expected:?} in output: {}",
        world.text_output
    );
}

// ---------------------------------------------------------------------------
// Scenario 4 — Gist payload building
// ---------------------------------------------------------------------------

#[given(expr = "a session title {string} and HTML content {string}")]
fn given_session_title_and_html(world: &mut SsWorld, title: String, html: String) {
    world.session_title = title;
    world.html_content_for_gist = html;
}

#[given(expr = "a GistOptions with description {string} and public false")]
fn given_gist_options(world: &mut SsWorld, desc: String) {
    world.gist_opts = Some(GistOptions {
        description: desc,
        public: false,
        github_token: None,
    });
}

#[when("I build the gist payload")]
fn when_build_payload(world: &mut SsWorld) {
    let opts = world.gist_opts.as_ref().cloned().unwrap_or_default();
    // Mirror the filename logic from GistClient::upload
    let safe_title: String = world
        .session_title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let filename = format!("session-{}.html", safe_title);
    world.gist_payload =
        GistClient::build_payload(&filename, &world.html_content_for_gist, &opts);
}

#[then(expr = "the payload contains the description {string}")]
fn then_payload_contains_desc(world: &mut SsWorld, desc: String) {
    assert!(
        world.gist_payload.contains(&desc),
        "Payload should contain description {desc:?}. Payload: {}",
        world.gist_payload
    );
}

#[then(expr = r#"the payload contains "public":false"#)]
fn then_payload_public_false(world: &mut SsWorld) {
    assert!(
        world.gist_payload.contains(r#""public":false"#),
        "Payload should contain '\"public\":false'. Payload: {}",
        world.gist_payload
    );
}

#[then(expr = "the payload contains the filename {string}")]
fn then_payload_contains_filename(world: &mut SsWorld, filename: String) {
    assert!(
        world.gist_payload.contains(&filename),
        "Payload should contain filename {filename:?}. Payload: {}",
        world.gist_payload
    );
}

#[then(expr = "the payload contains a {string} key")]
fn then_payload_contains_key(world: &mut SsWorld, key: String) {
    let expected = format!("\"{key}\"");
    assert!(
        world.gist_payload.contains(&expected),
        "Payload should contain key {key:?}. Payload: {}",
        world.gist_payload
    );
}

// ---------------------------------------------------------------------------
// Scenario 5 — Gist response parsing
// ---------------------------------------------------------------------------

#[given(
    expr = "a mock GitHub API response JSON with id {string} and html_url {string}"
)]
fn given_mock_gist_response(world: &mut SsWorld, id: String, html_url: String) {
    // Store the mock JSON into raw_content for use in the When step
    world.raw_content = format!(
        r#"{{
            "id": "{id}",
            "html_url": "{html_url}",
            "description": "VibeCody session share",
            "files": {{
                "session-test.html": {{
                    "raw_url": "https://gist.githubusercontent.com/user/{id}/raw/session-test.html"
                }}
            }}
        }}"#,
        id = id,
        html_url = html_url,
    );
}

#[when("I parse the gist response")]
fn when_parse_gist_response(world: &mut SsWorld) {
    match GistClient::parse_response(&world.raw_content) {
        Ok(result) => {
            world.parsed_gist = Some(result);
            world.parse_error = false;
        }
        Err(_) => {
            world.parsed_gist = None;
            world.parse_error = true;
        }
    }
}

#[then(expr = "the parsed gist_id equals {string}")]
fn then_parsed_gist_id(world: &mut SsWorld, expected: String) {
    let gist = world.parsed_gist.as_ref().expect("gist should be parsed successfully");
    assert_eq!(gist.gist_id, expected);
}

#[then(expr = "the parsed html_url equals {string}")]
fn then_parsed_html_url(world: &mut SsWorld, expected: String) {
    let gist = world.parsed_gist.as_ref().expect("gist should be parsed successfully");
    assert_eq!(gist.html_url, expected);
}

#[then("no error is returned")]
fn then_no_error(world: &mut SsWorld) {
    assert!(!world.parse_error, "Expected no error from parse_response");
    assert!(world.parsed_gist.is_some(), "Expected a parsed GistResult");
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(
        SsWorld::run("tests/features/session_share.feature"),
    );
}
