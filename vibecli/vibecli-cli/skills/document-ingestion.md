---
triggers: ["document ingestion", "document parsing", "PDF extraction", "text chunking", "document pipeline", "ETL documents"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data
---

# Document Ingestion

When building document ingestion and parsing pipelines:

1. **Multi-format parsing** — Use format-specific parsers for best quality: PyMuPDF or pdfplumber for PDFs, python-docx for DOCX, BeautifulSoup/lxml for HTML, markdown-it for Markdown, pandas for CSV, and standard json for JSON. Unstructured.io provides a unified API across formats. Detect file types by content (python-magic) rather than extension to handle mislabeled files.

2. **PDF text extraction quality** — PDFs vary widely in structure. Use PyMuPDF (fitz) for text-layer PDFs — it preserves reading order and handles multi-column layouts. Use pdfplumber for PDFs with complex layouts and tables. For scanned PDFs (image-only), fall back to OCR. Check if extracted text is garbled (encoding issues) or empty (scanned document) and route accordingly.

3. **OCR for scanned documents** — Use Tesseract OCR for general-purpose text recognition. For higher quality, use cloud OCR services (Google Document AI, AWS Textract, Azure Document Intelligence) which handle layout analysis, table extraction, and handwriting. Pre-process images (deskew, binarize, denoise) to improve OCR accuracy. Set the correct language model for non-English documents.

4. **Table extraction** — Tables are the hardest element to extract reliably. Use pdfplumber or Camelot for PDF tables. AWS Textract and Google Document AI excel at complex table structures. For HTML tables, use pandas `read_html`. Preserve table structure as Markdown or CSV in the output. Validate extracted tables by checking row/column consistency.

5. **Metadata extraction** — Extract document metadata: title, author, creation date, modification date, page count, language, and file size. Pull from PDF metadata dictionaries, DOCX core properties, HTML meta tags, and EXIF data for images. Store metadata alongside content for filtering during retrieval. Infer missing metadata (e.g., title from first heading, language from content detection).

6. **Section detection** — Identify document structure by detecting headings, chapters, and sections. Use font size and weight analysis in PDFs, heading tags in HTML, and heading markers in Markdown. Build a hierarchical document tree. Use section boundaries as natural chunk boundaries. Preserve section titles as metadata on each chunk for context.

7. **Chunking strategies with overlap** — Split documents into chunks suitable for embedding and retrieval. Respect section and paragraph boundaries when possible. Fixed-size chunking (512-1024 tokens) with 10-20% overlap is the baseline. Semantic chunking (split when embedding similarity between adjacent sentences drops) produces more coherent chunks. Always include the section title or document title as chunk context.

8. **Deduplication** — Detect and remove duplicate documents using content hashing (SHA-256 on normalized text). For near-duplicates, use SimHash or MinHash with a similarity threshold (e.g., > 0.9). Deduplicate at the document level before chunking and at the chunk level after chunking. Track source URLs or file paths to identify duplicate sources.

9. **Incremental updates** — Track document versions by content hash. On re-ingestion, compare hashes to identify new, modified, and deleted documents. Only re-process changed documents. Delete old chunks from the vector store before inserting updated chunks. Maintain a document registry (database table) mapping document IDs to content hashes, chunk IDs, and ingestion timestamps.

10. **Format-specific best practices** — PDFs: handle password-protected files, linearized vs non-linearized. DOCX: extract text from headers, footers, text boxes, and comments. HTML: strip scripts, styles, and navigation; follow content security. Markdown: resolve relative image/link paths. CSV: detect delimiters and encodings automatically. JSON: flatten nested structures for text extraction.

11. **Pipeline orchestration** — Build the ingestion pipeline as a DAG: detect format, extract text, extract metadata, detect language, chunk, embed, upsert to vector store. Use task queues (Celery, Temporal, Airflow) for production pipelines. Process documents in parallel but respect API rate limits for OCR and embedding services. Log each stage for debugging and reprocessing.

12. **Error handling and quality validation** — Validate extracted text is not empty, not garbled (character distribution check), and not truncated. Log extraction failures with document identifiers for manual review. Implement fallback chains (e.g., PyMuPDF fails, try pdfplumber, then OCR). Set quality thresholds (minimum text length, maximum special character ratio) and quarantine low-quality extractions. Generate extraction quality reports for monitoring.
