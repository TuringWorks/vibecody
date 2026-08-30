//! Edit one line of a PDF and show exactly where the re-read diverges.
use vibe_docfmt::{markdown, model::DocFormat, pdf};

fn main() {
    let path = std::env::args().nth(1).expect("usage: pdfdiff <file>");
    let bytes = std::fs::read(&path).unwrap();
    let doc = pdf::read(&bytes).unwrap();
    let text = markdown::to_plain_text(&doc);
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let target = lines
        .iter()
        .position(|l| l.chars().filter(|c| c.is_alphanumeric()).count() > 6 && !l.contains("vibedoc"))
        .expect("something to edit");
    let mut edited = lines[target].clone();
    edited.pop();
    lines[target] = edited;
    let buffer = format!("{}\n", lines.join("\n"));

    let parsed = markdown::from_plain_text(DocFormat::Pdf, &buffer);
    let rewrite = match pdf::write(&bytes, &parsed) {
        Ok(r) => r,
        Err(e) => { println!("WRITE ERROR: {e}"); return; }
    };
    let reread = pdf::read(&rewrite.bytes).unwrap();
    let want_text = markdown::to_plain_text(&rewrite.effective);
    let got_text = markdown::to_plain_text(&reread);
    let want: Vec<&str> = want_text.lines().collect();
    let got: Vec<&str> = got_text.lines().collect();
    println!("edited line {target}; want {} lines, got {}", want.len(), got.len());
    let mut shown = 0;
    for i in 0..want.len().max(got.len()) {
        let a = want.get(i).copied().unwrap_or("<EOF>");
        let b = got.get(i).copied().unwrap_or("<EOF>");
        if a != b {
            println!("{i}:\n  want {:?}\n  got  {:?}", &a[..a.len().min(150)], &b[..b.len().min(150)]);
            shown += 1;
            if shown > 5 { break; }
        }
    }
}
