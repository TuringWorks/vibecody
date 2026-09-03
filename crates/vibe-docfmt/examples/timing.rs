//! Where the time goes on one document.
use std::time::Instant;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let t = Instant::now();
    let bytes = std::fs::read(&path).unwrap();
    println!("read file      {:?} ({} bytes)", t.elapsed(), bytes.len());

    let t = Instant::now();
    let file = lopdf::Document::load_mem(&bytes).unwrap();
    let pages = file.get_pages();
    println!("load_mem       {:?} ({} pages)", t.elapsed(), pages.len());

    let t = Instant::now();
    let mut total = 0usize;
    for id in pages.values() {
        if let Ok(c) = file.get_page_content(*id) {
            total += c.len();
        }
    }
    println!("page content   {:?} ({total} bytes)", t.elapsed());

    let t = Instant::now();
    let mut fonts = 0usize;
    for id in pages.values() {
        fonts += file.get_page_fonts(*id).map(|f| f.len()).unwrap_or(0);
    }
    println!("get_page_fonts {:?} ({fonts} font refs)", t.elapsed());

    let t = Instant::now();
    let doc = vibe_docfmt::pdf::read(&bytes).unwrap();
    println!(
        "pdf::read      {:?} ({} lines)",
        t.elapsed(),
        doc.block_count()
    );

    let t = Instant::now();
    let text = vibe_docfmt::render(&doc);
    println!("render         {:?} ({} chars)", t.elapsed(), text.len());

    let t = Instant::now();
    let parsed = vibe_docfmt::parse_text(vibe_docfmt::DocFormat::Pdf, &text);
    println!(
        "parse          {:?} ({} blocks)",
        t.elapsed(),
        parsed.block_count()
    );

    let t = Instant::now();
    let rewrite = vibe_docfmt::pdf::write(&bytes, &parsed).unwrap();
    println!(
        "pdf::write     {:?} ({} bytes)",
        t.elapsed(),
        rewrite.bytes.len()
    );
}
