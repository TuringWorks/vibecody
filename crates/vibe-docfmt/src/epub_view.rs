//! Reading an EPUB for *display* rather than for editing.
//!
//! The editing model in [`crate::epub`] deliberately flattens a chapter to
//! blocks of text. A reader needs the opposite: the chapter's own markup, its
//! stylesheets, and the images it references, so the book looks like the book.
//!
//! Everything a chapter needs is resolved and returned with it. A viewer cannot
//! fetch `../images/fig1.png` out of a ZIP by URL, so the paths are resolved
//! here — against the chapter's own directory, with `..` segments normalised —
//! and the bytes travel with the chapter.

use serde::{Deserialize, Serialize};

use crate::error::DocError;
use crate::model::Warning;
use crate::xmltree::{self, Element, Node};
use crate::zipedit::{self, ZipEntry};

/// Largest single resource carried with a chapter.
const MAX_RESOURCE_BYTES: usize = 8 * 1024 * 1024;
/// Largest total payload for one chapter's resources.
const MAX_CHAPTER_BYTES: usize = 32 * 1024 * 1024;

/// A file from inside the book, carried alongside the chapter that needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    /// Container path, e.g. `OEBPS/images/fig1.png`.
    pub path: String,
    /// How the chapter referred to it, e.g. `../images/fig1.png`.
    pub href: String,
    pub mime: String,
    pub data: Vec<u8>,
}

/// One spine item, in reading order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterRef {
    /// Container path of the chapter document.
    pub path: String,
    pub title: Option<String>,
}

/// One entry of the book's table of contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TocEntry {
    pub label: String,
    /// Container path, with any fragment kept separately.
    pub path: String,
    pub fragment: Option<String>,
    /// Nesting depth, 0 for a top-level entry.
    pub level: u8,
}

/// The book, minus its chapter bodies.
#[derive(Debug, Clone)]
pub struct Book {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub chapters: Vec<ChapterRef>,
    pub toc: Vec<TocEntry>,
    pub cover: Option<Resource>,
    pub warnings: Vec<Warning>,
}

/// One chapter, ready to render.
#[derive(Debug, Clone)]
pub struct ChapterView {
    pub path: String,
    pub title: Option<String>,
    /// The chapter's `<body>` markup, unsanitised — the caller sanitises at the
    /// DOM sink, which is where the security argument belongs.
    pub html: String,
    /// Every stylesheet the chapter links, plus its inline `<style>` blocks,
    /// concatenated in document order.
    pub css: String,
    pub resources: Vec<Resource>,
    pub warnings: Vec<Warning>,
}

// ── Paths ────────────────────────────────────────────────────────────

/// Resolve `href` against the directory holding `base`, normalising `..`.
pub fn resolve_href(base: &str, href: &str) -> String {
    let href = href.split('#').next().unwrap_or(href);
    let href = percent_decode(href);
    if href.starts_with('/') {
        return normalize(href.trim_start_matches('/'));
    }
    let dir = match base.rfind('/') {
        Some(i) => &base[..=i],
        None => "",
    };
    normalize(&format!("{dir}{href}"))
}

fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(byte) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Content type for a path, by extension.
pub fn mime_for(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "css" => "text/css",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        // Refusing to guess beats claiming a type: the viewer treats an unknown
        // type as "do not render this", which is the safe reading.
        _ => "application/octet-stream",
    }
}

// ── Book ─────────────────────────────────────────────────────────────

/// Read everything about a book except its chapter bodies.
pub fn read_book(bytes: &[u8]) -> Result<Book, DocError> {
    let entries = zipedit::read_entries(bytes)?;
    let opf_path = crate::epub::opf_path_of(&entries)?;
    let opf = zipedit::find(&entries, &opf_path)
        .ok_or_else(|| DocError::Parse(format!("{opf_path} named by container.xml is missing")))?;
    let package = xmltree::parse_bytes(&opf.data)?;

    let manifest = package
        .root
        .children_named("manifest")
        .next()
        .ok_or_else(|| DocError::Parse("the OPF has no <manifest>".into()))?;
    let items: Vec<ManifestItem> = manifest
        .children_named("item")
        .filter_map(|item| {
            Some(ManifestItem {
                id: item.attr("id")?.to_string(),
                path: resolve_href(&opf_path, item.attr("href")?),
                media_type: item.attr("media-type").unwrap_or("").to_string(),
                properties: item.attr("properties").unwrap_or("").to_string(),
            })
        })
        .collect();

    let spine_el = package
        .root
        .children_named("spine")
        .next()
        .ok_or_else(|| DocError::Parse("the OPF has no <spine>".into()))?;

    let mut warnings = Vec::new();
    let chapters: Vec<ChapterRef> = spine_el
        .children_named("itemref")
        .filter_map(|itemref| {
            let idref = itemref.attr("idref")?;
            let item = items.iter().find(|i| i.id == idref)?;
            let readable = item.media_type.contains("xhtml")
                || item.media_type.contains("text/html")
                || item.media_type.is_empty();
            readable.then(|| ChapterRef {
                path: item.path.clone(),
                title: chapter_title(&entries, &item.path),
            })
        })
        .collect();
    if chapters.is_empty() {
        return Err(DocError::Parse("this EPUB's spine has no readable chapters".into()));
    }

    let toc = read_toc(&entries, &items, spine_el, &mut warnings);
    let cover = read_cover(&entries, &items, &package.root, &mut warnings);

    let metadata = package.root.children_named("metadata").next();
    let meta_text = |name: &str| {
        metadata
            .and_then(|m| m.children_named(name).next())
            .map(|e| collapse_ws(&e.text_content()))
            .filter(|t| !t.is_empty())
    };
    let authors: Vec<String> = metadata
        .map(|m| {
            m.children_named("creator")
                .map(|e| collapse_ws(&e.text_content()))
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(Book {
        title: meta_text("title"),
        authors,
        language: meta_text("language"),
        publisher: meta_text("publisher"),
        chapters,
        toc,
        cover,
        warnings,
    })
}

struct ManifestItem {
    id: String,
    path: String,
    media_type: String,
    properties: String,
}

fn collapse_ws(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_ws = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !in_ws && !out.is_empty() {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(c);
            in_ws = false;
        }
    }
    out.trim_end().to_string()
}

fn chapter_title(entries: &[ZipEntry], path: &str) -> Option<String> {
    let entry = zipedit::find(entries, path)?;
    let xml = xmltree::parse_bytes(&entry.data).ok()?;
    let head_title = xml
        .root
        .find_descendant("title")
        .map(|t| collapse_ws(&t.text_content()))
        .filter(|t| !t.is_empty());
    head_title.or_else(|| {
        ["h1", "h2", "h3"].iter().find_map(|tag| {
            xml.root
                .find_descendant(tag)
                .map(|h| collapse_ws(&h.text_content()))
                .filter(|t| !t.is_empty())
        })
    })
}

// ── Table of contents ────────────────────────────────────────────────

fn read_toc(
    entries: &[ZipEntry],
    items: &[ManifestItem],
    spine: &Element,
    warnings: &mut Vec<Warning>,
) -> Vec<TocEntry> {
    // EPUB 3: the navigation document, marked in the manifest.
    if let Some(nav) = items.iter().find(|i| i.properties.split_whitespace().any(|p| p == "nav")) {
        if let Some(toc) = read_nav_document(entries, nav) {
            if !toc.is_empty() {
                return toc;
            }
        }
    }
    // EPUB 2: the NCX, named by the spine.
    let ncx = spine
        .attr("toc")
        .and_then(|id| items.iter().find(|i| i.id == id))
        .or_else(|| items.iter().find(|i| i.media_type.contains("dtbncx")));
    if let Some(ncx) = ncx {
        if let Some(toc) = read_ncx(entries, ncx) {
            if !toc.is_empty() {
                return toc;
            }
        }
    }
    warnings.push(Warning::new(
        "epub.no_toc",
        "this book has no navigation document; the contents list is built from \
         the spine instead",
    ));
    Vec::new()
}

fn read_nav_document(entries: &[ZipEntry], nav: &ManifestItem) -> Option<Vec<TocEntry>> {
    let entry = zipedit::find(entries, &nav.path)?;
    let xml = xmltree::parse_bytes(&entry.data).ok()?;

    // Prefer the nav marked as the toc; fall back to the first <nav>.
    let toc_nav = find_element(&xml.root, &|el| {
        el.local_name() == "nav"
            && el
                .attrs
                .iter()
                .any(|(k, v)| k.ends_with("type") && v.split_whitespace().any(|t| t == "toc"))
    })
    .or_else(|| find_element(&xml.root, &|el| el.local_name() == "nav"))?;

    let list = toc_nav.children_named("ol").next()?;
    let mut out = Vec::new();
    collect_nav_items(list, &nav.path, 0, &mut out);
    Some(out)
}

fn collect_nav_items(list: &Element, base: &str, level: u8, out: &mut Vec<TocEntry>) {
    for item in list.children_named("li") {
        if let Some(anchor) = item.children_named("a").next() {
            if let Some(href) = anchor.attr("href") {
                out.push(TocEntry {
                    label: collapse_ws(&anchor.text_content()),
                    path: resolve_href(base, href),
                    fragment: fragment_of(href),
                    level,
                });
            }
        }
        for nested in item.children_named("ol") {
            collect_nav_items(nested, base, level.saturating_add(1), out);
        }
    }
}

fn read_ncx(entries: &[ZipEntry], ncx: &ManifestItem) -> Option<Vec<TocEntry>> {
    let entry = zipedit::find(entries, &ncx.path)?;
    let xml = xmltree::parse_bytes(&entry.data).ok()?;
    let map = xml.root.children_named("navMap").next()?;
    let mut out = Vec::new();
    collect_nav_points(map, &ncx.path, 0, &mut out);
    Some(out)
}

fn collect_nav_points(parent: &Element, base: &str, level: u8, out: &mut Vec<TocEntry>) {
    for point in parent.children_named("navPoint") {
        let label = point
            .children_named("navLabel")
            .next()
            .and_then(|l| l.children_named("text").next())
            .map(|t| collapse_ws(&t.text_content()))
            .unwrap_or_default();
        if let Some(src) = point.children_named("content").next().and_then(|c| c.attr("src")) {
            out.push(TocEntry {
                label,
                path: resolve_href(base, src),
                fragment: fragment_of(src),
                level,
            });
        }
        collect_nav_points(point, base, level.saturating_add(1), out);
    }
}

fn fragment_of(href: &str) -> Option<String> {
    href.split_once('#').map(|(_, f)| f.to_string()).filter(|f| !f.is_empty())
}

fn find_element<'a>(el: &'a Element, test: &dyn Fn(&Element) -> bool) -> Option<&'a Element> {
    if test(el) {
        return Some(el);
    }
    el.children.iter().find_map(|child| match child {
        Node::Element(e) => find_element(e, test),
        _ => None,
    })
}

// ── Cover ────────────────────────────────────────────────────────────

fn read_cover(
    entries: &[ZipEntry],
    items: &[ManifestItem],
    package: &Element,
    warnings: &mut Vec<Warning>,
) -> Option<Resource> {
    // EPUB 3 marks the cover in the manifest; EPUB 2 points at it from a
    // <meta name="cover" content="…"> in the metadata.
    let by_property =
        items.iter().find(|i| i.properties.split_whitespace().any(|p| p == "cover-image"));
    let by_meta = package
        .children_named("metadata")
        .next()
        .and_then(|m| {
            m.children_named("meta")
                .find(|meta| meta.attr("name") == Some("cover"))
                .and_then(|meta| meta.attr("content"))
                .map(str::to_string)
        })
        .and_then(|id| items.iter().find(|i| i.id == id));
    let item = by_property.or(by_meta).or_else(|| {
        items.iter().find(|i| i.media_type.starts_with("image/") && i.id.contains("cover"))
    })?;

    let entry = zipedit::find(entries, &item.path)?;
    if entry.data.len() > MAX_RESOURCE_BYTES {
        warnings.push(Warning::new(
            "epub.cover_too_large",
            format!("the cover image is {} and was not loaded", human_size(entry.data.len())),
        ));
        return None;
    }
    Some(Resource {
        path: item.path.clone(),
        href: item.path.clone(),
        mime: if item.media_type.is_empty() { mime_for(&item.path).to_string() } else { item.media_type.clone() },
        data: entry.data.clone(),
    })
}

fn human_size(bytes: usize) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    if bytes < 1024 * 1024 {
        return format!("{:.1} KB", bytes as f64 / 1024.0);
    }
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

// ── Chapter ──────────────────────────────────────────────────────────

/// Read one chapter with the stylesheets and media it references.
pub fn read_chapter(bytes: &[u8], path: &str) -> Result<ChapterView, DocError> {
    let entries = zipedit::read_entries(bytes)?;
    let entry = zipedit::find(&entries, path)
        .ok_or_else(|| DocError::Parse(format!("{path} is not in this book")))?;
    let xml = xmltree::parse_bytes(&entry.data)?;

    let body = crate::epub::body_of_root(&xml.root)
        .ok_or_else(|| DocError::Parse(format!("{path} has no <body>")))?;
    let html = xmltree::serialize_children(body);

    let mut collector = Collector {
        entries: &entries,
        base: path.to_string(),
        resources: Vec::new(),
        total: 0,
        warnings: Vec::new(),
    };

    // Stylesheets first: their own url() references become resources too.
    let mut css = String::new();
    for link in descendants(&xml.root, "link") {
        let is_stylesheet = link
            .attr("rel")
            .map(|rel| rel.split_whitespace().any(|r| r.eq_ignore_ascii_case("stylesheet")))
            .unwrap_or(false);
        let Some(href) = link.attr("href") else { continue };
        if !is_stylesheet {
            continue;
        }
        if let Some(text) = collector.text_of(href) {
            css.push_str(&text);
            css.push('\n');
        }
    }
    for style in descendants(&xml.root, "style") {
        css.push_str(&style.text_content());
        css.push('\n');
    }
    for path in css_urls(&css) {
        collector.take_path(&path);
    }

    // Then everything the markup points at.
    for (tag, attrs) in
        [("img", &["src"][..]), ("image", &["href", "xlink:href"]), ("source", &["src"]), ("audio", &["src"]), ("video", &["src", "poster"])]
    {
        for el in descendants(&xml.root, tag) {
            for attr in attrs {
                if let Some(href) = el.attr(attr) {
                    collector.take(href);
                }
            }
        }
    }

    let title = chapter_title(&entries, path);
    Ok(ChapterView {
        path: path.to_string(),
        title,
        html,
        css,
        resources: collector.resources,
        warnings: collector.warnings,
    })
}

struct Collector<'a> {
    entries: &'a [ZipEntry],
    base: String,
    resources: Vec<Resource>,
    total: usize,
    warnings: Vec<Warning>,
}

impl Collector<'_> {
    /// Pull a file the markup referenced, resolving it against the chapter.
    fn take(&mut self, href: &str) {
        // A remote or inline reference is not ours to resolve; the viewer's
        // sanitiser decides what to do with it.
        if href.starts_with("data:") || href.contains("://") || href.starts_with("//") {
            return;
        }
        let path = resolve_href(&self.base, href);
        self.insert(path, href.to_string());
    }

    /// Pull a file named by a container path that is already resolved — what a
    /// rebased stylesheet `url()` holds. Resolving it a second time against the
    /// chapter would point at a directory that does not exist.
    fn take_path(&mut self, path: &str) {
        self.insert(path.to_string(), path.to_string());
    }

    fn insert(&mut self, path: String, href: String) {
        if path.is_empty() || self.resources.iter().any(|r| r.href == href && r.path == path) {
            return;
        }
        let Some(entry) = zipedit::find(self.entries, &path) else {
            self.warn(
                "epub.missing_resource",
                format!("{href} is referenced by this chapter but is not in the book"),
            );
            return;
        };
        if entry.data.len() > MAX_RESOURCE_BYTES || self.total + entry.data.len() > MAX_CHAPTER_BYTES
        {
            self.warn(
                "epub.resource_too_large",
                format!(
                    "{href} ({}) was not loaded: this chapter's images already total {}",
                    human_size(entry.data.len()),
                    human_size(self.total)
                ),
            );
            return;
        }
        self.total += entry.data.len();
        self.resources.push(Resource {
            mime: mime_for(&path).to_string(),
            path,
            href,
            data: entry.data.clone(),
        });
    }

    /// Read a referenced text file (a stylesheet) and record it as a resource.
    fn text_of(&mut self, href: &str) -> Option<String> {
        if href.contains("://") {
            return None;
        }
        let path = resolve_href(&self.base, href);
        let entry = zipedit::find(self.entries, &path)?;
        let text = String::from_utf8(entry.data.clone()).ok()?;
        // The stylesheet's own url() references resolve against *its* directory,
        // not the chapter's, so rewrite them to container paths now.
        Some(rebase_css(&text, &path))
    }

    fn warn(&mut self, code: &str, message: String) {
        if !self.warnings.iter().any(|w| w.code == code) {
            self.warnings.push(Warning::new(code, message));
        }
    }
}

fn descendants<'a>(root: &'a Element, local: &str) -> Vec<&'a Element> {
    let mut out = Vec::new();
    fn walk<'a>(el: &'a Element, local: &str, out: &mut Vec<&'a Element>) {
        if el.local_name() == local {
            out.push(el);
        }
        for child in &el.children {
            if let Node::Element(e) = child {
                walk(e, local, out);
            }
        }
    }
    walk(root, local, &mut out);
    out
}

/// Every `url(...)` target in a stylesheet.
pub fn css_urls(css: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = css.as_bytes();
    let mut i = 0;
    while let Some(found) = css[i..].find("url(") {
        let start = i + found + 4;
        let Some(end_rel) = css[start..].find(')') else { break };
        let end = start + end_rel;
        let raw = css[start..end].trim().trim_matches(['"', '\''].as_ref()).trim();
        if !raw.is_empty() && !raw.starts_with("data:") && !raw.contains("://") {
            out.push(raw.to_string());
        }
        i = end + 1;
        if i >= bytes.len() {
            break;
        }
    }
    out
}

/// Rewrite a stylesheet's relative `url(...)` targets to container paths, so
/// they mean the same thing once the CSS is lifted out of its own directory.
fn rebase_css(css: &str, css_path: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(found) = rest.find("url(") {
        out.push_str(&rest[..found + 4]);
        rest = &rest[found + 4..];
        let Some(end) = rest.find(')') else {
            break;
        };
        let raw = &rest[..end];
        let trimmed = raw.trim();
        let quote = trimmed.starts_with('"') || trimmed.starts_with('\'');
        let bare = trimmed.trim_matches(['"', '\''].as_ref());
        if bare.is_empty() || bare.starts_with("data:") || bare.contains("://") {
            out.push_str(raw);
        } else {
            let resolved = resolve_href(css_path, bare);
            if quote {
                out.push('"');
                out.push_str(&resolved);
                out.push('"');
            } else {
                out.push_str(&resolved);
            }
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}
