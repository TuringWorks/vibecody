//! Draw.io (diagrams.net) deep integration.
//!
//! Provides:
//! - XML parse/validate/transform for .drawio files
//! - Template library for architecture, flowchart, ERD, sequence, C4, UML
//! - MCP bridge commands for drawio-mcp (jgraph/drawio-mcp)
//! - SVG export / embed helpers
//! - AI-to-draw.io XML generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::design_providers::{DesignError, DiagramDoc, DiagramFormat, DiagramKind, ProviderKind};

// ─── Draw.io cell / graph types ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawioCell {
    pub id: String,
    pub value: String,
    pub style: String,
    pub vertex: bool,
    pub edge: bool,
    pub source: Option<String>,
    pub target: Option<String>,
    pub parent: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

impl DrawioCell {
    pub fn vertex(id: &str, value: &str, style: &str, x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            id: id.to_string(),
            value: value.to_string(),
            style: style.to_string(),
            vertex: true,
            edge: false,
            source: None,
            target: None,
            parent: "1".to_string(),
            x: Some(x),
            y: Some(y),
            width: Some(w),
            height: Some(h),
        }
    }

    pub fn edge(id: &str, label: &str, source: &str, target: &str, style: &str) -> Self {
        Self {
            id: id.to_string(),
            value: label.to_string(),
            style: style.to_string(),
            vertex: false,
            edge: true,
            source: Some(source.to_string()),
            target: Some(target.to_string()),
            parent: "1".to_string(),
            x: None,
            y: None,
            width: None,
            height: None,
        }
    }

    fn to_xml(&self) -> String {
        let mut attrs = format!(
            r#"id="{}" value="{}" style="{}" parent="{}""#,
            xml_escape(&self.id),
            xml_escape(&self.value),
            xml_escape(&self.style),
            xml_escape(&self.parent),
        );
        if self.vertex {
            attrs.push_str(" vertex=\"1\"");
        }
        if self.edge {
            attrs.push_str(" edge=\"1\"");
        }
        if let Some(s) = &self.source {
            attrs.push_str(&format!(" source=\"{}\"", xml_escape(s)));
        }
        if let Some(t) = &self.target {
            attrs.push_str(&format!(" target=\"{}\"", xml_escape(t)));
        }
        let geo = if self.vertex {
            format!(
                "\n      <mxGeometry x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" as=\"geometry\" />",
                self.x.unwrap_or(0.0),
                self.y.unwrap_or(0.0),
                self.width.unwrap_or(120.0),
                self.height.unwrap_or(40.0),
            )
        } else {
            "\n      <mxGeometry relative=\"1\" as=\"geometry\" />".to_string()
        };
        format!("    <mxCell {}>{}\n    </mxCell>", attrs, geo)
    }
}

// ─── DrawioGraph ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawioGraph {
    pub cells: Vec<DrawioCell>,
    pub page_title: String,
}

impl DrawioGraph {
    pub fn new(title: &str) -> Self {
        Self { cells: Vec::new(), page_title: title.to_string() }
    }

    pub fn add_cell(&mut self, cell: DrawioCell) {
        self.cells.push(cell);
    }

    /// Render to draw.io XML format
    pub fn to_xml(&self) -> String {
        let cells_xml: String = self.cells.iter().map(|c| c.to_xml()).collect::<Vec<_>>().join("\n");
        format!(
            r#"<mxGraphModel dx="1422" dy="762" grid="1" gridSize="10" guides="1" tooltips="1" connect="1" arrows="1" fold="1" page="1" pageScale="1" pageWidth="1169" pageHeight="827" math="0" shadow="0">
  <root>
    <mxCell id="0" />
    <mxCell id="1" parent="0" />
{}
  </root>
</mxGraphModel>"#,
            cells_xml
        )
    }

    /// Wrap in full .drawio XML envelope
    pub fn to_drawio_file(&self) -> String {
        let title = xml_escape(&self.page_title);
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<mxfile host="VibeCody" modified="{}" agent="VibeCody/1.0" version="21.0">
  <diagram name="{}" id="{}">
    {}
  </diagram>
</mxfile>"#,
            chrono_now_iso(),
            title,
            uuid_short(),
            self.to_xml()
        )
    }

    /// Count of vertex cells
    pub fn vertex_count(&self) -> usize {
        self.cells.iter().filter(|c| c.vertex).count()
    }

    /// Count of edge cells
    pub fn edge_count(&self) -> usize {
        self.cells.iter().filter(|c| c.edge).count()
    }
}

// ─── Parse draw.io XML ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDrawio {
    pub pages: Vec<DrawioPage>,
    pub total_cells: usize,
    pub total_vertices: usize,
    pub total_edges: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawioPage {
    pub name: String,
    pub id: String,
    pub cells: Vec<DrawioCellInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawioCellInfo {
    pub id: String,
    pub value: String,
    pub is_vertex: bool,
    pub is_edge: bool,
    pub source: Option<String>,
    pub target: Option<String>,
}

/// Parse a .drawio XML string and return structural info.
/// Does lightweight text-based extraction (no full XML parser dep needed).
pub fn parse_drawio_xml(xml: &str) -> Result<ParsedDrawio, DesignError> {
    if xml.trim().is_empty() {
        return Err(DesignError::new("EMPTY_XML", "Draw.io XML is empty"));
    }

    let mut pages = Vec::new();
    let mut total_cells = 0usize;
    let mut total_vertices = 0usize;
    let mut total_edges = 0usize;

    // Extract diagram blocks
    let diagram_chunks = split_between_tags(xml, "<diagram", "</diagram>");
    for chunk in &diagram_chunks {
        let name = extract_attr(chunk, "name").unwrap_or_else(|| "Page".to_string());
        let id = extract_attr(chunk, "id").unwrap_or_else(uuid_short);

        let mut cells = Vec::new();
        let cell_chunks = split_between_tags(chunk, "<mxCell", "/>");
        for cell_chunk in &cell_chunks {
            let cell_id = match extract_attr(cell_chunk, "id") {
                Some(v) => v,
                None => continue,
            };
            if cell_id == "0" || cell_id == "1" {
                continue;
            }
            let value = extract_attr(cell_chunk, "value").unwrap_or_default();
            let is_vertex = cell_chunk.contains("vertex=\"1\"");
            let is_edge = cell_chunk.contains("edge=\"1\"");
            let source = extract_attr(cell_chunk, "source");
            let target = extract_attr(cell_chunk, "target");

            total_cells += 1;
            if is_vertex { total_vertices += 1; }
            if is_edge { total_edges += 1; }

            cells.push(DrawioCellInfo { id: cell_id, value, is_vertex, is_edge, source, target });
        }
        pages.push(DrawioPage { name, id, cells });
    }

    if pages.is_empty() {
        // Try simple mxGraphModel without diagram wrapper
        let cell_chunks = split_between_tags(xml, "<mxCell", "/>");
        let mut cells = Vec::new();
        for cell_chunk in &cell_chunks {
            let cell_id = match extract_attr(cell_chunk, "id") {
                Some(v) => v,
                None => continue,
            };
            if cell_id == "0" || cell_id == "1" { continue; }
            let value = extract_attr(cell_chunk, "value").unwrap_or_default();
            let is_vertex = cell_chunk.contains("vertex=\"1\"");
            let is_edge = cell_chunk.contains("edge=\"1\"");
            let source = extract_attr(cell_chunk, "source");
            let target = extract_attr(cell_chunk, "target");
            total_cells += 1;
            if is_vertex { total_vertices += 1; }
            if is_edge { total_edges += 1; }
            cells.push(DrawioCellInfo { id: cell_id, value, is_vertex, is_edge, source, target });
        }
        if !cells.is_empty() {
            pages.push(DrawioPage { name: "Page-1".to_string(), id: uuid_short(), cells });
        }
    }

    Ok(ParsedDrawio { pages, total_cells, total_vertices, total_edges })
}

// ─── Template library ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramTemplate {
    pub id: String,
    pub name: String,
    pub kind: DiagramKind,
    pub description: String,
    pub xml: String,
}

/// Generate a draw.io XML flowchart from a list of steps
pub fn template_flowchart(title: &str, steps: &[&str]) -> DrawioGraph {
    let mut g = DrawioGraph::new(title);
    let box_style = "rounded=1;whiteSpace=wrap;html=1;fillColor=#dae8fc;strokeColor=#6c8ebf;";
    let diamond_style = "rhombus;whiteSpace=wrap;html=1;fillColor=#fff2cc;strokeColor=#d6b656;";
    let edge_style = "edgeStyle=orthogonalEdgeStyle;rounded=0;orthogonalLoop=1;jettySize=auto;exitX=0.5;exitY=1;exitDx=0;exitDy=0;";

    let mut prev_id: Option<String> = None;
    for (i, step) in steps.iter().enumerate() {
        let id = format!("step{}", i);
        let y = (i as f64) * 80.0 + 20.0;

        let is_decision = step.ends_with('?');
        let style = if is_decision { diamond_style } else { box_style };
        let (w, h) = if is_decision { (120.0, 60.0) } else { (160.0, 40.0) };

        g.add_cell(DrawioCell::vertex(&id, step, style, 200.0, y, w, h));

        if let Some(pid) = &prev_id {
            let eid = format!("edge{}", i);
            g.add_cell(DrawioCell::edge(&eid, "", pid, &id, edge_style));
        }
        prev_id = Some(id);
    }
    g
}

/// Generate a draw.io XML architecture diagram
pub fn template_architecture(
    title: &str,
    layers: &[(&str, Vec<&str>)], // (layer_name, [component_names])
) -> DrawioGraph {
    let mut g = DrawioGraph::new(title);
    let swimlane_style = "swimlane;startSize=30;fontStyle=1;fillColor=#f5f5f5;strokeColor=#666666;fontColor=#333333;";
    let comp_style = "rounded=1;whiteSpace=wrap;html=1;fillColor=#ffffff;strokeColor=#82b366;";

    for (layer_i, (layer_name, components)) in layers.iter().enumerate() {
        let layer_id = format!("layer{}", layer_i);
        let lx = (layer_i as f64) * 260.0 + 20.0;
        let lh = (components.len() as f64) * 60.0 + 60.0;
        g.add_cell(DrawioCell::vertex(&layer_id, layer_name, swimlane_style, lx, 20.0, 220.0, lh));

        for (comp_i, comp_name) in components.iter().enumerate() {
            let cid = format!("comp{}_{}", layer_i, comp_i);
            let cy = (comp_i as f64) * 60.0 + 40.0;
            g.add_cell(DrawioCell::vertex(&cid, comp_name, comp_style, 10.0, cy, 200.0, 40.0));
        }
    }
    g
}

/// Generate an ERD diagram
pub fn template_erd(
    title: &str,
    entities: &[(&str, Vec<(&str, &str)>)], // (entity_name, [(field_name, type)])
) -> DrawioGraph {
    let mut g = DrawioGraph::new(title);
    let header_style = "shape=table;startSize=30;container=1;collapsible=1;childLayout=tableLayout;fixedRows=1;rowLines=0;fontStyle=1;align=center;resizeLast=1;fontSize=14;";
    let row_style = "shape=tableRow;horizontal=0;startSize=0;swimlaneHead=0;swimlaneBody=0;fillColor=none;collapsible=0;dropTarget=0;points=[[0,0.5],[1,0.5]];portConstraint=eastwest;fontSize=12;top=0;left=0;right=0;bottom=1;";

    for (ent_i, (ent_name, fields)) in entities.iter().enumerate() {
        let eid = format!("ent{}", ent_i);
        let ex = (ent_i as f64 % 3.0) * 280.0 + 20.0;
        let ey = (ent_i as f64 / 3.0).floor() * 200.0 + 20.0;
        let eh = (fields.len() as f64) * 30.0 + 40.0;
        g.add_cell(DrawioCell::vertex(&eid, ent_name, header_style, ex, ey, 240.0, eh));

        for (fi, (fname, ftype)) in fields.iter().enumerate() {
            let fid = format!("field{}_{}", ent_i, fi);
            let label = format!("{}: {}", fname, ftype);
            g.add_cell(DrawioCell::vertex(&fid, &label, row_style, ex, ey + 30.0 + (fi as f64) * 30.0, 240.0, 30.0));
        }
    }
    g
}

/// Generate a sequence diagram in draw.io XML (swimlane-based)
pub fn template_sequence(
    title: &str,
    actors: &[&str],
    messages: &[(&str, &str, &str)], // (from, to, label)
) -> DrawioGraph {
    let mut g = DrawioGraph::new(title);
    let actor_style = "shape=mxgraph.flowchart.actor;fillColor=#dae8fc;strokeColor=#6c8ebf;";
    let lifeline_style = "endArrow=none;dashed=1;strokeColor=#666666;";
    let msg_style = "edgeStyle=elbowEdgeStyle;elbow=vertical;exitX=0.5;exitY=1;entryX=0.5;entryY=0;rounded=0;";

    let actor_map: HashMap<String, String> = actors
        .iter()
        .enumerate()
        .map(|(i, &a)| (a.to_string(), format!("actor{}", i)))
        .collect();

    for (i, &actor) in actors.iter().enumerate() {
        let aid = format!("actor{}", i);
        let ax = (i as f64) * 180.0 + 60.0;
        g.add_cell(DrawioCell::vertex(&aid, actor, actor_style, ax, 20.0, 80.0, 40.0));

        let llid = format!("lifeline{}", i);
        let lifeline_xml = format!(
            r#"    <mxCell id="{}" value="" style="{}" edge="1" source="{}" target="{}" parent="1">
      <mxGeometry relative="1" as="geometry">
        <Array as="points"><mxPoint x="{}" y="400" /></Array>
      </mxGeometry>
    </mxCell>"#,
            llid, lifeline_style, aid, aid,
            ax + 40.0
        );
        // Store as raw XML in metadata approach — simplified
        let _ = lifeline_xml;
        // Just add a long box as lifeline representation
        let llbox = format!("ll_box{}", i);
        g.add_cell(DrawioCell::vertex(&llbox, "", "strokeColor=#999999;fillColor=none;dashed=1;", ax + 35.0, 60.0, 10.0, 340.0));
    }

    let y_step = 40.0;
    for (mi, (from, to, label)) in messages.iter().enumerate() {
        let msg_id = format!("msg{}", mi);
        let my = 80.0 + (mi as f64) * y_step;
        let from_id = actor_map.get(*from).cloned().unwrap_or_else(|| "actor0".to_string());
        let to_id = actor_map.get(*to).cloned().unwrap_or_else(|| "actor0".to_string());
        let _ = (from_id, to_id, msg_style);
        // Use positioned edge
        let from_x = actors.iter().position(|&a| a == *from).unwrap_or(0) as f64 * 180.0 + 100.0;
        let to_x = actors.iter().position(|&a| a == *to).unwrap_or(0) as f64 * 180.0 + 100.0;
        g.add_cell(DrawioCell::vertex(
            &msg_id,
            &format!("→ {}", label),
            "text;html=1;align=center;verticalAlign=middle;resizable=0;points=[];",
            from_x.min(to_x),
            my - 12.0,
            (from_x - to_x).abs(),
            24.0,
        ));
    }
    g
}

// ─── C4 model diagrams ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4Person {
    pub id: String,
    pub name: String,
    pub description: String,
    pub external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4System {
    pub id: String,
    pub name: String,
    pub description: String,
    pub external: bool,
    pub containers: Vec<C4Container>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4Container {
    pub id: String,
    pub name: String,
    pub technology: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4Relation {
    pub from_id: String,
    pub to_id: String,
    pub label: String,
    pub technology: Option<String>,
}

pub fn template_c4_context(
    title: &str,
    persons: &[C4Person],
    systems: &[C4System],
    relations: &[C4Relation],
) -> DrawioGraph {
    let mut g = DrawioGraph::new(title);

    let person_style = "shape=mxgraph.c4.person2;whiteSpace=wrap;html=1;fillColor=#08427b;fontColor=#ffffff;strokeColor=#073b6f;";
    let ext_person_style = "shape=mxgraph.c4.person2;whiteSpace=wrap;html=1;fillColor=#999999;fontColor=#ffffff;strokeColor=#8a8a8a;";
    let system_style = "rounded=1;whiteSpace=wrap;html=1;fillColor=#1168bd;fontColor=#ffffff;strokeColor=#0e5ca8;arcSize=10;";
    let ext_system_style = "rounded=1;whiteSpace=wrap;html=1;fillColor=#999999;fontColor=#ffffff;strokeColor=#8a8a8a;arcSize=10;";
    let edge_style = "edgeStyle=orthogonalEdgeStyle;rounded=0;orthogonalLoop=1;jettySize=auto;";

    for (i, p) in persons.iter().enumerate() {
        let style = if p.external { ext_person_style } else { person_style };
        let label = format!("{}\n[Person]\n{}", p.name, p.description);
        g.add_cell(DrawioCell::vertex(&p.id, &label, style, (i as f64) * 180.0 + 20.0, 200.0, 160.0, 100.0));
    }
    for (i, s) in systems.iter().enumerate() {
        let style = if s.external { ext_system_style } else { system_style };
        let label = format!("{}\n[Software System]\n{}", s.name, s.description);
        g.add_cell(DrawioCell::vertex(&s.id, &label, style, (i as f64) * 200.0 + 60.0, 400.0, 180.0, 100.0));
    }
    for (i, r) in relations.iter().enumerate() {
        let eid = format!("rel{}", i);
        let tech = r.technology.as_deref().unwrap_or("");
        let label = if tech.is_empty() { r.label.clone() } else { format!("{}\n[{}]", r.label, tech) };
        g.add_cell(DrawioCell::edge(&eid, &label, &r.from_id, &r.to_id, edge_style));
    }
    g
}

pub fn template_c4_container(
    title: &str,
    system_name: &str,
    containers: &[C4Container],
    external_systems: &[C4System],
    relations: &[C4Relation],
) -> DrawioGraph {
    let mut g = DrawioGraph::new(title);
    let boundary_style = "points=[[0,0],[0.25,0],[0.5,0],[0.75,0],[1,0],[1,0.25],[1,0.5],[1,0.75],[1,1],[0.75,1],[0.5,1],[0.25,1],[0,1],[0,0.75],[0,0.5],[0,0.25]];shape=mxgraph.c4.system_boundary;whiteSpace=wrap;html=1;fillColor=none;strokeColor=#666666;dashed=1;";
    let cont_style = "rounded=1;whiteSpace=wrap;html=1;fillColor=#438dd5;fontColor=#ffffff;strokeColor=#3c7fc0;arcSize=10;";
    let ext_style = "rounded=1;whiteSpace=wrap;html=1;fillColor=#999999;fontColor=#ffffff;strokeColor=#8a8a8a;arcSize=10;";
    let edge_style = "edgeStyle=orthogonalEdgeStyle;rounded=0;";

    let bound_w = containers.len() as f64 * 200.0 + 40.0;
    g.add_cell(DrawioCell::vertex("sys_boundary", system_name, boundary_style, 40.0, 80.0, bound_w, 280.0));

    for (i, c) in containers.iter().enumerate() {
        let label = format!("{}\n[{}]\n{}", c.name, c.technology, c.description);
        g.add_cell(DrawioCell::vertex(&c.id, &label, cont_style, (i as f64) * 200.0 + 60.0, 140.0, 180.0, 100.0));
    }
    for (i, es) in external_systems.iter().enumerate() {
        let label = format!("{}\n[External]\n{}", es.name, es.description);
        let ex = (i as f64) * 200.0 + 60.0;
        g.add_cell(DrawioCell::vertex(&es.id, &label, ext_style, ex, 420.0, 180.0, 80.0));
    }
    for (i, r) in relations.iter().enumerate() {
        let eid = format!("rel{}", i);
        g.add_cell(DrawioCell::edge(&eid, &r.label, &r.from_id, &r.to_id, edge_style));
    }
    g
}

// ─── MCP bridge ───────────────────────────────────────────────────────────────

/// Command descriptor for drawio-mcp operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawioMcpCommand {
    pub method: String,
    pub params: serde_json::Value,
}

impl DrawioMcpCommand {
    pub fn read_file(path: &str) -> Self {
        Self {
            method: "drawio/read_file".to_string(),
            params: serde_json::json!({ "path": path }),
        }
    }

    pub fn write_file(path: &str, xml: &str) -> Self {
        Self {
            method: "drawio/write_file".to_string(),
            params: serde_json::json!({ "path": path, "content": xml }),
        }
    }

    pub fn export_svg(path: &str, output_path: &str) -> Self {
        Self {
            method: "drawio/export".to_string(),
            params: serde_json::json!({ "path": path, "format": "svg", "output": output_path }),
        }
    }

    pub fn list_pages(path: &str) -> Self {
        Self {
            method: "drawio/list_pages".to_string(),
            params: serde_json::json!({ "path": path }),
        }
    }

    pub fn get_page(path: &str, page_index: u32) -> Self {
        Self {
            method: "drawio/get_page".to_string(),
            params: serde_json::json!({ "path": path, "page": page_index }),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

// ─── SVG embed helper ─────────────────────────────────────────────────────────

/// Wrap draw.io XML in an HTML page that renders via viewer.diagrams.net
pub fn embed_drawio_html(xml: &str, width: u32, height: u32) -> String {
    let encoded = xml.replace('"', "&quot;").replace('<', "%3C").replace('>', "%3E");
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  body {{ margin: 0; background: #fff; }}
  iframe {{ border: none; width: 100%; height: {}px; }}
</style>
</head>
<body>
<iframe src="https://viewer.diagrams.net/?lightbox=1&highlight=0000ff&edit=_blank&layers=1&nav=1&title=Diagram#R{}" width="{}" height="{}"></iframe>
</body>
</html>"#,
        height, encoded, width, height
    )
}

// ─── DiagramDoc builder ───────────────────────────────────────────────────────

pub fn build_diagram_doc(title: &str, kind: DiagramKind, graph: DrawioGraph) -> DiagramDoc {
    DiagramDoc {
        id: format!("diag-{}", uuid_short()),
        title: title.to_string(),
        kind,
        format: DiagramFormat::DrawIoXml,
        content: graph.to_drawio_file(),
        provider: ProviderKind::DrawIo,
        created_at_ms: epoch_ms(),
        metadata: HashMap::new(),
    }
}

// ─── AI generation: parse LLM output to draw.io ──────────────────────────────

/// Parse an LLM-generated description into a flowchart graph.
/// Handles "A -> B -> C" notation and "A\nB\nC" step lists.
pub fn parse_llm_flowchart(text: &str) -> DrawioGraph {
    let steps: Vec<String> = if text.contains("->") || text.contains("→") {
        text.replace("→", "->")
            .split("->")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        text.lines()
            .map(|l| l.trim().trim_start_matches(['*', '-', '•', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '.', ')']))
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    };

    let refs: Vec<&str> = steps.iter().map(|s| s.as_str()).collect();
    template_flowchart("Generated Flowchart", &refs)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
}

fn extract_attr(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = xml.find(&pattern)? + pattern.len();
    let rest = &xml[start..];
    let end = rest.find('"')?;
    Some(xml_unescape(&rest[..end]))
}

fn xml_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&apos;", "'")
}

/// Split XML into chunks delimited by tag start and end.
fn split_between_tags(xml: &str, open: &str, close: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut remaining = xml;
    while let Some(start) = remaining.find(open) {
        let from = &remaining[start..];
        let end = from.find(close).map(|e| e + close.len()).unwrap_or(from.len());
        results.push(from[..end].to_string());
        remaining = &from[end..];
    }
    results
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:x}{:04x}", t.as_secs(), t.subsec_micros() & 0xffff)
}

fn epoch_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

fn chrono_now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        1970 + secs / 31_536_000,
        1 + (secs % 31_536_000) / 2_628_000,
        1 + (secs % 2_628_000) / 86_400,
        (secs % 86_400) / 3_600,
        (secs % 3_600) / 60,
        secs % 60,
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_vertex_xml_contains_id() {
        let c = DrawioCell::vertex("v1", "Hello", "rounded=1;", 10.0, 20.0, 120.0, 40.0);
        let xml = c.to_xml();
        assert!(xml.contains("id=\"v1\""));
        assert!(xml.contains("value=\"Hello\""));
        assert!(xml.contains("vertex=\"1\""));
    }

    #[test]
    fn cell_edge_xml_has_source_target() {
        let c = DrawioCell::edge("e1", "calls", "A", "B", "edgeStyle=orthogonalEdgeStyle;");
        let xml = c.to_xml();
        assert!(xml.contains("edge=\"1\""));
        assert!(xml.contains("source=\"A\""));
        assert!(xml.contains("target=\"B\""));
    }

    #[test]
    fn graph_to_xml_has_root() {
        let g = DrawioGraph::new("Test");
        let xml = g.to_xml();
        assert!(xml.contains("<root>"));
        assert!(xml.contains("</root>"));
        assert!(xml.contains("<mxCell id=\"0\""));
    }

    #[test]
    fn graph_to_drawio_file_wraps_xml() {
        let g = DrawioGraph::new("My Diagram");
        let f = g.to_drawio_file();
        assert!(f.contains("<?xml version=\"1.0\""));
        assert!(f.contains("<mxfile"));
        assert!(f.contains("My Diagram"));
    }

    #[test]
    fn template_flowchart_generates_cells() {
        let steps = vec!["Start", "Process?", "End"];
        let g = template_flowchart("Test Flow", &steps);
        assert_eq!(g.vertex_count(), 3);
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn parse_drawio_xml_basic() {
        let xml = r#"<mxfile><diagram name="Page-1" id="abc">
<mxGraphModel><root>
  <mxCell id="0" />
  <mxCell id="1" parent="0" />
  <mxCell id="2" value="Box A" style="rounded=1;" vertex="1" parent="1"><mxGeometry x="10" y="10" width="120" height="40" as="geometry" /></mxCell>
  <mxCell id="3" value="Box B" style="rounded=1;" vertex="1" parent="1"><mxGeometry x="200" y="10" width="120" height="40" as="geometry" /></mxCell>
  <mxCell id="4" value="link" style="" edge="1" source="2" target="3" parent="1"><mxGeometry relative="1" as="geometry" /></mxCell>
</root></mxGraphModel></diagram></mxfile>"#;

        let result = parse_drawio_xml(xml).unwrap();
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.total_vertices, 2);
        assert_eq!(result.total_edges, 1);
    }

    #[test]
    fn parse_empty_returns_error() {
        let result = parse_drawio_xml("   ");
        assert!(result.is_err());
    }

    #[test]
    fn xml_escape_ampersand() {
        assert_eq!(xml_escape("a & b"), "a &amp; b");
    }

    #[test]
    fn mcp_command_serialises() {
        let cmd = DrawioMcpCommand::read_file("/tmp/test.drawio");
        let json = cmd.to_json();
        assert!(json.contains("drawio/read_file"));
        assert!(json.contains("/tmp/test.drawio"));
    }

    #[test]
    fn c4_context_generates_persons_systems() {
        let persons = vec![C4Person {
            id: "p1".into(), name: "User".into(), description: "End user".into(), external: false
        }];
        let systems = vec![C4System {
            id: "s1".into(), name: "Backend".into(), description: "API".into(), external: false,
            containers: vec![],
        }];
        let rels = vec![C4Relation {
            from_id: "p1".into(), to_id: "s1".into(), label: "uses".into(), technology: None
        }];
        let g = template_c4_context("Context", &persons, &systems, &rels);
        assert_eq!(g.vertex_count(), 2);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn parse_llm_flowchart_arrow_notation() {
        let text = "Start -> Validate Input -> Process -> End";
        let g = parse_llm_flowchart(text);
        assert!(g.vertex_count() >= 4);
    }
}
