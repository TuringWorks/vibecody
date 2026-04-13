/*!
 * BDD tests for design_providers using Cucumber.
 * Run with: cargo test --test design_providers_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::design_providers::{
    DesignError, DiagramDoc, DiagramFormat, DiagramKind, DesignToken, DesignTokenType,
    DesignProviderRegistry, ProviderKind, tokens_to_css,
};

#[derive(Debug, Default, World)]
pub struct DpWorld {
    provider_kind: Option<ProviderKind>,
    diagram_format: Option<DiagramFormat>,
    diagram_kind: Option<DiagramKind>,
    diagram_doc: Option<DiagramDoc>,
    tokens: Vec<DesignToken>,
    css_output: String,
    design_error: Option<DesignError>,
    registry: Option<DesignProviderRegistry>,
}

// ── Given ──────────────────────────────────────────────────────────────────

#[given(expr = "a provider kind {string}")]
fn given_provider_kind(world: &mut DpWorld, kind: String) {
    world.provider_kind = Some(match kind.as_str() {
        "figma" => ProviderKind::Figma,
        "penpot" => ProviderKind::Penpot,
        "pencil" => ProviderKind::Pencil,
        "draw_io" => ProviderKind::DrawIo,
        "mermaid" => ProviderKind::Mermaid,
        "plant_uml" => ProviderKind::PlantUml,
        "c4_model" => ProviderKind::C4Model,
        _ => ProviderKind::Inhouse,
    });
}

#[given(expr = "a diagram format {string}")]
fn given_diagram_format(world: &mut DpWorld, fmt: String) {
    world.diagram_format = Some(match fmt.as_str() {
        "mermaid_md" => DiagramFormat::MermaidMd,
        "draw_io_xml" => DiagramFormat::DrawIoXml,
        "plant_uml" => DiagramFormat::PlantUml,
        "c4_dsl" => DiagramFormat::C4Dsl,
        "svg_markup" => DiagramFormat::SvgMarkup,
        "png_bytes" => DiagramFormat::PngBytes,
        _ => DiagramFormat::Json,
    });
}

#[given(expr = "a diagram kind {string}")]
fn given_diagram_kind(world: &mut DpWorld, kind: String) {
    world.diagram_kind = Some(parse_diagram_kind(&kind));
}

#[given(expr = "I create a DiagramDoc titled {string} of kind {string} with content {string}")]
fn given_create_diagram_doc(world: &mut DpWorld, title: String, kind: String, content: String) {
    world.diagram_doc = Some(DiagramDoc::new(&title, parse_diagram_kind(&kind), content, ProviderKind::Mermaid));
}

#[given(expr = "a color token named {string} with value {string}")]
fn given_color_token(world: &mut DpWorld, name: String, value: String) {
    world.tokens.push(DesignToken {
        name,
        token_type: DesignTokenType::Color,
        value,
        description: None,
        provider: ProviderKind::Inhouse,
    });
}

#[given("a fresh provider registry")]
fn given_fresh_registry(world: &mut DpWorld) {
    world.registry = Some(DesignProviderRegistry::new());
}

#[given(expr = "a design error with code {string} and message {string}")]
fn given_design_error(world: &mut DpWorld, code: String, message: String) {
    world.design_error = Some(DesignError::new(&code, &message));
}

// ── When ───────────────────────────────────────────────────────────────────

#[when("I export tokens to CSS")]
fn when_export_css(world: &mut DpWorld) {
    world.css_output = tokens_to_css(&world.tokens);
}

// ── Then ───────────────────────────────────────────────────────────────────

#[then(expr = "its display name should be {string}")]
fn then_display_name(world: &mut DpWorld, expected: String) {
    let name = world.provider_kind.as_ref().unwrap().display_name();
    assert_eq!(name, expected.as_str());
}

#[then("it should support editing")]
fn then_supports_editing(world: &mut DpWorld) {
    assert!(world.provider_kind.as_ref().unwrap().supports_editing());
}

#[then(expr = "the file extension should be {string}")]
fn then_file_ext(world: &mut DpWorld, expected: String) {
    let ext = world.diagram_format.as_ref().unwrap().file_extension();
    assert_eq!(ext, expected.as_str());
}

#[then(expr = "the preferred format should be {string}")]
fn then_preferred_format(world: &mut DpWorld, expected: String) {
    let fmt = world.diagram_kind.as_ref().unwrap().preferred_format();
    let fmt_str = format!("{:?}", fmt).to_lowercase();
    // normalise: "c4dsl" or "c4_dsl"
    let e = expected.replace('_', "");
    let f = fmt_str.replace('_', "");
    assert_eq!(f, e, "Expected preferred format {expected} but got {fmt_str}");
}

#[then(expr = "the doc id should start with {string}")]
fn then_doc_id_starts(world: &mut DpWorld, prefix: String) {
    let doc = world.diagram_doc.as_ref().unwrap();
    assert!(doc.id.starts_with(&prefix), "id was: {}", doc.id);
}

#[then(expr = "the doc title should be {string}")]
fn then_doc_title(world: &mut DpWorld, expected: String) {
    assert_eq!(world.diagram_doc.as_ref().unwrap().title, expected);
}

#[then(expr = "the CSS should contain {string}")]
fn then_css_contains(world: &mut DpWorld, expected: String) {
    assert!(world.css_output.contains(&expected), "CSS missing: {expected}\nCSS: {}", world.css_output);
}

#[then("the available providers list should be empty")]
fn then_registry_empty(world: &mut DpWorld) {
    assert!(world.registry.as_ref().unwrap().available().is_empty());
}

#[then(expr = "the error string should equal {string}")]
fn then_error_string(world: &mut DpWorld, expected: String) {
    let err = world.design_error.as_ref().unwrap();
    assert_eq!(err.to_string(), expected);
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn parse_diagram_kind(s: &str) -> DiagramKind {
    match s {
        "flowchart" => DiagramKind::Flowchart,
        "sequence" => DiagramKind::Sequence,
        "class_diagram" => DiagramKind::ClassDiagram,
        "entity_relationship" => DiagramKind::EntityRelationship,
        "c4_context" => DiagramKind::C4Context,
        "c4_container" => DiagramKind::C4Container,
        "c4_component" => DiagramKind::C4Component,
        "architecture" => DiagramKind::Architecture,
        "state_machine" => DiagramKind::StateMachine,
        _ => DiagramKind::Flowchart,
    }
}

fn main() {
    futures::executor::block_on(DpWorld::run("tests/features/design_providers.feature"));
}
