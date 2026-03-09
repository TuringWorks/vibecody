//! MCP Apps — interactive UI widget rendering from MCP tool responses.
//!
//! Closes P1 Gap 3: Render charts, diagrams, forms, interactive widgets from
//! MCP tool responses inside the agent chat (Cursor 2.6 / VS Code style).
//!
//! # Architecture
//!
//! ```text
//! MCP Tool Response → WidgetRegistry → parse widget definition → render
//!   { type: "mcp-app", component: "chart", props: {...} }
//!     ├─ TableWidget     (rows + columns)
//!     ├─ ChartWidget     (bar, line, pie, scatter)
//!     ├─ FormWidget      (input fields with submit)
//!     ├─ ImageWidget     (base64 or URL)
//!     ├─ MermaidWidget   (diagram DSL)
//!     ├─ MarkdownWidget  (rich text)
//!     ├─ ProgressWidget  (progress bar)
//!     ├─ CodeWidget      (syntax-highlighted code block)
//!     ├─ TreeWidget      (collapsible tree view)
//!     └─ MetricWidget    (KPI cards with trend)
//! ```

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Widget types
// ---------------------------------------------------------------------------

/// Supported MCP App widget types.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    Table,
    Chart,
    Form,
    Image,
    Mermaid,
    Markdown,
    Progress,
    Code,
    Tree,
    Metric,
    Custom(String),
}

impl WidgetKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "table" => WidgetKind::Table,
            "chart" => WidgetKind::Chart,
            "form" => WidgetKind::Form,
            "image" => WidgetKind::Image,
            "mermaid" => WidgetKind::Mermaid,
            "markdown" => WidgetKind::Markdown,
            "progress" => WidgetKind::Progress,
            "code" => WidgetKind::Code,
            "tree" => WidgetKind::Tree,
            "metric" => WidgetKind::Metric,
            other => WidgetKind::Custom(other.to_string()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            WidgetKind::Table => "table",
            WidgetKind::Chart => "chart",
            WidgetKind::Form => "form",
            WidgetKind::Image => "image",
            WidgetKind::Mermaid => "mermaid",
            WidgetKind::Markdown => "markdown",
            WidgetKind::Progress => "progress",
            WidgetKind::Code => "code",
            WidgetKind::Tree => "tree",
            WidgetKind::Metric => "metric",
            WidgetKind::Custom(name) => name,
        }
    }
}

// ---------------------------------------------------------------------------
// Chart types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Scatter,
    Area,
}

impl ChartType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "bar" => ChartType::Bar,
            "line" => ChartType::Line,
            "pie" => ChartType::Pie,
            "scatter" => ChartType::Scatter,
            "area" => ChartType::Area,
            _ => ChartType::Bar,
        }
    }
}

// ---------------------------------------------------------------------------
// Widget definition
// ---------------------------------------------------------------------------

/// A widget definition parsed from an MCP tool response.
#[derive(Debug, Clone)]
pub struct WidgetDef {
    pub kind: WidgetKind,
    pub props: HashMap<String, String>,
    pub children: Vec<WidgetDef>,
    pub id: Option<String>,
    pub title: Option<String>,
}

impl WidgetDef {
    pub fn new(kind: WidgetKind) -> Self {
        Self {
            kind,
            props: HashMap::new(),
            children: Vec::new(),
            id: None,
            title: None,
        }
    }

    pub fn with_prop(mut self, key: &str, value: &str) -> Self {
        self.props.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn with_child(mut self, child: WidgetDef) -> Self {
        self.children.push(child);
        self
    }

    pub fn get_prop(&self, key: &str) -> Option<&str> {
        self.props.get(key).map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// Table widget data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TableData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub sortable: bool,
    pub filterable: bool,
}

impl TableData {
    pub fn new(columns: Vec<String>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            sortable: true,
            filterable: false,
        }
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.columns.len()
    }

    /// Render as ASCII table for TUI.
    pub fn to_ascii(&self) -> String {
        if self.columns.is_empty() {
            return String::new();
        }
        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.len()).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() && cell.len() > widths[i] {
                    widths[i] = cell.len();
                }
            }
        }
        let mut out = String::new();
        // Header
        let header: Vec<String> = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{:width$}", c, width = widths[i]))
            .collect();
        let sep: Vec<String> = widths.iter().map(|&w| "-".repeat(w)).collect();
        out.push_str(&format!("| {} |\n", header.join(" | ")));
        out.push_str(&format!("| {} |\n", sep.join(" | ")));
        // Rows
        for row in &self.rows {
            let cells: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let w = widths.get(i).copied().unwrap_or(0);
                    format!("{:width$}", c, width = w)
                })
                .collect();
            out.push_str(&format!("| {} |\n", cells.join(" | ")));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Chart widget data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ChartData {
    pub chart_type: ChartType,
    pub labels: Vec<String>,
    pub datasets: Vec<ChartDataset>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChartDataset {
    pub label: String,
    pub values: Vec<f64>,
    pub color: Option<String>,
}

impl ChartData {
    pub fn new(chart_type: ChartType) -> Self {
        Self {
            chart_type,
            labels: Vec::new(),
            datasets: Vec::new(),
            x_label: None,
            y_label: None,
        }
    }

    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn add_dataset(&mut self, dataset: ChartDataset) {
        self.datasets.push(dataset);
    }

    /// Render as ASCII bar chart for TUI.
    pub fn to_ascii_bars(&self, max_width: usize) -> String {
        let mut out = String::new();
        for dataset in &self.datasets {
            let max_val = dataset
                .values
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            if max_val <= 0.0 {
                continue;
            }
            if self.datasets.len() > 1 {
                out.push_str(&format!("  {}\n", dataset.label));
            }
            for (i, &val) in dataset.values.iter().enumerate() {
                let label = self.labels.get(i).map(|s| s.as_str()).unwrap_or("?");
                let bar_len = ((val / max_val) * max_width as f64) as usize;
                let bar = "#".repeat(bar_len);
                out.push_str(&format!("  {:>12} | {} {:.1}\n", label, bar, val));
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Form widget data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum FormFieldType {
    Text,
    Number,
    Select,
    Checkbox,
    TextArea,
    Hidden,
}

#[derive(Debug, Clone)]
pub struct FormField {
    pub name: String,
    pub label: String,
    pub field_type: FormFieldType,
    pub default_value: Option<String>,
    pub options: Vec<String>,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct FormData {
    pub fields: Vec<FormField>,
    pub submit_label: String,
    pub action: String,
}

impl FormData {
    pub fn new(action: &str) -> Self {
        Self {
            fields: Vec::new(),
            submit_label: "Submit".to_string(),
            action: action.to_string(),
        }
    }

    pub fn add_field(&mut self, field: FormField) {
        self.fields.push(field);
    }

    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}

// ---------------------------------------------------------------------------
// Progress widget data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ProgressData {
    pub current: f64,
    pub total: f64,
    pub label: Option<String>,
    pub show_percentage: bool,
}

impl ProgressData {
    pub fn new(current: f64, total: f64) -> Self {
        Self {
            current,
            total,
            label: None,
            show_percentage: true,
        }
    }

    pub fn percentage(&self) -> f64 {
        if self.total <= 0.0 {
            0.0
        } else {
            (self.current / self.total * 100.0).min(100.0)
        }
    }

    pub fn to_ascii(&self, width: usize) -> String {
        let pct = self.percentage();
        let filled = ((pct / 100.0) * width as f64) as usize;
        let empty = width.saturating_sub(filled);
        let bar = format!("[{}{}]", "=".repeat(filled), " ".repeat(empty));
        if self.show_percentage {
            format!("{} {:.1}%", bar, pct)
        } else {
            bar
        }
    }
}

// ---------------------------------------------------------------------------
// Metric widget data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TrendDirection {
    Up,
    Down,
    Flat,
}

#[derive(Debug, Clone)]
pub struct MetricData {
    pub label: String,
    pub value: String,
    pub unit: Option<String>,
    pub trend: Option<TrendDirection>,
    pub change: Option<String>,
}

impl MetricData {
    pub fn new(label: &str, value: &str) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            unit: None,
            trend: None,
            change: None,
        }
    }

    pub fn with_trend(mut self, direction: TrendDirection, change: &str) -> Self {
        self.trend = Some(direction);
        self.change = Some(change.to_string());
        self
    }

    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }
}

// ---------------------------------------------------------------------------
// Tree widget data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub icon: Option<String>,
}

impl TreeNode {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            children: Vec::new(),
            expanded: true,
            icon: None,
        }
    }

    pub fn with_child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Render as indented ASCII tree.
    pub fn to_ascii(&self, prefix: &str, is_last: bool) -> String {
        let mut out = String::new();
        let connector = if is_last { "└─ " } else { "├─ " };
        out.push_str(&format!("{}{}{}\n", prefix, connector, self.label));
        let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });
        for (i, child) in self.children.iter().enumerate() {
            let last = i == self.children.len() - 1;
            out.push_str(&child.to_ascii(&child_prefix, last));
        }
        out
    }

    pub fn total_nodes(&self) -> usize {
        1 + self.children.iter().map(|c| c.total_nodes()).sum::<usize>()
    }
}

// ---------------------------------------------------------------------------
// Widget registry
// ---------------------------------------------------------------------------

/// Registry of available widget renderers.
pub struct WidgetRegistry {
    supported: Vec<WidgetKind>,
}

impl WidgetRegistry {
    pub fn new() -> Self {
        Self {
            supported: vec![
                WidgetKind::Table,
                WidgetKind::Chart,
                WidgetKind::Form,
                WidgetKind::Image,
                WidgetKind::Mermaid,
                WidgetKind::Markdown,
                WidgetKind::Progress,
                WidgetKind::Code,
                WidgetKind::Tree,
                WidgetKind::Metric,
            ],
        }
    }

    pub fn is_supported(&self, kind: &WidgetKind) -> bool {
        self.supported.contains(kind)
    }

    pub fn supported_kinds(&self) -> &[WidgetKind] {
        &self.supported
    }

    pub fn register(&mut self, kind: WidgetKind) {
        if !self.supported.contains(&kind) {
            self.supported.push(kind);
        }
    }
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// MCP response parser
// ---------------------------------------------------------------------------

/// Check if an MCP tool response contains a widget definition.
pub fn is_mcp_app_response(response: &str) -> bool {
    response.contains("\"type\"")
        && response.contains("mcp-app")
        && response.contains("\"component\"")
}

/// Parse an MCP App response into a WidgetDef.
/// Expected format: `{ "type": "mcp-app", "component": "chart", "title": "...", "props": {...} }`
pub fn parse_mcp_app(response: &str) -> Option<WidgetDef> {
    if !is_mcp_app_response(response) {
        return None;
    }
    let component = extract_field(response, "component")?;
    let kind = WidgetKind::from_str(&component);
    let mut widget = WidgetDef::new(kind);
    if let Some(title) = extract_field(response, "title") {
        widget = widget.with_title(&title);
    }
    if let Some(id) = extract_field(response, "id") {
        widget = widget.with_id(&id);
    }
    // Extract props section
    if let Some(props_start) = response.find("\"props\"") {
        let rest = &response[props_start..];
        if let Some(brace) = rest.find('{') {
            let props_str = &rest[brace..];
            // Simple key-value extraction from props
            let mut depth = 0;
            let mut end = 0;
            for (i, ch) in props_str.chars().enumerate() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if end > 0 {
                let inner = &props_str[1..end];
                // Extract simple key-value pairs
                for part in inner.split(',') {
                    let part = part.trim();
                    if let Some(colon) = part.find(':') {
                        let key = part[..colon].trim().trim_matches('"');
                        let val = part[colon + 1..].trim().trim_matches('"');
                        widget.props.insert(key.to_string(), val.to_string());
                    }
                }
            }
        }
    }
    Some(widget)
}

fn extract_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\"", field);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let rest = rest.trim_start().strip_prefix(':')?;
    let rest = rest.trim_start();
    if rest.starts_with('"') {
        let rest = &rest[1..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    } else {
        let end = rest.find([',', '}', ']']).unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    }
}

// ---------------------------------------------------------------------------
// TUI rendering helpers
// ---------------------------------------------------------------------------

/// Render a widget definition as plain text for TUI display.
pub fn render_widget_text(widget: &WidgetDef) -> String {
    let mut out = String::new();
    if let Some(title) = &widget.title {
        out.push_str(&format!("╔══ {} ══╗\n", title));
    }
    out.push_str(&format!("[MCP App: {}]\n", widget.kind.name()));
    for (key, val) in &widget.props {
        out.push_str(&format!("  {}: {}\n", key, val));
    }
    for child in &widget.children {
        out.push_str(&render_widget_text(child));
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_kind_from_str() {
        assert_eq!(WidgetKind::from_str("table"), WidgetKind::Table);
        assert_eq!(WidgetKind::from_str("chart"), WidgetKind::Chart);
        assert_eq!(WidgetKind::from_str("form"), WidgetKind::Form);
        assert_eq!(WidgetKind::from_str("image"), WidgetKind::Image);
        assert_eq!(WidgetKind::from_str("mermaid"), WidgetKind::Mermaid);
        assert_eq!(WidgetKind::from_str("markdown"), WidgetKind::Markdown);
        assert_eq!(WidgetKind::from_str("progress"), WidgetKind::Progress);
        assert_eq!(WidgetKind::from_str("code"), WidgetKind::Code);
        assert_eq!(WidgetKind::from_str("tree"), WidgetKind::Tree);
        assert_eq!(WidgetKind::from_str("metric"), WidgetKind::Metric);
        assert_eq!(WidgetKind::from_str("custom_thing"), WidgetKind::Custom("custom_thing".into()));
    }

    #[test]
    fn test_widget_kind_name() {
        assert_eq!(WidgetKind::Table.name(), "table");
        assert_eq!(WidgetKind::Custom("x".into()).name(), "x");
    }

    #[test]
    fn test_chart_type_from_str() {
        assert_eq!(ChartType::from_str("bar"), ChartType::Bar);
        assert_eq!(ChartType::from_str("line"), ChartType::Line);
        assert_eq!(ChartType::from_str("pie"), ChartType::Pie);
        assert_eq!(ChartType::from_str("scatter"), ChartType::Scatter);
        assert_eq!(ChartType::from_str("area"), ChartType::Area);
        assert_eq!(ChartType::from_str("unknown"), ChartType::Bar);
    }

    #[test]
    fn test_widget_def_new() {
        let w = WidgetDef::new(WidgetKind::Table);
        assert_eq!(w.kind, WidgetKind::Table);
        assert!(w.props.is_empty());
        assert!(w.children.is_empty());
    }

    #[test]
    fn test_widget_def_with_props() {
        let w = WidgetDef::new(WidgetKind::Chart)
            .with_prop("type", "bar")
            .with_title("Sales")
            .with_id("chart-1");
        assert_eq!(w.get_prop("type"), Some("bar"));
        assert_eq!(w.title.as_deref(), Some("Sales"));
        assert_eq!(w.id.as_deref(), Some("chart-1"));
    }

    #[test]
    fn test_widget_def_children() {
        let w = WidgetDef::new(WidgetKind::Form)
            .with_child(WidgetDef::new(WidgetKind::Markdown))
            .with_child(WidgetDef::new(WidgetKind::Progress));
        assert_eq!(w.children.len(), 2);
    }

    #[test]
    fn test_table_data() {
        let mut table = TableData::new(vec!["Name".into(), "Value".into()]);
        table.add_row(vec!["foo".into(), "42".into()]);
        table.add_row(vec!["bar".into(), "99".into()]);
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.col_count(), 2);
    }

    #[test]
    fn test_table_to_ascii() {
        let mut table = TableData::new(vec!["Col1".into(), "Col2".into()]);
        table.add_row(vec!["a".into(), "b".into()]);
        let ascii = table.to_ascii();
        assert!(ascii.contains("Col1"));
        assert!(ascii.contains("Col2"));
        assert!(ascii.contains("a"));
        assert!(ascii.contains("b"));
        assert!(ascii.contains("---"));
    }

    #[test]
    fn test_table_empty_cols() {
        let table = TableData::new(vec![]);
        assert_eq!(table.to_ascii(), "");
    }

    #[test]
    fn test_chart_data() {
        let mut chart = ChartData::new(ChartType::Bar)
            .with_labels(vec!["Q1".into(), "Q2".into()]);
        chart.add_dataset(ChartDataset {
            label: "Revenue".into(),
            values: vec![100.0, 200.0],
            color: None,
        });
        assert_eq!(chart.labels.len(), 2);
        assert_eq!(chart.datasets.len(), 1);
    }

    #[test]
    fn test_chart_to_ascii_bars() {
        let mut chart = ChartData::new(ChartType::Bar)
            .with_labels(vec!["A".into(), "B".into()]);
        chart.add_dataset(ChartDataset {
            label: "Data".into(),
            values: vec![50.0, 100.0],
            color: None,
        });
        let ascii = chart.to_ascii_bars(20);
        assert!(ascii.contains("A"));
        assert!(ascii.contains("B"));
        assert!(ascii.contains("#"));
    }

    #[test]
    fn test_chart_empty_dataset() {
        let chart = ChartData::new(ChartType::Bar);
        let ascii = chart.to_ascii_bars(20);
        assert!(ascii.is_empty());
    }

    #[test]
    fn test_form_data() {
        let mut form = FormData::new("/api/submit");
        form.add_field(FormField {
            name: "email".into(),
            label: "Email".into(),
            field_type: FormFieldType::Text,
            default_value: None,
            options: vec![],
            required: true,
        });
        assert_eq!(form.field_count(), 1);
        assert_eq!(form.action, "/api/submit");
    }

    #[test]
    fn test_progress_data() {
        let p = ProgressData::new(75.0, 100.0);
        assert!((p.percentage() - 75.0).abs() < 0.01);
        let ascii = p.to_ascii(20);
        assert!(ascii.contains("75.0%"));
        assert!(ascii.contains("["));
    }

    #[test]
    fn test_progress_zero_total() {
        let p = ProgressData::new(50.0, 0.0);
        assert_eq!(p.percentage(), 0.0);
    }

    #[test]
    fn test_progress_over_100() {
        let p = ProgressData::new(150.0, 100.0);
        assert_eq!(p.percentage(), 100.0);
    }

    #[test]
    fn test_progress_no_percentage() {
        let mut p = ProgressData::new(50.0, 100.0);
        p.show_percentage = false;
        let ascii = p.to_ascii(10);
        assert!(!ascii.contains('%'));
    }

    #[test]
    fn test_metric_data() {
        let m = MetricData::new("CPU", "87%")
            .with_trend(TrendDirection::Up, "+5%")
            .with_unit("%");
        assert_eq!(m.label, "CPU");
        assert_eq!(m.value, "87%");
        assert_eq!(m.trend, Some(TrendDirection::Up));
        assert_eq!(m.change.as_deref(), Some("+5%"));
        assert_eq!(m.unit.as_deref(), Some("%"));
    }

    #[test]
    fn test_tree_node() {
        let tree = TreeNode::new("root")
            .with_child(TreeNode::new("child1").with_child(TreeNode::new("grandchild")))
            .with_child(TreeNode::new("child2"));
        assert_eq!(tree.total_nodes(), 4);
        assert!(!tree.is_leaf());
        assert!(tree.children[1].is_leaf());
    }

    #[test]
    fn test_tree_to_ascii() {
        let tree = TreeNode::new("src")
            .with_child(TreeNode::new("main.rs"))
            .with_child(TreeNode::new("lib.rs"));
        let ascii = tree.to_ascii("", true);
        assert!(ascii.contains("src"));
        assert!(ascii.contains("main.rs"));
        assert!(ascii.contains("lib.rs"));
    }

    #[test]
    fn test_widget_registry() {
        let reg = WidgetRegistry::new();
        assert!(reg.is_supported(&WidgetKind::Table));
        assert!(reg.is_supported(&WidgetKind::Chart));
        assert!(!reg.is_supported(&WidgetKind::Custom("x".into())));
        assert_eq!(reg.supported_kinds().len(), 10);
    }

    #[test]
    fn test_widget_registry_register() {
        let mut reg = WidgetRegistry::new();
        let custom = WidgetKind::Custom("datepicker".into());
        reg.register(custom.clone());
        assert!(reg.is_supported(&custom));
        // Register again — no duplicate
        reg.register(WidgetKind::Custom("datepicker".into()));
        assert_eq!(reg.supported_kinds().len(), 11);
    }

    #[test]
    fn test_is_mcp_app_response() {
        let yes = r#"{"type": "mcp-app", "component": "chart"}"#;
        assert!(is_mcp_app_response(yes));
        let no = r#"{"result": "hello"}"#;
        assert!(!is_mcp_app_response(no));
    }

    #[test]
    fn test_parse_mcp_app_chart() {
        let response = r#"{"type": "mcp-app", "component": "chart", "title": "Revenue", "props": {"chartType": "bar", "labels": "Q1,Q2"}}"#;
        let widget = parse_mcp_app(response).unwrap();
        assert_eq!(widget.kind, WidgetKind::Chart);
        assert_eq!(widget.title.as_deref(), Some("Revenue"));
        assert_eq!(widget.get_prop("chartType"), Some("bar"));
    }

    #[test]
    fn test_parse_mcp_app_table() {
        let response = r#"{"type": "mcp-app", "component": "table", "id": "t1", "props": {"rows": "3"}}"#;
        let widget = parse_mcp_app(response).unwrap();
        assert_eq!(widget.kind, WidgetKind::Table);
        assert_eq!(widget.id.as_deref(), Some("t1"));
    }

    #[test]
    fn test_parse_mcp_app_none() {
        assert!(parse_mcp_app("not a widget").is_none());
    }

    #[test]
    fn test_render_widget_text() {
        let w = WidgetDef::new(WidgetKind::Progress)
            .with_title("Build")
            .with_prop("current", "75")
            .with_prop("total", "100");
        let text = render_widget_text(&w);
        assert!(text.contains("Build"));
        assert!(text.contains("progress"));
    }

    #[test]
    fn test_render_widget_text_nested() {
        let parent = WidgetDef::new(WidgetKind::Form)
            .with_title("Config")
            .with_child(WidgetDef::new(WidgetKind::Markdown));
        let text = render_widget_text(&parent);
        assert!(text.contains("Config"));
        assert!(text.contains("markdown"));
    }

    #[test]
    fn test_form_field_types() {
        assert_eq!(FormFieldType::Text, FormFieldType::Text);
        assert_ne!(FormFieldType::Text, FormFieldType::Number);
        assert_ne!(FormFieldType::Select, FormFieldType::Checkbox);
    }

    #[test]
    fn test_trend_direction() {
        assert_eq!(TrendDirection::Up, TrendDirection::Up);
        assert_ne!(TrendDirection::Up, TrendDirection::Down);
    }
}
