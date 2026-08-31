//! Read a real document, save the same text back, and report what moved.
//!
//! Fixtures prove a reader handles the shapes someone thought of. This proves
//! it against a file that already exists — which is where every round-trip bug
//! in this crate was actually found.
//!
//! ```bash
//! cargo run -p vibe-docfmt --release --example roundtrip -- report.docx
//! ```
//!
//! Two checks run. The first is the invariant the whole crate rests on: the
//! buffer and its own parse must agree, because a save is verified by comparing
//! them. The second is the save itself, against a copy — the original file is
//! never touched.

use std::path::PathBuf;

fn main() {
    let Some(source) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: roundtrip <file>");
        std::process::exit(2);
    };
    let work = std::env::temp_dir().join(source.file_name().unwrap_or_default());
    if let Err(e) = std::fs::copy(&source, &work) {
        println!("COPY FAILED: {e}");
        return;
    }

    let buffer = match vibe_docfmt::read_text(&work) {
        Ok(buffer) => buffer,
        Err(e) => {
            println!("READ FAILED: {e}");
            return;
        }
    };
    println!(
        "format={:?} sections={} chars={} warnings={}",
        buffer.format,
        buffer.sections,
        buffer.text.len(),
        buffer.warnings.len()
    );
    for warning in &buffer.warnings {
        println!("  warn {}: {}", warning.code, warning.message);
    }

    let reparsed = vibe_docfmt::render(&vibe_docfmt::parse_text(buffer.format, &buffer.text));
    report("[buffer]", &buffer.text, &reparsed);

    match vibe_docfmt::write_text(&work, &buffer.text) {
        Ok(report) => println!(
            "WRITE OK bytes={} verified={}",
            report.bytes_written, report.verified
        ),
        Err(e) => println!("WRITE FAILED: {e}"),
    }

    let _ = std::fs::remove_file(&work);
}

/// Print the first few lines on which two renderings disagree.
fn report(label: &str, want: &str, got: &str) {
    let want: Vec<&str> = want.lines().collect();
    let got: Vec<&str> = got.lines().collect();
    let mut shown = 0;
    for line in 0..want.len().max(got.len()) {
        let (a, b) = (
            want.get(line).copied().unwrap_or("<end of buffer>"),
            got.get(line).copied().unwrap_or("<end of buffer>"),
        );
        if a == b {
            continue;
        }
        println!("{label} line {}:\n  want {a:?}\n  got  {b:?}", line + 1);
        shown += 1;
        if shown == 6 {
            println!("{label} … and more");
            return;
        }
    }
    if shown == 0 {
        println!("{label} identical ({} lines)", want.len());
    }
}
