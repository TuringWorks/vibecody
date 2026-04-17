//! Spec-to-test generator — BDD spec → test stub generator.
//! FIT-GAP v11 Phase 47 — closes gap vs Copilot Workspace v2, Devin 2.0.

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Language target for generated test stubs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestLanguage {
    Rust,
    TypeScript,
    Python,
}

impl TestLanguage {
    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "rs"  => Some(Self::Rust),
            "ts" | "tsx" => Some(Self::TypeScript),
            "py"  => Some(Self::Python),
            _ => None,
        }
    }

    pub fn file_ext(&self) -> &'static str {
        match self { Self::Rust => "rs", Self::TypeScript => "ts", Self::Python => "py" }
    }
}

/// A parsed Gherkin scenario.
#[derive(Debug, Clone)]
pub struct GherkinScenario {
    pub title: String,
    pub given: Vec<String>,
    pub when: Vec<String>,
    pub then: Vec<String>,
}

impl GherkinScenario {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), given: vec![], when: vec![], then: vec![] }
    }

    pub fn with_given(mut self, s: impl Into<String>) -> Self { self.given.push(s.into()); self }
    pub fn with_when(mut self, s: impl Into<String>) -> Self  { self.when.push(s.into()); self }
    pub fn with_then(mut self, s: impl Into<String>) -> Self  { self.then.push(s.into()); self }
}

/// A parsed Gherkin feature.
#[derive(Debug, Clone)]
pub struct GherkinFeature {
    pub name: String,
    pub description: Option<String>,
    pub scenarios: Vec<GherkinScenario>,
}

impl GherkinFeature {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), description: None, scenarios: vec![] }
    }

    pub fn with_scenario(mut self, s: GherkinScenario) -> Self {
        self.scenarios.push(s);
        self
    }
}

/// A generated test stub file.
#[derive(Debug, Clone)]
pub struct TestStub {
    pub filename: String,
    pub language: TestLanguage,
    pub content: String,
    pub scenario_count: usize,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Minimal Gherkin feature file parser (line-based, no full grammar).
pub struct GherkinParser;

impl GherkinParser {
    pub fn parse(text: &str) -> GherkinFeature {
        let mut feature = GherkinFeature::new("Unknown");
        let mut current: Option<GherkinScenario> = None;

        for raw in text.lines() {
            let line = raw.trim();
            if let Some(rest) = line.strip_prefix("Feature:") {
                feature.name = rest.trim().to_string();
            } else if line.starts_with("Scenario:") || line.starts_with("Scenario Outline:") {
                if let Some(sc) = current.take() { feature.scenarios.push(sc); }
                let title = if let Some(rest) = line.strip_prefix("Scenario Outline:") {
                    rest.trim().to_string()
                } else {
                    line["Scenario:".len()..].trim().to_string()
                };
                current = Some(GherkinScenario::new(title));
            } else if let Some(sc) = current.as_mut() {
                if let Some(rest) = line.strip_prefix("Given") {
                    sc.given.push(rest.trim().to_string());
                } else if let Some(rest) = line.strip_prefix("When") {
                    sc.when.push(rest.trim().to_string());
                } else if line.starts_with("Then") || line.starts_with("And") {
                    sc.then.push(line[line.find(' ').map(|i| i+1).unwrap_or(line.len())..].trim().to_string());
                }
            }
        }
        if let Some(sc) = current { feature.scenarios.push(sc); }
        feature
    }
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

fn to_snake(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Generates test stub files from Gherkin features.
pub struct SpecToTestGenerator {
    pub language: TestLanguage,
}

impl SpecToTestGenerator {
    pub fn new(language: TestLanguage) -> Self { Self { language } }

    pub fn generate(&self, feature: &GherkinFeature) -> TestStub {
        let content = match self.language {
            TestLanguage::Rust       => self.gen_rust(feature),
            TestLanguage::TypeScript => self.gen_typescript(feature),
            TestLanguage::Python     => self.gen_python(feature),
        };
        let mod_name = to_snake(&feature.name);
        let filename = format!("{}_spec.{}", mod_name, self.language.file_ext());
        TestStub {
            filename,
            language: self.language.clone(),
            content,
            scenario_count: feature.scenarios.len(),
        }
    }

    fn gen_rust(&self, feature: &GherkinFeature) -> String {
        let mut out = format!("//! Generated stubs for: {}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n", feature.name);
        for sc in &feature.scenarios {
            let fn_name = to_snake(&sc.title);
            out.push_str(&format!("    #[test]\n    fn test_{}() {{\n", fn_name));
            for g in &sc.given { out.push_str(&format!("        // Given {}\n", g)); }
            for w in &sc.when  { out.push_str(&format!("        // When {}\n", w)); }
            for t in &sc.then  { out.push_str(&format!("        // Then {}\n", t)); }
            out.push_str("        todo!(\"implement test\")\n    }\n\n");
        }
        out.push_str("}\n");
        out
    }

    fn gen_typescript(&self, feature: &GherkinFeature) -> String {
        let mut out = format!("// Generated stubs for: {}\nimport {{ describe, it }} from 'vitest';\n\n", feature.name);
        out.push_str(&format!("describe('{}', () => {{\n", feature.name));
        for sc in &feature.scenarios {
            out.push_str(&format!("  it('{}', () => {{\n", sc.title));
            for g in &sc.given { out.push_str(&format!("    // Given {}\n", g)); }
            for w in &sc.when  { out.push_str(&format!("    // When {}\n", w)); }
            for t in &sc.then  { out.push_str(&format!("    // Then {}\n", t)); }
            out.push_str("    throw new Error('not implemented');\n  });\n\n");
        }
        out.push_str("});\n");
        out
    }

    fn gen_python(&self, feature: &GherkinFeature) -> String {
        let mut out = format!("# Generated stubs for: {}\nimport pytest\n\n", feature.name);
        for sc in &feature.scenarios {
            let fn_name = to_snake(&sc.title);
            out.push_str(&format!("def test_{}():\n", fn_name));
            for g in &sc.given { out.push_str(&format!("    # Given {}\n", g)); }
            for w in &sc.when  { out.push_str(&format!("    # When {}\n", w)); }
            for t in &sc.then  { out.push_str(&format!("    # Then {}\n", t)); }
            out.push_str("    raise NotImplementedError\n\n");
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_feature() -> GherkinFeature {
        GherkinFeature::new("User Login")
            .with_scenario(
                GherkinScenario::new("Successful login")
                    .with_given("a registered user")
                    .with_when("they submit valid credentials")
                    .with_then("they are authenticated"),
            )
            .with_scenario(
                GherkinScenario::new("Failed login")
                    .with_given("a registered user")
                    .with_when("they submit wrong password")
                    .with_then("authentication fails"),
            )
    }

    #[test]
    fn test_to_snake_basic() {
        assert_eq!(to_snake("Successful login"), "successful_login");
        assert_eq!(to_snake("Failed login!"), "failed_login");
    }

    #[test]
    fn test_rust_stub_contains_test_fn() {
        let gen = SpecToTestGenerator::new(TestLanguage::Rust);
        let stub = gen.generate(&sample_feature());
        assert!(stub.content.contains("#[test]"));
        assert!(stub.content.contains("fn test_successful_login"));
        assert!(stub.content.contains("fn test_failed_login"));
    }

    #[test]
    fn test_rust_stub_contains_todo() {
        let gen = SpecToTestGenerator::new(TestLanguage::Rust);
        let stub = gen.generate(&sample_feature());
        assert!(stub.content.contains("todo!"));
    }

    #[test]
    fn test_typescript_stub_contains_describe() {
        let gen = SpecToTestGenerator::new(TestLanguage::TypeScript);
        let stub = gen.generate(&sample_feature());
        assert!(stub.content.contains("describe("));
        assert!(stub.content.contains("it("));
    }

    #[test]
    fn test_python_stub_contains_def() {
        let gen = SpecToTestGenerator::new(TestLanguage::Python);
        let stub = gen.generate(&sample_feature());
        assert!(stub.content.contains("def test_successful_login"));
        assert!(stub.content.contains("NotImplementedError"));
    }

    #[test]
    fn test_scenario_count_matches() {
        let gen = SpecToTestGenerator::new(TestLanguage::Rust);
        let stub = gen.generate(&sample_feature());
        assert_eq!(stub.scenario_count, 2);
    }

    #[test]
    fn test_filename_uses_feature_name() {
        let gen = SpecToTestGenerator::new(TestLanguage::Rust);
        let stub = gen.generate(&sample_feature());
        assert_eq!(stub.filename, "user_login_spec.rs");
    }

    #[test]
    fn test_gherkin_parser_feature_name() {
        let text = "Feature: Shopping Cart\n  Scenario: Add item\n    Given an empty cart\n    When I add an item\n    Then cart has 1 item\n";
        let f = GherkinParser::parse(text);
        assert_eq!(f.name, "Shopping Cart");
        assert_eq!(f.scenarios.len(), 1);
    }

    #[test]
    fn test_gherkin_parser_scenario_steps() {
        let text = "Feature: Login\n  Scenario: Valid credentials\n    Given a user exists\n    When they log in\n    Then they are authenticated\n";
        let f = GherkinParser::parse(text);
        let sc = &f.scenarios[0];
        assert_eq!(sc.given, vec!["a user exists"]);
        assert_eq!(sc.when,  vec!["they log in"]);
        assert_eq!(sc.then,  vec!["they are authenticated"]);
    }

    #[test]
    fn test_gherkin_parser_multiple_scenarios() {
        let text = "Feature: F\n  Scenario: S1\n    Given g1\n  Scenario: S2\n    Given g2\n";
        let f = GherkinParser::parse(text);
        assert_eq!(f.scenarios.len(), 2);
    }

    #[test]
    fn test_test_language_from_ext() {
        assert_eq!(TestLanguage::from_ext("rs"), Some(TestLanguage::Rust));
        assert_eq!(TestLanguage::from_ext("ts"), Some(TestLanguage::TypeScript));
        assert_eq!(TestLanguage::from_ext("py"), Some(TestLanguage::Python));
        assert_eq!(TestLanguage::from_ext("go"), None);
    }

    #[test]
    fn test_stub_content_includes_given_comments() {
        let gen = SpecToTestGenerator::new(TestLanguage::Rust);
        let stub = gen.generate(&sample_feature());
        assert!(stub.content.contains("// Given a registered user"));
    }
}
