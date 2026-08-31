---
layout: page
title: Documents (DOCX, EPUB, PDF, Pages)
permalink: /documents/
---

> Open a Word document, an e-book, a PDF, or an Apple Pages file in VibeCoder, read it, edit its text in Monaco, and save it back into the original file. Nothing is written until the file has been re-read and checked.

VibeCoder opens four document formats in the editor area, and all four are **editable as text**:

| Format | View | Edit as | Save back |
|---|---|---|---|
| `.docx` | Rendered document | Markdown | Yes |
| `.epub` | Full rendering — the book's own styles, images and links | Markdown | Yes |
| `.pdf` | Rendered page view | Plain text, one line per line of the page | Yes — change or remove a line, never add one |
| `.pages` | Embedded page preview + recovered text | Plain text | Yes, text only |

Open the file from the sidebar as usual. Press **Edit text** in the viewer toolbar to switch to Monaco, and **Save** (or ⌘S) to write it back. **View** returns to the rendered document.

---

## Two pages side by side

**Two up** in the viewer toolbar puts a document into a spread. What that means
depends on whether the format has pages of its own:

| Format | One up | Two up |
|---|---|---|
| `.pdf` | One page, drawn at the zoom you set | The pages either side of a spread — 1–2, 3–4, … — with ← and → turning a whole spread at a time |
| `.docx` | The document, scrolling | Two columns of one screen, paged sideways |
| `.epub` | The chapter, scrolling | Two columns of one screen, paged sideways |

A PDF has real pages, so a spread is two of them and the toolbar names them:
*Pages 7–8 of 120*. Switching between one up and two up keeps you on the page
you were reading — page 8 one-up becomes the spread 7–8, not 8–9, which is a
pair no printed book has. An odd-length document ends on a single page rather
than a blank one.

A DOCX or an EPUB chapter is text, not sheets: how much fits beside how much
depends on the window and the font size, so a spread is **two columns of one
screenful** and the pager counts screens rather than pages. Turning a page
scrolls the pane by exactly its own width, which is also why links still work —
clicking a footnote reference lands on the right column by itself.

The choice is remembered per document for the session, so switching tabs and
coming back does not undo it. Apple Pages files are not offered a spread: that
viewer already shows two things side by side, the page preview and the text.

`.pdf` pages are drawn here rather than handed to the platform's own PDF
viewer in a frame — which is what the editor used to do, and the reason it had
no page navigation and no spread. The renderer is loaded the first time a PDF is
opened, so a session that never opens one does not pay for it.

---

## What editing a document actually changes

The editor does not rebuild your document from the text buffer. It **edits the container in place**: the paragraph you changed is rewritten and every other part of the file is copied across byte-for-byte.

That is what keeps a save from quietly deleting things the text model knows nothing about:

- **DOCX** — images, footnotes, comments, headers and footers, page size and margins, styles, tracked-change marks.
- **EPUB** — stylesheets, cover images, the OPF and navigation document, metadata, fonts.
- **Pages** — everything. Only the text field inside the archives is touched.
- **PDF** — everything. The original bytes are kept and the changed page is *appended* as an incremental update, which is what PDF was designed for; a file that came in packed comes out the same size.

An element with no text — a spacer paragraph, an image-only paragraph — is deliberately **not** shown in the buffer, so an edit cannot delete it by omission.

## The guarantee on save

Every save runs the same sequence:

1. Rewrite the container **in memory**.
2. Re-read the rewritten document with the same reader the editor used.
3. Compare what came back against the text you saved.
4. Replace the file only if they match.

If step 3 disagrees, you get an error naming the first line that differs, and **your file is not modified**. A save that could not be verified is never reported as a save.

## Unsaved edits

The editor area renders only the active tab, so a document buffer is unmounted when you switch files. An unsaved buffer is kept for the session and restored when you come back — the tab reopens in the text editor, still marked **unsaved**. Closing the tab does not throw the edit away either; there is no prompt in front of that gesture, so the buffer is kept rather than discarded on your behalf. It lives in memory only: quitting VibeCoder drops it.

## Section markers

An EPUB has one section per chapter; a PDF has one per page; a Pages document has one per text storage (body, header, a text box, …). Multi-section buffers carry a marker line for each:

```markdown
<!-- vibedoc:section id="OEBPS/ch3.xhtml" title="Chapter Three" -->
```

```text
<<< vibedoc:storage Index/Document.iwa:1001:0 >>>
<<< vibedoc:storage page-4 >>>
```

**Keep them.** They are how each edited region is routed back to the right chapter, page or storage. Deleting one, or adding a section that was not there, is refused with an explanation rather than guessed at.

---

## Per-format detail

### DOCX

Read as Markdown: headings (`Heading1`–`Heading6`), bold, italic, monospace runs, hyperlinks (resolved through `word/_rels/document.xml.rels`), bullet and numbered lists (read from `word/numbering.xml`), tables, and horizontal rules.

Writing supports the same set. New hyperlinks get a relationship added; a list added to a document with no numbering part gets one written, along with its content-type override; a heading level with no style definition gets one, so Word renders `# Title` as a heading rather than as body text.

Not supported, and refused rather than approximated:

- Adding or removing **table rows or columns** — edit cell text freely, change the table's shape in Word.
- Fenced **code blocks** — DOCX has no code-block element, so they are stored as monospaced paragraphs. This is reported as a warning at save time, and the paragraphs read back as paragraphs.

### EPUB

**Reading.** A book renders as the book: its own stylesheets, its images, its
cover, its table of contents (EPUB 3 navigation document, or the EPUB 2 NCX,
with nesting preserved), and working links. Clicking a cross-reference moves to
that chapter and scrolls to the anchor; an `http(s)` or `mailto` link opens in
your browser rather than navigating the editor away. Chapter text is selectable,
and the font size control scales the chapter without discarding the book's own
typography.

Two things constrain what is rendered, both deliberate:

- **The chapter's markup is sanitised** before it reaches the DOM — EPUB content
  is treated as attacker-controlled, because the file came from somewhere else.
  Scripts, forms, frames and script URLs are removed.
- **The book's CSS is scoped to the chapter container**, and the constructs that
  would let it escape are dropped: `@import`, `position: fixed`/`sticky`,
  `expression()`, script URLs, and any `url()` the book's own files do not
  satisfy. Everything else — fonts, margins, drop caps, colours, media queries —
  is applied.

A remote `<img>` in an offline book is not fetched. It is a tracking pixel far
more often than a picture, and the chapter reports it as a reference it did not
resolve.

**Editing.** Each spine item becomes one section. Read as Markdown: headings, paragraphs, bold/italic/code, links, ordered and unordered lists (including nesting), `<pre>` blocks, horizontal rules and tables.

Writing edits the chapter's XHTML in place. XHTML indentation is normalised out of the buffer, so a chapter written across several source lines reads as one paragraph and stays one paragraph.

Not supported:

- **Adding or removing chapters** — the spine and manifest are not restructured.
- **Chapter titles** — they come from the EPUB and are not written back from a marker.
- **Inline images** stay in their paragraph but move to its end, because the text buffer does not record where in the sentence they sat. Reported as a warning.

### PDF

A PDF has no paragraphs. It has glyphs at coordinates, and the text you read on
a page is something a reader *reconstructs* — which is why a PDF opens as plain
text, one line per line of the page, rather than as Markdown. There is no
emphasis to recover and no structure to trust.

**What a save does.**

- **Rewrites the words on a line.** Each character goes back to the run that
  drew the one it replaced, encoded through *that* run's font. This is what
  makes a line set in more than one font editable at all: every font in a
  modern PDF is subset to the glyphs its own words used, so a sentence with one
  bold word has no single font that can draw the whole of it.
- **Deletes a line** when you clear its text.
- **Refuses to add one.** A new line has no position, no font and no place in
  the page's content stream. This is an error naming the line, not a guess.
- **Never re-flows.** Glyph positions are absolute, so a longer line runs past
  where the original ended. You are told in a warning rather than finding out
  in a reader.
- **Refuses text the font cannot draw.** A subset font carries only the glyphs
  the document already used, so a character outside that set would be written
  as a code the reader renders as something else. The save stops and names the
  character.

**What it can and cannot read.** A font says what its codes mean in one of three
ways, and each is used in turn: a `/ToUnicode` map (which maps both ways, and is
the only one that makes a font safely writable), a named base encoding, or a
`/Differences` array naming each glyph. Where none of them answers — a symbolic
font with a built-in encoding, a composite font with no `/ToUnicode` — the run
is **left out of the buffer** and a warning names the font. No character is
guessed at, and a line drawn with such a font cannot be edited.

Words set apart by moving the pen rather than by drawing a space — how TeX and
its descendants typeset, and why their fonts often carry no space glyph — are
read as spaces and written back as gaps.

Not supported, and refused rather than approximated:

- **Adding or removing a page.** Every page marker must stay exactly as it is.
- **Adding a line**, for the reason above.
- **Editing a page that draws an inline image** — pixels written into the
  content stream between `BI` and `EI` do not survive it being rebuilt, so the
  page is left alone and says so. Its text still reads.
- **Encrypted PDFs.** Opening one asks you to save an unprotected copy first,
  by name, rather than failing obscurely.

**A backup is written.** A PDF is the second format whose save can rebuild the
file rather than patch a container, so the original is copied to `<name>.pdf.bak`
before it is replaced. The path is shown in the save confirmation.

**A very large PDF is slow to open, and the cost is in the parser.** Measured on
a 48 MB, 1 373-page document: 174 s to parse the file and 6 s for everything
this crate then does with it — reading every page's content stream, decoding
77 839 lines of text, and writing them back. A save pays it twice, once to read
the file and once to read the result back and check it. Most documents are not
like that (a 107 MB, 523-page PDF parses in 2.2 s), and nothing blocks the
window while it happens, but a book-sized PDF is a wait rather than an
instant.

Two things are worth knowing about the text you see. Line breaks are the page's,
not the paragraph's: a sentence that wraps is several lines here, and each is
edited on its own. And the spacing between words is reconstructed from the
positions the page draws at, so it is what a reader would show, not a field
stored in the file.

### Pages

Apple publishes no specification for the `.pages` format. Its content lives in `Index/*.iwa` archives: Snappy-compressed protobuf, in a framing that is not the Snappy stream format, with a schema that only exists as community reverse-engineering. Both the ZIP form and the "save as package" bundle directory are supported, as is the older nested `Index.zip` layout.

What that means in practice:

- **You get text.** Paragraphs, in document order, per storage. Fonts, sizes, colours, layout, tables, shapes and images are not modelled and are not shown.
- **The page preview is real.** The image in the viewer's *Preview* pane is the one Pages itself embedded in the file — it is what the document actually looks like, next to the text you can actually edit.
- **Writing is text substitution.** The text field of the storage you edited is replaced; nothing else in the archive is rebuilt.
- **Style ranges are shifted, not recomputed.** Character-index tables (which run of text is bold, where a list item starts) are remapped to follow the new text when its length changes. Only tables whose shape is unambiguous are touched — two or more entries, strictly increasing, none past the end of the old text. This is a best effort against an unpublished format, it is reported as a warning when it happens, and formatting is worth a look after opening the document in Pages.
- **A backup is written.** Pages is the one format whose container is reverse-engineered, so a save copies the original to `<name>.pages.bak` before replacing it — for a bundle, the whole package is copied beside itself, never into itself. The path is shown in the save confirmation.
- **Pages '09 and earlier** (`index.xml`-based documents) are not supported, and say so by name.

The verification step applies here too: the rewritten archives are re-read and their text compared before anything on disk changes. What it cannot check is how Pages itself renders the result — no automated check in this repo has opened one of these files in Pages, and the save confirmation says so rather than implying otherwise.

---

## Where it lives

| Piece | Path |
|---|---|
| Format readers/writers, verification, backups | [`crates/vibe-docfmt`](https://github.com/ravituringworks/vibecody/tree/main/crates/vibe-docfmt) |
| EPUB reading for display — spine, TOC, cover, per-chapter resources | `crates/vibe-docfmt/src/epub_view.rs` |
| Chapter sanitising, CSS scoping, link and image rewriting | `vibecoder/src/lib/epubRender.ts` |
| Byte-preserving XML tree | `crates/vibe-docfmt/src/xmltree.rs` |
| Edit engine shared by DOCX and EPUB | `crates/vibe-docfmt/src/surgical.rs` |
| PDF content streams, page lines, incremental update | `crates/vibe-docfmt/src/pdf/mod.rs` |
| PDF font encodings and ToUnicode CMaps | `crates/vibe-docfmt/src/pdf/font.rs` |
| IWA framing, protobuf walker, text substitution | `crates/vibe-docfmt/src/pages/` |
| Tauri commands | `vibecoder/src-tauri/src/commands.rs` |
| Viewers and the text editor | `vibecoder/src/components/DocumentViewer.tsx`, `DocumentTextEditor.tsx` |
| Typed command boundary | `vibecoder/src/lib/richDocuments.ts` |

### Commands

| Command | Description |
|---|---|
| `is_rich_document(path)` | Whether the path is a format this build can open as text |
| `read_document_text(path)` | Open the document as a text buffer (`format`, `language`, `text`, `sections`, `warnings`, `writable`) |
| `write_document_text(path, text)` | Save the buffer back; errors, with the file untouched, if it does not verify |
| `read_document_preview(path)` | The preview image the document embeds, base64-encoded (Pages only) |
| `read_epub_book(path)` | A book's metadata, cover, spine and table of contents |
| `read_epub_chapter(path, chapter)` | One chapter's markup, stylesheets and referenced media |

### Tests

Two examples measure the crate against documents on your own disk rather than
against fixtures, which is where every bug listed above was found:

```bash
cargo run -p vibe-docfmt --release --example roundtrip -- <file>   # read → save the same text → verify
cargo run -p vibe-docfmt --release --example editcheck -- <file>   # change one line → save → read it back
```

```bash
cargo test -p vibe-docfmt --no-fail-fast          # readers, writers, framing, remapping
cd vibecoder && npx vitest run src/lib/__tests__/richDocuments.test.ts
cd vibecoder && npx vitest run src/lib/__tests__/epubBook.test.ts
cd vibecoder && npx vitest run src/lib/__tests__/epubRender.test.ts      # CSS scoping + sanitising
cd vibecoder && npx vitest run src/components/__tests__/EpubViewer.test.tsx
cd vibecoder && npx vitest run src/components/__tests__/DocumentViewer.sanitize.test.ts
cd vibecoder && npx vitest run src/components/__tests__/DocumentTextEditor.test.tsx
```
