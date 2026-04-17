//! IME/CJK input support — CURSOR_MARKER APC escape + wide-char handling.
//! Pi-mono gap bridge: Phase C2.
//!
//! The CURSOR_MARKER is a zero-width APC escape sequence embedded in rendered
//! TUI output so that the physical terminal cursor can be positioned at the
//! exact column where text input is occurring — enabling IME candidate windows
//! (e.g. for Chinese, Japanese, Korean) to appear at the correct screen location.

// ── CURSOR_MARKER ─────────────────────────────────────────────────────────────

/// The zero-width APC escape sequence used to mark cursor position for IME.
/// Format: ESC _ pi-cursor ESC \  (APC string per ECMA-48 §8.3.2)
///
/// APC sequences are silently ignored by virtually all terminal emulators, so
/// embedding this string in rendered output is invisible to the user but can be
/// detected by VibeCLI's render layer to compute cursor column offsets.
pub const CURSOR_MARKER: &str = "\x1b_pi-cursor\x1b\\";

/// Insert the CURSOR_MARKER at visible column `col` in a rendered line string.
///
/// The column is measured in terminal display columns (wide chars count as 2).
/// ANSI escape sequences are skipped without consuming column budget.
/// If `col` exceeds the line's visible width the marker is appended at the end.
pub fn insert_cursor_marker(line: &str, col: usize) -> String {
    let mut out = String::with_capacity(line.len() + CURSOR_MARKER.len());
    let mut current_col: usize = 0;
    let mut inserted = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Detect the start of an ANSI / CURSOR_MARKER escape sequence.
        if chars[i] == '\x1b' {
            // Check if this is our own CURSOR_MARKER — skip it wholesale so we
            // don't double-insert.
            let remaining: String = chars[i..].iter().collect();
            if remaining.starts_with(CURSOR_MARKER) {
                // Skip over the existing marker.
                for c in CURSOR_MARKER.chars() {
                    out.push(c);
                    let _ = c; // suppress lint
                }
                i += CURSOR_MARKER.chars().count();
                continue;
            }

            // Generic ANSI/OSC/APC escape — consume to end of sequence.
            if !inserted && current_col >= col {
                out.push_str(CURSOR_MARKER);
                inserted = true;
            }
            let seq_start = i;
            i += 1; // consume ESC
            if i < chars.len() {
                match chars[i] {
                    '[' => {
                        // CSI: ESC [ ... final-byte (0x40–0x7E)
                        i += 1;
                        while i < chars.len() && !(('\x40'..='\x7e').contains(&chars[i])) {
                            i += 1;
                        }
                        if i < chars.len() {
                            i += 1; // consume final byte
                        }
                    }
                    ']' | '_' | 'P' | 'X' | '^' => {
                        // OSC / APC / DCS / SOS / PM: terminated by ST (ESC \) or BEL
                        i += 1;
                        while i < chars.len() {
                            if chars[i] == '\x07' {
                                i += 1;
                                break;
                            }
                            if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '\\' {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                    }
                    _ => {
                        // Fe or Fp: two-byte sequence
                        i += 1;
                    }
                }
            }
            // Re-emit the escape sequence verbatim.
            for c in chars[seq_start..i].iter() {
                out.push(*c);
            }
            continue;
        }

        // Before emitting a printable char, check if we've reached `col`.
        if !inserted && current_col >= col {
            out.push_str(CURSOR_MARKER);
            inserted = true;
        }

        let c = chars[i];
        let w = EawCategory::for_char(c).display_width();
        out.push(c);
        current_col += w;
        i += 1;
    }

    if !inserted {
        out.push_str(CURSOR_MARKER);
    }
    out
}

/// Find the byte offset (in display columns) just before the CURSOR_MARKER in
/// rendered output.  Returns `None` if the marker is not present.
pub fn find_cursor_marker(rendered: &str) -> Option<usize> {
    // Split on the marker and compute the visible width of everything before it.
    let (before, _after) = rendered.split_once(CURSOR_MARKER)?;
    Some(visible_width(before))
}

/// Strip every occurrence of CURSOR_MARKER from `s`.
pub fn strip_cursor_marker(s: &str) -> String {
    s.replace(CURSOR_MARKER, "")
}

// ── Unicode East Asian Width ───────────────────────────────────────────────────

/// East Asian Width (EAW) category per Unicode Standard Annex #11.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EawCategory {
    /// Narrow — occupies 1 column (most Latin, digits, symbols).
    Narrow,
    /// Wide — occupies 2 columns (CJK Unified Ideographs, full-width forms, etc.)
    Wide,
    /// Ambiguous — context-dependent; treated as 1 column in most terminals.
    Ambiguous,
    /// Neutral — non-East-Asian script, typically 1 column.
    Neutral,
    /// Halfwidth — 1 column (halfwidth Katakana, Latin in KR compatibility block).
    Halfwidth,
    /// Fullwidth — 2 columns (fullwidth ASCII / punctuation variants).
    Fullwidth,
}

impl EawCategory {
    /// Determine the EAW category for a Unicode scalar value.
    ///
    /// Wide ranges follow Unicode 15.1 UAX #11 plus common emoji blocks.
    /// No external crates — ranges are checked directly.
    pub fn for_char(c: char) -> Self {
        let cp = c as u32;
        match cp {
            // --- Wide (W) ---
            // Hangul Jamo
            0x1100..=0x115F => Self::Wide,
            // CJK Radicals Supplement … CJK Symbols and Punctuation
            0x2E80..=0x303F => Self::Wide,
            // Hiragana … CJK Compatibility
            0x3040..=0x33FF => Self::Wide,
            // CJK Unified Ideographs Extension A
            0x3400..=0x4DBF => Self::Wide,
            // CJK Unified Ideographs
            0x4E00..=0x9FFF => Self::Wide,
            // Yi Syllables + Yi Radicals
            0xA000..=0xA4CF => Self::Wide,
            // Hangul Syllables
            0xAC00..=0xD7AF => Self::Wide,
            // CJK Compatibility Ideographs
            0xF900..=0xFAFF => Self::Wide,
            // Vertical Forms … CJK Compatibility Forms
            0xFE10..=0xFE4F => Self::Wide,
            // Fullwidth Latin / Halfwidth Katakana block
            // (halfwidth Katakana are actually Halfwidth, handled below)
            0xFF01..=0xFF60 => Self::Fullwidth,
            // Halfwidth CJK punctuation / Halfwidth Hangul / Halfwidth Katakana
            0xFF61..=0xFFEF => Self::Halfwidth,
            // Kana Supplement
            0x1B000..=0x1B0FF => Self::Wide,
            // Mahjong Tiles (U+1F004 is wide)
            0x1F004 => Self::Wide,
            // Playing Cards joker (U+1F0CF)
            0x1F0CF => Self::Wide,
            // Enclosed CJK Letters Supplement
            0x1F200..=0x1F2FF => Self::Wide,
            // CJK Unified Ideographs Extension B
            0x20000..=0x2A6DF => Self::Wide,
            // CJK Unified Ideographs Extension C–D
            0x2A700..=0x2CEAF => Self::Wide,
            // CJK Unified Ideographs Extension E–F
            0x2CEB0..=0x2EBEF => Self::Wide,
            // CJK Unified Ideographs Extension G
            0x30000..=0x3134F => Self::Wide,

            // --- Fullwidth (F) — matched above in 0xFF01..=0xFF60 ---

            // --- Halfwidth (H) — matched above in 0xFF61..=0xFFEF ---

            // --- Ambiguous (A) — Greek, Cyrillic letterforms, math symbols ---
            // A representative subset; full UAX#11 table not enumerated here
            // to keep the implementation lean and allocation-free.
            0x00B2..=0x00B3 => Self::Ambiguous, // superscript 2, 3
            0x00B7 => Self::Ambiguous,           // middle dot
            0x00BC..=0x00BE => Self::Ambiguous,  // fractions
            0x00D7 => Self::Ambiguous,           // multiplication sign
            0x00F7 => Self::Ambiguous,           // division sign
            0x2018..=0x2019 => Self::Ambiguous,  // curly single quotes
            0x201C..=0x201D => Self::Ambiguous,  // curly double quotes
            0x2026 => Self::Ambiguous,           // horizontal ellipsis
            0x2103 => Self::Ambiguous,           // degree celsius
            0x2116 => Self::Ambiguous,           // numero sign
            0x2190..=0x2199 => Self::Ambiguous,  // arrows
            0x21D2 | 0x21D4 => Self::Ambiguous,  // double arrows
            0x2200..=0x22FF => Self::Ambiguous,  // mathematical operators
            0x2308..=0x230B => Self::Ambiguous,  // ceiling / floor brackets
            0x2329..=0x232A => Self::Ambiguous,  // angle brackets (legacy)
            0x2500..=0x25FF => Self::Ambiguous,  // box drawing / geometric shapes
            0x2600..=0x27FF => Self::Ambiguous,  // misc symbols, arrows
            0x2900..=0x297F => Self::Ambiguous,  // supplemental arrows

            // Everything else is Neutral / Narrow.
            0x0021..=0x007E => Self::Narrow, // printable ASCII
            0xFF00 => Self::Fullwidth,       // fullwidth space (rare)
            _ => Self::Neutral,
        }
    }

    /// Display column width for this category.
    /// Wide and Fullwidth occupy 2 columns; everything else occupies 1.
    pub fn display_width(&self) -> usize {
        match self {
            Self::Wide | Self::Fullwidth => 2,
            _ => 1,
        }
    }
}

// ── Visible-width and string utilities ────────────────────────────────────────

/// Compute the visible display width of a string, correctly handling:
/// - ANSI / VT escape sequences (zero width)
/// - CURSOR_MARKER (zero width)
/// - CJK wide characters (2 columns)
/// - Control characters (zero width for most; 1 for tab if desired — treated as 0 here)
pub fn visible_width(s: &str) -> usize {
    // Strip marker first so its escape bytes don't confuse the ANSI parser.
    let s = strip_cursor_marker(s);
    let chars: Vec<char> = s.chars().collect();
    let mut width: usize = 0;
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\x1b' {
            // Skip escape sequence.
            i += 1;
            if i < chars.len() {
                match chars[i] {
                    '[' => {
                        i += 1;
                        while i < chars.len() && !(('\x40'..='\x7e').contains(&chars[i])) {
                            i += 1;
                        }
                        if i < chars.len() {
                            i += 1;
                        }
                    }
                    ']' | '_' | 'P' | 'X' | '^' => {
                        i += 1;
                        while i < chars.len() {
                            if chars[i] == '\x07' {
                                i += 1;
                                break;
                            }
                            if chars[i] == '\x1b'
                                && i + 1 < chars.len()
                                && chars[i + 1] == '\\'
                            {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            continue;
        }

        // Skip control characters (C0/C1) except ordinary printable chars.
        if (chars[i] as u32) < 0x20 || chars[i] == '\x7f' {
            i += 1;
            continue;
        }

        width += EawCategory::for_char(chars[i]).display_width();
        i += 1;
    }
    width
}

/// Truncate `s` to at most `max_cols` visible columns, preserving ANSI
/// escape sequences.  A truncated string always ends with the SGR reset
/// `\x1b[0m` if any ANSI sequences were open when the cut was made.
pub fn truncate_to_width(s: &str, max_cols: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut current_col: usize = 0;
    let mut i = 0;
    let mut ansi_open = false;

    while i < chars.len() {
        if current_col >= max_cols {
            break;
        }

        // Check for CURSOR_MARKER.
        if chars[i] == '\x1b' {
            let remaining: String = chars[i..].iter().collect();
            if remaining.starts_with(CURSOR_MARKER) {
                out.push_str(CURSOR_MARKER);
                i += CURSOR_MARKER.chars().count();
                continue;
            }

            // ANSI escape — emit verbatim (zero width).
            let seq_start = i;
            i += 1;
            if i < chars.len() {
                match chars[i] {
                    '[' => {
                        i += 1;
                        while i < chars.len() && !(('\x40'..='\x7e').contains(&chars[i])) {
                            i += 1;
                        }
                        if i < chars.len() {
                            i += 1;
                        }
                    }
                    ']' | '_' | 'P' | 'X' | '^' => {
                        i += 1;
                        while i < chars.len() {
                            if chars[i] == '\x07' {
                                i += 1;
                                break;
                            }
                            if chars[i] == '\x1b'
                                && i + 1 < chars.len()
                                && chars[i + 1] == '\\'
                            {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            let seq: String = chars[seq_start..i].iter().collect();
            // Heuristic: track whether we opened a non-reset SGR.
            if seq.starts_with("\x1b[") && !seq.starts_with("\x1b[0") {
                ansi_open = true;
            } else if seq == "\x1b[0m" || seq == "\x1b[m" {
                ansi_open = false;
            }
            out.push_str(&seq);
            continue;
        }

        if (chars[i] as u32) < 0x20 || chars[i] == '\x7f' {
            i += 1;
            continue;
        }

        let w = EawCategory::for_char(chars[i]).display_width();
        if current_col + w > max_cols {
            // Wide char would overflow — stop.
            break;
        }
        out.push(chars[i]);
        current_col += w;
        i += 1;
    }

    if ansi_open {
        out.push_str("\x1b[0m");
    }
    out
}

/// Wrap text to `max_cols` visible columns, breaking at word or character
/// boundaries, preserving ANSI escape sequences on every line.
///
/// - Words are separated by ASCII space; long words are hard-wrapped by char.
/// - ANSI SGR state is re-emitted at the start of continuation lines so colors
///   carry across line breaks.
#[allow(unused_assignments)]
pub fn wrap_to_width(s: &str, max_cols: usize) -> Vec<String> {
    if max_cols == 0 {
        return vec![];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut current_width: usize = 0;
    // Accumulated SGR state to re-open on continuation lines.
    let mut pending_sgr = String::new();

    // Helper: emit `pending_sgr` at the start of a new line if non-empty.
    macro_rules! new_line {
        () => {{
            lines.push(current_line.trim_end().to_string());
            current_line = String::new();
            current_width = 0;
            if !pending_sgr.is_empty() {
                current_line.push_str(&pending_sgr);
            }
        }};
    }

    // Split into "tokens": ANSI sequences (width 0) or individual chars.
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    // We process word by word for proper wrapping, but also handle ANSI inline.
    // Strategy: tokenise into words (space-delimited) and zero-width ANSI chunks.
    // For simplicity we do a single-pass char iteration with word-lookahead.

    // Collect a "word" starting at index `i`: all printable non-space chars plus
    // any ANSI sequences interspersed, up to the next ASCII space.
    while i < chars.len() {
        // Handle ANSI escape sequences: emit on current line, track SGR.
        if chars[i] == '\x1b' {
            let remaining: String = chars[i..].iter().collect();
            if remaining.starts_with(CURSOR_MARKER) {
                current_line.push_str(CURSOR_MARKER);
                i += CURSOR_MARKER.chars().count();
                continue;
            }

            let seq_start = i;
            i += 1;
            if i < chars.len() {
                match chars[i] {
                    '[' => {
                        i += 1;
                        while i < chars.len() && !(('\x40'..='\x7e').contains(&chars[i])) {
                            i += 1;
                        }
                        if i < chars.len() {
                            i += 1;
                        }
                    }
                    ']' | '_' | 'P' | 'X' | '^' => {
                        i += 1;
                        while i < chars.len() {
                            if chars[i] == '\x07' {
                                i += 1;
                                break;
                            }
                            if chars[i] == '\x1b'
                                && i + 1 < chars.len()
                                && chars[i + 1] == '\\'
                            {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            let seq: String = chars[seq_start..i].iter().collect();
            if seq.starts_with("\x1b[") {
                if seq == "\x1b[0m" || seq == "\x1b[m" {
                    pending_sgr.clear();
                } else {
                    // Accumulate — last colour wins in most terminals.
                    pending_sgr.push_str(&seq);
                }
            }
            current_line.push_str(&seq);
            continue;
        }

        // Newline in input — flush current line.
        if chars[i] == '\n' {
            new_line!();
            i += 1;
            continue;
        }

        // Space — word separator.
        if chars[i] == ' ' {
            if current_width < max_cols {
                current_line.push(' ');
                current_width += 1;
            } else {
                new_line!();
            }
            i += 1;
            continue;
        }

        // Collect the next word (printable chars up to space/newline/escape).
        let word_start = i;
        let mut word = String::new();
        let mut word_width: usize = 0;
        while i < chars.len()
            && chars[i] != ' '
            && chars[i] != '\n'
            && chars[i] != '\x1b'
            && (chars[i] as u32) >= 0x20
            && chars[i] != '\x7f'
        {
            let w = EawCategory::for_char(chars[i]).display_width();
            word.push(chars[i]);
            word_width += w;
            i += 1;
        }
        let _ = word_start; // suppress unused warning

        if word.is_empty() {
            // Control character or other — skip.
            if i < chars.len() && (chars[i] as u32) < 0x20 {
                i += 1;
            }
            continue;
        }

        // If the whole word fits on current line, emit it.
        if current_width + word_width <= max_cols {
            current_line.push_str(&word);
            current_width += word_width;
        } else if word_width > max_cols {
            // Word longer than max_cols: hard-wrap char by char.
            for wc in word.chars() {
                let w = EawCategory::for_char(wc).display_width();
                if current_width + w > max_cols {
                    new_line!();
                }
                current_line.push(wc);
                current_width += w;
            }
        } else {
            // Word fits on a fresh line.
            new_line!();
            current_line.push_str(&word);
            current_width = word_width;
        }
    }

    if !current_line.is_empty()
        || (!lines.is_empty() && current_line.is_empty() && s.ends_with('\n'))
    {
        lines.push(current_line);
    }
    lines
}

// ── Terminal / locale detection ────────────────────────────────────────────────

/// Detect whether the current terminal is likely IME-capable.
///
/// Checks `$LC_ALL`, `$LC_CTYPE`, `$LANG` (in that order) for a UTF-8 locale
/// that also names a CJK language code (`zh`, `ja`, `ko`).
pub fn is_ime_capable_terminal() -> bool {
    let vars = ["LC_ALL", "LC_CTYPE", "LANG"];
    for var in &vars {
        if let Ok(val) = std::env::var(var) {
            let lower = val.to_lowercase();
            // Must be UTF-8 capable …
            if lower.contains("utf-8") || lower.contains("utf8") {
                // … and named for a CJK language.
                if lower.starts_with("zh")
                    || lower.starts_with("ja")
                    || lower.starts_with("ko")
                {
                    return true;
                }
            }
        }
    }
    false
}

// ── Cursor positioning ─────────────────────────────────────────────────────────

/// Build the CSI `CUP` sequence that moves the hardware cursor to `(row, col)`.
///
/// Both `row` and `col` are **1-based** (as required by the CSI H command).
/// Pass the values returned from your layout engine — e.g. the row of the input
/// widget and the column computed from `find_cursor_marker()`.
///
/// ```
/// use vibecli_cli::tui_ime::cursor_position_sequence;
/// assert_eq!(cursor_position_sequence(5, 20), "\x1b[5;20H");
/// ```
pub fn cursor_position_sequence(row: u16, col: u16) -> String {
    format!("\x1b[{};{}H", row, col)
}

// ── IME State Machine ─────────────────────────────────────────────────────────

/// Lifecycle state of an IME composition session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeState {
    /// No active IME composition.
    Idle,
    /// The IME candidate/composition window is open; `composition()` holds the
    /// preedit text.
    Composing,
    /// The IME committed text to the buffer.  Call `committed()` to retrieve it,
    /// then call `reset()` to return to `Idle`.
    Committed,
}

/// IME composition handler — tracks preedit and committed text.
#[derive(Debug)]
pub struct ImeHandler {
    state: ImeState,
    composition_buffer: String,
    committed_text: String,
}

impl ImeHandler {
    /// Create a new handler in the `Idle` state.
    pub fn new() -> Self {
        Self {
            state: ImeState::Idle,
            composition_buffer: String::new(),
            committed_text: String::new(),
        }
    }

    /// Current IME lifecycle state.
    pub fn state(&self) -> &ImeState {
        &self.state
    }

    /// Preedit / composition string (non-empty only during `Composing`).
    pub fn composition(&self) -> &str {
        &self.composition_buffer
    }

    /// Committed text (non-empty only when state is `Committed`).
    pub fn committed(&self) -> &str {
        &self.committed_text
    }

    /// Signal that the IME has opened a composition session.
    /// Transitions `Idle` → `Composing`.
    pub fn on_composition_start(&mut self) {
        self.composition_buffer.clear();
        self.committed_text.clear();
        self.state = ImeState::Composing;
    }

    /// Update the preedit text while the composition session is active.
    /// No-op if not currently `Composing`.
    pub fn on_composition_update(&mut self, text: &str) {
        if self.state == ImeState::Composing {
            self.composition_buffer = text.to_owned();
        }
    }

    /// Signal that the IME has finalised a selection.
    /// Transitions `Composing` → `Committed`; stores `final_text`.
    pub fn on_composition_end(&mut self, final_text: &str) {
        self.committed_text = final_text.to_owned();
        self.composition_buffer.clear();
        self.state = ImeState::Committed;
    }

    /// Reset back to `Idle`, clearing both buffers.
    /// Call this after consuming `committed()`.
    pub fn reset(&mut self) {
        self.state = ImeState::Idle;
        self.composition_buffer.clear();
        self.committed_text.clear();
    }
}

impl Default for ImeHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── CURSOR_MARKER helpers ──────────────────────────────────────────────────

    #[test]
    fn insert_cursor_marker_at_col_zero() {
        let result = insert_cursor_marker("hello", 0);
        assert!(
            result.starts_with(CURSOR_MARKER),
            "marker should be at position 0, got: {:?}",
            result
        );
        assert!(result.ends_with("hello"), "text after marker");
    }

    #[test]
    fn insert_cursor_marker_at_end() {
        let result = insert_cursor_marker("hi", 10);
        assert!(
            result.ends_with(CURSOR_MARKER),
            "marker should be appended when col > len"
        );
        assert!(result.starts_with("hi"));
    }

    #[test]
    fn insert_cursor_marker_mid_ascii() {
        // Insert at column 3 of "hello world" → "hel<MARKER>lo world"
        let result = insert_cursor_marker("hello world", 3);
        let stripped = strip_cursor_marker(&result);
        assert_eq!(stripped, "hello world");
        let col = find_cursor_marker(&result).unwrap();
        assert_eq!(col, 3);
    }

    #[test]
    fn find_cursor_marker_returns_none_when_absent() {
        assert_eq!(find_cursor_marker("plain text"), None);
    }

    #[test]
    fn find_cursor_marker_column_with_cjk() {
        // "你好" is 4 cols; insert marker at col 4 (after both chars).
        let line = "你好";
        let marked = insert_cursor_marker(line, 4);
        let col = find_cursor_marker(&marked).unwrap();
        assert_eq!(col, 4);
    }

    #[test]
    fn strip_cursor_marker_removes_all() {
        let s = format!("a{}b{}c", CURSOR_MARKER, CURSOR_MARKER);
        assert_eq!(strip_cursor_marker(&s), "abc");
    }

    // ── EawCategory ───────────────────────────────────────────────────────────

    #[test]
    fn eaw_ascii_is_narrow() {
        assert_eq!(EawCategory::for_char('A'), EawCategory::Narrow);
        assert_eq!(EawCategory::for_char('A').display_width(), 1);
    }

    #[test]
    fn eaw_cjk_ideograph_is_wide() {
        // U+4E2D "中"
        assert_eq!(EawCategory::for_char('中'), EawCategory::Wide);
        assert_eq!(EawCategory::for_char('中').display_width(), 2);
    }

    #[test]
    fn eaw_hangul_syllable_is_wide() {
        // U+AC00 가
        assert_eq!(EawCategory::for_char('가'), EawCategory::Wide);
        assert_eq!(EawCategory::for_char('가').display_width(), 2);
    }

    #[test]
    fn eaw_hiragana_is_wide() {
        // U+3042 あ
        assert_eq!(EawCategory::for_char('あ').display_width(), 2);
    }

    #[test]
    fn eaw_fullwidth_latin_is_fullwidth() {
        // U+FF21 Ａ (fullwidth A)
        assert_eq!(EawCategory::for_char('\u{FF21}'), EawCategory::Fullwidth);
        assert_eq!(EawCategory::for_char('\u{FF21}').display_width(), 2);
    }

    #[test]
    fn eaw_halfwidth_katakana_is_halfwidth() {
        // U+FF65 halfwidth katakana middle dot
        assert_eq!(EawCategory::for_char('\u{FF65}'), EawCategory::Halfwidth);
        assert_eq!(EawCategory::for_char('\u{FF65}').display_width(), 1);
    }

    // ── visible_width ─────────────────────────────────────────────────────────

    #[test]
    fn visible_width_plain_ascii() {
        assert_eq!(visible_width("hello"), 5);
    }

    #[test]
    fn visible_width_strips_ansi() {
        // "\x1b[32mhello\x1b[0m" — ANSI colour codes are zero-width.
        let s = "\x1b[32mhello\x1b[0m";
        assert_eq!(visible_width(s), 5);
    }

    #[test]
    fn visible_width_cjk_wide() {
        // "中文" — 2 wide chars = 4 columns.
        assert_eq!(visible_width("中文"), 4);
    }

    #[test]
    fn visible_width_mixed_ansi_cjk() {
        // bold "你" (2 cols) + " hi" (3 cols) = 5.
        let s = "\x1b[1m你\x1b[0m hi";
        assert_eq!(visible_width(s), 5);
    }

    #[test]
    fn visible_width_ignores_cursor_marker() {
        let s = format!("ab{}cd", CURSOR_MARKER);
        assert_eq!(visible_width(&s), 4);
    }

    // ── truncate_to_width ─────────────────────────────────────────────────────

    #[test]
    fn truncate_ascii_shorter_than_max() {
        assert_eq!(truncate_to_width("hi", 10), "hi");
    }

    #[test]
    fn truncate_ascii_exact() {
        assert_eq!(truncate_to_width("hello", 5), "hello");
    }

    #[test]
    fn truncate_ascii_over() {
        let t = truncate_to_width("hello world", 5);
        assert_eq!(visible_width(&t), 5);
        assert_eq!(strip_cursor_marker(&t), "hello");
    }

    #[test]
    fn truncate_cjk_wide_chars() {
        // "你好世界" = 8 cols; truncate to 4 → "你好"
        let t = truncate_to_width("你好世界", 4);
        assert_eq!(visible_width(&t), 4);
    }

    #[test]
    fn truncate_cjk_stops_before_overflow() {
        // max_cols = 3, wide chars are 2 each → only one wide char fits (2 cols).
        let t = truncate_to_width("你好", 3);
        assert_eq!(visible_width(&t), 2, "wide char must not partially overflow");
    }

    #[test]
    fn truncate_resets_ansi_if_open() {
        // Open bold — truncation should close it.
        let s = "\x1b[1mhello world";
        let t = truncate_to_width(s, 5);
        assert!(t.ends_with("\x1b[0m"), "should close open ANSI: {:?}", t);
    }

    // ── wrap_to_width ─────────────────────────────────────────────────────────

    #[test]
    fn wrap_short_line_unchanged() {
        let lines = wrap_to_width("hello", 20);
        assert_eq!(lines, vec!["hello"]);
    }

    #[test]
    fn wrap_breaks_at_word_boundary() {
        let lines = wrap_to_width("hello world", 7);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "hello");
        assert_eq!(lines[1], "world");
    }

    #[test]
    fn wrap_hard_wraps_long_word() {
        // A single 20-char word, max 10 → 2 lines of 10.
        let word = "abcdefghijklmnopqrst"; // 20 chars
        let lines = wrap_to_width(word, 10);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "abcdefghij");
        assert_eq!(lines[1], "klmnopqrst");
    }

    #[test]
    fn wrap_cjk_text() {
        // "你好世界" (8 cols) wraps to 4 → 2 lines each 4 cols.
        let lines = wrap_to_width("你好世界", 4);
        assert_eq!(lines.len(), 2);
        assert_eq!(visible_width(&lines[0]), 4);
        assert_eq!(visible_width(&lines[1]), 4);
    }

    // ── cursor_position_sequence ──────────────────────────────────────────────

    #[test]
    fn cursor_position_sequence_format() {
        assert_eq!(cursor_position_sequence(5, 20), "\x1b[5;20H");
    }

    #[test]
    fn cursor_position_sequence_row1_col1() {
        assert_eq!(cursor_position_sequence(1, 1), "\x1b[1;1H");
    }

    // ── ImeHandler state machine ──────────────────────────────────────────────

    #[test]
    fn ime_handler_initial_state_is_idle() {
        let h = ImeHandler::new();
        assert_eq!(h.state(), &ImeState::Idle);
        assert_eq!(h.composition(), "");
        assert_eq!(h.committed(), "");
    }

    #[test]
    fn ime_handler_composition_start_transitions_to_composing() {
        let mut h = ImeHandler::new();
        h.on_composition_start();
        assert_eq!(h.state(), &ImeState::Composing);
    }

    #[test]
    fn ime_handler_composition_update_sets_preedit() {
        let mut h = ImeHandler::new();
        h.on_composition_start();
        h.on_composition_update("にほん");
        assert_eq!(h.composition(), "にほん");
    }

    #[test]
    fn ime_handler_composition_end_transitions_to_committed() {
        let mut h = ImeHandler::new();
        h.on_composition_start();
        h.on_composition_update("ni");
        h.on_composition_end("日本");
        assert_eq!(h.state(), &ImeState::Committed);
        assert_eq!(h.committed(), "日本");
        assert_eq!(h.composition(), "");
    }

    #[test]
    fn ime_handler_reset_returns_to_idle() {
        let mut h = ImeHandler::new();
        h.on_composition_start();
        h.on_composition_end("안녕");
        h.reset();
        assert_eq!(h.state(), &ImeState::Idle);
        assert_eq!(h.committed(), "");
    }

    #[test]
    fn ime_handler_update_ignored_when_not_composing() {
        let mut h = ImeHandler::new();
        h.on_composition_update("ignored");
        assert_eq!(h.composition(), "");
        assert_eq!(h.state(), &ImeState::Idle);
    }

    #[test]
    fn ime_handler_full_lifecycle() {
        let mut h = ImeHandler::new();
        assert_eq!(h.state(), &ImeState::Idle);
        h.on_composition_start();
        assert_eq!(h.state(), &ImeState::Composing);
        h.on_composition_update("中");
        assert_eq!(h.composition(), "中");
        h.on_composition_update("中文");
        assert_eq!(h.composition(), "中文");
        h.on_composition_end("中文输入");
        assert_eq!(h.state(), &ImeState::Committed);
        assert_eq!(h.committed(), "中文输入");
        h.reset();
        assert_eq!(h.state(), &ImeState::Idle);
    }
}
