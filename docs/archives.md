---
layout: page
title: Archives (ZIP, TAR, and friends)
permalink: /archives/
---

> Expand a `.zip` or a `.tar.gz` in VibeCoder's file explorer the way you expand a folder, read anything inside it, and — the moment you try to edit — extract the archive into a folder named after it and carry on editing there. The archive itself is never rewritten.

## What you can do

**Browse.** An archive in the explorer gets a chevron. Click it and its contents appear as a tree, nested folders and all, sorted the same way the on-disk tree is: directories first, then files.

**Read.** Click a file inside an archive and it opens in the editor with syntax highlighting, search, and everything else Monaco gives a normal file. Images and PDFs render. The tab carries a lock, and a strip above the editor names the archive the file came from.

**Extract to edit.** Type into that file — or press save, or right-click it and pick **Extract to Edit…** — and VibeCoder offers to unpack the whole archive into a sibling folder named after it, then reopens your file from there, writable. The prompt names the exact folder before anything is written.

## Formats

| Family | Extensions |
|---|---|
| ZIP containers | `.zip` `.jar` `.war` `.ear` `.apk` `.aab` `.ipa` `.whl` `.egg` `.vsix` `.xpi` `.nupkg` `.zipx` `.maff` `.sketch` |
| Tarballs | `.tar` `.tar.gz` / `.tgz` · `.tar.bz2` / `.tbz` / `.tbz2` · `.tar.zst` / `.tzst` · `.tar.xz` / `.txz` |
| One compressed file | `.gz` `.bz2` `.zst` `.xz` — browses as a single member named after the archive with the suffix cut, so `server.log.gz` shows `server.log` |

**DOCX, XLSX, PPTX, ODT and EPUB are deliberately not in that list.** They are ZIP containers, but VibeCoder already opens them as documents — see [documents.md](/vibecody/documents/) — and expanding a `.docx` into a tree of XML parts instead of rendering it would be a regression, not a feature.

**A nested archive is not browsed in place.** A `.jar` inside a `.zip` has no chevron: decoding the inner container out of the outer one's bytes, in memory, is a different piece of machinery from reading a file on disk. Clicking it offers the extraction instead, which lands the inner archive on disk where it browses for real.

`.7z` and `.rar` are not supported. Neither has a decoder in the dependency set, and shelling out to a binary that may not be installed is not something a file explorer should do silently.

## Why editing extracts instead of writing back

Rewriting a member in place means re-encoding the container. For a plain ZIP of source files that is nearly safe; for a signed `.apk`, a `.vsix`, or an EPUB whose `mimetype` entry must be stored first and uncompressed, it is the difference between "edited a file" and "produced an archive that no longer works". VibeCody does rewrite two container formats in place — DOCX and EPUB, in `vibe_docfmt::zipedit` — and does it by copying every untouched entry back byte for byte, in its original order, with its original compression method. That is what it costs to be careful with a format we *do* understand. Generalising it to every archive anyone might drop into a workspace is not a promise worth making, so an edit becomes an extraction instead.

## What extraction does

- Creates **a sibling folder named after the archive with its extension removed** — `dist.tar.gz` → `dist/`, `plugin.vsix` → `plugin/`.
- **Never merges into an existing folder.** If `dist/` is already there, the extraction goes to `dist-1/`, and the prompt says so. Silently overwriting a previous extraction is how someone loses the edits they made last time.
- Extracts **the whole archive**, not just the one file, so relative imports and neighbouring resources still resolve in the file you are editing.
- **Skips** symlinks, hard links, device nodes, and any entry whose path tries to escape the destination. The count of skipped entries is reported in the toast.
- **Leaves the archive untouched.**
- **Fails atomically**: a partial extraction is deleted rather than left behind, because a folder missing half its files is harder to diagnose than no folder at all.

## Limits

Archive headers are attacker-controlled input — a member can declare a size it does not have, and 40 KiB of zip can expand to a terabyte. So nothing here allocates from a declared size; every bound is enforced by reading.

| Bound | Value |
|---|---|
| One member opened in the editor | 64 MiB |
| Total output of one extraction | 2 GiB |
| Entries indexed or extracted | 200 000 |
| Compressed size of a tarball to index | 8 GiB |

A ZIP's central directory is read without touching member bodies, so a large `.zip` lists instantly. A compressed tarball has no index — finding out what is in it means decompressing it — so its listing is cached per (path, mtime, size) for the eight most recently browsed archives, and dropped the moment the file changes on disk.

## Paths

A file inside an archive is addressed with a **virtual path**: the archive's real path, `!/`, then the member path.

```text
/home/me/proj/dist.zip!/src/index.js
```

That is the separator `jar:` URLs have used since 1997. `!` cannot appear in a Windows path, and the left half must itself name an archive before the split is taken — so `~/we!/there/main.rs`, a real directory that happens to end in `!`, stays an ordinary path.

Virtual paths are for the explorer and the editor. They are not filesystem paths: no language server, terminal, git operation or AI tool sees one, which is also why a file opened from an archive gets no IntelliSense.

## Tauri commands

| Command | Description |
|---|---|
| `list_archive(path)` | One level inside an archive. `path` is the archive, or a virtual path for a folder within it |
| `read_archive_file(path)` | A member's contents as text; fails on non-UTF-8 |
| `read_archive_file_base64(path)` | A member's bytes, base64 — images, PDFs |
| `plan_archive_extraction(path)` | What extracting would do: the destination folder, whether it had to be renamed, where the member lands. Writes nothing |
| `extract_archive(path, destination?)` | Extract, and report where the member in `path` ended up |

The workspace boundary applies to the archive file, as it does to any other path; a member path is not a filesystem path and is resolved inside the container.

Implementation: `vibecoder/crates/vibe-core/src/archive.rs` (formats, bounds, extraction), `vibecoder/src/utils/archive.ts` (the same path rules, front-end half).
