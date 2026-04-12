#![allow(dead_code)]
//! Design mode — visual annotation, change spec, and design token extraction.

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
        AnnotationKind::Arrow { from_label, to_label, label } => {
            format!("Move {} to align with {}: {}", from_label, to_label, label)
        }
        AnnotationKind::Region { x, y, width, height, description } => {
            format!(
                "Update the region at ({},{}) size {}x{}: {}",
                x, y, width, height, description
            )
        }
        AnnotationKind::TextLabel { x, y, text } => {
            format!("Change text at ({},{}) to: {}", x, y, text)
        }
        AnnotationKind::BeforeAfter { before_url, after_url } => {
            format!(
                "Apply before/after change: before={} after={}",
                before_url, after_url
            )
        }
        AnnotationKind::ColorSwatch { hex, label } => {
            format!("Use color {} for {}", hex, label)
        }
        AnnotationKind::Measurement { from_label, to_label, expected_value } => {
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
        sorted.iter().map(|a| annotation_to_instruction(a)).collect()
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

    /// Parses CSS for `--var-name: #hexcolor` patterns and stores them.
    pub fn extract_from_css(&mut self, css: &str) {
        for line in css.lines() {
            let line = line.trim();
            // Look for patterns like `--some-var: #abc123`
            if let Some(colon_pos) = line.find(':') {
                let var_part = line[..colon_pos].trim();
                let val_part = line[colon_pos + 1..].trim().trim_end_matches(';').trim();
                if var_part.starts_with("--") && val_part.starts_with('#') {
                    // Validate it looks like a hex color (3, 4, 6, or 8 hex digits after #)
                    let hex_digits: String = val_part[1..]
                        .chars()
                        .take_while(|c| c.is_ascii_hexdigit())
                        .collect();
                    let len = hex_digits.len();
                    if len == 3 || len == 4 || len == 6 || len == 8 {
                        let hex_value = format!("#{}", hex_digits);
                        // Avoid duplicates by var_name
                        if !self.tokens.iter().any(|t| t.var_name == var_part) {
                            self.tokens.push(DesignTokenRef {
                                var_name: var_part.to_string(),
                                hex_value,
                                usage_context: String::new(),
                            });
                        }
                    }
                }
            }
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
        spec.add(make_text_label());   // priority 3
        spec.add(make_region());       // priority 2
        spec.add(make_arrow());        // priority 1
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

    #[test]
    fn test_extractor_mixed_css() {
        let css = ":root {\n  --primary: #123456;\n  font-size: 16px;\n  --secondary: #654321;\n}";
        let mut ext = DesignTokenExtractor::new();
        ext.extract_from_css(css);
        assert_eq!(ext.all_tokens().len(), 2);
    }
}
