//! Make a real edit to a document and check it survived the save.
//!
//! The edit is a deletion of one character from the first line that has one,
//! so it needs no glyph the document does not already carry.
use std::path::PathBuf;

fn main() {
    let src = PathBuf::from(std::env::args().nth(1).expect("usage: editcheck <file>"));
    let work = std::env::temp_dir().join(format!("editcheck-{}", src.file_name().unwrap().to_string_lossy()));
    std::fs::copy(&src, &work).unwrap();
    let buf = match vibe_docfmt::read_text(&work) {
        Ok(b) => b,
        Err(e) => { println!("READ FAILED: {e}"); return; }
    };
    let mut lines: Vec<String> = buf.text.lines().map(str::to_string).collect();
    let target = lines.iter().position(|l| l.chars().filter(|c| c.is_alphanumeric()).count() > 6 && !l.contains("vibedoc"));
    let Some(target) = target else { println!("SKIP: nothing to edit"); return; };
    let mut edited: String = lines[target].clone();
    edited.pop();
    lines[target] = edited.clone();
    let text = format!("{}\n", lines.join("\n"));

    match vibe_docfmt::write_text(&work, &text) {
        Ok(_) => {}
        Err(e) => { println!("WRITE FAILED: {e}"); let _ = std::fs::remove_file(&work); return; }
    }
    let after = match vibe_docfmt::read_text(&work) {
        Ok(b) => b,
        Err(e) => { println!("REREAD FAILED: {e}"); return; }
    };
    let got: Vec<&str> = after.text.lines().collect();
    if got.get(target).copied() == Some(edited.as_str()) {
        println!("EDIT OK line {target}");
    } else {
        println!("EDIT MISMATCH line {target}: want {:?} got {:?}", &edited[..edited.len().min(60)], got.get(target).map(|g| &g[..g.len().min(60)]));
    }
    let _ = std::fs::remove_file(&work);
    let _ = std::fs::remove_file(work.with_file_name(format!("{}.bak", work.file_name().unwrap().to_string_lossy())));
}
