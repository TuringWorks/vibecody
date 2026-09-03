//! Design mode — visual annotation, change spec, and design token extraction.

use crate::design_providers::DesignTokenType;
use serde::{Deserialize, Serialize};

// ─── AnnotationKind ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnnotationKind {
    Arrow {
        from_label: String,
        to_label: String,
        label: String,
    },
    Region {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        description: String,
    },
    TextLabel {
        x: u32,
        y: u32,
        text: String,
    },
    BeforeAfter {
        before_url: String,
        after_url: String,
    },
    ColorSwatch {
        hex: String,
        label: String,
    },
    Measurement {
        from_label: String,
        to_label: String,
        expected_value: String,
    },
}

// ─── Annotation ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub annotation_id: String,
    pub kind: AnnotationKind,
    /// 1 = highest priority, 5 = lowest priority
    pub priority: u8,
    pub created_at_ms: u64,
}

/// Convert an annotation to a natural-language instruction string.
pub fn annotation_to_instruction(ann: &Annotation) -> String {
    match &ann.kind {
        AnnotationKind::Arrow {
            from_label,
            to_label,
            label,
        } => {
            format!("Move {} to align with {}: {}", from_label, to_label, label)
        }
        AnnotationKind::Region {
            x,
            y,
            width,
            height,
            description,
        } => {
            format!(
                "Update the region at ({},{}) size {}x{}: {}",
                x, y, width, height, description
            )
        }
        AnnotationKind::TextLabel { x, y, text } => {
            format!("Change text at ({},{}) to: {}", x, y, text)
        }
        AnnotationKind::BeforeAfter {
            before_url,
            after_url,
        } => {
            format!(
                "Apply before/after change: before={} after={}",
                before_url, after_url
            )
        }
        AnnotationKind::ColorSwatch { hex, label } => {
            format!("Use color {} for {}", hex, label)
        }
        AnnotationKind::Measurement {
            from_label,
            to_label,
            expected_value,
        } => {
            format!(
                "Set distance from {} to {} to {}",
                from_label, to_label, expected_value
            )
        }
    }
}

// ─── ChangeSpec ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSpec {
    pub spec_id: String,
    pub annotations: Vec<Annotation>,
}

impl ChangeSpec {
    pub fn new() -> Self {
        Self {
            spec_id: uuid_v4(),
            annotations: Vec::new(),
        }
    }

    pub fn add(&mut self, ann: Annotation) {
        self.annotations.push(ann);
    }

    /// Returns instructions sorted by priority (1 = highest first).
    pub fn to_instructions(&self) -> Vec<String> {
        let mut sorted = self.annotations.clone();
        sorted.sort_by_key(|a| a.priority);
        sorted.iter().map(annotation_to_instruction).collect()
    }

    /// Renders a markdown change spec document.
    pub fn to_markdown(&self) -> String {
        let instructions = self.to_instructions();
        let mut md = String::from("# Change Spec\n\n");
        for (i, instruction) in instructions.iter().enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, instruction));
        }
        md
    }

    pub fn annotation_count(&self) -> usize {
        self.annotations.len()
    }
}

impl Default for ChangeSpec {
    fn default() -> Self {
        Self::new()
    }
}

// ─── DesignTokenRef ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignTokenRef {
    pub var_name: String,
    pub hex_value: String,
    pub usage_context: String,
}

// ─── DesignTokenExtractor ────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DesignTokenExtractor {
    tokens: Vec<DesignTokenRef>,
}

impl DesignTokenExtractor {
    pub fn new() -> Self {
        Self { tokens: Vec::new() }
    }

    /// The tokens extracted so far.
    ///
    /// Without this the type could be filled but never read, which is what
    /// left VibeCoder's design-token view unable to use it.
    pub fn tokens(&self) -> &[DesignTokenRef] {
        &self.tokens
    }

    /// Parses CSS for `--var-name: #hexcolor` declarations and stores them.
    ///
    /// Delegates to [`extract_css_variables`] so there is one CSS parser here,
    /// then keeps only the color-valued declarations this type is about.
    pub fn extract_from_css(&mut self, css: &str) {
        for var in extract_css_variables(css) {
            if !var.value.starts_with('#') {
                continue;
            }
            if self.tokens.iter().any(|t| t.var_name == var.name) {
                continue;
            }
            self.tokens.push(DesignTokenRef {
                var_name: var.name,
                hex_value: var.value,
                usage_context: String::new(),
            });
        }
    }

    /// Finds a token by hex value (case-insensitive).
    pub fn find_for_hex(&self, hex: &str) -> Option<&DesignTokenRef> {
        let needle = hex.to_lowercase();
        self.tokens
            .iter()
            .find(|t| t.hex_value.to_lowercase() == needle)
    }

    pub fn all_tokens(&self) -> &[DesignTokenRef] {
        &self.tokens
    }
}

// ─── CSS custom properties ───────────────────────────────────────────────────

/// One `--name: value` declaration found in a stylesheet.
///
/// `token_type` is inferred from the declaration, never guessed: a value whose
/// shape matches no rule is [`DesignTokenType::Other`] rather than being filed
/// under a plausible-looking category it was never shown to belong to. It is
/// the design system's own token type — not a second vocabulary — so a scanned
/// stylesheet drops straight into `design_system_hub`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CssVariable {
    pub name: String,
    pub value: String,
    pub token_type: DesignTokenType,
}

/// Remove `/* … */` comments so a commented-out declaration is not reported as
/// a live token.
fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let bytes = css.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            match css[i + 2..].find("*/") {
                Some(end) => i = i + 2 + end + 2,
                // Unterminated comment: everything after it is commented out.
                None => break,
            }
        } else {
            // Push the whole UTF-8 character, not the byte, so a multi-byte
            // character in a `content:` string cannot corrupt the output.
            let ch = css[i..].chars().next().unwrap_or('\0');
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Every `--name: value` declaration in a stylesheet, in source order, with the
/// first definition of a name winning.
///
/// Declarations are separated by `;`, `{` or `}` — a line-based scan misses
/// `:root{--a:#fff;--b:8px}`, which is exactly what a minified or generated
/// sheet looks like.
pub fn extract_css_variables(css: &str) -> Vec<CssVariable> {
    let cleaned = strip_css_comments(css);
    let mut out: Vec<CssVariable> = Vec::new();
    for decl in cleaned.split([';', '{', '}']) {
        let decl = decl.trim();
        if !decl.starts_with("--") {
            continue;
        }
        let Some(colon) = decl.find(':') else {
            continue;
        };
        let name = decl[..colon].trim();
        let value = decl[colon + 1..].trim();
        // A name with whitespace in it is not a custom property; it is the
        // tail of a selector that happened to contain a colon.
        if name.len() < 3 || name.contains(char::is_whitespace) || value.is_empty() {
            continue;
        }
        if out.iter().any(|v| v.name == name) {
            continue;
        }
        out.push(CssVariable {
            name: name.to_string(),
            value: value.to_string(),
            token_type: categorize_css_value(name, value),
        });
    }
    out
}

/// Does this value read as a color literal or color function?
fn looks_like_color(value: &str) -> bool {
    let v = value.trim();
    if let Some(hex) = v.strip_prefix('#') {
        let digits = hex.chars().take_while(|c| c.is_ascii_hexdigit()).count();
        return digits == hex.len() && matches!(digits, 3 | 4 | 6 | 8);
    }
    let lower = v.to_ascii_lowercase();
    [
        "rgb(", "rgba(", "hsl(", "hsla(", "oklch(", "oklab(", "lab(", "lch(", "color(",
    ]
    .iter()
    .any(|f| lower.starts_with(f))
}

/// The design-system token type a `--name: value` declaration belongs to.
///
/// The name is consulted before the unit because `--font-size-md: 14px` and
/// `--space-md: 14px` have identical values and different meanings; the value
/// only decides when the name says nothing. Nothing here guesses — a
/// declaration matching no rule is `Other`.
pub fn categorize_css_value(name: &str, value: &str) -> DesignTokenType {
    if looks_like_color(value) {
        return DesignTokenType::Color;
    }
    let n = name.trim_start_matches('-').to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|k| n.contains(k));
    if has(&[
        "font",
        "text-size",
        "leading",
        "line-height",
        "tracking",
        "letter-spacing",
    ]) {
        return DesignTokenType::Typography;
    }
    if has(&["radius", "rounded"]) {
        return DesignTokenType::BorderRadius;
    }
    if has(&["shadow", "elevation"]) {
        return DesignTokenType::Shadow;
    }
    if has(&["duration", "transition", "ease", "delay", "animation"]) {
        return DesignTokenType::Animation;
    }
    if has(&["breakpoint", "screen-", "viewport"]) {
        return DesignTokenType::Breakpoint;
    }
    if has(&["z-index", "zindex", "z-layer", "layer-z"]) {
        return DesignTokenType::ZIndex;
    }
    if has(&[
        "color",
        "bg",
        "background",
        "border-color",
        "accent",
        "fg",
        "foreground",
    ]) {
        return DesignTokenType::Color;
    }
    if has(&[
        "space", "spacing", "gap", "margin", "padding", "inset", "size", "width", "height",
    ]) {
        return DesignTokenType::Spacing;
    }
    let v = value.trim().to_ascii_lowercase();
    if v.ends_with("ms") || (v.ends_with('s') && v[..v.len() - 1].parse::<f64>().is_ok()) {
        return DesignTokenType::Animation;
    }
    if ["px", "rem", "em", "vh", "vw", "ch", "%"]
        .iter()
        .any(|u| v.ends_with(u))
    {
        return DesignTokenType::Spacing;
    }
    DesignTokenType::Other
}

// ─── Sketch shape recognition ────────────────────────────────────────────────

/// One shape drawn on the sketch canvas, in canvas units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SketchShape {
    /// The tool it was drawn with: `rect`, `circle`, `line`, `arrow`, `text`.
    pub kind: String,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub text: Option<String>,
}

/// What a drawn shape most plausibly stands for, and why.
///
/// `fit` is deliberately not called "confidence": it is how centrally the
/// drawn geometry sits inside the band the matched rule accepts, not a
/// probability that the suggestion is correct. A shape that only matched a
/// catch-all rule has **no** fit — reporting one would be inventing a
/// measurement — so it is `None`, and the UI must render that as "—".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeSuggestion {
    pub component: String,
    pub rule: String,
    pub fit: Option<f64>,
    pub reason: String,
}

/// How centrally `value` sits inside `[lo, hi]`: 1.0 at the midpoint, 0.0 at
/// either edge, `None` outside the band.
fn band_fit(value: f64, lo: f64, hi: f64) -> Option<f64> {
    if !(lo..=hi).contains(&value) || hi <= lo {
        return None;
    }
    let mid = (lo + hi) / 2.0;
    let half = (hi - lo) / 2.0;
    Some((1.0 - (value - mid).abs() / half).clamp(0.0, 1.0))
}

/// Map a drawn shape onto the UI component it most plausibly stands for.
///
/// Pure and total: every shape gets an answer, and the answer always says
/// which rule produced it, so a wrong suggestion can be argued with instead of
/// merely disbelieved.
pub fn suggest_component(shape: &SketchShape) -> ShapeSuggestion {
    let w = shape.width.abs();
    let h = shape.height.abs();
    let aspect = if h > 0.0 { w / h } else { f64::INFINITY };

    match shape.kind.as_str() {
        "text" => {
            let len = shape.text.as_deref().map(str::len).unwrap_or(0);
            if len == 0 {
                return ShapeSuggestion {
                    component: "Text".into(),
                    rule: "text.empty".into(),
                    fit: None,
                    reason: "Empty text shape — nothing to classify by.".into(),
                };
            }
            if len <= 24 {
                ShapeSuggestion {
                    component: "Heading".into(),
                    rule: "text.short".into(),
                    fit: band_fit(len as f64, 0.0, 24.0),
                    reason: format!("{len} characters — short enough to be a heading or label."),
                }
            } else {
                ShapeSuggestion {
                    component: "Paragraph".into(),
                    rule: "text.long".into(),
                    fit: None,
                    reason: format!("{len} characters — body copy rather than a label."),
                }
            }
        }
        "line" => ShapeSuggestion {
            component: "Divider".into(),
            rule: "line".into(),
            fit: None,
            reason: "A bare line separates content.".into(),
        },
        "arrow" => ShapeSuggestion {
            component: "Connector".into(),
            rule: "arrow".into(),
            fit: None,
            reason: "An arrow is flow between screens, not a rendered element.".into(),
        },
        "circle" | "ellipse" => {
            let roundness = if w.max(h) > 0.0 {
                w.min(h) / w.max(h)
            } else {
                1.0
            };
            if roundness > 0.85 && w.max(h) <= 72.0 {
                ShapeSuggestion {
                    component: "Avatar".into(),
                    rule: "circle.small_round".into(),
                    fit: band_fit(w.max(h), 16.0, 72.0),
                    reason: format!("Near-circular and {:.0}px across.", w.max(h)),
                }
            } else if roundness > 0.85 {
                ShapeSuggestion {
                    component: "Badge".into(),
                    rule: "circle.large_round".into(),
                    fit: None,
                    reason: format!(
                        "Circular but {:.0}px across — too large for an avatar.",
                        w.max(h)
                    ),
                }
            } else {
                ShapeSuggestion {
                    component: "Ellipse".into(),
                    rule: "circle.oblong".into(),
                    fit: None,
                    reason: "Oblong — no standard component matches.".into(),
                }
            }
        }
        "rect" | "rectangle" => {
            if h <= 10.0 && aspect >= 8.0 {
                return ShapeSuggestion {
                    component: "Divider".into(),
                    rule: "rect.hairline".into(),
                    fit: band_fit(h, 1.0, 10.0),
                    reason: format!("{h:.0}px tall and {aspect:.0}× as wide — a rule, not a box."),
                };
            }
            if h <= 56.0 {
                if let Some(fit) = band_fit(aspect, 1.5, 6.0) {
                    return ShapeSuggestion {
                        component: "Button".into(),
                        rule: "rect.button".into(),
                        fit: Some(fit),
                        reason: format!("{h:.0}px tall, {aspect:.1}:1 — button proportions."),
                    };
                }
                if aspect > 6.0 {
                    return ShapeSuggestion {
                        component: "Input".into(),
                        rule: "rect.input".into(),
                        fit: band_fit(aspect, 6.0, 20.0),
                        reason: format!("{h:.0}px tall and {aspect:.1}× as wide — a text field."),
                    };
                }
                if let Some(fit) = band_fit(aspect, 0.7, 1.5) {
                    return ShapeSuggestion {
                        component: "IconButton".into(),
                        rule: "rect.icon".into(),
                        fit: Some(fit),
                        reason: format!("{w:.0}×{h:.0} and roughly square — an icon target."),
                    };
                }
            }
            if let Some(fit) = band_fit(aspect, 0.5, 2.5) {
                if h > 56.0 && h <= 420.0 {
                    return ShapeSuggestion {
                        component: "Card".into(),
                        rule: "rect.card".into(),
                        fit: Some(fit),
                        reason: format!("{w:.0}×{h:.0} — card proportions."),
                    };
                }
            }
            ShapeSuggestion {
                component: "Container".into(),
                rule: "rect.fallback".into(),
                fit: None,
                reason: format!(
                    "{w:.0}×{h:.0} matched no component rule — a container is the safe reading."
                ),
            }
        }
        other => ShapeSuggestion {
            component: "Unknown".into(),
            rule: "unrecognised_tool".into(),
            fit: None,
            reason: format!("No rule covers a `{other}` shape."),
        },
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("dm-{:x}", t)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_arrow() -> Annotation {
        Annotation {
            annotation_id: "a1".into(),
            kind: AnnotationKind::Arrow {
                from_label: "Button".into(),
                to_label: "Header".into(),
                label: "move up".into(),
            },
            priority: 1,
            created_at_ms: 0,
        }
    }

    fn make_region() -> Annotation {
        Annotation {
            annotation_id: "a2".into(),
            kind: AnnotationKind::Region {
                x: 10,
                y: 20,
                width: 100,
                height: 50,
                description: "hero section".into(),
            },
            priority: 2,
            created_at_ms: 0,
        }
    }

    fn make_text_label() -> Annotation {
        Annotation {
            annotation_id: "a3".into(),
            kind: AnnotationKind::TextLabel {
                x: 5,
                y: 15,
                text: "Submit".into(),
            },
            priority: 3,
            created_at_ms: 0,
        }
    }

    fn make_before_after() -> Annotation {
        Annotation {
            annotation_id: "a4".into(),
            kind: AnnotationKind::BeforeAfter {
                before_url: "http://old.png".into(),
                after_url: "http://new.png".into(),
            },
            priority: 4,
            created_at_ms: 0,
        }
    }

    fn make_color_swatch() -> Annotation {
        Annotation {
            annotation_id: "a5".into(),
            kind: AnnotationKind::ColorSwatch {
                hex: "#ff0000".into(),
                label: "primary".into(),
            },
            priority: 5,
            created_at_ms: 0,
        }
    }

    fn make_measurement() -> Annotation {
        Annotation {
            annotation_id: "a6".into(),
            kind: AnnotationKind::Measurement {
                from_label: "Title".into(),
                to_label: "Body".into(),
                expected_value: "16px".into(),
            },
            priority: 1,
            created_at_ms: 0,
        }
    }

    // ── annotation_to_instruction ──────────────────────────────────────────

    #[test]
    fn test_arrow_instruction() {
        let ann = make_arrow();
        let s = annotation_to_instruction(&ann);
        assert_eq!(s, "Move Button to align with Header: move up");
    }

    #[test]
    fn test_arrow_instruction_contains_from() {
        let ann = make_arrow();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("Button"));
    }

    #[test]
    fn test_arrow_instruction_contains_to() {
        let ann = make_arrow();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("Header"));
    }

    #[test]
    fn test_arrow_instruction_contains_label() {
        let ann = make_arrow();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("move up"));
    }

    #[test]
    fn test_region_instruction() {
        let ann = make_region();
        let s = annotation_to_instruction(&ann);
        assert_eq!(s, "Update the region at (10,20) size 100x50: hero section");
    }

    #[test]
    fn test_region_instruction_contains_coords() {
        let ann = make_region();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("10"));
        assert!(s.contains("20"));
    }

    #[test]
    fn test_region_instruction_contains_size() {
        let ann = make_region();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("100x50"));
    }

    #[test]
    fn test_region_instruction_contains_description() {
        let ann = make_region();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("hero section"));
    }

    #[test]
    fn test_text_label_instruction() {
        let ann = make_text_label();
        let s = annotation_to_instruction(&ann);
        assert_eq!(s, "Change text at (5,15) to: Submit");
    }

    #[test]
    fn test_text_label_instruction_contains_coords() {
        let ann = make_text_label();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("5"));
        assert!(s.contains("15"));
    }

    #[test]
    fn test_text_label_instruction_contains_text() {
        let ann = make_text_label();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("Submit"));
    }

    #[test]
    fn test_before_after_instruction() {
        let ann = make_before_after();
        let s = annotation_to_instruction(&ann);
        assert_eq!(
            s,
            "Apply before/after change: before=http://old.png after=http://new.png"
        );
    }

    #[test]
    fn test_before_after_instruction_contains_before_url() {
        let ann = make_before_after();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("http://old.png"));
    }

    #[test]
    fn test_before_after_instruction_contains_after_url() {
        let ann = make_before_after();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("http://new.png"));
    }

    #[test]
    fn test_color_swatch_instruction() {
        let ann = make_color_swatch();
        let s = annotation_to_instruction(&ann);
        assert_eq!(s, "Use color #ff0000 for primary");
    }

    #[test]
    fn test_color_swatch_instruction_contains_hex() {
        let ann = make_color_swatch();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("#ff0000"));
    }

    #[test]
    fn test_color_swatch_instruction_contains_label() {
        let ann = make_color_swatch();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("primary"));
    }

    #[test]
    fn test_measurement_instruction() {
        let ann = make_measurement();
        let s = annotation_to_instruction(&ann);
        assert_eq!(s, "Set distance from Title to Body to 16px");
    }

    #[test]
    fn test_measurement_instruction_contains_from() {
        let ann = make_measurement();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("Title"));
    }

    #[test]
    fn test_measurement_instruction_contains_to() {
        let ann = make_measurement();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("Body"));
    }

    #[test]
    fn test_measurement_instruction_contains_value() {
        let ann = make_measurement();
        let s = annotation_to_instruction(&ann);
        assert!(s.contains("16px"));
    }

    // ── ChangeSpec ─────────────────────────────────────────────────────────

    #[test]
    fn test_change_spec_new_empty() {
        let spec = ChangeSpec::new();
        assert_eq!(spec.annotation_count(), 0);
    }

    #[test]
    fn test_change_spec_add_one() {
        let mut spec = ChangeSpec::new();
        spec.add(make_arrow());
        assert_eq!(spec.annotation_count(), 1);
    }

    #[test]
    fn test_change_spec_add_multiple() {
        let mut spec = ChangeSpec::new();
        spec.add(make_arrow());
        spec.add(make_region());
        spec.add(make_text_label());
        assert_eq!(spec.annotation_count(), 3);
    }

    #[test]
    fn test_change_spec_sort_by_priority() {
        let mut spec = ChangeSpec::new();
        // Add in reverse priority order
        spec.add(make_color_swatch()); // priority 5
        spec.add(make_before_after()); // priority 4
        spec.add(make_text_label()); // priority 3
        spec.add(make_region()); // priority 2
        spec.add(make_arrow()); // priority 1
        let instructions = spec.to_instructions();
        // First instruction should be for priority 1 (Arrow)
        assert!(instructions[0].contains("Button"));
        // Second should be for priority 2 (Region)
        assert!(instructions[1].contains("hero section"));
    }

    #[test]
    fn test_change_spec_sort_priority_1_first() {
        let mut spec = ChangeSpec::new();
        spec.add(make_measurement()); // priority 1
        spec.add(make_color_swatch()); // priority 5
        let instructions = spec.to_instructions();
        assert!(instructions[0].contains("Title"));
    }

    #[test]
    fn test_change_spec_to_instructions_count() {
        let mut spec = ChangeSpec::new();
        spec.add(make_arrow());
        spec.add(make_region());
        let instrs = spec.to_instructions();
        assert_eq!(instrs.len(), 2);
    }

    #[test]
    fn test_change_spec_to_markdown_header() {
        let spec = ChangeSpec::new();
        let md = spec.to_markdown();
        assert!(md.starts_with("# Change Spec\n\n"));
    }

    #[test]
    fn test_change_spec_to_markdown_numbered_list() {
        let mut spec = ChangeSpec::new();
        spec.add(make_arrow());
        spec.add(make_region());
        let md = spec.to_markdown();
        assert!(md.contains("1. "));
        assert!(md.contains("2. "));
    }

    #[test]
    fn test_change_spec_to_markdown_empty() {
        let spec = ChangeSpec::new();
        let md = spec.to_markdown();
        assert_eq!(md, "# Change Spec\n\n");
    }

    #[test]
    fn test_change_spec_to_markdown_contains_instruction() {
        let mut spec = ChangeSpec::new();
        spec.add(make_arrow());
        let md = spec.to_markdown();
        assert!(md.contains("Button"));
        assert!(md.contains("Header"));
    }

    #[test]
    fn test_change_spec_spec_id_not_empty() {
        let spec = ChangeSpec::new();
        assert!(!spec.spec_id.is_empty());
    }

    // ── DesignTokenExtractor ───────────────────────────────────────────────

    #[test]
    fn test_extractor_new_empty() {
        let ext = DesignTokenExtractor::new();
        assert_eq!(ext.all_tokens().len(), 0);
    }

    #[test]
    fn test_extractor_parses_6digit_hex() {
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css("--primary-color: #ff0000;");
        assert_eq!(ext.all_tokens().len(), 1);
        assert_eq!(ext.all_tokens()[0].var_name, "--primary-color");
        assert_eq!(ext.all_tokens()[0].hex_value, "#ff0000");
    }

    #[test]
    fn test_extractor_parses_3digit_hex() {
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css("--accent: #f0a;");
        assert_eq!(ext.all_tokens().len(), 1);
        assert_eq!(ext.all_tokens()[0].hex_value, "#f0a");
    }

    #[test]
    fn test_extractor_parses_8digit_hex() {
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css("--overlay: #00000080;");
        assert_eq!(ext.all_tokens().len(), 1);
        assert_eq!(ext.all_tokens()[0].hex_value, "#00000080");
    }

    #[test]
    fn test_extractor_ignores_non_hex() {
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css("--spacing: 16px;");
        assert_eq!(ext.all_tokens().len(), 0);
    }

    #[test]
    fn test_extractor_parses_multiple_lines() {
        let css = "--color-a: #aabbcc;\n--color-b: #112233;";
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css(css);
        assert_eq!(ext.all_tokens().len(), 2);
    }

    #[test]
    fn test_extractor_no_duplicates() {
        let css = "--primary: #ff0000;\n--primary: #ff0000;";
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css(css);
        assert_eq!(ext.all_tokens().len(), 1);
    }

    #[test]
    fn test_find_for_hex_exact_match() {
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css("--main: #abcdef;");
        let tok = ext.find_for_hex("#abcdef");
        assert!(tok.is_some());
        assert_eq!(tok.unwrap().var_name, "--main");
    }

    #[test]
    fn test_find_for_hex_case_insensitive() {
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css("--main: #ABCDEF;");
        let tok = ext.find_for_hex("#abcdef");
        assert!(tok.is_some());
    }

    #[test]
    fn test_find_for_hex_case_insensitive_upper_needle() {
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css("--main: #abcdef;");
        let tok = ext.find_for_hex("#ABCDEF");
        assert!(tok.is_some());
    }

    #[test]
    fn test_find_for_hex_not_found() {
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css("--main: #abcdef;");
        let tok = ext.find_for_hex("#000000");
        assert!(tok.is_none());
    }

    #[test]
    fn test_extractor_ignores_non_var() {
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css("color: #ff0000;");
        // "color" doesn't start with "--", so it should not be extracted
        assert_eq!(ext.all_tokens().len(), 0);
    }

    fn shape(kind: &str, w: f64, h: f64) -> SketchShape {
        SketchShape {
            kind: kind.into(),
            width: w,
            height: h,
            text: None,
        }
    }

    #[test]
    fn wide_short_rect_reads_as_a_button() {
        let s = suggest_component(&shape("rect", 120.0, 40.0));
        assert_eq!(s.component, "Button");
        assert_eq!(s.rule, "rect.button");
        assert!(s.fit.is_some());
    }

    #[test]
    fn very_wide_short_rect_reads_as_an_input() {
        let s = suggest_component(&shape("rect", 400.0, 36.0));
        assert_eq!(s.component, "Input");
    }

    #[test]
    fn tall_rect_reads_as_a_card() {
        let s = suggest_component(&shape("rect", 240.0, 180.0));
        assert_eq!(s.component, "Card");
    }

    #[test]
    fn hairline_rect_reads_as_a_divider() {
        let s = suggest_component(&shape("rect", 300.0, 2.0));
        assert_eq!(s.component, "Divider");
    }

    #[test]
    fn small_circle_reads_as_an_avatar() {
        let s = suggest_component(&shape("circle", 48.0, 48.0));
        assert_eq!(s.component, "Avatar");
    }

    #[test]
    fn fallback_rule_reports_no_fit_rather_than_inventing_one() {
        // Taller than any card band, and far too narrow for one. A number
        // here would be a measurement nobody took.
        let s = suggest_component(&shape("rect", 40.0, 600.0));
        assert_eq!(s.rule, "rect.fallback");
        assert!(s.fit.is_none(), "a catch-all rule must not report a fit");
        // A page-sized frame is also past the card band, not a big card.
        let s = suggest_component(&shape("rect", 900.0, 600.0));
        assert_eq!(s.rule, "rect.fallback");
        assert!(s.fit.is_none());
    }

    #[test]
    fn band_fit_peaks_at_the_midpoint_and_vanishes_at_the_edges() {
        assert_eq!(band_fit(5.0, 0.0, 10.0), Some(1.0));
        assert_eq!(band_fit(0.0, 0.0, 10.0), Some(0.0));
        assert_eq!(band_fit(10.0, 0.0, 10.0), Some(0.0));
        assert_eq!(band_fit(11.0, 0.0, 10.0), None);
    }

    #[test]
    fn unknown_tool_is_reported_as_unknown_not_guessed() {
        let s = suggest_component(&shape("spiral", 10.0, 10.0));
        assert_eq!(s.component, "Unknown");
        assert!(s.fit.is_none());
    }

    #[test]
    fn css_vars_parse_minified_single_line() {
        // A line-based scan finds one of these; a declaration-based scan
        // finds all three, which is what a generated sheet looks like.
        let vars = extract_css_variables(":root{--a:#fff;--b:8px;--c:1.5}");
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0].name, "--a");
        assert_eq!(vars[1].value, "8px");
    }

    #[test]
    fn css_vars_skip_commented_out_declarations() {
        let vars = extract_css_variables(":root{ /* --dead: #000; */ --live: #fff; }");
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "--live");
    }

    #[test]
    fn css_vars_first_definition_wins() {
        let vars = extract_css_variables("--x: #111;\n--x: #222;");
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].value, "#111");
    }

    #[test]
    fn css_vars_categorise_by_name_before_unit() {
        // Identical values, different meanings — the name is what separates them.
        assert_eq!(
            categorize_css_value("--font-size-md", "14px"),
            DesignTokenType::Typography
        );
        assert_eq!(
            categorize_css_value("--space-md", "14px"),
            DesignTokenType::Spacing
        );
        assert_eq!(
            categorize_css_value("--radius-sm", "4px"),
            DesignTokenType::BorderRadius
        );
    }

    #[test]
    fn css_vars_categorise_colors_by_value() {
        for value in ["#6366f1", "oklch(0.7 0.1 250)", "rgba(0,0,0,.5)"] {
            assert_eq!(
                categorize_css_value("--brand", value),
                DesignTokenType::Color,
                "{value}"
            );
        }
    }

    #[test]
    fn css_vars_unknown_shape_is_other_not_a_guess() {
        // Nothing about `--grid-template: auto 1fr` says spacing, colour or
        // motion, so it must not be filed under one.
        assert_eq!(
            categorize_css_value("--grid-template", "auto 1fr"),
            DesignTokenType::Other
        );
        assert_eq!(
            categorize_css_value("--opacity-muted", "0.6"),
            DesignTokenType::Other
        );
    }

    #[test]
    fn css_vars_reject_selector_fragments() {
        // `a--b: hover` is not a custom property; only a leading `--` is.
        let vars = extract_css_variables("a--b: hover; --real: 1px;");
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "--real");
    }

    #[test]
    fn test_extractor_mixed_css() {
        let css = ":root {\n  --primary: #123456;\n  font-size: 16px;\n  --secondary: #654321;\n}";
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css(css);
        assert_eq!(ext.all_tokens().len(), 2);
    }
}
