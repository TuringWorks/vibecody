//! Reading an EPUB for display: chapters with their images, stylesheets,
//! navigation and metadata.
//!
//! The fixture is deliberately awkward in the ways real books are — content in
//! a subdirectory, images one level up from the chapter, a stylesheet whose own
//! `url()` is relative to *itself*, and an EPUB 3 nav document with nesting.

use vibe_docfmt::epub_view::{self, mime_for, resolve_href};
use vibe_docfmt::zipedit::{self, ZipEntry};

fn entry(name: &str, data: Vec<u8>) -> ZipEntry {
    ZipEntry {
        name: name.to_string(),
        data,
        compression: zip::CompressionMethod::Deflated,
        is_dir: false,
    }
}

const CHAPTER_ONE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>The First Chapter</title>
<link rel="stylesheet" type="text/css" href="../css/book.css"/>
<style>p.drop { font-size: 2em }</style>
</head><body><h1>The First Chapter</h1><p>Text with <em>emphasis</em>.</p>
<img src="../images/fig1.png" alt="Figure 1"/>
<p><a href="ch2.xhtml#later">forward</a></p></body></html>"#;

const CHAPTER_TWO: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>The Second Chapter</title></head>
<body><h1>The Second Chapter</h1><p id="later">Landing point.</p></body></html>"#;

const NAV: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Contents</title></head><body>
<nav epub:type="toc"><ol>
<li><a href="text/ch1.xhtml">One</a><ol><li><a href="text/ch1.xhtml#part2">One, part two</a></li></ol></li>
<li><a href="text/ch2.xhtml">Two</a></li>
</ol></nav></body></html>"#;

fn book_bytes() -> Vec<u8> {
    let opf = r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:title>A Test Book</dc:title><dc:creator>Ada Lovelace</dc:creator><dc:creator>Charles Babbage</dc:creator>
<dc:language>en</dc:language><dc:publisher>Analytical Press</dc:publisher></metadata>
<manifest>
<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
<item id="c1" href="text/ch1.xhtml" media-type="application/xhtml+xml"/>
<item id="c2" href="text/ch2.xhtml" media-type="application/xhtml+xml"/>
<item id="css" href="css/book.css" media-type="text/css"/>
<item id="fig1" href="images/fig1.png" media-type="image/png"/>
<item id="cover" href="images/cover.jpg" media-type="image/jpeg" properties="cover-image"/>
<item id="font" href="css/fonts/serif.woff2" media-type="font/woff2"/>
</manifest>
<spine><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#;
    let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles>
<rootfile full-path="OEBPS/book.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;
    // The stylesheet's url() is relative to the stylesheet, not the chapter.
    let css = "@font-face { font-family: Serif; src: url(fonts/serif.woff2); }\nbody { font-family: Serif }";

    let mut entries = vec![
        entry("mimetype", b"application/epub+zip".to_vec()),
        entry("META-INF/container.xml", container.as_bytes().to_vec()),
        entry("OEBPS/book.opf", opf.as_bytes().to_vec()),
        entry("OEBPS/nav.xhtml", NAV.as_bytes().to_vec()),
        entry("OEBPS/text/ch1.xhtml", CHAPTER_ONE.as_bytes().to_vec()),
        entry("OEBPS/text/ch2.xhtml", CHAPTER_TWO.as_bytes().to_vec()),
        entry("OEBPS/css/book.css", css.as_bytes().to_vec()),
        entry("OEBPS/css/fonts/serif.woff2", b"woff2-bytes".to_vec()),
        entry("OEBPS/images/fig1.png", b"png-bytes".to_vec()),
        entry("OEBPS/images/cover.jpg", b"cover-jpeg-bytes".to_vec()),
    ];
    entries[0].compression = zip::CompressionMethod::Stored;
    zipedit::write_entries(&entries).expect("fixture book")
}

#[test]
fn deflated_chapters_are_readable() {
    // The whole reason this path moved to the backend: every chapter in a real
    // book is deflate-compressed, and the browser-side reader dropped them.
    let bytes = book_bytes();
    let entries = zipedit::read_entries(&bytes).expect("entries");
    let chapter = zipedit::find(&entries, "OEBPS/text/ch1.xhtml").expect("chapter entry");
    assert_eq!(chapter.compression, zip::CompressionMethod::Deflated);

    let view = epub_view::read_chapter(&bytes, "OEBPS/text/ch1.xhtml").expect("read chapter");
    assert!(view.html.contains("The First Chapter"), "chapter body decoded: {}", view.html);
}

#[test]
fn reads_metadata_spine_and_cover() {
    let book = epub_view::read_book(&book_bytes()).expect("read book");
    assert_eq!(book.title.as_deref(), Some("A Test Book"));
    assert_eq!(book.authors, vec!["Ada Lovelace", "Charles Babbage"]);
    assert_eq!(book.language.as_deref(), Some("en"));
    assert_eq!(book.publisher.as_deref(), Some("Analytical Press"));

    // The nav document is not a chapter; the two spine items are.
    let paths: Vec<&str> = book.chapters.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(paths, vec!["OEBPS/text/ch1.xhtml", "OEBPS/text/ch2.xhtml"]);
    assert_eq!(book.chapters[0].title.as_deref(), Some("The First Chapter"));

    let cover = book.cover.expect("cover image");
    assert_eq!(cover.mime, "image/jpeg");
    assert_eq!(cover.data, b"cover-jpeg-bytes");
}

#[test]
fn reads_a_nested_epub3_table_of_contents() {
    let book = epub_view::read_book(&book_bytes()).expect("read book");
    let labels: Vec<(&str, u8)> =
        book.toc.iter().map(|e| (e.label.as_str(), e.level)).collect();
    assert_eq!(labels, vec![("One", 0), ("One, part two", 1), ("Two", 0)]);

    assert_eq!(book.toc[0].path, "OEBPS/text/ch1.xhtml");
    assert_eq!(book.toc[1].fragment.as_deref(), Some("part2"), "the anchor is kept");
    assert!(book.warnings.is_empty(), "a book with a nav document warns about nothing");
}

#[test]
fn falls_back_to_the_ncx_when_there_is_no_nav_document() {
    let ncx = r#"<?xml version="1.0" encoding="utf-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/"><navMap>
<navPoint id="n1" playOrder="1"><navLabel><text>Opening</text></navLabel><content src="text/ch1.xhtml"/>
<navPoint id="n1a" playOrder="2"><navLabel><text>A section</text></navLabel><content src="text/ch1.xhtml#s1"/></navPoint>
</navPoint>
<navPoint id="n2" playOrder="3"><navLabel><text>Closing</text></navLabel><content src="text/ch2.xhtml"/></navPoint>
</navMap></ncx>"#;
    let opf = r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Older Book</dc:title></metadata>
<manifest><item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
<item id="c1" href="text/ch1.xhtml" media-type="application/xhtml+xml"/>
<item id="c2" href="text/ch2.xhtml" media-type="application/xhtml+xml"/></manifest>
<spine toc="ncx"><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#;
    let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles>
<rootfile full-path="OEBPS/book.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;

    let bytes = zipedit::write_entries(&[
        entry("META-INF/container.xml", container.as_bytes().to_vec()),
        entry("OEBPS/book.opf", opf.as_bytes().to_vec()),
        entry("OEBPS/toc.ncx", ncx.as_bytes().to_vec()),
        entry("OEBPS/text/ch1.xhtml", CHAPTER_TWO.as_bytes().to_vec()),
        entry("OEBPS/text/ch2.xhtml", CHAPTER_TWO.as_bytes().to_vec()),
    ])
    .expect("ncx book");

    let book = epub_view::read_book(&bytes).expect("read book");
    let labels: Vec<(&str, u8)> = book.toc.iter().map(|e| (e.label.as_str(), e.level)).collect();
    assert_eq!(labels, vec![("Opening", 0), ("A section", 1), ("Closing", 0)]);
    assert_eq!(book.toc[1].path, "OEBPS/text/ch1.xhtml");
}

#[test]
fn a_chapter_carries_the_images_it_references() {
    let view = epub_view::read_chapter(&book_bytes(), "OEBPS/text/ch1.xhtml").expect("chapter");
    let image = view
        .resources
        .iter()
        .find(|r| r.href == "../images/fig1.png")
        .expect("the image the chapter points at");
    assert_eq!(image.path, "OEBPS/images/fig1.png", "resolved against the chapter's directory");
    assert_eq!(image.mime, "image/png");
    assert_eq!(image.data, b"png-bytes");
}

#[test]
fn stylesheets_come_with_their_own_relative_assets_resolved() {
    let view = epub_view::read_chapter(&book_bytes(), "OEBPS/text/ch1.xhtml").expect("chapter");
    assert!(view.css.contains("font-family: Serif"), "linked stylesheet included");
    assert!(view.css.contains("p.drop"), "inline <style> included");
    // `url(fonts/serif.woff2)` is relative to the stylesheet, which lives in a
    // different directory from the chapter — resolving it against the chapter
    // would 404 in a way that looks like a missing font rather than a bug.
    assert!(
        view.css.contains("url(\"OEBPS/css/fonts/serif.woff2\")")
            || view.css.contains("url(OEBPS/css/fonts/serif.woff2)"),
        "stylesheet url() rebased to a container path: {}",
        view.css
    );
    assert!(
        view.resources.iter().any(|r| r.path == "OEBPS/css/fonts/serif.woff2"),
        "the font travels with the chapter"
    );
}

#[test]
fn a_missing_resource_is_reported_not_skipped_silently() {
    let chapter = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head>
<body><p>x</p><img src="../images/gone.png"/></body></html>"#;
    let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles>
<rootfile full-path="OEBPS/book.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;
    let opf = r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata/><manifest>
<item id="c1" href="text/ch1.xhtml" media-type="application/xhtml+xml"/></manifest>
<spine><itemref idref="c1"/></spine></package>"#;
    let bytes = zipedit::write_entries(&[
        entry("META-INF/container.xml", container.as_bytes().to_vec()),
        entry("OEBPS/book.opf", opf.as_bytes().to_vec()),
        entry("OEBPS/text/ch1.xhtml", chapter.as_bytes().to_vec()),
    ])
    .expect("book");

    let view = epub_view::read_chapter(&bytes, "OEBPS/text/ch1.xhtml").expect("chapter");
    assert!(
        view.warnings.iter().any(|w| w.code == "epub.missing_resource"),
        "a broken image is named: {:?}",
        view.warnings
    );
}

#[test]
fn remote_and_inline_references_are_left_for_the_sanitiser() {
    let chapter = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body>
<img src="https://tracker.example/pixel.gif"/>
<img src="data:image/gif;base64,R0lGODlhAQABAAAAACw="/>
<p>x</p></body></html>"#;
    let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles>
<rootfile full-path="book.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;
    let opf = r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata/><manifest>
<item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/></manifest>
<spine><itemref idref="c1"/></spine></package>"#;
    let bytes = zipedit::write_entries(&[
        entry("META-INF/container.xml", container.as_bytes().to_vec()),
        entry("book.opf", opf.as_bytes().to_vec()),
        entry("ch1.xhtml", chapter.as_bytes().to_vec()),
    ])
    .expect("book");

    let view = epub_view::read_chapter(&bytes, "ch1.xhtml").expect("chapter");
    assert!(view.resources.is_empty(), "nothing was fetched for a remote or inline src");
    assert!(
        !view.warnings.iter().any(|w| w.code == "epub.missing_resource"),
        "and neither is reported as missing from the book"
    );
}

#[test]
fn paths_resolve_the_way_a_browser_would() {
    assert_eq!(resolve_href("OEBPS/text/ch1.xhtml", "../images/a.png"), "OEBPS/images/a.png");
    assert_eq!(resolve_href("OEBPS/text/ch1.xhtml", "ch2.xhtml"), "OEBPS/text/ch2.xhtml");
    assert_eq!(resolve_href("OEBPS/text/ch1.xhtml", "./ch2.xhtml#x"), "OEBPS/text/ch2.xhtml");
    assert_eq!(resolve_href("OEBPS/text/ch1.xhtml", "/top.xhtml"), "top.xhtml");
    assert_eq!(resolve_href("ch1.xhtml", "images/a.png"), "images/a.png");
    // Percent-encoded spaces are common in hand-made books.
    assert_eq!(resolve_href("a/b.xhtml", "my%20image.png"), "a/my image.png");
}

#[test]
fn unknown_extensions_do_not_get_a_guessed_content_type() {
    assert_eq!(mime_for("a/b.png"), "image/png");
    assert_eq!(mime_for("a/b.WOFF2"), "font/woff2");
    assert_eq!(mime_for("a/b.bin"), "application/octet-stream");
    assert_eq!(mime_for("noextension"), "application/octet-stream");
}
