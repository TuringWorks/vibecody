#![allow(dead_code)]
//! Auto-stub generator — generate test stubs and mock implementations from
//! function signatures and trait definitions.
//!
//! Matches Devin 2.0's automated test stub generator.

use std::fmt::Write;

// ---------------------------------------------------------------------------
// Signature types
// ---------------------------------------------------------------------------

/// A parameter in a function signature.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_: String,
    /// Whether this param is mutable.
    pub mutable: bool,
}

impl Param {
    pub fn new(name: impl Into<String>, type_: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_: type_.into(),
            mutable: false,
        }
    }

    pub fn mutable(mut self) -> Self {
        self.mutable = true;
        self
    }
}

/// A function signature to generate a stub for.
#[derive(Debug, Clone)]
pub struct FnSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_pub: bool,
    /// Whether the function is a method (takes &self or &mut self).
    pub is_method: bool,
    /// Whether the first receiver is &mut self.
    pub mut_self: bool,
    pub doc_comment: Option<String>,
}

impl FnSignature {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: vec![],
            return_type: None,
            is_async: false,
            is_pub: true,
            is_method: false,
            mut_self: false,
            doc_comment: None,
        }
    }

    pub fn with_params(mut self, params: Vec<Param>) -> Self {
        self.params = params;
        self
    }

    pub fn with_return(mut self, ret: impl Into<String>) -> Self {
        self.return_type = Some(ret.into());
        self
    }

    pub fn async_(mut self) -> Self {
        self.is_async = true;
        self
    }

    pub fn method(mut self, mut_self: bool) -> Self {
        self.is_method = true;
        self.mut_self = mut_self;
        self
    }

    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc_comment = Some(doc.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Trait definition
// ---------------------------------------------------------------------------

/// A trait definition to generate a mock implementation for.
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<FnSignature>,
    pub generic_params: Vec<String>,
}

impl TraitDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            methods: vec![],
            generic_params: vec![],
        }
    }

    pub fn with_method(mut self, method: FnSignature) -> Self {
        self.methods.push(method);
        self
    }
}

// ---------------------------------------------------------------------------
// Generated stub
// ---------------------------------------------------------------------------

/// A generated test stub or mock.
#[derive(Debug, Clone)]
pub struct GeneratedStub {
    pub kind: StubKind,
    pub source: String,
    pub function_name: String,
    pub language: StubLanguage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StubKind {
    /// `#[test] fn test_xxx() { todo!() }`
    TestFunction,
    /// A struct + impl that satisfies a trait with unimplemented! bodies.
    MockImpl,
    /// A spy that records calls.
    SpyImpl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StubLanguage {
    Rust,
    TypeScript,
}

// ---------------------------------------------------------------------------
// Stub generator
// ---------------------------------------------------------------------------

pub struct StubGenerator {
    pub language: StubLanguage,
    /// Indent string (default: 4 spaces).
    pub indent: String,
}

impl StubGenerator {
    pub fn rust() -> Self {
        Self {
            language: StubLanguage::Rust,
            indent: "    ".into(),
        }
    }

    pub fn typescript() -> Self {
        Self {
            language: StubLanguage::TypeScript,
            indent: "  ".into(),
        }
    }

    /// Generate a test stub for a function.
    pub fn generate_test_stub(&self, sig: &FnSignature) -> GeneratedStub {
        let source = match self.language {
            StubLanguage::Rust => self.rust_test_stub(sig),
            StubLanguage::TypeScript => self.ts_test_stub(sig),
        };
        GeneratedStub {
            kind: StubKind::TestFunction,
            source,
            function_name: sig.name.clone(),
            language: self.language.clone(),
        }
    }

    /// Generate a mock implementation of a trait.
    pub fn generate_mock(&self, trait_def: &TraitDef, struct_name: &str) -> GeneratedStub {
        let source = match self.language {
            StubLanguage::Rust => self.rust_mock(trait_def, struct_name),
            StubLanguage::TypeScript => self.ts_mock(trait_def, struct_name),
        };
        GeneratedStub {
            kind: StubKind::MockImpl,
            source,
            function_name: format!("Mock{}", trait_def.name),
            language: self.language.clone(),
        }
    }

    /// Generate stubs for multiple functions at once.
    pub fn generate_all(&self, sigs: &[FnSignature]) -> Vec<GeneratedStub> {
        sigs.iter()
            .map(|s| self.generate_test_stub(s))
            .collect()
    }

    // ---- Rust generators ----

    fn rust_test_stub(&self, sig: &FnSignature) -> String {
        let mut out = String::new();
        if let Some(doc) = &sig.doc_comment {
            let _ = writeln!(out, "    /// {doc}");
        }
        let _ = writeln!(out, "    #[test]");
        let async_kw = if sig.is_async { "async " } else { "" };
        let _ = writeln!(out, "    {async_kw}fn test_{}() {{", sig.name);
        // Arrange — generate let bindings for each non-self param.
        for param in &sig.params {
            let default = rust_default_value(&param.type_);
            let _ = writeln!(out, "        let {} = {default};", param.name);
        }
        // Act
        let args = sig
            .params
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        if sig.is_method {
            let _ = writeln!(out, "        let result = subject.{}({args});", sig.name);
        } else {
            let _ = writeln!(out, "        let result = {}({args});", sig.name);
        }
        // Assert
        if let Some(ret) = &sig.return_type {
            let expected = rust_default_value(ret);
            let _ = writeln!(out, "        assert_eq!(result, {expected});");
        }
        let _ = writeln!(out, "    }}");
        out
    }

    fn rust_mock(&self, trait_def: &TraitDef, struct_name: &str) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "#[derive(Default)]");
        let _ = writeln!(out, "pub struct {struct_name} {{");
        let _ = writeln!(out, "    // TODO: add spy fields");
        let _ = writeln!(out, "}}");
        let _ = writeln!(out);
        let _ = writeln!(out, "impl {} for {struct_name} {{", trait_def.name);
        for method in &trait_def.methods {
            let receiver = if method.mut_self {
                "&mut self"
            } else {
                "&self"
            };
            let params_str = method
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.type_))
                .collect::<Vec<_>>()
                .join(", ");
            let params_full = if params_str.is_empty() {
                receiver.to_string()
            } else {
                format!("{receiver}, {params_str}")
            };
            let ret = method
                .return_type
                .as_deref()
                .map(|r| format!(" -> {r}"))
                .unwrap_or_default();
            let async_kw = if method.is_async { "async " } else { "" };
            let _ = writeln!(out, "    {async_kw}fn {}({params_full}){ret} {{", method.name);
            let _ = writeln!(out, "        unimplemented!(\"TODO: implement mock\")");
            let _ = writeln!(out, "    }}");
        }
        let _ = writeln!(out, "}}");
        out
    }

    // ---- TypeScript generators ----

    fn ts_test_stub(&self, sig: &FnSignature) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "it('{}', () => {{", sig.name);
        for param in &sig.params {
            let default = ts_default_value(&param.type_);
            let _ = writeln!(out, "  const {} = {default};", param.name);
        }
        let args = sig
            .params
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "  const result = {}({args});", sig.name);
        if let Some(ret) = &sig.return_type {
            let expected = ts_default_value(ret);
            let _ = writeln!(out, "  expect(result).toEqual({expected});");
        }
        let _ = writeln!(out, "}});");
        out
    }

    fn ts_mock(&self, trait_def: &TraitDef, struct_name: &str) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "class {struct_name} implements {} {{", trait_def.name);
        for method in &trait_def.methods {
            let params_str = method
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.type_))
                .collect::<Vec<_>>()
                .join(", ");
            let ret = method
                .return_type
                .as_deref()
                .map(|r| format!(": {r}"))
                .unwrap_or_default();
            let async_kw = if method.is_async { "async " } else { "" };
            let _ = writeln!(out, "  {async_kw}{}({params_str}){ret} {{", method.name);
            let _ = writeln!(out, "    throw new Error('TODO: implement mock');");
            let _ = writeln!(out, "  }}");
        }
        let _ = writeln!(out, "}}");
        out
    }
}

fn rust_default_value(type_: &str) -> &'static str {
    match type_.trim() {
        "String" | "&str" => "String::new()",
        "bool" => "false",
        "usize" | "u8" | "u16" | "u32" | "u64" | "u128" => "0",
        "isize" | "i8" | "i16" | "i32" | "i64" | "i128" => "0",
        "f32" | "f64" => "0.0",
        _ if type_.starts_with("Vec<") => "vec![]",
        _ if type_.starts_with("Option<") => "None",
        _ if type_.starts_with("Result<") => "Ok(Default::default())",
        _ if type_.starts_with("HashMap<") || type_.starts_with("HashSet<") => {
            "Default::default()"
        }
        _ => "Default::default()",
    }
}

fn ts_default_value(type_: &str) -> &'static str {
    match type_.trim() {
        "string" | "String" => "''",
        "number" | "Number" => "0",
        "boolean" | "Boolean" => "false",
        _ if type_.ends_with("[]") => "[]",
        _ => "undefined",
    }
}

// ---------------------------------------------------------------------------
// Stub batch (multiple stubs for a file)
// ---------------------------------------------------------------------------

/// A collection of stubs to be written to a test file.
#[derive(Debug, Clone)]
pub struct StubBatch {
    pub source_file: String,
    pub stubs: Vec<GeneratedStub>,
    pub language: StubLanguage,
}

impl StubBatch {
    pub fn render(&self) -> String {
        match self.language {
            StubLanguage::Rust => {
                let header = format!(
                    "// Auto-generated test stubs for {}\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n",
                    self.source_file
                );
                let body: String = self.stubs.iter().map(|s| s.source.clone()).collect();
                format!("{header}{body}\n}}")
            }
            StubLanguage::TypeScript => {
                let header = format!(
                    "// Auto-generated test stubs for {}\nimport {{ {} }} from '{}';\n\n",
                    self.source_file,
                    "/* TODO: add imports */",
                    self.source_file.trim_end_matches(".ts")
                );
                let body: String = self.stubs.iter().map(|s| s.source.clone()).collect();
                format!("{header}{body}")
            }
        }
    }

    pub fn stub_count(&self) -> usize {
        self.stubs.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_fn(name: &str) -> FnSignature {
        FnSignature::new(name)
            .with_params(vec![Param::new("x", "i32"), Param::new("y", "i32")])
            .with_return("i32")
    }

    #[test]
    fn test_rust_test_stub_contains_fn_name() {
        let gen = StubGenerator::rust();
        let stub = gen.generate_test_stub(&simple_fn("add"));
        assert!(stub.source.contains("test_add"));
        assert!(stub.source.contains("#[test]"));
    }

    #[test]
    fn test_rust_test_stub_has_assert() {
        let gen = StubGenerator::rust();
        let stub = gen.generate_test_stub(&simple_fn("multiply"));
        assert!(stub.source.contains("assert_eq!"));
    }

    #[test]
    fn test_rust_test_stub_kind() {
        let gen = StubGenerator::rust();
        let stub = gen.generate_test_stub(&simple_fn("foo"));
        assert_eq!(stub.kind, StubKind::TestFunction);
    }

    #[test]
    fn test_rust_mock_contains_impl() {
        let gen = StubGenerator::rust();
        let trait_def = TraitDef::new("Provider")
            .with_method(FnSignature::new("send").method(false).with_return("bool"));
        let stub = gen.generate_mock(&trait_def, "MockProvider");
        assert!(stub.source.contains("impl Provider for MockProvider"));
        assert!(stub.source.contains("unimplemented!"));
    }

    #[test]
    fn test_rust_mock_async_method() {
        let gen = StubGenerator::rust();
        let trait_def = TraitDef::new("AsyncTrait")
            .with_method(FnSignature::new("fetch").async_().method(false));
        let stub = gen.generate_mock(&trait_def, "MockAsync");
        assert!(stub.source.contains("async fn fetch"));
    }

    #[test]
    fn test_ts_test_stub() {
        let gen = StubGenerator::typescript();
        let sig = FnSignature::new("greet")
            .with_params(vec![Param::new("name", "string")])
            .with_return("string");
        let stub = gen.generate_test_stub(&sig);
        assert!(stub.source.contains("it('greet'"));
        assert!(stub.source.contains("expect(result)"));
    }

    #[test]
    fn test_ts_mock_contains_class() {
        let gen = StubGenerator::typescript();
        let trait_def = TraitDef::new("IStorage")
            .with_method(FnSignature::new("get").with_params(vec![Param::new("key", "string")]));
        let stub = gen.generate_mock(&trait_def, "MockStorage");
        assert!(stub.source.contains("class MockStorage implements IStorage"));
    }

    #[test]
    fn test_generate_all_returns_multiple() {
        let gen = StubGenerator::rust();
        let sigs = vec![simple_fn("a"), simple_fn("b"), simple_fn("c")];
        let stubs = gen.generate_all(&sigs);
        assert_eq!(stubs.len(), 3);
    }

    #[test]
    fn test_stub_batch_render_rust() {
        let gen = StubGenerator::rust();
        let stubs = gen.generate_all(&[simple_fn("foo")]);
        let batch = StubBatch {
            source_file: "src/lib.rs".into(),
            stubs,
            language: StubLanguage::Rust,
        };
        let rendered = batch.render();
        assert!(rendered.contains("#[cfg(test)]"));
        assert!(rendered.contains("mod tests"));
    }

    #[test]
    fn test_stub_batch_count() {
        let gen = StubGenerator::rust();
        let batch = StubBatch {
            source_file: "src/lib.rs".into(),
            stubs: gen.generate_all(&[simple_fn("a"), simple_fn("b")]),
            language: StubLanguage::Rust,
        };
        assert_eq!(batch.stub_count(), 2);
    }

    #[test]
    fn test_rust_default_values() {
        assert_eq!(rust_default_value("String"), "String::new()");
        assert_eq!(rust_default_value("bool"), "false");
        assert_eq!(rust_default_value("Vec<i32>"), "vec![]");
        assert_eq!(rust_default_value("Option<String>"), "None");
    }

    #[test]
    fn test_ts_default_values() {
        assert_eq!(ts_default_value("string"), "''");
        assert_eq!(ts_default_value("number"), "0");
        assert_eq!(ts_default_value("boolean"), "false");
    }

    #[test]
    fn test_async_stub_contains_async() {
        let gen = StubGenerator::rust();
        let sig = FnSignature::new("fetch_data").async_().with_return("String");
        let stub = gen.generate_test_stub(&sig);
        assert!(stub.source.contains("async fn test_fetch_data"));
    }
}
