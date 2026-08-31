//! Character codes ↔ text, per font.
//!
//! A PDF stores glyph *codes*, not characters. What a code means is decided by
//! the font it is drawn with, and there are three ways to find out: a
//! `/ToUnicode` CMap (which maps both ways, and is therefore the only one that
//! makes a font safely writable), a named base encoding, or a `/Differences`
//! array naming each glyph.
//!
//! Where none of those answers, this module says so. It never guesses a
//! character: a code with no mapping makes its run [`Encoder::Opaque`], the run
//! is left out of the buffer, and any edit to the line it sits on is refused
//! rather than written as something the reader never saw.

use std::collections::HashMap;

use lopdf::{Dictionary, Document, Object};

/// A code slot no glyph occupies. `\0` is never a drawn character, so it needs
/// no parallel array of flags.
const NOTDEF: char = '\0';

/// How one font's codes and characters correspond.
pub enum Encoder {
    /// A single-byte font: 256 codes, each either a character or nothing.
    Simple(Box<[Option<char>; 256]>),
    /// A font carrying a `/ToUnicode` CMap, which maps both ways.
    Unicode(CMap),
    /// A font whose codes this build cannot map to characters.
    Opaque,
}

/// Why a font could not be read faithfully — surfaced as a warning naming it.
pub struct EncoderReport {
    pub encoder: Encoder,
    /// The font named no encoding, so a base one had to be assumed.
    pub assumed_base_encoding: bool,
    /// Glyph names in `/Differences` that this build does not know.
    pub unknown_glyph_names: Vec<String>,
}

impl Encoder {
    /// Work out how to read (and write) the font in `dict`.
    pub fn for_font(dict: &Dictionary, doc: &Document) -> EncoderReport {
        let plain = |encoder| EncoderReport {
            encoder,
            assumed_base_encoding: false,
            unknown_glyph_names: Vec::new(),
        };

        if let Some(cmap) = to_unicode(dict, doc) {
            return plain(Encoder::Unicode(cmap));
        }

        // A composite font without a ToUnicode CMap maps codes through a CID
        // system this build does not carry. Reading it would be invention.
        if dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Type0") {
            return plain(Encoder::Opaque);
        }

        let encoding = deref_dict(dict.get(b"Encoding").ok(), doc);
        let base_name = match &encoding {
            Some(Encoding::Named(name)) => Some(name.clone()),
            Some(Encoding::Dict(d)) => d
                .get(b"BaseEncoding")
                .and_then(Object::as_name)
                .ok()
                .map(<[u8]>::to_vec),
            None => None,
        };
        let differences = match &encoding {
            Some(Encoding::Dict(d)) => d.get(b"Differences").and_then(Object::as_array).ok(),
            _ => None,
        };

        // With no base encoding named and no differences given, the spec defers
        // to the encoding built into the font program. This build cannot read
        // one, and for a symbolic font — a dingbat or an icon set — guessing
        // Latin text out of it would be nonsense rather than an approximation.
        if base_name.is_none() && differences.is_none() && is_symbolic(dict, doc) {
            return plain(Encoder::Opaque);
        }

        let mut table = base_table(base_name.as_deref());
        let mut unknown = Vec::new();
        if let Some(differences) = differences {
            apply_differences(&mut table, differences, &mut unknown);
        }

        EncoderReport {
            encoder: Encoder::Simple(table),
            assumed_base_encoding: base_name.is_none(),
            unknown_glyph_names: unknown,
        }
    }

    /// Decode a show operator's bytes.
    ///
    /// `None` means at least one code has no character: the run is opaque and
    /// the caller must not pretend it read it.
    pub fn decode(&self, bytes: &[u8]) -> Option<String> {
        match self {
            Encoder::Opaque => None,
            Encoder::Simple(table) => bytes
                .iter()
                .map(|byte| table[*byte as usize])
                .collect::<Option<String>>(),
            Encoder::Unicode(cmap) => cmap.decode(bytes),
        }
    }

    /// Encode text back into character codes.
    ///
    /// `Err(c)` names the first character the font has no code for. The save
    /// stops there rather than dropping it — a silently shorter line is exactly
    /// the kind of "successful" write this crate exists to prevent.
    pub fn encode(&self, text: &str) -> Result<Vec<u8>, char> {
        match self {
            Encoder::Opaque => match text.chars().next() {
                Some(c) => Err(c),
                None => Ok(Vec::new()),
            },
            Encoder::Simple(table) => text
                .chars()
                .map(|c| {
                    table
                        .iter()
                        .position(|slot| *slot == Some(c))
                        .map(|code| code as u8)
                        .ok_or(c)
                })
                .collect(),
            Encoder::Unicode(cmap) => cmap.encode(text),
        }
    }

    /// Whether text can be written back through this font at all.
    pub const fn is_writable(&self) -> bool {
        !matches!(self, Encoder::Opaque)
    }
}

enum Encoding {
    Named(Vec<u8>),
    Dict(Dictionary),
}

fn deref_dict(object: Option<&Object>, doc: &Document) -> Option<Encoding> {
    match object {
        Some(Object::Name(name)) => Some(Encoding::Named(name.clone())),
        Some(Object::Dictionary(d)) => Some(Encoding::Dict(d.clone())),
        Some(Object::Reference(id)) => match doc.get_object(*id).ok()? {
            Object::Name(name) => Some(Encoding::Named(name.clone())),
            Object::Dictionary(d) => Some(Encoding::Dict(d.clone())),
            _ => None,
        },
        _ => None,
    }
}

/// Whether the font descriptor marks the font symbolic (flag bit 3).
fn is_symbolic(dict: &Dictionary, doc: &Document) -> bool {
    let descriptor = match dict.get(b"FontDescriptor").ok() {
        Some(Object::Dictionary(d)) => Some(d.clone()),
        Some(Object::Reference(id)) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    };
    descriptor
        .and_then(|d| d.get(b"Flags").and_then(Object::as_i64).ok())
        .map(|flags| flags & 4 != 0)
        .unwrap_or(false)
}

fn to_unicode(dict: &Dictionary, doc: &Document) -> Option<CMap> {
    let object = match dict.get(b"ToUnicode").ok()? {
        Object::Reference(id) => doc.get_object(*id).ok()?,
        other => other,
    };
    // `get_plain_content`, not `decompressed_content`: the latter returns an
    // empty vector — not an error — for a stream with no filter, and an
    // uncompressed ToUnicode map would then look like a font that has none.
    let bytes = object.as_stream().ok()?.get_plain_content().ok()?;
    CMap::parse(&bytes)
}

// ── ToUnicode CMaps ──────────────────────────────────────────────────

/// A `/ToUnicode` CMap, in both directions.
///
/// Ranges wider than [`EXPAND_LIMIT`] are kept as ranges rather than expanded:
/// `<0000> <ffff>` is one line of a CMap and 65 536 entries of a map, and an
/// Identity CMap of that shape is entirely ordinary.
pub struct CMap {
    /// `(code, code length in bytes)` → text.
    single: HashMap<(u32, u8), String>,
    /// text → `(code, code length)`, for writing.
    reverse: HashMap<String, (u32, u8)>,
    /// Ranges too wide to expand, mapped by offset from the range's first code.
    wide: Vec<WideRange>,
}

struct WideRange {
    low: u32,
    high: u32,
    length: u8,
    /// UTF-16 units of the range's first code; the last one advances with it.
    base: Vec<u16>,
}

/// Above this many codes a `bfrange` is kept rather than expanded.
const EXPAND_LIMIT: u32 = 4096;

impl CMap {
    fn parse(bytes: &[u8]) -> Option<CMap> {
        let tokens = lex(bytes);
        let mut map = CMap {
            single: HashMap::new(),
            reverse: HashMap::new(),
            wide: Vec::new(),
        };
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Word(word) if word == "beginbfchar" => i = map.read_chars(&tokens, i + 1),
                Token::Word(word) if word == "beginbfrange" => i = map.read_ranges(&tokens, i + 1),
                _ => i += 1,
            }
        }
        (!map.single.is_empty() || !map.wide.is_empty()).then_some(map)
    }

    fn read_chars(&mut self, tokens: &[Token], mut i: usize) -> usize {
        while i + 1 < tokens.len() {
            match (&tokens[i], &tokens[i + 1]) {
                (Token::Hex(src), Token::Hex(dst)) => {
                    if let Some(text) = utf16_be(dst) {
                        self.insert(code_of(src), src.len() as u8, text);
                    }
                    i += 2;
                }
                _ => return i + 1,
            }
        }
        i
    }

    fn read_ranges(&mut self, tokens: &[Token], mut i: usize) -> usize {
        while i + 2 < tokens.len() {
            let (Token::Hex(low), Token::Hex(high)) = (&tokens[i], &tokens[i + 1]) else {
                return i + 1;
            };
            let (low_code, high_code) = (code_of(low), code_of(high));
            let length = low.len() as u8;
            match &tokens[i + 2] {
                Token::Hex(dst) => {
                    let base: Vec<u16> = dst
                        .chunks_exact(2)
                        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                        .collect();
                    self.insert_range(low_code, high_code, length, base);
                    i += 3;
                }
                Token::Open => {
                    let mut code = low_code;
                    let mut k = i + 3;
                    while k < tokens.len() {
                        match &tokens[k] {
                            Token::Hex(dst) => {
                                if let Some(text) = utf16_be(dst) {
                                    self.insert(code, length, text);
                                }
                                code = code.saturating_add(1);
                                k += 1;
                            }
                            _ => break,
                        }
                    }
                    i = k + 1;
                }
                _ => return i + 1,
            }
        }
        i
    }

    fn insert(&mut self, code: u32, length: u8, text: String) {
        self.reverse.entry(text.clone()).or_insert((code, length));
        self.single.insert((code, length), text);
    }

    fn insert_range(&mut self, low: u32, high: u32, length: u8, base: Vec<u16>) {
        if base.is_empty() || high < low {
            return;
        }
        if high - low > EXPAND_LIMIT {
            self.wide.push(WideRange {
                low,
                high,
                length,
                base,
            });
            return;
        }
        for offset in 0..=(high - low) {
            let mut units = base.clone();
            if let Some(last) = units.last_mut() {
                *last = last.wrapping_add(offset as u16);
            }
            if let Ok(text) = String::from_utf16(&units) {
                self.insert(low + offset, length, text);
            }
        }
    }

    fn lookup(&self, code: u32, length: u8) -> Option<String> {
        if let Some(text) = self.single.get(&(code, length)) {
            return Some(text.clone());
        }
        self.wide.iter().find_map(|range| {
            (range.length == length && (range.low..=range.high).contains(&code))
                .then(|| {
                    let mut units = range.base.clone();
                    if let Some(last) = units.last_mut() {
                        *last = last.wrapping_add((code - range.low) as u16);
                    }
                    String::from_utf16(&units).ok()
                })
                .flatten()
        })
    }

    /// Decode bytes, failing rather than substituting U+FFFD for a code the map
    /// does not carry.
    fn decode(&self, bytes: &[u8]) -> Option<String> {
        let mut out = String::new();
        let mut code = 0u32;
        let mut length = 0u8;
        for byte in bytes {
            if length == 4 {
                return None;
            }
            code = code * 256 + u32::from(*byte);
            length += 1;
            if let Some(text) = self.lookup(code, length) {
                out.push_str(&text);
                code = 0;
                length = 0;
            }
        }
        (length == 0).then_some(out)
    }

    fn encode(&self, text: &str) -> Result<Vec<u8>, char> {
        let mut out = Vec::new();
        for c in text.chars() {
            let (code, length) = self.code_for(c).ok_or(c)?;
            match length {
                1 => out.push(code as u8),
                2 => out.extend_from_slice(&(code as u16).to_be_bytes()),
                3 => out.extend_from_slice(&code.to_be_bytes()[1..]),
                _ => out.extend_from_slice(&code.to_be_bytes()),
            }
        }
        Ok(out)
    }

    fn code_for(&self, c: char) -> Option<(u32, u8)> {
        let mut buffer = [0u8; 4];
        let text = c.encode_utf8(&mut buffer);
        if let Some(found) = self.reverse.get(text as &str) {
            return Some(*found);
        }
        let units: Vec<u16> = c.to_string().encode_utf16().collect();
        self.wide.iter().find_map(|range| {
            let (Some(unit), Some(base)) = (units.last(), range.base.last()) else {
                return None;
            };
            if units.len() != range.base.len()
                || units[..units.len() - 1] != range.base[..range.base.len() - 1]
            {
                return None;
            }
            let offset = u32::from(unit.wrapping_sub(*base));
            let code = range.low.checked_add(offset)?;
            (code <= range.high).then_some((code, range.length))
        })
    }
}

fn code_of(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0u32, |acc, b| acc * 256 + u32::from(*b))
}

fn utf16_be(bytes: &[u8]) -> Option<String> {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).ok()
}

enum Token {
    Hex(Vec<u8>),
    Open,
    Close,
    Word(String),
}

/// Split a CMap into the only tokens this reader needs: hex strings, array
/// brackets, and bare words (the `beginbfchar` family).
fn lex(bytes: &[u8]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'<' if bytes.get(i + 1) == Some(&b'<') => i += 2,
            b'>' if bytes.get(i + 1) == Some(&b'>') => i += 2,
            b'<' => {
                let mut digits = Vec::new();
                i += 1;
                while i < bytes.len() && bytes[i] != b'>' {
                    if bytes[i].is_ascii_hexdigit() {
                        digits.push(bytes[i]);
                    }
                    i += 1;
                }
                i += 1;
                // An odd digit count is padded, as the CMap syntax specifies.
                if digits.len() % 2 == 1 {
                    digits.push(b'0');
                }
                let value = digits
                    .chunks_exact(2)
                    .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
                    .collect();
                tokens.push(Token::Hex(value));
            }
            b'[' => {
                tokens.push(Token::Open);
                i += 1;
            }
            b']' => {
                tokens.push(Token::Close);
                i += 1;
            }
            b if b.is_ascii_whitespace() => i += 1,
            _ => {
                let start = i;
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && !matches!(bytes[i], b'<' | b'>' | b'[' | b']' | b'%' | b'(' | b')')
                {
                    i += 1;
                }
                if i == start {
                    i += 1;
                    continue;
                }
                tokens.push(Token::Word(
                    String::from_utf8_lossy(&bytes[start..i]).into_owned(),
                ));
            }
        }
    }
    tokens
}

// ── Base encodings ───────────────────────────────────────────────────

/// The upper half (0x80–0xFF) of WinAnsiEncoding, which is CP1252.
const WIN_ANSI_HIGH: &str = "\u{20ac}\0\u{201a}\u{192}\u{201e}\u{2026}\u{2020}\u{2021}\
\u{2c6}\u{2030}\u{160}\u{2039}\u{152}\0\u{17d}\0\
\0\u{2018}\u{2019}\u{201c}\u{201d}\u{2022}\u{2013}\u{2014}\
\u{2dc}\u{2122}\u{161}\u{203a}\u{153}\0\u{17e}\u{178}\
\u{a0}¡¢£¤¥¦§¨©ª«¬\u{ad}®¯°±²³´µ¶·¸¹º»¼½¾¿\
ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞß\
àáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ";

/// The upper half (0x80–0xFF) of MacRomanEncoding.
const MAC_ROMAN_HIGH: &str = "ÄÅÇÉÑÖÜáàâäãåçéèêëíìîïñóòôöõúùûü\
†°¢£§•¶ß®©\u{2122}´¨≠ÆØ∞±≤≥¥µ∂∑∏π∫ªºΩæø\
¿¡¬√\u{192}≈∆«»…\u{a0}ÀÃÕŒœ–—\u{201c}\u{201d}\u{2018}\u{2019}÷◊ÿŸ⁄€‹›\u{fb01}\u{fb02}\
‡·\u{201a}\u{201e}‰ÂÊÁËÈÍÎÏÌÓÔ\u{f8ff}ÒÚÛÙı\u{2c6}\u{2dc}¯\u{2d8}\u{2d9}\u{2da}¸\u{2dd}\u{2db}\u{2c7}";

/// StandardEncoding, which is ASCII with two different quotes and a sparse
/// upper half.
const STANDARD_DIFFERENCES: &[(u8, char)] = &[
    (0x27, '\u{2019}'),
    (0x60, '\u{2018}'),
    (0xa1, '¡'),
    (0xa2, '¢'),
    (0xa3, '£'),
    (0xa4, '\u{2044}'),
    (0xa5, '¥'),
    (0xa6, '\u{192}'),
    (0xa7, '§'),
    (0xa8, '¤'),
    (0xa9, '\''),
    (0xaa, '\u{201c}'),
    (0xab, '«'),
    (0xac, '\u{2039}'),
    (0xad, '\u{203a}'),
    (0xae, '\u{fb01}'),
    (0xaf, '\u{fb02}'),
    (0xb1, '\u{2013}'),
    (0xb2, '\u{2020}'),
    (0xb3, '\u{2021}'),
    (0xb4, '·'),
    (0xb6, '¶'),
    (0xb7, '\u{2022}'),
    (0xb8, '\u{201a}'),
    (0xb9, '\u{201e}'),
    (0xba, '\u{201d}'),
    (0xbb, '»'),
    (0xbc, '\u{2026}'),
    (0xbd, '\u{2030}'),
    (0xbf, '¿'),
    (0xc1, '`'),
    (0xc2, '´'),
    (0xc3, '\u{2c6}'),
    (0xc4, '\u{2dc}'),
    (0xc5, '¯'),
    (0xc6, '\u{2d8}'),
    (0xc7, '\u{2d9}'),
    (0xc8, '¨'),
    (0xca, '\u{2da}'),
    (0xcb, '¸'),
    (0xcd, '\u{2dd}'),
    (0xce, '\u{2db}'),
    (0xcf, '\u{2c7}'),
    (0xd0, '\u{2014}'),
    (0xe1, 'Æ'),
    (0xe3, 'ª'),
    (0xe8, '\u{141}'),
    (0xe9, 'Ø'),
    (0xea, '\u{152}'),
    (0xeb, 'º'),
    (0xf1, 'æ'),
    (0xf5, '\u{131}'),
    (0xf8, '\u{142}'),
    (0xf9, 'ø'),
    (0xfa, '\u{153}'),
    (0xfb, 'ß'),
];

/// The 256-entry table a named base encoding stands for.
///
/// With no name the spec defers to the font program's built-in encoding, which
/// this build cannot read. StandardEncoding is the documented stand-in for a
/// non-symbolic font; symbolic ones never reach here. The caller reports the
/// assumption rather than hiding it.
fn base_table(name: Option<&[u8]>) -> Box<[Option<char>; 256]> {
    let mut table = Box::new([None; 256]);
    for code in 0x20u8..=0x7e {
        table[code as usize] = Some(code as char);
    }
    match name {
        Some(b"WinAnsiEncoding") => fill_high(&mut table, WIN_ANSI_HIGH),
        Some(b"MacRomanEncoding") => fill_high(&mut table, MAC_ROMAN_HIGH),
        _ => {
            for (code, c) in STANDARD_DIFFERENCES {
                table[*code as usize] = Some(*c);
            }
        }
    }
    table
}

fn fill_high(table: &mut [Option<char>; 256], high: &str) {
    for (offset, c) in high.chars().enumerate() {
        match (0x80 + offset, c) {
            (code, _) if code > 0xff => break,
            (_, NOTDEF) => {}
            (code, c) => table[code] = Some(c),
        }
    }
}

/// Apply a `/Differences` array: `[code name name … code name …]`.
fn apply_differences(
    table: &mut [Option<char>; 256],
    differences: &[Object],
    unknown: &mut Vec<String>,
) {
    let mut code: usize = 0;
    for item in differences {
        match item {
            Object::Integer(n) if *n >= 0 => code = *n as usize,
            Object::Real(n) if *n >= 0.0 => code = *n as usize,
            Object::Name(name) => {
                if code < 256 {
                    let name = String::from_utf8_lossy(name).into_owned();
                    match glyph_char(&name) {
                        Some(c) => table[code] = Some(c),
                        None => {
                            table[code] = None;
                            if !unknown.contains(&name) {
                                unknown.push(name);
                            }
                        }
                    }
                }
                code += 1;
            }
            _ => {}
        }
    }
}

/// The character a glyph name stands for.
///
/// The algorithmic forms come first — `uni0041`, `u1F600` — then single letters
/// and digits, then the Adobe standard Latin names. A name outside all of them
/// is not guessed at.
fn glyph_char(name: &str) -> Option<char> {
    let base = name.split('.').next().unwrap_or(name);
    if let Some(hex) = base.strip_prefix("uni") {
        if hex.len() >= 4 && hex[..4].chars().all(|c| c.is_ascii_hexdigit()) {
            return u32::from_str_radix(&hex[..4], 16)
                .ok()
                .and_then(char::from_u32);
        }
    }
    if let Some(hex) = base.strip_prefix('u') {
        if (4..=6).contains(&hex.len()) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
        }
    }
    let mut chars = base.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_alphanumeric() {
            return Some(c);
        }
    }
    GLYPHS
        .iter()
        .find(|(glyph, _)| *glyph == base)
        .map(|(_, c)| *c)
}

/// The Adobe standard Latin glyph names, minus the single-letter and digit ones
/// [`glyph_char`] answers directly.
const GLYPHS: &[(&str, char)] = &[
    ("space", ' '),
    ("exclam", '!'),
    ("quotedbl", '"'),
    ("numbersign", '#'),
    ("dollar", '$'),
    ("percent", '%'),
    ("ampersand", '&'),
    ("quotesingle", '\''),
    ("quoteright", '\u{2019}'),
    ("quoteleft", '\u{2018}'),
    ("parenleft", '('),
    ("parenright", ')'),
    ("asterisk", '*'),
    ("plus", '+'),
    ("comma", ','),
    ("hyphen", '-'),
    ("period", '.'),
    ("slash", '/'),
    ("colon", ':'),
    ("semicolon", ';'),
    ("less", '<'),
    ("equal", '='),
    ("greater", '>'),
    ("question", '?'),
    ("at", '@'),
    ("bracketleft", '['),
    ("backslash", '\\'),
    ("bracketright", ']'),
    ("asciicircum", '^'),
    ("underscore", '_'),
    ("grave", '`'),
    ("braceleft", '{'),
    ("bar", '|'),
    ("braceright", '}'),
    ("asciitilde", '~'),
    ("nbspace", '\u{a0}'),
    ("exclamdown", '¡'),
    ("cent", '¢'),
    ("sterling", '£'),
    ("fraction", '\u{2044}'),
    ("yen", '¥'),
    ("florin", '\u{192}'),
    ("section", '§'),
    ("currency", '¤'),
    ("quotedblleft", '\u{201c}'),
    ("quotedblright", '\u{201d}'),
    ("guillemotleft", '«'),
    ("guillemotright", '»'),
    ("guilsinglleft", '\u{2039}'),
    ("guilsinglright", '\u{203a}'),
    ("fi", '\u{fb01}'),
    ("fl", '\u{fb02}'),
    ("endash", '\u{2013}'),
    ("emdash", '\u{2014}'),
    ("dagger", '\u{2020}'),
    ("daggerdbl", '\u{2021}'),
    ("periodcentered", '·'),
    ("paragraph", '¶'),
    ("bullet", '\u{2022}'),
    ("quotesinglbase", '\u{201a}'),
    ("quotedblbase", '\u{201e}'),
    ("ellipsis", '\u{2026}'),
    ("perthousand", '\u{2030}'),
    ("questiondown", '¿'),
    ("acute", '´'),
    ("circumflex", '\u{2c6}'),
    ("tilde", '\u{2dc}'),
    ("macron", '¯'),
    ("breve", '\u{2d8}'),
    ("dotaccent", '\u{2d9}'),
    ("dieresis", '¨'),
    ("ring", '\u{2da}'),
    ("cedilla", '¸'),
    ("hungarumlaut", '\u{2dd}'),
    ("ogonek", '\u{2db}'),
    ("caron", '\u{2c7}'),
    ("AE", 'Æ'),
    ("ordfeminine", 'ª'),
    ("Lslash", '\u{141}'),
    ("Oslash", 'Ø'),
    ("OE", '\u{152}'),
    ("ordmasculine", 'º'),
    ("ae", 'æ'),
    ("dotlessi", '\u{131}'),
    ("lslash", '\u{142}'),
    ("oslash", 'ø'),
    ("oe", '\u{153}'),
    ("germandbls", 'ß'),
    ("Aacute", 'Á'),
    ("Acircumflex", 'Â'),
    ("Adieresis", 'Ä'),
    ("Agrave", 'À'),
    ("Aring", 'Å'),
    ("Atilde", 'Ã'),
    ("Ccedilla", 'Ç'),
    ("Eacute", 'É'),
    ("Ecircumflex", 'Ê'),
    ("Edieresis", 'Ë'),
    ("Egrave", 'È'),
    ("Eth", 'Ð'),
    ("Iacute", 'Í'),
    ("Icircumflex", 'Î'),
    ("Idieresis", 'Ï'),
    ("Igrave", 'Ì'),
    ("Ntilde", 'Ñ'),
    ("Oacute", 'Ó'),
    ("Ocircumflex", 'Ô'),
    ("Odieresis", 'Ö'),
    ("Ograve", 'Ò'),
    ("Otilde", 'Õ'),
    ("Scaron", '\u{160}'),
    ("Thorn", 'Þ'),
    ("Uacute", 'Ú'),
    ("Ucircumflex", 'Û'),
    ("Udieresis", 'Ü'),
    ("Ugrave", 'Ù'),
    ("Yacute", 'Ý'),
    ("Ydieresis", '\u{178}'),
    ("Zcaron", '\u{17d}'),
    ("aacute", 'á'),
    ("acircumflex", 'â'),
    ("adieresis", 'ä'),
    ("agrave", 'à'),
    ("aring", 'å'),
    ("atilde", 'ã'),
    ("brokenbar", '¦'),
    ("ccedilla", 'ç'),
    ("copyright", '©'),
    ("degree", '°'),
    ("divide", '÷'),
    ("eacute", 'é'),
    ("ecircumflex", 'ê'),
    ("edieresis", 'ë'),
    ("egrave", 'è'),
    ("eth", 'ð'),
    ("iacute", 'í'),
    ("icircumflex", 'î'),
    ("idieresis", 'ï'),
    ("igrave", 'ì'),
    ("logicalnot", '¬'),
    ("minus", '\u{2212}'),
    ("mu", 'µ'),
    ("multiply", '×'),
    ("ntilde", 'ñ'),
    ("oacute", 'ó'),
    ("ocircumflex", 'ô'),
    ("odieresis", 'ö'),
    ("ograve", 'ò'),
    ("onehalf", '½'),
    ("onequarter", '¼'),
    ("onesuperior", '¹'),
    ("otilde", 'õ'),
    ("plusminus", '±'),
    ("registered", '®'),
    ("scaron", '\u{161}'),
    ("thorn", 'þ'),
    ("threequarters", '¾'),
    ("threesuperior", '³'),
    ("trademark", '\u{2122}'),
    ("twosuperior", '²'),
    ("uacute", 'ú'),
    ("ucircumflex", 'û'),
    ("udieresis", 'ü'),
    ("ugrave", 'ù'),
    ("yacute", 'ý'),
    ("ydieresis", 'ÿ'),
    ("zcaron", '\u{17e}'),
    ("Euro", '\u{20ac}'),
    ("hyphenminus", '-'),
    ("nonbreakingspace", '\u{a0}'),
    ("softhyphen", '\u{ad}'),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glyph_name_is_never_guessed() {
        assert_eq!(glyph_char("eacute"), Some('é'));
        assert_eq!(glyph_char("uni0041"), Some('A'));
        assert_eq!(glyph_char("u1F600"), Some('\u{1f600}'));
        assert_eq!(glyph_char("A"), Some('A'));
        assert_eq!(glyph_char("7"), Some('7'));
        assert_eq!(glyph_char("g123"), None);
        assert_eq!(glyph_char("nonesuch"), None);
    }

    #[test]
    fn the_base_encodings_line_up() {
        let win = base_table(Some(b"WinAnsiEncoding"));
        assert_eq!(
            win[0xa0],
            Some('\u{a0}'),
            "the whole upper half is 128 wide"
        );
        assert_eq!(win[0x41], Some('A'));
        assert_eq!(win[0x92], Some('\u{2019}'), "curly apostrophe");
        assert_eq!(win[0xe9], Some('é'));
        assert_eq!(win[0x81], None, "CP1252 leaves 0x81 undefined");

        let mac = base_table(Some(b"MacRomanEncoding"));
        assert_eq!(mac[0x8e], Some('é'));
        assert_eq!(mac[0xd5], Some('\u{2019}'));

        let standard = base_table(None);
        assert_eq!(
            standard[0x27],
            Some('\u{2019}'),
            "quoteright, not apostrophe"
        );
        assert_eq!(standard[0xe9], Some('Ø'));
        assert_eq!(standard[0xff], None);
    }

    #[test]
    fn an_unmapped_code_fails_the_read_rather_than_vanishing() {
        let mut table = Box::new([None; 256]);
        table[b'a' as usize] = Some('a');
        let encoder = Encoder::Simple(table);
        assert_eq!(encoder.decode(b"aa").as_deref(), Some("aa"));
        assert_eq!(encoder.decode(b"ab"), None);
    }

    #[test]
    fn a_character_the_font_lacks_names_itself() {
        let mut table = Box::new([None; 256]);
        table[b'a' as usize] = Some('a');
        let encoder = Encoder::Simple(table);
        assert_eq!(encoder.encode("aa"), Ok(vec![b'a', b'a']));
        assert_eq!(encoder.encode("a€"), Err('€'));
    }

    const CMAP: &[u8] = b"/CIDInit /ProcSet findresource begin
12 dict begin begincmap
1 begincodespacerange <0000> <FFFF> endcodespacerange
2 beginbfchar <0003> <0020> <0024> <0041> endbfchar
1 beginbfrange <0044> <0046> <0061> endbfrange
1 beginbfrange <0050> <0052> [<0058> <0059> <005a>] endbfrange
endcmap end end";

    #[test]
    fn a_to_unicode_cmap_maps_both_ways() {
        let cmap = CMap::parse(CMAP).expect("parsed");
        assert_eq!(cmap.decode(&[0x00, 0x03]).as_deref(), Some(" "));
        assert_eq!(cmap.decode(&[0x00, 0x24]).as_deref(), Some("A"));
        assert_eq!(cmap.decode(&[0x00, 0x45]).as_deref(), Some("b"), "range");
        assert_eq!(
            cmap.decode(&[0x00, 0x51]).as_deref(),
            Some("Y"),
            "array range"
        );
        assert_eq!(cmap.decode(&[0x00, 0x99]), None, "no such code");

        assert_eq!(cmap.encode("Ab"), Ok(vec![0x00, 0x24, 0x00, 0x45]));
        assert_eq!(
            cmap.encode("Y"),
            Ok(vec![0x00, 0x51]),
            "array range, reversed"
        );
        assert_eq!(
            cmap.encode("ß"),
            Err('ß'),
            "a character this font cannot draw"
        );
    }

    #[test]
    fn a_wide_range_is_not_expanded_but_still_maps_both_ways() {
        let cmap = CMap::parse(b"1 beginbfrange <0000> <ffff> <0000> endbfrange").expect("parsed");
        assert!(cmap.single.is_empty(), "65k entries are not materialised");
        assert_eq!(cmap.decode(&[0x00, 0x41]).as_deref(), Some("A"));
        assert_eq!(cmap.encode("A"), Ok(vec![0x00, 0x41]));
    }
}
