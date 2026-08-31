//! Change one line of a real document, save it, and check the change survived.
//!
//! ```bash
//! cargo run -p vibe-docfmt --release --example editcheck -- report.docx
//! ```
//!
//! The edit is the deletion of one character from a line that carries no markup
//! and no edge whitespace, so it needs no glyph the document does not already
//! have and leaves text that is still its own canonical form. Anything this
//! reports is therefore a real disagreement rather than the buffer being
//! normalised. The original file is never touched — the work happens on a copy.

use std::path::PathBuf;

fn main() {
    let Some(source) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: editcheck <file>");
        std::process::exit(2);
    };
    let name = source.file_name().unwrap_or_default().to_string_lossy();
    let work = std::env::temp_dir().join(format!("editcheck-{name}"));
    if let Err(e) = std::fs::copy(&source, &work) {
        println!("COPY FAILED: {e}");
        return;
    }
    // The writer names a backup `<whole file name>.bak`, not `<stem>.bak`.
    let backup = work.with_file_name(format!("editcheck-{name}.bak"));
    let cleanup = || {
        let _ = std::fs::remove_file(&work);
        let _ = std::fs::remove_file(&backup);
    };

    let buffer = match vibe_docfmt::read_text(&work) {
        Ok(buffer) => buffer,
        Err(e) => {
            println!("READ FAILED: {e}");
            cleanup();
            return;
        }
    };

    let mut lines: Vec<String> = buffer.text.lines().map(str::to_string).collect();
    let markdown = buffer.syntax == vibe_docfmt::Syntax::Markdown;
    let Some(target) = lines.iter().position(|line| is_plain(line, markdown)) else {
        println!("SKIP: no line to edit");
        cleanup();
        return;
    };
    let mut edited = lines[target].clone();
    edited.pop();
    lines[target] = edited.clone();

    if let Err(e) = vibe_docfmt::write_text(&work, &format!("{}\n", lines.join("\n"))) {
        println!("WRITE FAILED: {e}");
        cleanup();
        return;
    }
    match vibe_docfmt::read_text(&work) {
        Ok(after) => match after.text.lines().nth(target) {
            Some(line) if line == edited => println!("EDIT OK line {target}"),
            other => println!("EDIT MISMATCH line {target}: want {edited:?} got {other:?}"),
        },
        Err(e) => println!("RE-READ FAILED: {e}"),
    }
    cleanup();
}

/// A line worth editing: enough text to shorten, nothing at its edges that the
/// format would trim, and — in a Markdown buffer — no inline syntax, so the
/// shortened line is still its own canonical form.
fn is_plain(line: &str, markdown: bool) -> bool {
    line.chars().filter(char::is_ascii_alphanumeric).count() > 6
        && line.trim() == line
        // A section marker is not text: editing one asks the writer to route an
        // edit to a chapter or page that does not exist, which it refuses.
        && !line.contains("vibedoc:")
        && !(markdown && line.contains(['*', '_', '`', '[', ']', '\\', '|', '<']))
}
