/*!
 * BDD tests for diagram_generator using Cucumber.
 * Run with: cargo test --test diagram_generator_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::diagram_generator::{
    build_system_prompt, build_user_prompt, post_process_diagram_output,
    make_mermaid_doc, C4Templates, DiagramRequest, MermaidTemplates, PlantUmlTemplates,
};
use vibecli_cli::design_providers::{DiagramFormat, DiagramKind};

#[derive(Debug, Default, World)]
pub struct DgWorld {
    request: Option<DiagramRequest>,
    system_prompt: String,
    user_prompt: String,
    raw_output: String,
    post_result: Option<Result<String, String>>,
    template_output: String,
    doc: Option<vibecli_cli::design_providers::DiagramDoc>,
}

// ── Given ──────────────────────────────────────────────────────────────────

#[given(expr = "a diagram request for kind {string} and format {string}")]
fn given_request_kind_format(world: &mut DgWorld, kind: String, fmt: String) {
    let k = parse_kind(&kind);
    let f = parse_format(&fmt);
    world.request = Some(DiagramRequest::new("test description", k).with_format(f));
}

#[given(expr = "a diagram request with description {string}")]
fn given_request_desc(world: &mut DgWorld, desc: String) {
    world.request = Some(DiagramRequest::new(&desc, DiagramKind::Sequence));
}

#[given(expr = "a diagram request for kind {string}")]
fn given_request_kind(world: &mut DgWorld, kind: String) {
    world.request = Some(DiagramRequest::new("test", parse_kind(&kind)));
}

#[given(expr = "raw LLM output {string}")]
fn given_raw_output(world: &mut DgWorld, raw: String) {
    world.raw_output = raw.replace("\\n", "\n");
}

#[given(expr = "a C4 context template for system {string}")]
fn given_c4_template(world: &mut DgWorld, system: String) {
    world.template_output = C4Templates::saas_context(&system);
}

#[given(expr = "a PlantUML component template for {string} with component {string} as {string}")]
fn given_plantuml_template(world: &mut DgWorld, system: String, comp: String, tech: String) {
    world.template_output = PlantUmlTemplates::component_diagram(&system, &[(comp.as_str(), tech.as_str())]);
}

#[given(expr = "I create a Mermaid diagram doc titled {string}")]
fn given_mermaid_doc(world: &mut DgWorld, title: String) {
    world.doc = Some(make_mermaid_doc(&title, DiagramKind::Flowchart, "flowchart TD\nA-->B"));
}

// ── When ───────────────────────────────────────────────────────────────────

#[when("I build the system prompt")]
fn when_system_prompt(world: &mut DgWorld) {
    let req = world.request.as_ref().unwrap();
    world.system_prompt = build_system_prompt(&req.kind, &req.format);
}

#[when("I build the user prompt")]
fn when_user_prompt(world: &mut DgWorld) {
    world.user_prompt = build_user_prompt(world.request.as_ref().unwrap());
}

#[when(expr = "I post-process for format {string}")]
fn when_post_process(world: &mut DgWorld, fmt: String) {
    let f = parse_format(&fmt);
    let result = post_process_diagram_output(&world.raw_output, &f);
    world.post_result = Some(result);
}

#[when("I get the microservices architecture Mermaid template")]
fn when_microservices(world: &mut DgWorld) {
    world.template_output = MermaidTemplates::microservices_architecture().to_string();
}

#[when("I get the ER diagram Mermaid template")]
fn when_er_template(world: &mut DgWorld) {
    world.template_output = MermaidTemplates::er_saas_schema().to_string();
}

#[when(expr = "I override format to {string}")]
fn when_override_format(world: &mut DgWorld, fmt: String) {
    let f = parse_format(&fmt);
    if let Some(req) = world.request.take() {
        world.request = Some(req.with_format(f));
    }
}

// ── Then ───────────────────────────────────────────────────────────────────

#[then(expr = "the prompt should contain {string}")]
fn then_prompt_contains(world: &mut DgWorld, s: String) {
    let text = if !world.system_prompt.is_empty() { &world.system_prompt } else { &world.user_prompt };
    assert!(text.contains(s.as_str()), "Prompt missing: {s}");
}

#[then("the result should be OK")]
fn then_result_ok(world: &mut DgWorld) {
    assert!(world.post_result.as_ref().unwrap().is_ok(), "Expected OK, got: {:?}", world.post_result);
}

#[then("the result should be an error")]
fn then_result_err(world: &mut DgWorld) {
    assert!(world.post_result.as_ref().unwrap().is_err());
}

#[then(expr = "the output should contain {string}")]
fn then_output_contains(world: &mut DgWorld, s: String) {
    let out = world.post_result.as_ref().unwrap().as_ref().unwrap();
    assert!(out.contains(s.as_str()), "Output missing: {s}");
}

#[then(expr = "the output should not contain {string}")]
fn then_output_not_contains(world: &mut DgWorld, s: String) {
    let out = world.post_result.as_ref().unwrap().as_ref().unwrap();
    assert!(!out.contains(s.as_str()), "Output should not contain: {s}");
}

#[then(expr = "the template should contain {string}")]
fn then_template_contains(world: &mut DgWorld, s: String) {
    assert!(world.template_output.contains(s.as_str()), "Template missing: {s}");
}

#[then(expr = "the DSL should contain {string}")]
fn then_dsl_contains(world: &mut DgWorld, s: String) {
    assert!(world.template_output.contains(s.as_str()), "DSL missing: {s}");
}

#[then(expr = "the doc provider should be {string}")]
fn then_doc_provider(world: &mut DgWorld, provider: String) {
    let p = format!("{:?}", world.doc.as_ref().unwrap().provider).to_lowercase();
    assert_eq!(p, provider);
}

#[then(expr = "the doc format should be {string}")]
fn then_doc_format(world: &mut DgWorld, fmt: String) {
    let f = format!("{:?}", world.doc.as_ref().unwrap().format).to_lowercase().replace('_', "");
    let e = fmt.replace('_', "");
    assert_eq!(f, e, "Format mismatch: {f} vs {e}");
}

#[then(expr = "the request format should be {string}")]
fn then_request_format(world: &mut DgWorld, fmt: String) {
    let f = format!("{:?}", world.request.as_ref().unwrap().format).to_lowercase().replace('_', "");
    let e = fmt.replace('_', "");
    assert_eq!(f, e);
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn parse_kind(s: &str) -> DiagramKind {
    match s {
        "flowchart" => DiagramKind::Flowchart,
        "sequence" => DiagramKind::Sequence,
        "class_diagram" => DiagramKind::ClassDiagram,
        "entity_relationship" => DiagramKind::EntityRelationship,
        "c4_context" => DiagramKind::C4Context,
        "architecture" => DiagramKind::Architecture,
        _ => DiagramKind::Flowchart,
    }
}

fn parse_format(s: &str) -> DiagramFormat {
    match s {
        "mermaid_md" => DiagramFormat::MermaidMd,
        "draw_io_xml" => DiagramFormat::DrawIoXml,
        "plant_uml" => DiagramFormat::PlantUml,
        "c4_dsl" => DiagramFormat::C4Dsl,
        _ => DiagramFormat::Json,
    }
}

fn main() {
    futures::executor::block_on(DgWorld::run("tests/features/diagram_generator.feature"));
}
