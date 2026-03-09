// Document Ingestion Pipeline — multi-format document parsing, chunking, and metadata extraction for RAG.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Supported document formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFormat {
    Markdown,
    Html,
    PlainText,
    Pdf,
    Json,
    Csv,
    Xml,
    Rst,         // reStructuredText
    Latex,
    CodeFile,    // source code (auto-detected by extension)
}

impl DocumentFormat {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "md" | "mdx" => Self::Markdown,
            "html" | "htm" => Self::Html,
            "txt" | "text" => Self::PlainText,
            "pdf" => Self::Pdf,
            "json" | "jsonl" | "ndjson" => Self::Json,
            "csv" | "tsv" => Self::Csv,
            "xml" | "xhtml" | "svg" => Self::Xml,
            "rst" => Self::Rst,
            "tex" | "latex" => Self::Latex,
            _ => Self::CodeFile,
        }
    }
}

/// Document metadata extracted during ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub source_url: Option<String>,
    pub source_path: Option<String>,
    pub format: DocumentFormat,
    pub language: Option<String>,
    pub created_at: Option<String>,
    pub ingested_at: String,
    pub word_count: usize,
    pub char_count: usize,
    pub tags: Vec<String>,
    pub custom: HashMap<String, String>,
}

/// A chunk of text from a document, ready for embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: String,
    pub content: String,
    pub metadata: DocumentMetadata,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub section_title: Option<String>,
    pub embedding: Option<Vec<f32>>,
}

/// Chunking strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    pub max_tokens: usize,        // target chunk size in tokens (default: 512)
    pub overlap_tokens: usize,    // overlap between chunks (default: 50)
    pub min_chunk_size: usize,    // minimum chunk size to keep (default: 50)
    pub respect_boundaries: bool, // try to break at paragraph/section boundaries
    pub include_metadata: bool,   // prepend section title to chunk
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            overlap_tokens: 50,
            min_chunk_size: 50,
            respect_boundaries: true,
            include_metadata: true,
        }
    }
}

/// Ingested document before chunking
#[derive(Debug, Clone)]
pub struct IngestedDocument {
    pub content: String,
    pub metadata: DocumentMetadata,
    pub sections: Vec<DocumentSection>,
}

/// A section within a document (e.g., a heading + content)
#[derive(Debug, Clone)]
pub struct DocumentSection {
    pub title: Option<String>,
    pub content: String,
    pub level: usize,  // heading level (1-6) or 0 for no heading
    pub start_offset: usize,
}

/// Document ingestion pipeline
pub struct DocumentIngestor {
    pub config: ChunkingConfig,
}

impl DocumentIngestor {
    pub fn new() -> Self {
        Self { config: ChunkingConfig::default() }
    }

    pub fn with_config(config: ChunkingConfig) -> Self {
        Self { config }
    }

    /// Ingest a file from the filesystem
    pub fn ingest_file(&self, path: &Path) -> anyhow::Result<IngestedDocument> {
        let content = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
        let format = DocumentFormat::from_extension(ext);
        let title = path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string());

        let metadata = DocumentMetadata {
            title,
            author: None,
            source_url: None,
            source_path: Some(path.to_string_lossy().to_string()),
            format: format.clone(),
            language: None,
            created_at: None,
            ingested_at: now_iso(),
            word_count: content.split_whitespace().count(),
            char_count: content.len(),
            tags: vec![],
            custom: HashMap::new(),
        };

        let sections = self.extract_sections(&content, &format);

        Ok(IngestedDocument { content, metadata, sections })
    }

    /// Ingest raw text content with metadata
    pub fn ingest_text(&self, content: &str, title: Option<&str>, source_url: Option<&str>, format: DocumentFormat) -> IngestedDocument {
        let metadata = DocumentMetadata {
            title: title.map(|s| s.to_string()),
            author: None,
            source_url: source_url.map(|s| s.to_string()),
            source_path: None,
            format: format.clone(),
            language: None,
            created_at: None,
            ingested_at: now_iso(),
            word_count: content.split_whitespace().count(),
            char_count: content.len(),
            tags: vec![],
            custom: HashMap::new(),
        };

        let sections = self.extract_sections(content, &format);

        IngestedDocument { content: content.to_string(), metadata, sections }
    }

    /// Ingest HTML content, stripping tags and extracting text
    pub fn ingest_html(&self, html: &str, source_url: Option<&str>) -> IngestedDocument {
        let text = strip_html_tags(html);
        let title = extract_html_title(html);
        self.ingest_text(&text, title.as_deref(), source_url, DocumentFormat::Html)
    }

    /// Extract sections from document based on format
    fn extract_sections(&self, content: &str, format: &DocumentFormat) -> Vec<DocumentSection> {
        match format {
            DocumentFormat::Markdown | DocumentFormat::Rst => self.extract_markdown_sections(content),
            DocumentFormat::Html => self.extract_markdown_sections(&strip_html_tags(content)),
            DocumentFormat::Latex => self.extract_latex_sections(content),
            _ => {
                // For plain text and code, split by double newlines (paragraphs)
                self.extract_paragraph_sections(content)
            }
        }
    }

    /// Extract sections from Markdown by heading boundaries
    fn extract_markdown_sections(&self, content: &str) -> Vec<DocumentSection> {
        let mut sections = Vec::new();
        let mut current_title: Option<String> = None;
        let mut current_level = 0usize;
        let mut current_content = String::new();
        let mut current_start = 0usize;

        for (offset, line) in content.lines().enumerate() {
            // Check for heading
            if let Some(heading) = parse_markdown_heading(line) {
                // Save previous section if it has content
                if !current_content.trim().is_empty() {
                    sections.push(DocumentSection {
                        title: current_title.take(),
                        content: current_content.trim().to_string(),
                        level: current_level,
                        start_offset: current_start,
                    });
                }
                current_title = Some(heading.1.to_string());
                current_level = heading.0;
                current_content = String::new();
                current_start = offset;
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Save last section
        if !current_content.trim().is_empty() {
            sections.push(DocumentSection {
                title: current_title,
                content: current_content.trim().to_string(),
                level: current_level,
                start_offset: current_start,
            });
        }

        if sections.is_empty() {
            sections.push(DocumentSection {
                title: None,
                content: content.to_string(),
                level: 0,
                start_offset: 0,
            });
        }

        sections
    }

    fn extract_latex_sections(&self, content: &str) -> Vec<DocumentSection> {
        // Simple LaTeX section extraction
        let mut sections = Vec::new();
        let mut current_title: Option<String> = None;
        let mut current_content = String::new();
        let mut current_start = 0usize;

        for (offset, line) in content.lines().enumerate() {
            if line.trim().starts_with("\\section{") || line.trim().starts_with("\\subsection{") || line.trim().starts_with("\\chapter{") {
                if !current_content.trim().is_empty() {
                    sections.push(DocumentSection {
                        title: current_title.take(),
                        content: current_content.trim().to_string(),
                        level: if line.contains("chapter") { 1 } else if line.contains("subsection") { 3 } else { 2 },
                        start_offset: current_start,
                    });
                }
                // Extract title from \section{Title}
                if let Some(start) = line.find('{') {
                    if let Some(end) = line.rfind('}') {
                        current_title = Some(line[start+1..end].to_string());
                    }
                }
                current_content = String::new();
                current_start = offset;
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        if !current_content.trim().is_empty() {
            sections.push(DocumentSection {
                title: current_title,
                content: current_content.trim().to_string(),
                level: 0,
                start_offset: current_start,
            });
        }

        if sections.is_empty() {
            sections.push(DocumentSection {
                title: None,
                content: content.to_string(),
                level: 0,
                start_offset: 0,
            });
        }

        sections
    }

    fn extract_paragraph_sections(&self, content: &str) -> Vec<DocumentSection> {
        let paragraphs: Vec<&str> = content.split("\n\n").collect();
        let mut sections = Vec::new();
        let mut offset = 0;

        for para in paragraphs {
            let trimmed = para.trim();
            if !trimmed.is_empty() {
                sections.push(DocumentSection {
                    title: None,
                    content: trimmed.to_string(),
                    level: 0,
                    start_offset: offset,
                });
            }
            offset += para.len() + 2; // +2 for \n\n
        }

        if sections.is_empty() {
            sections.push(DocumentSection {
                title: None,
                content: content.to_string(),
                level: 0,
                start_offset: 0,
            });
        }

        sections
    }

    /// Chunk a document into embedding-ready pieces
    pub fn chunk(&self, doc: &IngestedDocument) -> Vec<DocumentChunk> {
        let mut chunks = Vec::new();
        let mut chunk_index = 0;

        for section in &doc.sections {
            let section_chunks = self.chunk_text(&section.content, &self.config);

            for text in section_chunks {
                let content = if self.config.include_metadata {
                    if let Some(ref title) = section.title {
                        format!("## {}\n\n{}", title, text)
                    } else {
                        text.clone()
                    }
                } else {
                    text.clone()
                };

                let chunk_id = format!("{}-{}",
                    doc.metadata.source_url.as_deref()
                        .or(doc.metadata.source_path.as_deref())
                        .or(doc.metadata.title.as_deref())
                        .unwrap_or("unknown"),
                    chunk_index);

                chunks.push(DocumentChunk {
                    id: chunk_id,
                    content,
                    metadata: doc.metadata.clone(),
                    chunk_index,
                    total_chunks: 0, // filled in below
                    start_offset: section.start_offset,
                    end_offset: section.start_offset + text.len(),
                    section_title: section.title.clone(),
                    embedding: None,
                });

                chunk_index += 1;
            }
        }

        // Fill total_chunks
        let total = chunks.len();
        for c in &mut chunks {
            c.total_chunks = total;
        }

        chunks
    }

    /// Split text into chunks respecting token limits
    fn chunk_text(&self, text: &str, config: &ChunkingConfig) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return vec![];
        }

        // Approximate tokens as words * 1.3
        let max_words = (config.max_tokens as f64 / 1.3) as usize;
        let overlap_words = (config.overlap_tokens as f64 / 1.3) as usize;
        let min_words = (config.min_chunk_size as f64 / 1.3) as usize;

        if words.len() <= max_words {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < words.len() {
            let end = (start + max_words).min(words.len());

            // Try to break at sentence boundary if respecting boundaries
            let actual_end = if config.respect_boundaries && end < words.len() {
                // Look backwards for sentence-ending punctuation
                let mut best = end;
                for i in (start + min_words..end).rev() {
                    let word = words[i];
                    if word.ends_with('.') || word.ends_with('!') || word.ends_with('?') || word.ends_with(':') {
                        best = i + 1;
                        break;
                    }
                }
                best
            } else {
                end
            };

            let chunk: String = words[start..actual_end].join(" ");
            if chunk.split_whitespace().count() >= min_words || chunks.is_empty() {
                chunks.push(chunk);
            }

            // Advance with overlap
            start = if actual_end >= words.len() {
                words.len()
            } else if overlap_words > 0 && actual_end > overlap_words {
                actual_end - overlap_words
            } else {
                actual_end
            };
        }

        chunks
    }

    /// Ingest a directory of documents recursively
    pub fn ingest_directory(&self, dir: &Path, extensions: &[&str]) -> anyhow::Result<Vec<IngestedDocument>> {
        let mut docs = Vec::new();

        if !dir.is_dir() {
            anyhow::bail!("{} is not a directory", dir.display());
        }

        self.walk_directory(dir, extensions, &mut docs)?;
        Ok(docs)
    }

    fn walk_directory(&self, dir: &Path, extensions: &[&str], docs: &mut Vec<IngestedDocument>) -> anyhow::Result<()> {
        let skip_dirs = [".git", "node_modules", "target", "dist", "build", "__pycache__", ".venv", "venv"];

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !skip_dirs.contains(&dir_name) {
                    self.walk_directory(&path, extensions, docs)?;
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.is_empty() || extensions.contains(&ext) {
                    match self.ingest_file(&path) {
                        Ok(doc) => docs.push(doc),
                        Err(e) => eprintln!("Warning: failed to ingest {}: {}", path.display(), e),
                    }
                }
            }
        }

        Ok(())
    }
}

/// Parse a Markdown heading line, returning (level, text)
fn parse_markdown_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level > 6 || level == 0 {
        return None;
    }
    let text = trimmed[level..].trim().trim_end_matches('#').trim();
    if text.is_empty() {
        return None;
    }
    Some((level, text))
}

/// Strip HTML tags and return plain text
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let lower = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        if !in_tag && chars[i] == '<' {
            in_tag = true;
            // Check for script/style start
            let remaining: String = lower_chars[i..].iter().take(20).collect();
            if remaining.starts_with("<script") {
                in_script = true;
            } else if remaining.starts_with("<style") {
                in_style = true;
            } else if remaining.starts_with("</script") {
                in_script = false;
            } else if remaining.starts_with("</style") {
                in_style = false;
            }
            // Convert <br>, <p>, <div>, <li>, <tr> to newlines
            if remaining.starts_with("<br") || remaining.starts_with("<p") || remaining.starts_with("<div")
                || remaining.starts_with("<li") || remaining.starts_with("<tr") || remaining.starts_with("</p") {
                result.push('\n');
            }
        } else if in_tag && chars[i] == '>' {
            in_tag = false;
        } else if !in_tag && !in_script && !in_style {
            // Decode common HTML entities
            if chars[i] == '&' {
                let entity: String = chars[i..].iter().take(10).collect();
                if entity.starts_with("&amp;") {
                    result.push('&');
                    i += 4;
                } else if entity.starts_with("&lt;") {
                    result.push('<');
                    i += 3;
                } else if entity.starts_with("&gt;") {
                    result.push('>');
                    i += 3;
                } else if entity.starts_with("&quot;") {
                    result.push('"');
                    i += 5;
                } else if entity.starts_with("&nbsp;") {
                    result.push(' ');
                    i += 5;
                } else if entity.starts_with("&#39;") || entity.starts_with("&apos;") {
                    result.push('\'');
                    i += if entity.starts_with("&#39;") { 4 } else { 5 };
                } else {
                    result.push('&');
                }
            } else {
                result.push(chars[i]);
            }
        }
        i += 1;
    }

    // Collapse multiple newlines
    let mut cleaned = String::with_capacity(result.len());
    let mut last_was_newline = false;
    for ch in result.chars() {
        if ch == '\n' {
            if !last_was_newline {
                cleaned.push('\n');
            }
            last_was_newline = true;
        } else {
            last_was_newline = false;
            cleaned.push(ch);
        }
    }

    cleaned.trim().to_string()
}

/// Extract title from HTML <title> tag
fn extract_html_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")?;
    let end = lower[start..].find("</title>")?;
    let title = &html[start + 7..start + end];
    let trimmed = title.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_format_from_extension() {
        assert_eq!(DocumentFormat::from_extension("md"), DocumentFormat::Markdown);
        assert_eq!(DocumentFormat::from_extension("html"), DocumentFormat::Html);
        assert_eq!(DocumentFormat::from_extension("txt"), DocumentFormat::PlainText);
        assert_eq!(DocumentFormat::from_extension("pdf"), DocumentFormat::Pdf);
        assert_eq!(DocumentFormat::from_extension("json"), DocumentFormat::Json);
        assert_eq!(DocumentFormat::from_extension("csv"), DocumentFormat::Csv);
        assert_eq!(DocumentFormat::from_extension("rs"), DocumentFormat::CodeFile);
        assert_eq!(DocumentFormat::from_extension("tex"), DocumentFormat::Latex);
    }

    #[test]
    fn test_parse_markdown_heading() {
        assert_eq!(parse_markdown_heading("# Title"), Some((1, "Title")));
        assert_eq!(parse_markdown_heading("## Sub"), Some((2, "Sub")));
        assert_eq!(parse_markdown_heading("### Three"), Some((3, "Three")));
        assert_eq!(parse_markdown_heading("Not a heading"), None);
        assert_eq!(parse_markdown_heading("#"), None); // empty heading
        assert_eq!(parse_markdown_heading("####### Too deep"), None);
    }

    #[test]
    fn test_strip_html_tags() {
        assert_eq!(strip_html_tags("<p>Hello</p>"), "Hello");
        assert_eq!(strip_html_tags("<b>bold</b> text"), "bold text");
        assert_eq!(strip_html_tags("no tags"), "no tags");
        assert_eq!(strip_html_tags("<script>alert('x')</script>visible"), "visible");
        assert_eq!(strip_html_tags("&amp; &lt; &gt;"), "& < >");
    }

    #[test]
    fn test_extract_html_title() {
        assert_eq!(extract_html_title("<html><title>My Page</title></html>"), Some("My Page".to_string()));
        assert_eq!(extract_html_title("<html></html>"), None);
    }

    #[test]
    fn test_ingest_text() {
        let ingestor = DocumentIngestor::new();
        let doc = ingestor.ingest_text("Hello world", Some("test"), None, DocumentFormat::PlainText);
        assert_eq!(doc.metadata.word_count, 2);
        assert_eq!(doc.metadata.char_count, 11);
        assert!(doc.metadata.title.as_deref() == Some("test"));
    }

    #[test]
    fn test_ingest_markdown_sections() {
        let ingestor = DocumentIngestor::new();
        let md = "# Introduction\n\nHello world.\n\n## Details\n\nSome details here.\n\n## Conclusion\n\nThe end.";
        let doc = ingestor.ingest_text(md, Some("test"), None, DocumentFormat::Markdown);
        assert_eq!(doc.sections.len(), 3);
        assert_eq!(doc.sections[0].title.as_deref(), Some("Introduction"));
        assert_eq!(doc.sections[1].title.as_deref(), Some("Details"));
        assert_eq!(doc.sections[2].title.as_deref(), Some("Conclusion"));
    }

    #[test]
    fn test_chunk_small_document() {
        let ingestor = DocumentIngestor::new();
        let doc = ingestor.ingest_text("Small text.", Some("test"), None, DocumentFormat::PlainText);
        let chunks = ingestor.chunk(&doc);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].total_chunks, 1);
    }

    #[test]
    fn test_chunk_large_document() {
        let ingestor = DocumentIngestor::with_config(ChunkingConfig {
            max_tokens: 20,
            overlap_tokens: 5,
            min_chunk_size: 5,
            respect_boundaries: false,
            include_metadata: false,
        });
        let text = (0..100).map(|i| format!("word{}", i)).collect::<Vec<_>>().join(" ");
        let doc = ingestor.ingest_text(&text, Some("test"), None, DocumentFormat::PlainText);
        let chunks = ingestor.chunk(&doc);
        assert!(chunks.len() > 1);
        // Verify no chunk is empty
        for c in &chunks {
            assert!(!c.content.trim().is_empty());
        }
    }

    #[test]
    fn test_chunk_with_section_titles() {
        let ingestor = DocumentIngestor::new();
        let md = "# Section A\n\nContent of section A.\n\n# Section B\n\nContent of section B.";
        let doc = ingestor.ingest_text(md, None, None, DocumentFormat::Markdown);
        let chunks = ingestor.chunk(&doc);
        assert!(chunks.len() >= 2);
        assert!(chunks[0].content.contains("Section A"));
        assert!(chunks[1].content.contains("Section B"));
    }

    #[test]
    fn test_ingest_html() {
        let ingestor = DocumentIngestor::new();
        let html = "<html><title>Test</title><body><h1>Hello</h1><p>World</p><script>evil()</script></body></html>";
        let doc = ingestor.ingest_html(html, Some("https://example.com"));
        assert!(doc.content.contains("Hello"));
        assert!(doc.content.contains("World"));
        assert!(!doc.content.contains("evil"));
        assert_eq!(doc.metadata.title.as_deref(), Some("Test"));
        assert_eq!(doc.metadata.source_url.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn test_latex_sections() {
        let ingestor = DocumentIngestor::new();
        let latex = "\\section{Intro}\nSome text.\n\\subsection{Details}\nMore text.";
        let doc = ingestor.ingest_text(latex, None, None, DocumentFormat::Latex);
        assert!(doc.sections.len() >= 2);
    }

    #[test]
    fn test_paragraph_sections() {
        let ingestor = DocumentIngestor::new();
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let doc = ingestor.ingest_text(text, None, None, DocumentFormat::PlainText);
        assert_eq!(doc.sections.len(), 3);
    }

    #[test]
    fn test_chunking_config_default() {
        let config = ChunkingConfig::default();
        assert_eq!(config.max_tokens, 512);
        assert_eq!(config.overlap_tokens, 50);
        assert_eq!(config.min_chunk_size, 50);
        assert!(config.respect_boundaries);
        assert!(config.include_metadata);
    }

    #[test]
    fn test_document_chunk_ids() {
        let ingestor = DocumentIngestor::new();
        let doc = ingestor.ingest_text("Content", Some("mydoc"), None, DocumentFormat::PlainText);
        let chunks = ingestor.chunk(&doc);
        assert!(chunks[0].id.contains("mydoc"));
    }
}
