//! Round-trip a real document and show exactly where the text diverges.
use std::path::PathBuf;

fn diff(label: &str, want: &str, got: &str) {
    let w: Vec<&str> = want.lines().collect();
    let g: Vec<&str> = got.lines().collect();
    let mut shown = 0;
    for i in 0..w.len().max(g.len()) {
        let a = w.get(i).copied().unwrap_or("<EOF>");
        let b = g.get(i).copied().unwrap_or("<EOF>");
        if a != b {
            println!("{label} line {}:\n  want {:?}\n  got  {:?}", i + 1, a, b);
            shown += 1;
            if shown >= 6 {
                println!("{label} … more");
                return;
            }
        }
    }
    if shown == 0 {
        println!("{label} identical ({} lines)", w.len());
    }
}

fn main() {
    let src = PathBuf::from(std::env::args().nth(1).expect("usage: roundtrip <file>"));
    let work = std::env::temp_dir().join(src.file_name().unwrap());
    std::fs::copy(&src, &work).unwrap();
    let buf = match vibe_docfmt::read_text(&work) {
        Ok(b) => b,
        Err(e) => { println!("READ FAILED: {e}"); return; }
    };
    println!("format={:?} sections={} chars={}", buf.format, buf.sections, buf.text.len());

    // Stage 1: text -> model -> text (pure markdown round trip)
    let parsed = vibe_docfmt::parse_text(buf.format, &buf.text);
    diff("[md]", &buf.text, &vibe_docfmt::render(&parsed));

    // Stage 2: full write path
    match vibe_docfmt::write_text(&work, &buf.text) {
        Ok(r) => println!("WRITE OK bytes={} verified={}", r.bytes_written, r.verified),
        Err(e) => println!("WRITE FAILED: {e}"),
    }
    // Stage 3: what the container actually produced, vs what was asked
    if let Ok(orig) = std::fs::read(&src) {
        if let Ok(rw) = vibe_docfmt::docx::write(&orig, &parsed) {
            if let Ok(re) = vibe_docfmt::docx::read(&rw.bytes) {
                diff("[docx]", &vibe_docfmt::render(&rw.effective), &vibe_docfmt::render(&re));
            }
        }
    }
    let _ = std::fs::remove_file(&work);
}
