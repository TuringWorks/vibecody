/*!
 * BDD tests for pencil_connector using Cucumber.
 * Run with: cargo test --test pencil_integration_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::pencil_connector::{
    parse_ep_xml, template_dashboard, template_landing_page, template_mobile_app,
    PencilDocument, PencilMcpOp, PencilPage, PencilShape, PencilShapeKind, PencilStyle,
};

#[derive(Debug, Default, World)]
pub struct PencilWorld {
    document: Option<PencilDocument>,
    ep_xml: String,
    parse_result: Option<Result<PencilDocument, vibecli_cli::design_providers::DesignError>>,
    mcp_op: Option<PencilMcpOp>,
    mcp_json: String,
    design_file: Option<vibecli_cli::design_providers::DesignFile>,
}

// ── Given ──────────────────────────────────────────────────────────────────

#[given(expr = "a PencilDocument named {string} with one page {string} of size {int}x{int}")]
fn given_doc_with_page(world: &mut PencilWorld, name: String, page: String, w: u32, h: u32) {
    let mut doc = PencilDocument::new(&name);
    doc.add_page(PencilPage::new(&page, w as f64, h as f64));
    world.document = Some(doc);
}

#[given(expr = "a valid EP XML string with document {string} and page {string}")]
fn given_valid_ep_xml(world: &mut PencilWorld, doc_name: String, page_name: String) {
    let xml = format!(
        r#"<?xml version="1.0"?><Document name="{}" id="d1">
  <Page name="{}" id="p1" width="1280" height="800"></Page>
</Document>"#,
        doc_name, page_name
    );
    world.parse_result = Some(parse_ep_xml(&xml));
}

#[given("an empty string")]
fn given_empty_string(world: &mut PencilWorld) {
    world.parse_result = Some(parse_ep_xml(""));
}

#[given("a valid EP XML with one rectangle shape")]
fn given_ep_xml_with_shape(world: &mut PencilWorld) {
    let xml = r#"<?xml version="1.0"?><Document name="D" id="d1">
  <Page name="P1" id="p1" width="1280" height="800">
    <Shape id="s1" type="rectangle" x="10" y="20" width="100" height="40"></Shape>
  </Page>
</Document>"#;
    world.parse_result = Some(parse_ep_xml(xml));
}

#[given(expr = "I generate a landing page template titled {string}")]
fn given_landing_page(world: &mut PencilWorld, title: String) {
    world.document = Some(template_landing_page(&title));
}

#[given(expr = "I generate a dashboard template with sections {string} and {string}")]
fn given_dashboard(world: &mut PencilWorld, s1: String, s2: String) {
    world.document = Some(template_dashboard("Dashboard", &[s1.as_str(), s2.as_str()]));
}

#[given(expr = "I generate a mobile app with screens {string} and {string} and {string}")]
fn given_mobile_app(world: &mut PencilWorld, s1: String, s2: String, s3: String) {
    world.document = Some(template_mobile_app("MyApp", &[s1.as_str(), s2.as_str(), s3.as_str()]));
}

#[given("a PencilDocument with 2 pages")]
fn given_doc_2_pages(world: &mut PencilWorld) {
    let mut doc = PencilDocument::new("D");
    doc.add_page(PencilPage::new("P1", 390.0, 844.0));
    doc.add_page(PencilPage::new("P2", 1440.0, 900.0));
    world.document = Some(doc);
}

#[given("a PencilDocument with a shape having fill color \"#3b82f6\"")]
fn given_doc_with_color_shape(world: &mut PencilWorld) {
    let mut doc = PencilDocument::new("D");
    let mut page = PencilPage::new("P1", 1280.0, 800.0);
    page.add_shape(PencilShape {
        id: "s1".into(), kind: PencilShapeKind::Rectangle,
        x: 0.0, y: 0.0, width: 100.0, height: 40.0, label: String::new(),
        style: PencilStyle { fill_color: Some("#3b82f6".into()), ..Default::default() },
        children: Vec::new(),
    });
    doc.add_page(page);
    world.document = Some(doc);
}

#[given("a Pencil MCP operation for get_editor_state")]
fn given_mcp_op(world: &mut PencilWorld) {
    world.mcp_op = Some(PencilMcpOp::get_editor_state());
}

// ── When ───────────────────────────────────────────────────────────────────

#[when("I serialise to EP XML")]
fn when_serialise_ep(world: &mut PencilWorld) {
    world.ep_xml = world.document.as_ref().unwrap().to_ep_xml();
}

#[when("I parse the EP XML")]
fn when_parse_ep(_world: &mut PencilWorld) { /* result set in given */ }

#[when("I convert to a DesignFile")]
fn when_convert_to_design_file(world: &mut PencilWorld) {
    world.design_file = Some(world.document.as_ref().unwrap().to_design_file());
}

#[when("I serialise to JSON")]
fn when_serialise_json(world: &mut PencilWorld) {
    world.mcp_json = world.mcp_op.as_ref().unwrap().to_json();
}

// ── Then ───────────────────────────────────────────────────────────────────

#[then(expr = "the XML should contain {string}")]
fn then_xml_contains(world: &mut PencilWorld, s: String) {
    assert!(world.ep_xml.contains(s.as_str()), "EP XML missing: {s}\n{}", world.ep_xml);
}

#[then(expr = "the document name should be {string}")]
fn then_doc_name(world: &mut PencilWorld, name: String) {
    let doc = world.parse_result.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(doc.name, name);
}

#[then(expr = "the page count should be {int}")]
fn then_page_count(world: &mut PencilWorld, count: usize) {
    let doc = world.parse_result.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(doc.pages.len(), count);
}

#[then("a design error should be returned")]
fn then_design_error(world: &mut PencilWorld) {
    assert!(world.parse_result.as_ref().unwrap().is_err());
}

#[then(expr = "the first page should have {int} shape")]
fn then_first_page_shapes(world: &mut PencilWorld, count: usize) {
    let doc = world.parse_result.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(doc.pages[0].shapes.len(), count);
}

#[then(expr = "the template should have {int} page")]
fn then_template_pages(world: &mut PencilWorld, count: usize) {
    assert_eq!(world.document.as_ref().unwrap().pages.len(), count);
}

#[then(expr = "the template should have {int} pages")]
fn then_template_pages_plural(world: &mut PencilWorld, count: usize) {
    assert_eq!(world.document.as_ref().unwrap().pages.len(), count);
}

#[then(expr = "the page should contain a shape with id {string}")]
fn then_page_has_shape(world: &mut PencilWorld, id: String) {
    let doc = world.document.as_ref().unwrap();
    assert!(!doc.pages.is_empty());
    let page = &doc.pages[0];
    assert!(
        page.shapes.iter().any(|s| s.id == id),
        "Page does not contain shape id={id}"
    );
}

#[then(expr = "the DesignFile should have {int} frames")]
fn then_design_file_frames(world: &mut PencilWorld, count: usize) {
    assert_eq!(world.design_file.as_ref().unwrap().frames.len(), count);
}

#[then(expr = "the DesignFile provider should be {string}")]
fn then_design_file_provider(world: &mut PencilWorld, provider: String) {
    let p = &world.design_file.as_ref().unwrap().provider;
    let p_str = format!("{:?}", p).to_lowercase();
    assert_eq!(p_str, provider, "Provider mismatch: got {p_str}");
}

#[then(expr = "the DesignFile should have at least {int} token")]
fn then_design_file_tokens(world: &mut PencilWorld, min: usize) {
    assert!(world.design_file.as_ref().unwrap().tokens.len() >= min);
}

#[then(expr = "the JSON should contain {string}")]
fn then_json_contains(world: &mut PencilWorld, s: String) {
    assert!(world.mcp_json.contains(s.as_str()), "JSON missing: {s}");
}

fn main() {
    futures::executor::block_on(PencilWorld::run("tests/features/pencil_integration.feature"));
}
