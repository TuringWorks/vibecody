/*!
 * BDD tests for tui_images using Cucumber.
 * Run with: cargo test --test tui_images_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::tui_images::{
    ImageProtocol, RenderOptions, RenderResult,
    kitty_escape, iterm2_escape, parse_image_dimensions, render_image_bytes,
};

// ─── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct TuiImagesWorld {
    /// Detected or manually-set protocol.
    protocol: Option<ImageProtocol>,
    /// Raw bytes used for building escape sequences or rendering.
    image_data: Vec<u8>,
    /// Escape sequence produced by the builder functions.
    escape_sequence: String,
    /// Full render result (for render_image_bytes scenarios).
    render_result: Option<RenderResult>,
    /// Parsed dimensions.
    parsed_dims: Option<(u32, u32)>,
    /// Render options for the current scenario.
    render_opts: Option<RenderOptions>,
    /// Column hint for kitty_escape.
    cols: u32,
    /// Row hint for kitty_escape.
    rows: u32,
    /// Pixel width hint for iterm2_escape.
    width_px: u32,
    /// Pixel height hint for iterm2_escape.
    height_px: u32,
}

// ─── Given steps ─────────────────────────────────────────────────────────────

#[given(expr = "the environment variable {string} is set to {string}")]
fn set_env_var(world: &mut TuiImagesWorld, var: String, value: String) {
    // Safety: tests run single-threaded in Cucumber's executor.
    unsafe { std::env::set_var(&var, &value) };
}

#[given(expr = "the environment variable {string} is unset")]
fn unset_env_var(world: &mut TuiImagesWorld, var: String) {
    unsafe { std::env::remove_var(&var) };
}

#[given("a synthetic 320x240 PNG header")]
fn synthetic_png_header(world: &mut TuiImagesWorld) {
    // PNG signature (8 bytes) + IHDR length (4) + "IHDR" (4) + width BE (4) + height BE (4)
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(b"\x89PNG\r\n\x1a\n"); // PNG magic
    data.extend_from_slice(&13u32.to_be_bytes());  // IHDR data length
    data.extend_from_slice(b"IHDR");               // chunk type
    data.extend_from_slice(&320u32.to_be_bytes()); // width
    data.extend_from_slice(&240u32.to_be_bytes()); // height
    world.image_data = data;
}

#[given(expr = "raw image data {string}")]
fn set_raw_image_data(world: &mut TuiImagesWorld, data: String) {
    world.image_data = data.into_bytes();
}

#[given(expr = "the render protocol is {string}")]
fn set_render_protocol(world: &mut TuiImagesWorld, proto: String) {
    let protocol = match proto.as_str() {
        "kitty" => ImageProtocol::Kitty,
        "iterm2" => ImageProtocol::ITerm2,
        _ => ImageProtocol::None,
    };
    world.render_opts = Some(RenderOptions {
        protocol,
        ..Default::default()
    });
}

// ─── When steps ───────────────────────────────────────────────────────────────

#[when("I detect the image protocol")]
fn detect_protocol(world: &mut TuiImagesWorld) {
    world.protocol = Some(ImageProtocol::detect());
}

#[when("I parse the image dimensions")]
fn parse_dims(world: &mut TuiImagesWorld) {
    world.parsed_dims = parse_image_dimensions(&world.image_data);
}

#[when(expr = "I build the escape sequence with cols {int} and rows {int}")]
fn build_kitty_sequence(world: &mut TuiImagesWorld, cols: u32, rows: u32) {
    world.cols = cols;
    world.rows = rows;
    world.escape_sequence = kitty_escape(&world.image_data, cols, rows);
}

#[when(expr = "I build the escape sequence with width {int} and height {int}")]
fn build_iterm2_sequence(world: &mut TuiImagesWorld, width: u32, height: u32) {
    world.width_px = width;
    world.height_px = height;
    world.escape_sequence = iterm2_escape(&world.image_data, width, height);
}

#[when("I render the image bytes")]
fn render_bytes(world: &mut TuiImagesWorld) {
    let opts = world.render_opts.clone().unwrap_or_default();
    world.render_result = Some(render_image_bytes(&world.image_data, &opts));
}

// ─── Then steps ───────────────────────────────────────────────────────────────

#[then(expr = "the protocol should be {string}")]
fn check_protocol(world: &mut TuiImagesWorld, expected: String) {
    let proto = world.protocol.as_ref().expect("protocol not set");
    assert_eq!(
        proto.name(),
        expected.as_str(),
        "expected protocol '{expected}' but got '{}'",
        proto.name()
    );
}

#[then("the protocol should be supported")]
fn check_protocol_supported(world: &mut TuiImagesWorld) {
    let proto = world.protocol.as_ref().expect("protocol not set");
    assert!(proto.is_supported(), "expected protocol to be supported");
}

#[then(expr = "the width should be {int}")]
fn check_width(world: &mut TuiImagesWorld, expected: u32) {
    let (w, _) = world.parsed_dims.expect("dims not parsed");
    assert_eq!(w, expected, "width mismatch");
}

#[then(expr = "the height should be {int}")]
fn check_height(world: &mut TuiImagesWorld, expected: u32) {
    let (_, h) = world.parsed_dims.expect("dims not parsed");
    assert_eq!(h, expected, "height mismatch");
}

#[then(expr = r#"the escape sequence should start with {string}"#)]
fn check_seq_prefix(world: &mut TuiImagesWorld, prefix: String) {
    // Unescape \x1b etc. so the Gherkin string matches the actual bytes.
    let prefix = unescape(&prefix);
    assert!(
        world.escape_sequence.starts_with(&prefix),
        "sequence does not start with {:?}; got: {:?}",
        prefix,
        &world.escape_sequence[..world.escape_sequence.len().min(40)]
    );
}

#[then(expr = r#"the escape sequence should end with {string}"#)]
fn check_seq_suffix(world: &mut TuiImagesWorld, suffix: String) {
    let suffix = unescape(&suffix);
    assert!(
        world.escape_sequence.ends_with(&suffix),
        "sequence does not end with {:?}",
        suffix
    );
}

#[then(expr = r#"the escape sequence should contain {string}"#)]
fn check_seq_contains(world: &mut TuiImagesWorld, needle: String) {
    let needle = unescape(&needle);
    assert!(
        world.escape_sequence.contains(&needle),
        "sequence does not contain {:?}",
        needle
    );
}

#[then("the result should be a fallback")]
fn check_fallback(world: &mut TuiImagesWorld) {
    let r = world.render_result.as_ref().expect("render result not set");
    assert!(r.fallback, "expected fallback but got visual render");
}

#[then(expr = r#"the placeholder text should contain {string}"#)]
fn check_placeholder_contains(world: &mut TuiImagesWorld, needle: String) {
    let r = world.render_result.as_ref().expect("render result not set");
    assert!(
        r.placeholder_text.contains(&needle),
        "placeholder {:?} does not contain {:?}",
        r.placeholder_text,
        needle
    );
}

#[then("the escape sequence should be empty")]
fn check_seq_empty(world: &mut TuiImagesWorld) {
    let r = world.render_result.as_ref().expect("render result not set");
    assert!(
        r.escape_sequence.is_empty(),
        "expected empty escape sequence, got: {:?}",
        r.escape_sequence
    );
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Convert `\x1b`, `\x07` etc. in a Gherkin string to actual bytes.
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('x') => {
                    let h1 = chars.next().unwrap_or('0');
                    let h2 = chars.next().unwrap_or('0');
                    let hex = format!("{h1}{h2}");
                    if let Ok(b) = u8::from_str_radix(&hex, 16) {
                        out.push(b as char);
                    }
                }
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    // max_concurrent_scenarios(1) prevents env-var races between scenarios.
    futures::executor::block_on(
        TuiImagesWorld::cucumber()
            .max_concurrent_scenarios(1)
            .run("tests/features/tui_images.feature"),
    );
}
