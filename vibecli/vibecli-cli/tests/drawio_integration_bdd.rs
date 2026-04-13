/*!
 * BDD tests for drawio_connector using Cucumber.
 * Run with: cargo test --test drawio_integration_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::drawio_connector::{
    DrawioCell, DrawioGraph, DrawioMcpCommand, C4Person, C4Relation, C4System,
    parse_drawio_xml, template_c4_context, template_flowchart, parse_llm_flowchart,
};

#[derive(Debug, Default, World)]
pub struct DioWorld {
    cell: Option<DrawioCell>,
    cell_xml: String,
    graph: Option<DrawioGraph>,
    graph_xml: String,
    drawio_file: String,
    parse_result: Option<Result<vibecli_cli::drawio_connector::ParsedDrawio, vibecli_cli::design_providers::DesignError>>,
    mcp_cmd: Option<DrawioMcpCommand>,
    mcp_json: String,
    llm_output: String,
}

// ── Given ──────────────────────────────────────────────────────────────────

#[given(expr = "a vertex cell with id {string} value {string} at position {int},{int} size {int}x{int}")]
fn given_vertex_cell(world: &mut DioWorld, id: String, value: String, x: i32, y: i32, w: i32, h: i32) {
    world.cell = Some(DrawioCell::vertex(&id, &value, "rounded=1;", x as f64, y as f64, w as f64, h as f64));
}

#[given(expr = "an edge cell from {string} to {string} labeled {string}")]
fn given_edge_cell(world: &mut DioWorld, src: String, tgt: String, label: String) {
    world.cell = Some(DrawioCell::edge("e1", &label, &src, &tgt, "edgeStyle=orthogonalEdgeStyle;"));
}

#[given(expr = "an empty DrawioGraph named {string}")]
fn given_empty_graph(world: &mut DioWorld, name: String) {
    world.graph = Some(DrawioGraph::new(&name));
}

#[given(expr = "a flowchart with steps {string} and {string} and {string}")]
fn given_flowchart_steps(world: &mut DioWorld, s1: String, s2: String, s3: String) {
    let refs: Vec<&str> = vec![s1.as_str(), s2.as_str(), s3.as_str()];
    world.graph = Some(template_flowchart("Test", &refs));
}

#[given("a valid draw.io XML string with 2 vertices and 1 edge")]
fn given_valid_xml(world: &mut DioWorld) {
    let xml = r#"<mxfile><diagram name="Page-1" id="abc">
<mxGraphModel><root>
  <mxCell id="0" /><mxCell id="1" parent="0" />
  <mxCell id="2" value="A" style="r" vertex="1" parent="1"><mxGeometry x="0" y="0" width="100" height="40" as="geometry" /></mxCell>
  <mxCell id="3" value="B" style="r" vertex="1" parent="1"><mxGeometry x="200" y="0" width="100" height="40" as="geometry" /></mxCell>
  <mxCell id="4" value="calls" style="" edge="1" source="2" target="3" parent="1"><mxGeometry relative="1" as="geometry" /></mxCell>
</root></mxGraphModel></diagram></mxfile>"#;
    world.parse_result = Some(parse_drawio_xml(xml));
}

#[given("an empty XML string")]
fn given_empty_xml(world: &mut DioWorld) {
    world.parse_result = Some(parse_drawio_xml("  "));
}

#[given(expr = "a drawio MCP command to read file {string}")]
fn given_mcp_cmd(world: &mut DioWorld, path: String) {
    world.mcp_cmd = Some(DrawioMcpCommand::read_file(&path));
}

#[given("a C4 context with 1 person and 1 system and 1 relation")]
fn given_c4_context(world: &mut DioWorld) {
    let persons = vec![C4Person { id: "p1".into(), name: "User".into(), description: "End user".into(), external: false }];
    let systems = vec![C4System { id: "s1".into(), name: "Backend".into(), description: "API".into(), external: false, containers: vec![] }];
    let rels = vec![C4Relation { from_id: "p1".into(), to_id: "s1".into(), label: "uses".into(), technology: None }];
    world.graph = Some(template_c4_context("Context", &persons, &systems, &rels));
}

#[given(expr = "LLM output {string}")]
fn given_llm_output(world: &mut DioWorld, output: String) {
    world.llm_output = output;
}

// ── When ───────────────────────────────────────────────────────────────────

#[when("I render the cell to XML")]
fn when_render_cell(world: &mut DioWorld) {
    if let Some(cell) = &world.cell {
        // Access inner XML via the graph approach
        let mut g = DrawioGraph::new("tmp");
        g.add_cell(cell.clone());
        world.cell_xml = g.to_xml();
    }
}

#[when("I render to XML")]
fn when_render_graph(world: &mut DioWorld) {
    world.graph_xml = world.graph.as_ref().unwrap().to_xml();
}

#[when("I render to drawio file format")]
fn when_render_drawio_file(world: &mut DioWorld) {
    world.drawio_file = world.graph.as_ref().unwrap().to_drawio_file();
}

#[when("I generate the flowchart template")]
fn when_gen_flowchart(_world: &mut DioWorld) { /* already generated in given */ }

#[when("I parse the XML")]
fn when_parse_xml(_world: &mut DioWorld) { /* parse result already stored in given */ }

#[when("I serialise to JSON")]
fn when_serialise_mcp(world: &mut DioWorld) {
    world.mcp_json = world.mcp_cmd.as_ref().unwrap().to_json();
}

#[when("I generate the C4 context template")]
fn when_gen_c4(_world: &mut DioWorld) { /* already generated */ }

#[when("I parse as LLM flowchart")]
fn when_parse_llm(world: &mut DioWorld) {
    world.graph = Some(parse_llm_flowchart(&world.llm_output.clone()));
}

// ── Then ───────────────────────────────────────────────────────────────────

#[then(expr = "the XML should contain id {string}")]
fn then_xml_has_id(world: &mut DioWorld, id: String) {
    let xml = if !world.cell_xml.is_empty() { &world.cell_xml } else { &world.graph_xml };
    assert!(xml.contains(&format!("id=\"{}\"", id)), "XML missing id={id}\n{}", xml);
}

#[then(expr = "the XML should contain value {string}")]
fn then_xml_has_value(world: &mut DioWorld, value: String) {
    let xml = if !world.cell_xml.is_empty() { &world.cell_xml } else { &world.graph_xml };
    assert!(xml.contains(&format!("value=\"{}\"", value)));
}

#[then(expr = "the XML should contain {string}")]
fn then_xml_contains(world: &mut DioWorld, s: String) {
    let xml = if !world.cell_xml.is_empty() { &world.cell_xml } else { &world.graph_xml };
    assert!(xml.contains(s.as_str()), "XML missing: {s}");
}

#[then(expr = "the XML should contain source {string}")]
fn then_xml_source(world: &mut DioWorld, src: String) {
    assert!(world.cell_xml.contains(&format!("source=\"{}\"", src)));
}

#[then(expr = "the XML should contain target {string}")]
fn then_xml_target(world: &mut DioWorld, tgt: String) {
    assert!(world.cell_xml.contains(&format!("target=\"{}\"", tgt)));
}

#[then(expr = "the output should contain {string}")]
fn then_output_contains(world: &mut DioWorld, s: String) {
    assert!(world.drawio_file.contains(s.as_str()), "Output missing: {s}");
}

#[then(expr = "the graph should have {int} vertices")]
fn then_vertex_count(world: &mut DioWorld, count: usize) {
    let g = world.graph.as_ref().unwrap();
    assert_eq!(g.vertex_count(), count, "Expected {} vertices, got {}", count, g.vertex_count());
}

#[then(expr = "the graph should have {int} edges")]
fn then_edge_count(world: &mut DioWorld, count: usize) {
    let g = world.graph.as_ref().unwrap();
    assert_eq!(g.edge_count(), count);
}

#[then(expr = "the parse result should have {int} page")]
fn then_parse_pages(world: &mut DioWorld, count: usize) {
    let r = world.parse_result.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(r.pages.len(), count);
}

#[then(expr = "the total vertex count should be {int}")]
fn then_total_vertices(world: &mut DioWorld, count: usize) {
    let r = world.parse_result.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(r.total_vertices, count);
}

#[then(expr = "the total edge count should be {int}")]
fn then_total_edges(world: &mut DioWorld, count: usize) {
    let r = world.parse_result.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(r.total_edges, count);
}

#[then("a design error should be returned")]
fn then_design_error(world: &mut DioWorld) {
    assert!(world.parse_result.as_ref().unwrap().is_err());
}

#[then(expr = "the JSON should contain {string}")]
fn then_json_contains(world: &mut DioWorld, s: String) {
    assert!(world.mcp_json.contains(s.as_str()), "JSON missing: {s}");
}

#[then(expr = "the flowchart graph should have at least {int} vertices")]
fn then_flowchart_min_vertices(world: &mut DioWorld, min: usize) {
    let g = world.graph.as_ref().unwrap();
    assert!(g.vertex_count() >= min, "Expected >= {} vertices, got {}", min, g.vertex_count());
}

fn main() {
    futures::executor::block_on(DioWorld::run("tests/features/drawio_integration.feature"));
}
