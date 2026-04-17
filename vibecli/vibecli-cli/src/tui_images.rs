//! Inline image rendering — Kitty Graphics Protocol + iTerm2.
//! Pi-mono gap bridge: Phase C1.
//!
//! Detects terminal capabilities at runtime, parses image metadata from
//! magic-byte headers (PNG/JPEG/GIF), emits the correct escape sequence,
//! and falls back to a text placeholder when neither protocol is available.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use std::path::Path;

// ─── Protocol detection ──────────────────────────────────────────────────────

/// Which terminal image protocol to use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageProtocol {
    /// Kitty Graphics Protocol — full pixel-accurate inline images.
    Kitty,
    /// iTerm2 inline image protocol — supported by iTerm2, WezTerm, Hyper.
    ITerm2,
    /// Terminal doesn't support either; use a text placeholder.
    None,
}

impl ImageProtocol {
    /// Detect the best available protocol from the current environment.
    ///
    /// Priority: Kitty > iTerm2 > None.
    /// Checks `$KITTY_WINDOW_ID`, `$TERM`, and `$TERM_PROGRAM`.
    pub fn detect() -> Self {
        // Kitty sets KITTY_WINDOW_ID for every child process.
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            return ImageProtocol::Kitty;
        }
        // Some terminals advertise Kitty support through $TERM.
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("kitty") {
                return ImageProtocol::Kitty;
            }
        }
        // iTerm2 and compatible terminals set $TERM_PROGRAM.
        if let Ok(prog) = std::env::var("TERM_PROGRAM") {
            let p = prog.to_ascii_lowercase();
            if p.contains("iterm") || p.contains("wezterm") || p.contains("hyper") {
                return ImageProtocol::ITerm2;
            }
        }
        ImageProtocol::None
    }

    /// Human-readable protocol name.
    pub fn name(&self) -> &str {
        match self {
            ImageProtocol::Kitty => "kitty",
            ImageProtocol::ITerm2 => "iterm2",
            ImageProtocol::None => "none",
        }
    }

    /// Returns `true` when a visual protocol is available.
    pub fn is_supported(&self) -> bool {
        !matches!(self, ImageProtocol::None)
    }
}

// ─── Image format ────────────────────────────────────────────────────────────

/// Recognised image container formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Webp,
    Svg,
    Unknown,
}

impl ImageFormat {
    /// Detect format from leading magic bytes.
    pub fn from_header(bytes: &[u8]) -> Self {
        if bytes.len() >= 8 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" {
            return ImageFormat::Png;
        }
        if bytes.len() >= 3 && &bytes[..3] == b"\xff\xd8\xff" {
            return ImageFormat::Jpeg;
        }
        if bytes.len() >= 6 && (&bytes[..6] == b"GIF87a" || &bytes[..6] == b"GIF89a") {
            return ImageFormat::Gif;
        }
        if bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
            return ImageFormat::Webp;
        }
        // SVG is XML text; look for the opening tag (possibly with BOM).
        let snip = std::str::from_utf8(&bytes[..bytes.len().min(64)]).unwrap_or("");
        if snip.contains("<svg") || snip.contains("<?xml") {
            return ImageFormat::Svg;
        }
        ImageFormat::Unknown
    }

    /// Detect format from a file extension (case-insensitive).
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "gif" => ImageFormat::Gif,
            "webp" => ImageFormat::Webp,
            "svg" => ImageFormat::Svg,
            _ => ImageFormat::Unknown,
        }
    }

    /// MIME type string suitable for HTTP / data-URI use.
    pub fn mime_type(&self) -> &str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Webp => "image/webp",
            ImageFormat::Svg => "image/svg+xml",
            ImageFormat::Unknown => "application/octet-stream",
        }
    }

    /// Returns `true` for pixel-based formats (PNG, JPEG, GIF, WebP).
    pub fn is_raster(&self) -> bool {
        matches!(
            self,
            ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::Webp
        )
    }
}

// ─── Image metadata ──────────────────────────────────────────────────────────

/// Parsed image metadata extracted from file headers.
#[derive(Debug, Clone)]
pub struct ImageMeta {
    pub width_px: u32,
    pub height_px: u32,
    pub format: ImageFormat,
    pub size_bytes: usize,
}

// ─── Render options ──────────────────────────────────────────────────────────

/// Controls how an image is displayed inline.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Maximum terminal columns to use (default: 80).
    pub max_width_cols: u32,
    /// Maximum terminal rows to use (default: 24).
    pub max_height_rows: u32,
    /// Which protocol to use; overrides auto-detection when set explicitly.
    pub protocol: ImageProtocol,
    /// Emit a text placeholder when the protocol is `None`.
    pub show_placeholder_on_unsupported: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            max_width_cols: 80,
            max_height_rows: 24,
            protocol: ImageProtocol::detect(),
            show_placeholder_on_unsupported: true,
        }
    }
}

// ─── Render result ───────────────────────────────────────────────────────────

/// Outcome of a render attempt.
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// The full terminal escape sequence to write to stdout (may be empty).
    pub escape_sequence: String,
    /// Which protocol was actually used.
    pub protocol_used: ImageProtocol,
    /// `true` when we emitted a placeholder instead of a visual sequence.
    pub fallback: bool,
    /// Textual description, e.g. `"[image: 800x600 PNG 42KB]"`.
    pub placeholder_text: String,
}

impl RenderResult {
    /// Returns `true` when a real visual sequence was produced.
    pub fn is_visual(&self) -> bool {
        !self.fallback
    }

    /// The string to write to the terminal — visual sequence or placeholder.
    pub fn output(&self) -> &str {
        if self.fallback {
            &self.placeholder_text
        } else {
            &self.escape_sequence
        }
    }
}

// ─── Dimension parsing ───────────────────────────────────────────────────────

/// Parse width × height in pixels from PNG, JPEG, or GIF raw bytes.
///
/// Returns `None` if the data is too short or the format is unrecognised.
pub fn parse_image_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // PNG: IHDR chunk always starts at byte 16; width/height are 4-byte BE at 16/20.
    if data.len() >= 8 && &data[..8] == b"\x89PNG\r\n\x1a\n" && data.len() >= 24 {
        let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return Some((w, h));
    }
    // JPEG: scan for SOF markers (0xFFC0..0xFFC3, 0xFFC5..0xFFC7, 0xFFC9..0xFFCB).
    if data.len() >= 3 && &data[..3] == b"\xff\xd8\xff" {
        let mut i = 2usize;
        while i + 3 < data.len() {
            if data[i] != 0xFF {
                break;
            }
            let marker = data[i + 1];
            // SOF markers carry image dimensions.
            if matches!(
                marker,
                0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB
            ) && i + 9 < data.len()
            {
                let h = u32::from_be_bytes([0, 0, data[i + 5], data[i + 6]]);
                let w = u32::from_be_bytes([0, 0, data[i + 7], data[i + 8]]);
                return Some((w, h));
            }
            // Skip over this segment (length field is 2-byte BE at i+2, includes itself).
            if i + 3 < data.len() {
                let seg_len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                if seg_len < 2 {
                    break;
                }
                i += 2 + seg_len;
            } else {
                break;
            }
        }
    }
    // GIF: width/height are 2-byte LE at bytes 6 and 8.
    if data.len() >= 10 && (&data[..6] == b"GIF87a" || &data[..6] == b"GIF89a") {
        let w = u16::from_le_bytes([data[6], data[7]]) as u32;
        let h = u16::from_le_bytes([data[8], data[9]]) as u32;
        return Some((w, h));
    }
    None
}

// ─── Escape sequence builders ─────────────────────────────────────────────────

/// Build a Kitty Graphics Protocol escape sequence for the given raw image bytes.
///
/// Uses action=T (transmit and display), format=100 (PNG/any), base64-encoded
/// payload, and the caller-supplied column/row size hints.
pub fn kitty_escape(data: &[u8], width_cols: u32, height_rows: u32) -> String {
    let encoded = B64.encode(data);
    // APC: \x1b_ ... \x1b\
    // Payload keys: a=T (transmit+display), f=100 (PNG), c=cols, r=rows, m=0 (last chunk)
    format!(
        "\x1b_Ga=T,f=100,c={},r={},m=0;{}\x1b\\",
        width_cols, height_rows, encoded
    )
}

/// Build an iTerm2 inline image escape sequence.
///
/// The sequence embeds the base64-encoded file, with `width` and `height`
/// set in pixels; `inline=1` instructs the terminal to display immediately.
pub fn iterm2_escape(data: &[u8], width_px: u32, height_px: u32) -> String {
    let encoded = B64.encode(data);
    let size = data.len();
    // OSC 1337 ; File=... : <base64> ST
    format!(
        "\x1b]1337;File=inline=1;size={size};width={width_px}px;height={height_px}px:{encoded}\x07"
    )
}

/// Produce a text placeholder describing the image.
///
/// Format: `[image: WxH FORMAT SIZE — /path]`
pub fn image_placeholder(meta: &ImageMeta, path_hint: Option<&str>) -> String {
    let fmt = match meta.format {
        ImageFormat::Png => "PNG",
        ImageFormat::Jpeg => "JPEG",
        ImageFormat::Gif => "GIF",
        ImageFormat::Webp => "WebP",
        ImageFormat::Svg => "SVG",
        ImageFormat::Unknown => "image",
    };
    let size_kb = (meta.size_bytes + 512) / 1024;
    let dims = format!("{}x{}", meta.width_px, meta.height_px);
    match path_hint {
        Some(p) => format!("[image: {} {} {}KB \u{2014} {}]", dims, fmt, size_kb, p),
        None => format!("[image: {} {} {}KB]", dims, fmt, size_kb),
    }
}

// ─── High-level render functions ─────────────────────────────────────────────

/// Render raw image bytes for inline terminal display.
pub fn render_image_bytes(data: &[u8], opts: &RenderOptions) -> RenderResult {
    let format = ImageFormat::from_header(data);
    let (width_px, height_px) = parse_image_dimensions(data).unwrap_or((0, 0));
    let meta = ImageMeta {
        width_px,
        height_px,
        format,
        size_bytes: data.len(),
    };
    let placeholder = image_placeholder(&meta, None);

    match &opts.protocol {
        ImageProtocol::Kitty => {
            let seq = kitty_escape(data, opts.max_width_cols, opts.max_height_rows);
            RenderResult {
                escape_sequence: seq,
                protocol_used: ImageProtocol::Kitty,
                fallback: false,
                placeholder_text: placeholder,
            }
        }
        ImageProtocol::ITerm2 => {
            let seq = iterm2_escape(data, width_px.max(1), height_px.max(1));
            RenderResult {
                escape_sequence: seq,
                protocol_used: ImageProtocol::ITerm2,
                fallback: false,
                placeholder_text: placeholder,
            }
        }
        ImageProtocol::None => RenderResult {
            escape_sequence: String::new(),
            protocol_used: ImageProtocol::None,
            fallback: true,
            placeholder_text: placeholder,
        },
    }
}

/// Render an image from a file path.
pub fn render_image_file(path: &Path, opts: &RenderOptions) -> Result<RenderResult, String> {
    let data =
        std::fs::read(path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    // If the format is Unknown from header, try the extension.
    let mut result = render_image_bytes(&data, opts);
    if matches!(ImageFormat::from_header(&data), ImageFormat::Unknown) {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            // Re-derive the placeholder with extension-based format hint.
            let format = ImageFormat::from_extension(ext);
            let (w, h) = parse_image_dimensions(&data).unwrap_or((0, 0));
            let meta = ImageMeta {
                width_px: w,
                height_px: h,
                format,
                size_bytes: data.len(),
            };
            result.placeholder_text = image_placeholder(&meta, path.to_str());
        }
    }
    Ok(result)
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid 1×1 PNG (67 bytes, white pixel).
    fn tiny_png() -> Vec<u8> {
        // Produced by: `python3 -c "import base64,sys; sys.stdout.buffer.write(base64.b64decode(...))"`.
        // This is a real 1x1 white PNG.
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwADhQGAWjR9awAAAABJRU5ErkJggg==";
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap()
    }

    #[test]
    fn test_png_magic_byte_detection() {
        let png = tiny_png();
        assert_eq!(ImageFormat::from_header(&png), ImageFormat::Png);
    }

    #[test]
    fn test_jpeg_magic_byte_detection() {
        let jpeg_header = b"\xff\xd8\xff\xe0some data";
        assert_eq!(ImageFormat::from_header(jpeg_header), ImageFormat::Jpeg);
    }

    #[test]
    fn test_gif_magic_byte_detection() {
        let gif_header = b"GIF89a\x01\x00\x01\x00";
        assert_eq!(ImageFormat::from_header(gif_header), ImageFormat::Gif);
    }

    #[test]
    fn test_unknown_magic_bytes() {
        let garbage = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08";
        assert_eq!(ImageFormat::from_header(garbage), ImageFormat::Unknown);
    }

    #[test]
    fn test_image_format_from_extension() {
        assert_eq!(ImageFormat::from_extension("png"), ImageFormat::Png);
        assert_eq!(ImageFormat::from_extension("PNG"), ImageFormat::Png);
        assert_eq!(ImageFormat::from_extension("jpg"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("jpeg"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("gif"), ImageFormat::Gif);
        assert_eq!(ImageFormat::from_extension("webp"), ImageFormat::Webp);
        assert_eq!(ImageFormat::from_extension("svg"), ImageFormat::Svg);
        assert_eq!(ImageFormat::from_extension("bmp"), ImageFormat::Unknown);
    }

    #[test]
    fn test_image_format_mime_type() {
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Gif.mime_type(), "image/gif");
        assert_eq!(ImageFormat::Webp.mime_type(), "image/webp");
        assert_eq!(ImageFormat::Svg.mime_type(), "image/svg+xml");
        assert_eq!(ImageFormat::Unknown.mime_type(), "application/octet-stream");
    }

    #[test]
    fn test_image_format_is_raster() {
        assert!(ImageFormat::Png.is_raster());
        assert!(ImageFormat::Jpeg.is_raster());
        assert!(ImageFormat::Gif.is_raster());
        assert!(ImageFormat::Webp.is_raster());
        assert!(!ImageFormat::Svg.is_raster());
        assert!(!ImageFormat::Unknown.is_raster());
    }

    #[test]
    fn test_parse_image_dimensions_png() {
        let png = tiny_png();
        let dims = parse_image_dimensions(&png);
        assert_eq!(dims, Some((1, 1)));
    }

    #[test]
    fn test_parse_image_dimensions_gif() {
        // Synthetic GIF header: GIF89a + 2-byte LE width=320, height=240.
        let mut header = b"GIF89a".to_vec();
        header.extend_from_slice(&320u16.to_le_bytes());
        header.extend_from_slice(&240u16.to_le_bytes());
        assert_eq!(parse_image_dimensions(&header), Some((320, 240)));
    }

    #[test]
    fn test_parse_image_dimensions_unknown_returns_none() {
        let data = b"\x00\x01\x02\x03";
        assert_eq!(parse_image_dimensions(data), None);
    }

    #[test]
    fn test_kitty_escape_prefix() {
        let data = b"fake image bytes";
        let seq = kitty_escape(data, 40, 12);
        // Kitty sequence starts with APC introducer.
        assert!(seq.starts_with("\x1b_G"), "expected Kitty APC prefix");
        assert!(seq.contains("a=T"), "expected transmit action");
        assert!(seq.ends_with("\x1b\\"), "expected ST terminator");
    }

    #[test]
    fn test_kitty_escape_contains_base64() {
        let data = b"hello";
        let seq = kitty_escape(data, 10, 5);
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        assert!(
            seq.contains(&encoded),
            "base64 payload missing from kitty sequence"
        );
    }

    #[test]
    fn test_iterm2_escape_prefix() {
        let data = b"fake image bytes";
        let seq = iterm2_escape(data, 100, 50);
        assert!(seq.starts_with("\x1b]1337;"), "expected OSC 1337 prefix");
        assert!(seq.contains("inline=1"), "expected inline=1");
        assert!(seq.ends_with('\x07'), "expected BEL terminator");
    }

    #[test]
    fn test_iterm2_escape_contains_base64() {
        let data = b"world";
        let seq = iterm2_escape(data, 50, 30);
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        assert!(
            seq.contains(&encoded),
            "base64 payload missing from iterm2 sequence"
        );
    }

    #[test]
    fn test_image_placeholder_no_path() {
        let meta = ImageMeta {
            width_px: 800,
            height_px: 600,
            format: ImageFormat::Png,
            size_bytes: 43_008, // ~42 KB
        };
        let p = image_placeholder(&meta, None);
        assert!(p.contains("800x600"), "dimensions missing");
        assert!(p.contains("PNG"), "format missing");
        assert!(p.starts_with("[image:"), "bad prefix");
    }

    #[test]
    fn test_image_placeholder_with_path() {
        let meta = ImageMeta {
            width_px: 1920,
            height_px: 1080,
            format: ImageFormat::Jpeg,
            size_bytes: 102_400,
        };
        let p = image_placeholder(&meta, Some("/home/user/photo.jpg"));
        assert!(p.contains("/home/user/photo.jpg"), "path missing");
        assert!(p.contains("1920x1080"), "dimensions missing");
        assert!(p.contains("JPEG"), "format missing");
    }

    #[test]
    fn test_render_options_default() {
        let opts = RenderOptions::default();
        assert_eq!(opts.max_width_cols, 80);
        assert_eq!(opts.max_height_rows, 24);
        assert!(opts.show_placeholder_on_unsupported);
    }

    #[test]
    fn test_render_image_bytes_none_protocol_fallback() {
        let opts = RenderOptions {
            protocol: ImageProtocol::None,
            ..Default::default()
        };
        let result = render_image_bytes(b"irrelevant", &opts);
        assert!(result.fallback);
        assert!(!result.is_visual());
        assert_eq!(result.output(), result.placeholder_text.as_str());
        assert!(result.escape_sequence.is_empty());
    }

    #[test]
    fn test_render_image_bytes_kitty() {
        let png = tiny_png();
        let opts = RenderOptions {
            protocol: ImageProtocol::Kitty,
            ..Default::default()
        };
        let result = render_image_bytes(&png, &opts);
        assert!(!result.fallback);
        assert!(result.is_visual());
        assert!(result.escape_sequence.starts_with("\x1b_G"));
    }

    #[test]
    fn test_render_image_bytes_iterm2() {
        let png = tiny_png();
        let opts = RenderOptions {
            protocol: ImageProtocol::ITerm2,
            ..Default::default()
        };
        let result = render_image_bytes(&png, &opts);
        assert!(!result.fallback);
        assert!(result.is_visual());
        assert!(result.escape_sequence.starts_with("\x1b]1337;"));
    }

    #[test]
    fn test_protocol_name() {
        assert_eq!(ImageProtocol::Kitty.name(), "kitty");
        assert_eq!(ImageProtocol::ITerm2.name(), "iterm2");
        assert_eq!(ImageProtocol::None.name(), "none");
    }

    #[test]
    fn test_protocol_is_supported() {
        assert!(ImageProtocol::Kitty.is_supported());
        assert!(ImageProtocol::ITerm2.is_supported());
        assert!(!ImageProtocol::None.is_supported());
    }
}
