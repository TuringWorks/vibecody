//! Mermaid ASCII renderer — converts Mermaid flowchart/graph/sequence diagram
//! blocks into Unicode box-drawing art for terminal output.
//!
//! Supports a subset of Mermaid syntax:
//! - `graph TD` / `graph LR` / `flowchart TD` / `flowchart LR`
//! - `sequenceDiagram`
//! - Nodes: `A[text]`, `A((text))`, `A{text}`, `A(text)`
//! - Edges: `A --> B`, `A -->|label| B`, `A --- B`, `A ==> B`

use std::collections::HashMap;

// ── Public API ───────────────────────────────────────────────────────────────

/// Detects and renders all ```mermaid blocks in the given text.
/// Returns the text with mermaid blocks replaced by ASCII art.
pub fn render_mermaid_blocks(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_mermaid = false;
    let mut mermaid_buf = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if !in_mermaid && (trimmed == "```mermaid" || trimmed == "``` mermaid") {
            in_mermaid = true;
            mermaid_buf.clear();
            continue;
        }
        if in_mermaid {
            if trimmed == "```" {
                in_mermaid = false;
                let rendered = render_mermaid(&mermaid_buf);
                result.push_str(&rendered);
                result.push('\n');
                continue;
            }
            mermaid_buf.push_str(line);
            mermaid_buf.push('\n');
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }

    // If we ended inside a mermaid block (unclosed), just dump raw
    if in_mermaid {
        result.push_str("```mermaid\n");
        result.push_str(&mermaid_buf);
    }

    // Remove trailing newline if original didn't have one
    if !text.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }
    result
}

/// Render a single mermaid diagram string to ASCII art.
pub fn render_mermaid(source: &str) -> String {
    let trimmed = source.trim();
    let first_line = trimmed.lines().next().unwrap_or("");
    let first_lower = first_line.trim().to_lowercase();

    if first_lower.starts_with("sequencediagram") {
        render_sequence(trimmed)
    } else if first_lower.starts_with("graph") || first_lower.starts_with("flowchart") {
        let direction = if first_lower.contains("lr") { Direction::LR } else { Direction::TD };
        render_graph(trimmed, direction)
    } else {
        // Unknown diagram type — return as-is
        format!("[Mermaid diagram: unsupported type]\n{}", source)
    }
}

// ── Graph / Flowchart ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    TD, // Top-Down
    LR, // Left-Right
}

#[derive(Debug, Clone)]
struct Node {
    id: String,
    label: String,
    shape: NodeShape,
}

#[derive(Debug, Clone)]
enum NodeShape {
    Rect,    // [text]
    Round,   // (text)
    Diamond, // {text}
    Circle,  // ((text))
}

#[derive(Debug, Clone)]
struct Edge {
    from: String,
    to: String,
    label: Option<String>,
    style: EdgeStyle,
}

#[derive(Debug, Clone)]
enum EdgeStyle {
    Arrow,  // -->
    Line,   // ---
    Thick,  // ==>
}

fn parse_graph(source: &str) -> (Vec<Node>, Vec<Edge>) {
    let mut nodes: HashMap<String, Node> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") || line.starts_with("style") || line.starts_with("class") {
            continue;
        }

        // Try to parse edge: A -->|label| B or A --> B
        if let Some((edge, left_raw, right_raw)) = try_parse_edge_with_parts(line) {
            // Parse inline node definitions from edge parts (e.g., A[Start] --> B[End])
            if let Some(node) = try_parse_node_def(left_raw) {
                nodes.insert(node.id.clone(), node);
            }
            if let Some(node) = try_parse_node_def(&right_raw) {
                nodes.insert(node.id.clone(), node);
            }
            // Ensure nodes exist (fallback if no inline def)
            ensure_node(&mut nodes, &edge.from);
            ensure_node(&mut nodes, &edge.to);
            edges.push(edge);
            continue;
        }

        // Try to parse standalone node definition: A[text]
        if let Some(node) = try_parse_node_def(line) {
            nodes.insert(node.id.clone(), node);
        }
    }

    let node_list: Vec<Node> = if edges.is_empty() {
        nodes.into_values().collect()
    } else {
        // Order nodes by first appearance in edges
        let mut ordered = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for edge in &edges {
            for id in [&edge.from, &edge.to] {
                if seen.insert(id.clone()) {
                    if let Some(n) = nodes.get(id) {
                        ordered.push(n.clone());
                    }
                }
            }
        }
        // Add any remaining nodes not in edges
        for (id, node) in &nodes {
            if !seen.contains(id) {
                ordered.push(node.clone());
            }
        }
        ordered
    };

    (node_list, edges)
}

fn ensure_node(nodes: &mut HashMap<String, Node>, id: &str) {
    if !nodes.contains_key(id) {
        nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            label: id.to_string(),
            shape: NodeShape::Rect,
        });
    }
}

fn try_parse_node_def(s: &str) -> Option<Node> {
    // Match patterns like: A[text], B((text)), C{text}, D(text)
    let s = s.trim().trim_end_matches(';');

    // Find the ID (everything before the first bracket-type char)
    let id_end = s.find(['[', '(', '{'])?;
    let id = s[..id_end].trim().to_string();
    if id.is_empty() {
        return None;
    }

    let rest = &s[id_end..];
    let (label, shape) = if rest.starts_with("((") && rest.ends_with("))") {
        (rest[2..rest.len()-2].to_string(), NodeShape::Circle)
    } else if rest.starts_with('[') && rest.ends_with(']') {
        (rest[1..rest.len()-1].to_string(), NodeShape::Rect)
    } else if rest.starts_with('{') && rest.ends_with('}') {
        (rest[1..rest.len()-1].to_string(), NodeShape::Diamond)
    } else if rest.starts_with('(') && rest.ends_with(')') {
        (rest[1..rest.len()-1].to_string(), NodeShape::Round)
    } else {
        return None;
    };

    Some(Node { id, label, shape })
}

/// Returns (Edge, left_raw_part, right_raw_part) so callers can also parse inline node defs.
fn try_parse_edge_with_parts(s: &str) -> Option<(Edge, &str, String)> {
    let s = s.trim().trim_end_matches(';');

    // Arrow patterns: -->, --->, ==>, ---
    let arrow_patterns = [
        ("==>", EdgeStyle::Thick),
        ("--->", EdgeStyle::Arrow),
        ("-->", EdgeStyle::Arrow),
        ("---", EdgeStyle::Line),
    ];

    for (arrow, style) in &arrow_patterns {
        // Check for labeled edge: A -->|label| B
        if let Some(arrow_pos) = s.find(arrow) {
            let left = s[..arrow_pos].trim();
            let right_part = &s[arrow_pos + arrow.len()..];

            let (label, to_part) = if let Some(stripped) = right_part.strip_prefix('|') {
                // A -->|label| B
                if let Some(end_pipe) = stripped.find('|') {
                    let lbl = stripped[..end_pipe].to_string();
                    let rest = stripped[end_pipe + 1..].trim().to_string();
                    (Some(lbl), rest)
                } else {
                    (None, right_part.trim().to_string())
                }
            } else {
                (None, right_part.trim().to_string())
            };

            // Parse left and right for inline node defs
            let from_id = extract_node_id(left);
            let to_id = extract_node_id(&to_part);

            if !from_id.is_empty() && !to_id.is_empty() {
                return Some((Edge {
                    from: from_id,
                    to: to_id,
                    label,
                    style: style.clone(),
                }, left, to_part));
            }
        }
    }
    None
}

fn extract_node_id(s: &str) -> String {
    let s = s.trim();
    // If it has brackets, extract ID before them
    if let Some(pos) = s.find(['[', '(', '{']) {
        s[..pos].trim().to_string()
    } else {
        s.to_string()
    }
}

fn render_graph(source: &str, direction: Direction) -> String {
    let (nodes, edges) = parse_graph(source);
    if nodes.is_empty() {
        return "[Empty graph]".to_string();
    }

    match direction {
        Direction::TD => render_td(&nodes, &edges),
        Direction::LR => render_lr(&nodes, &edges),
    }
}

fn render_box(label: &str, shape: &NodeShape) -> Vec<String> {
    let w = label.len() + 4;
    match shape {
        NodeShape::Rect => {
            vec![
                format!("┌{}┐", "─".repeat(w)),
                format!("│  {}  │", label),
                format!("└{}┘", "─".repeat(w)),
            ]
        }
        NodeShape::Round => {
            vec![
                format!("╭{}╮", "─".repeat(w)),
                format!("│  {}  │", label),
                format!("╰{}╯", "─".repeat(w)),
            ]
        }
        NodeShape::Diamond => {
            let half = w / 2 + 2;
            vec![
                format!("{}/\\{}", " ".repeat(half), " ".repeat(half)),
                format!("<  {}  >", label),
                format!("{}\\/{}",  " ".repeat(half), " ".repeat(half)),
            ]
        }
        NodeShape::Circle => {
            vec![
                format!("╭{}╮", "─".repeat(w)),
                format!("│  {}  │", label),
                format!("╰{}╯", "─".repeat(w)),
            ]
        }
    }
}

fn render_td(nodes: &[Node], edges: &[Edge]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let node_map: HashMap<&str, &Node> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Simple vertical layout: each node on its own row, arrows between
    let mut placed: Vec<&str> = Vec::new();
    for edge in edges {
        if !placed.contains(&edge.from.as_str()) {
            let node = node_map.get(edge.from.as_str());
            if let Some(n) = node {
                let box_lines = render_box(&n.label, &n.shape);
                for bl in &box_lines {
                    lines.push(format!("  {}", bl));
                }
            }
            placed.push(&edge.from);
        }

        // Arrow
        let arrow_label = edge.label.as_deref().unwrap_or("");
        let connector = match edge.style {
            EdgeStyle::Arrow | EdgeStyle::Thick => "▼",
            EdgeStyle::Line => "│",
        };
        if arrow_label.is_empty() {
            lines.push(format!("  {:>width$}│", "", width = 4));
            lines.push(format!("  {:>width$}{}", "", connector, width = 4));
        } else {
            lines.push(format!("  {:>width$}│ {}", "", arrow_label, width = 4));
            lines.push(format!("  {:>width$}{}", "", connector, width = 4));
        }

        if !placed.contains(&edge.to.as_str()) {
            let node = node_map.get(edge.to.as_str());
            if let Some(n) = node {
                let box_lines = render_box(&n.label, &n.shape);
                for bl in &box_lines {
                    lines.push(format!("  {}", bl));
                }
            }
            placed.push(&edge.to);
        }
    }

    // Any nodes not connected
    for node in nodes {
        if !placed.contains(&node.id.as_str()) {
            let box_lines = render_box(&node.label, &node.shape);
            for bl in &box_lines {
                lines.push(format!("  {}", bl));
            }
        }
    }

    lines.join("\n")
}

fn render_lr(nodes: &[Node], edges: &[Edge]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let node_map: HashMap<&str, &Node> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Simple horizontal layout: nodes left-to-right with arrows
    let mut placed: Vec<&str> = Vec::new();
    let mut row_top = String::new();
    let mut row_mid = String::new();
    let mut row_bot = String::new();

    for edge in edges {
        if !placed.contains(&edge.from.as_str()) {
            if let Some(n) = node_map.get(edge.from.as_str()) {
                let bx = render_box(&n.label, &n.shape);
                row_top.push_str(&bx[0]);
                row_mid.push_str(&bx[1]);
                row_bot.push_str(&bx[2]);
                placed.push(&edge.from);
            }
        }

        // Horizontal arrow
        let label = edge.label.as_deref().unwrap_or("");
        let arrow = match edge.style {
            EdgeStyle::Arrow => if label.is_empty() {
                " ──▶ ".to_string()
            } else {
                format!(" ─{}─▶ ", label)
            },
            EdgeStyle::Thick => if label.is_empty() {
                " ══▶ ".to_string()
            } else {
                format!(" ═{}═▶ ", label)
            },
            EdgeStyle::Line => if label.is_empty() {
                " ─── ".to_string()
            } else {
                format!(" ─{}── ", label)
            },
        };
        let aw = arrow.len();
        row_top.push_str(&" ".repeat(aw));
        row_mid.push_str(&arrow);
        row_bot.push_str(&" ".repeat(aw));

        if !placed.contains(&edge.to.as_str()) {
            if let Some(n) = node_map.get(edge.to.as_str()) {
                let bx = render_box(&n.label, &n.shape);
                row_top.push_str(&bx[0]);
                row_mid.push_str(&bx[1]);
                row_bot.push_str(&bx[2]);
                placed.push(&edge.to);
            }
        }
    }

    if !row_top.is_empty() {
        lines.push(format!("  {}", row_top));
        lines.push(format!("  {}", row_mid));
        lines.push(format!("  {}", row_bot));
    }

    lines.join("\n")
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Pad a string with ASCII dashes to reach the target width.
/// Unlike format fill, this uses plain `-` to avoid multi-byte Unicode issues.
fn pad_with_dashes(s: &str, target: usize) -> String {
    if s.len() >= target {
        s.to_string()
    } else {
        let pad = target - s.len();
        format!("{}{}", s, "-".repeat(pad))
    }
}

// ── Sequence Diagram ─────────────────────────────────────────────────────────

fn render_sequence(source: &str) -> String {
    let mut participants: Vec<String> = Vec::new();
    let mut messages: Vec<(String, String, String, bool)> = Vec::new(); // (from, to, label, is_reply)

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // participant A as Alice
        if let Some(rest) = line.strip_prefix("participant ") {
            let name = if let Some(pos) = rest.find(" as ") {
                rest[pos + 4..].trim().to_string()
            } else {
                rest.trim().to_string()
            };
            if !participants.contains(&name) {
                participants.push(name);
            }
            continue;
        }

        // A->>B: message  or  A-->>B: reply
        if let Some(msg) = try_parse_seq_message(line) {
            if !participants.contains(&msg.0) {
                participants.push(msg.0.clone());
            }
            if !participants.contains(&msg.1) {
                participants.push(msg.1.clone());
            }
            messages.push(msg);
        }
    }

    if participants.is_empty() {
        return "[Empty sequence diagram]".to_string();
    }

    let col_width = participants.iter().map(|p| p.len()).max().unwrap_or(6).max(8) + 4;
    let mut lines: Vec<String> = Vec::new();

    // Header: participant names
    let mut header = String::new();
    for p in &participants {
        header.push_str(&format!("{:^width$}", p, width = col_width));
    }
    lines.push(header);

    // Vertical lines
    let mut vline = String::new();
    for _ in &participants {
        vline.push_str(&format!("{:^width$}", "│", width = col_width));
    }
    lines.push(vline.clone());

    // Messages
    for (from, to, label, is_reply) in &messages {
        let from_idx = participants.iter().position(|p| p == from).unwrap_or(0);
        let to_idx = participants.iter().position(|p| p == to).unwrap_or(0);

        let mut msg_line: Vec<String> = participants.iter().map(|_| " ".repeat(col_width)).collect();

        if from_idx < to_idx {
            // Left to right arrow
            let span = (to_idx - from_idx) * col_width;
            let arrow_body = if *is_reply {
                format!("- - {} - ->", label)
            } else {
                format!("-- {} -->", label)
            };
            let padded = pad_with_dashes(&arrow_body, span.saturating_sub(2));
            msg_line[from_idx] = format!("{:^width$}", padded, width = span);
            // Flatten
            let flat: String = msg_line.iter().take(from_idx).map(|s| s.as_str()).collect::<String>()
                + &msg_line[from_idx]
                + &msg_line.iter().skip(to_idx + 1).map(|s| s.as_str()).collect::<String>();
            lines.push(flat);
        } else if to_idx < from_idx {
            // Right to left arrow
            let span = (from_idx - to_idx) * col_width;
            let arrow_body = if *is_reply {
                format!("<- - {} - -", label)
            } else {
                format!("<-- {} --", label)
            };
            let padded = pad_with_dashes(&arrow_body, span.saturating_sub(2));
            msg_line[to_idx] = format!("{:^width$}", padded, width = span);
            let flat: String = msg_line.iter().take(to_idx).map(|s| s.as_str()).collect::<String>()
                + &msg_line[to_idx]
                + &msg_line.iter().skip(from_idx + 1).map(|s| s.as_str()).collect::<String>();
            lines.push(flat);
        } else {
            // Self-message
            lines.push(format!("{:>width$}↻ {}", "", label, width = from_idx * col_width + col_width / 2));
        }

        lines.push(vline.clone());
    }

    lines.join("\n")
}

fn try_parse_seq_message(line: &str) -> Option<(String, String, String, bool)> {
    // Patterns: A->>B: msg, A-->>B: msg, A->>+B: msg, A->>-B: msg, A->B: msg
    let arrow_patterns = ["-->>", "->>", "-->", "->"];

    for arrow in &arrow_patterns {
        if let Some(pos) = line.find(arrow) {
            let from = line[..pos].trim().to_string();
            let rest = &line[pos + arrow.len()..];
            let is_reply = arrow.starts_with("--");

            // Strip activation markers (+/-)
            let rest = rest.trim_start_matches('+').trim_start_matches('-');

            let (to, label) = if let Some(colon) = rest.find(':') {
                (rest[..colon].trim().to_string(), rest[colon + 1..].trim().to_string())
            } else {
                (rest.trim().to_string(), String::new())
            };

            if !from.is_empty() && !to.is_empty() {
                return Some((from, to, label, is_reply));
            }
        }
    }
    None
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_simple_td_graph() {
        let src = "graph TD\n    A[Start] --> B[End]";
        let result = render_mermaid(src);
        assert!(result.contains("Start"));
        assert!(result.contains("End"));
        assert!(result.contains("▼"));
    }

    #[test]
    fn render_simple_lr_graph() {
        let src = "graph LR\n    A[Input] --> B[Output]";
        let result = render_mermaid(src);
        assert!(result.contains("Input"));
        assert!(result.contains("Output"));
        assert!(result.contains("▶"));
    }

    #[test]
    fn render_labeled_edge() {
        let src = "graph TD\n    A[Start] -->|yes| B[End]";
        let result = render_mermaid(src);
        assert!(result.contains("yes"));
    }

    #[test]
    fn render_sequence_diagram() {
        let src = "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi back";
        let result = render_mermaid(src);
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
        assert!(result.contains("Hello"));
    }

    #[test]
    fn render_mermaid_blocks_in_text() {
        let input = "Here is a diagram:\n```mermaid\ngraph TD\n    A --> B\n```\nDone.";
        let result = render_mermaid_blocks(input);
        assert!(!result.contains("```mermaid"));
        assert!(result.contains("Done."));
    }

    #[test]
    fn no_mermaid_blocks_unchanged() {
        let input = "Just normal text\nwith no diagrams.";
        let result = render_mermaid_blocks(input);
        assert_eq!(result, input);
    }

    #[test]
    fn parse_node_rect() {
        let node = try_parse_node_def("A[Hello World]").unwrap();
        assert_eq!(node.id, "A");
        assert_eq!(node.label, "Hello World");
        assert!(matches!(node.shape, NodeShape::Rect));
    }

    #[test]
    fn parse_node_round() {
        let node = try_parse_node_def("B(Rounded)").unwrap();
        assert_eq!(node.id, "B");
        assert_eq!(node.label, "Rounded");
        assert!(matches!(node.shape, NodeShape::Round));
    }

    #[test]
    fn parse_node_diamond() {
        let node = try_parse_node_def("C{Decision}").unwrap();
        assert_eq!(node.id, "C");
        assert_eq!(node.label, "Decision");
        assert!(matches!(node.shape, NodeShape::Diamond));
    }

    #[test]
    fn parse_node_circle() {
        let node = try_parse_node_def("D((Circle))").unwrap();
        assert_eq!(node.id, "D");
        assert_eq!(node.label, "Circle");
        assert!(matches!(node.shape, NodeShape::Circle));
    }

    #[test]
    fn parse_edge_arrow() {
        let (edge, _, _) = try_parse_edge_with_parts("A --> B").unwrap();
        assert_eq!(edge.from, "A");
        assert_eq!(edge.to, "B");
        assert!(edge.label.is_none());
    }

    #[test]
    fn parse_edge_labeled() {
        let (edge, _, _) = try_parse_edge_with_parts("A -->|yes| B").unwrap();
        assert_eq!(edge.from, "A");
        assert_eq!(edge.to, "B");
        assert_eq!(edge.label.as_deref(), Some("yes"));
    }

    #[test]
    fn parse_edge_thick() {
        let (edge, _, _) = try_parse_edge_with_parts("A ==> B").unwrap();
        assert_eq!(edge.from, "A");
        assert_eq!(edge.to, "B");
        assert!(matches!(edge.style, EdgeStyle::Thick));
    }

    #[test]
    fn parse_seq_message() {
        let msg = try_parse_seq_message("Alice->>Bob: Hello").unwrap();
        assert_eq!(msg.0, "Alice");
        assert_eq!(msg.1, "Bob");
        assert_eq!(msg.2, "Hello");
        assert!(!msg.3);
    }

    #[test]
    fn parse_seq_reply() {
        let msg = try_parse_seq_message("Bob-->>Alice: reply").unwrap();
        assert_eq!(msg.0, "Bob");
        assert_eq!(msg.1, "Alice");
        assert_eq!(msg.2, "reply");
        assert!(msg.3);
    }

    #[test]
    fn render_flowchart_keyword() {
        let src = "flowchart LR\n    X[Hello] --> Y[World]";
        let result = render_mermaid(src);
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
    }

    #[test]
    fn unsupported_diagram_type() {
        let src = "pie\n    title Pets\n    \"Dogs\" : 386";
        let result = render_mermaid(src);
        assert!(result.contains("unsupported"));
    }

    #[test]
    fn render_box_rect() {
        let lines = render_box("Test", &NodeShape::Rect);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("┌"));
        assert!(lines[1].contains("Test"));
        assert!(lines[2].contains("└"));
    }

    #[test]
    fn render_box_round() {
        let lines = render_box("Round", &NodeShape::Round);
        assert!(lines[0].contains("╭"));
        assert!(lines[2].contains("╰"));
    }

    #[test]
    fn multi_node_graph() {
        let src = "graph TD\n    A[First] --> B[Second]\n    B --> C[Third]";
        let result = render_mermaid(src);
        assert!(result.contains("First"));
        assert!(result.contains("Second"));
        assert!(result.contains("Third"));
    }

    #[test]
    fn sequence_with_participants() {
        let src = "sequenceDiagram\n    participant A as Alice\n    participant B as Bob\n    A->>B: Hi";
        let result = render_mermaid(src);
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }
}
