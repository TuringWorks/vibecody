//! LangGraph Bridge — Bridges VibeCLI agent pipelines with LangGraph-style
//! stateful graph execution, checkpointing, and event-driven orchestration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Tool,
    Agent,
    Router,
    Checkpoint,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: NodeType,
    pub name: String,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from_node: String,
    pub to_node: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StateValue {
    Text(String),
    Number(f64),
    Bool(bool),
    List(Vec<String>),
    Map(HashMap<String, String>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentState {
    pub values: HashMap<String, StateValue>,
    pub checkpoint_id: Option<String>,
    pub step_count: u32,
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            checkpoint_id: None,
            step_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub state: AgentState,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PipelineStatus {
    Idle,
    Running,
    Paused,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventType {
    NodeEnter,
    NodeExit,
    EdgeTraversal,
    CheckpointSaved,
    StateUpdated,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineEvent {
    pub event_type: EventType,
    pub node_id: Option<String>,
    pub data: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LangGraphPipeline {
    pub id: String,
    pub name: String,
    pub nodes: HashMap<String, GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub entry_node: String,
    pub checkpoints: Vec<Checkpoint>,
    pub status: PipelineStatus,
    pub state: AgentState,
    pub current_node: Option<String>,
}

impl LangGraphPipeline {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            nodes: HashMap::new(),
            edges: Vec::new(),
            entry_node: String::new(),
            checkpoints: Vec::new(),
            status: PipelineStatus::Idle,
            state: AgentState::new(),
            current_node: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Config & Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub api_port: u16,
    pub checkpoint_dir: String,
    pub max_steps: u32,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            api_port: 8765,
            checkpoint_dir: String::from("~/.vibecli/langgraph/checkpoints"),
            max_steps: 1000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeMetrics {
    pub pipelines_created: u64,
    pub steps_executed: u64,
    pub checkpoints_saved: u64,
    pub events_logged: u64,
}

impl Default for BridgeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl BridgeMetrics {
    pub fn new() -> Self {
        Self {
            pipelines_created: 0,
            steps_executed: 0,
            checkpoints_saved: 0,
            events_logged: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// LangGraphBridge
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangGraphBridge {
    pub pipelines: HashMap<String, LangGraphPipeline>,
    pub event_log: Vec<PipelineEvent>,
    pub config: BridgeConfig,
    pub metrics: BridgeMetrics,
}

impl Default for LangGraphBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl LangGraphBridge {
    /// Create a new bridge with default config.
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
            event_log: Vec::new(),
            config: BridgeConfig::default(),
            metrics: BridgeMetrics::new(),
        }
    }

    /// Create a new pipeline and register it.
    pub fn create_pipeline(&mut self, id: &str, name: &str) -> Result<String, String> {
        if self.pipelines.contains_key(id) {
            return Err(format!("Pipeline '{}' already exists", id));
        }
        let pipeline = LangGraphPipeline::new(id, name);
        self.pipelines.insert(id.to_string(), pipeline);
        self.metrics.pipelines_created += 1;
        self.log_event(EventType::StateUpdated, None, &format!("Pipeline '{}' created", id));
        Ok(id.to_string())
    }

    /// Add a node to an existing pipeline.
    pub fn add_node(
        &mut self,
        pipeline_id: &str,
        node_id: &str,
        node_type: NodeType,
        name: &str,
        config: HashMap<String, String>,
    ) -> Result<(), String> {
        let pipeline = self
            .pipelines
            .get_mut(pipeline_id)
            .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_id))?;
        if pipeline.nodes.contains_key(node_id) {
            return Err(format!("Node '{}' already exists in pipeline '{}'", node_id, pipeline_id));
        }
        let node = GraphNode {
            id: node_id.to_string(),
            node_type,
            name: name.to_string(),
            config,
        };
        pipeline.nodes.insert(node_id.to_string(), node);
        Ok(())
    }

    /// Add a directed edge between two nodes.
    pub fn add_edge(
        &mut self,
        pipeline_id: &str,
        from_node: &str,
        to_node: &str,
        condition: Option<String>,
    ) -> Result<(), String> {
        let pipeline = self
            .pipelines
            .get_mut(pipeline_id)
            .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_id))?;
        if !pipeline.nodes.contains_key(from_node) {
            return Err(format!("Source node '{}' not found", from_node));
        }
        if !pipeline.nodes.contains_key(to_node) {
            return Err(format!("Target node '{}' not found", to_node));
        }
        pipeline.edges.push(GraphEdge {
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            condition,
        });
        self.log_event(
            EventType::EdgeTraversal,
            Some(from_node),
            &format!("Edge added: {} -> {}", from_node, to_node),
        );
        Ok(())
    }

    /// Set the entry node for a pipeline.
    pub fn set_entry(&mut self, pipeline_id: &str, node_id: &str) -> Result<(), String> {
        let pipeline = self
            .pipelines
            .get_mut(pipeline_id)
            .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_id))?;
        if !pipeline.nodes.contains_key(node_id) {
            return Err(format!("Node '{}' not found in pipeline", node_id));
        }
        pipeline.entry_node = node_id.to_string();
        Ok(())
    }

    /// Execute one step — advance the pipeline state through the graph.
    /// Returns the node id that was executed.
    pub fn execute_step(&mut self, pipeline_id: &str) -> Result<String, String> {
        let max_steps = self.config.max_steps;

        let pipeline = self
            .pipelines
            .get_mut(pipeline_id)
            .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_id))?;

        if pipeline.entry_node.is_empty() {
            return Err("No entry node set".to_string());
        }
        if pipeline.nodes.is_empty() {
            return Err("Pipeline has no nodes".to_string());
        }

        // Determine the current node
        let current = match &pipeline.current_node {
            Some(n) => n.clone(),
            None => {
                pipeline.current_node = Some(pipeline.entry_node.clone());
                pipeline.status = PipelineStatus::Running;
                pipeline.entry_node.clone()
            }
        };

        // Check step limit
        if pipeline.state.step_count >= max_steps {
            pipeline.status = PipelineStatus::Failed("Max steps exceeded".to_string());
            return Err("Max steps exceeded".to_string());
        }

        // Check if current node is End
        let node = pipeline
            .nodes
            .get(&current)
            .ok_or_else(|| format!("Current node '{}' not found", current))?
            .clone();

        if node.node_type == NodeType::End {
            pipeline.status = PipelineStatus::Completed;
            return Err("Pipeline already completed (at End node)".to_string());
        }

        // "Execute" the node (simulated)
        pipeline.state.step_count += 1;
        pipeline.state.values.insert(
            format!("last_node_{}", pipeline.state.step_count),
            StateValue::Text(current.clone()),
        );

        // Find outgoing edges and advance to the first matching one
        let edges: Vec<GraphEdge> = pipeline
            .edges
            .iter()
            .filter(|e| e.from_node == current)
            .cloned()
            .collect();

        if let Some(edge) = edges.first() {
            pipeline.current_node = Some(edge.to_node.clone());
        } else {
            // No outgoing edges — mark completed
            pipeline.status = PipelineStatus::Completed;
        }

        let node_id = current.clone();
        let step = pipeline.state.step_count;
        let _ = pipeline;

        self.metrics.steps_executed += 1;
        self.log_event(
            EventType::NodeEnter,
            Some(&node_id),
            &format!("Executed node '{}' (step {})", node_id, step),
        );
        self.log_event(
            EventType::NodeExit,
            Some(&node_id),
            &format!("Exited node '{}'", node_id),
        );

        Ok(node_id)
    }

    /// Save a checkpoint for a pipeline.
    pub fn save_checkpoint(
        &mut self,
        pipeline_id: &str,
        checkpoint_id: &str,
        timestamp: u64,
    ) -> Result<(), String> {
        let pipeline = self
            .pipelines
            .get_mut(pipeline_id)
            .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_id))?;

        let mut state = pipeline.state.clone();
        state.checkpoint_id = Some(checkpoint_id.to_string());

        let checkpoint = Checkpoint {
            id: checkpoint_id.to_string(),
            state,
            timestamp,
            metadata: HashMap::new(),
        };
        pipeline.checkpoints.push(checkpoint);

        self.metrics.checkpoints_saved += 1;
        self.log_event(
            EventType::CheckpointSaved,
            None,
            &format!("Checkpoint '{}' saved for pipeline '{}'", checkpoint_id, pipeline_id),
        );
        Ok(())
    }

    /// Restore a pipeline state from a checkpoint.
    pub fn restore_checkpoint(
        &mut self,
        pipeline_id: &str,
        checkpoint_id: &str,
    ) -> Result<(), String> {
        let pipeline = self
            .pipelines
            .get_mut(pipeline_id)
            .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_id))?;

        let checkpoint = pipeline
            .checkpoints
            .iter()
            .find(|c| c.id == checkpoint_id)
            .ok_or_else(|| format!("Checkpoint '{}' not found", checkpoint_id))?
            .clone();

        pipeline.state = checkpoint.state;
        pipeline.status = PipelineStatus::Paused;
        pipeline.current_node = None; // will restart from entry on next step

        self.log_event(
            EventType::StateUpdated,
            None,
            &format!("Restored checkpoint '{}' for pipeline '{}'", checkpoint_id, pipeline_id),
        );
        Ok(())
    }

    /// Get a reference to a pipeline.
    pub fn get_pipeline(&self, pipeline_id: &str) -> Result<&LangGraphPipeline, String> {
        self.pipelines
            .get(pipeline_id)
            .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_id))
    }

    /// List all pipeline ids with their statuses.
    pub fn list_pipelines(&self) -> Vec<(String, String, PipelineStatus)> {
        self.pipelines
            .values()
            .map(|p| (p.id.clone(), p.name.clone(), p.status.clone()))
            .collect()
    }

    /// Export a pipeline graph to JSON.
    pub fn export_graph_json(&self, pipeline_id: &str) -> Result<String, String> {
        let pipeline = self.get_pipeline(pipeline_id)?;
        serde_json::to_string_pretty(pipeline)
            .map_err(|e| format!("JSON serialization failed: {}", e))
    }

    /// Import a pipeline from JSON.
    pub fn import_graph_json(&mut self, json: &str) -> Result<String, String> {
        let pipeline: LangGraphPipeline =
            serde_json::from_str(json).map_err(|e| format!("JSON parse failed: {}", e))?;
        let id = pipeline.id.clone();
        if self.pipelines.contains_key(&id) {
            return Err(format!("Pipeline '{}' already exists", id));
        }
        self.pipelines.insert(id.clone(), pipeline);
        self.metrics.pipelines_created += 1;
        Ok(id)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn log_event(&mut self, event_type: EventType, node_id: Option<&str>, data: &str) {
        self.event_log.push(PipelineEvent {
            event_type,
            node_id: node_id.map(String::from),
            data: data.to_string(),
            timestamp: self.event_log.len() as u64, // monotonic stand-in
        });
        self.metrics.events_logged += 1;
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bridge() -> LangGraphBridge {
        LangGraphBridge::new()
    }

    fn setup_simple_pipeline(bridge: &mut LangGraphBridge) -> String {
        let pid = "p1";
        bridge.create_pipeline(pid, "Test Pipeline").unwrap();
        bridge.add_node(pid, "start", NodeType::Agent, "Start Agent", HashMap::new()).unwrap();
        bridge.add_node(pid, "tool1", NodeType::Tool, "Search Tool", HashMap::new()).unwrap();
        bridge.add_node(pid, "end", NodeType::End, "End Node", HashMap::new()).unwrap();
        bridge.add_edge(pid, "start", "tool1", None).unwrap();
        bridge.add_edge(pid, "tool1", "end", None).unwrap();
        bridge.set_entry(pid, "start").unwrap();
        pid.to_string()
    }

    // -- Pipeline creation --

    #[test]
    fn test_create_pipeline() {
        let mut b = make_bridge();
        let id = b.create_pipeline("p1", "My Pipeline").unwrap();
        assert_eq!(id, "p1");
        assert_eq!(b.pipelines.len(), 1);
        assert_eq!(b.metrics.pipelines_created, 1);
    }

    #[test]
    fn test_create_duplicate_pipeline() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        let err = b.create_pipeline("p1", "B").unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn test_pipeline_initial_status() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        let p = b.get_pipeline("p1").unwrap();
        assert_eq!(p.status, PipelineStatus::Idle);
    }

    // -- Node management --

    #[test]
    fn test_add_node() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "n1", NodeType::Tool, "ToolNode", HashMap::new()).unwrap();
        let p = b.get_pipeline("p1").unwrap();
        assert_eq!(p.nodes.len(), 1);
        assert_eq!(p.nodes["n1"].name, "ToolNode");
    }

    #[test]
    fn test_add_node_with_config() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        let mut cfg = HashMap::new();
        cfg.insert("model".to_string(), "claude-3".to_string());
        b.add_node("p1", "n1", NodeType::Agent, "Claude", cfg).unwrap();
        let node = &b.get_pipeline("p1").unwrap().nodes["n1"];
        assert_eq!(node.config.get("model").unwrap(), "claude-3");
    }

    #[test]
    fn test_add_duplicate_node() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "n1", NodeType::Tool, "T", HashMap::new()).unwrap();
        let err = b.add_node("p1", "n1", NodeType::Agent, "A", HashMap::new()).unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn test_add_node_missing_pipeline() {
        let mut b = make_bridge();
        let err = b.add_node("nope", "n1", NodeType::Tool, "T", HashMap::new()).unwrap_err();
        assert!(err.contains("not found"));
    }

    // -- Edge management --

    #[test]
    fn test_add_edge() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "a", NodeType::Agent, "A", HashMap::new()).unwrap();
        b.add_node("p1", "b", NodeType::Tool, "B", HashMap::new()).unwrap();
        b.add_edge("p1", "a", "b", None).unwrap();
        assert_eq!(b.get_pipeline("p1").unwrap().edges.len(), 1);
    }

    #[test]
    fn test_add_edge_with_condition() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "a", NodeType::Router, "R", HashMap::new()).unwrap();
        b.add_node("p1", "b", NodeType::Tool, "T", HashMap::new()).unwrap();
        b.add_edge("p1", "a", "b", Some("score > 0.8".to_string())).unwrap();
        let edge = &b.get_pipeline("p1").unwrap().edges[0];
        assert_eq!(edge.condition, Some("score > 0.8".to_string()));
    }

    #[test]
    fn test_add_edge_missing_source() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "b", NodeType::Tool, "B", HashMap::new()).unwrap();
        let err = b.add_edge("p1", "missing", "b", None).unwrap_err();
        assert!(err.contains("Source node"));
    }

    #[test]
    fn test_add_edge_missing_target() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "a", NodeType::Agent, "A", HashMap::new()).unwrap();
        let err = b.add_edge("p1", "a", "missing", None).unwrap_err();
        assert!(err.contains("Target node"));
    }

    #[test]
    fn test_add_edge_missing_pipeline() {
        let mut b = make_bridge();
        let err = b.add_edge("nope", "a", "b", None).unwrap_err();
        assert!(err.contains("not found"));
    }

    // -- Entry node --

    #[test]
    fn test_set_entry() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "start", NodeType::Agent, "S", HashMap::new()).unwrap();
        b.set_entry("p1", "start").unwrap();
        assert_eq!(b.get_pipeline("p1").unwrap().entry_node, "start");
    }

    #[test]
    fn test_set_entry_missing_node() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        let err = b.set_entry("p1", "nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    // -- Step execution --

    #[test]
    fn test_execute_step_basic() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        let executed = b.execute_step(&pid).unwrap();
        assert_eq!(executed, "start");
        assert_eq!(b.get_pipeline(&pid).unwrap().state.step_count, 1);
    }

    #[test]
    fn test_execute_step_advances() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap(); // start -> tool1
        let executed = b.execute_step(&pid).unwrap(); // tool1 -> end
        assert_eq!(executed, "tool1");
        assert_eq!(b.get_pipeline(&pid).unwrap().current_node, Some("end".to_string()));
    }

    #[test]
    fn test_execute_step_completes_at_end() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap(); // start
        b.execute_step(&pid).unwrap(); // tool1
        let err = b.execute_step(&pid).unwrap_err(); // end node
        assert!(err.contains("completed"));
        assert_eq!(b.get_pipeline(&pid).unwrap().status, PipelineStatus::Completed);
    }

    #[test]
    fn test_execute_step_no_entry() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "n1", NodeType::Tool, "T", HashMap::new()).unwrap();
        let err = b.execute_step("p1").unwrap_err();
        assert!(err.contains("No entry node"));
    }

    #[test]
    fn test_execute_step_no_nodes() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        // Manually set entry without adding nodes
        b.pipelines.get_mut("p1").unwrap().entry_node = "ghost".to_string();
        let err = b.execute_step("p1").unwrap_err();
        assert!(err.contains("no nodes"));
    }

    #[test]
    fn test_execute_step_sets_running() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap();
        assert_eq!(b.get_pipeline(&pid).unwrap().status, PipelineStatus::Running);
    }

    #[test]
    fn test_execute_step_max_steps() {
        let mut b = make_bridge();
        b.config.max_steps = 1;
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "a", NodeType::Agent, "A", HashMap::new()).unwrap();
        b.add_node("p1", "b", NodeType::Agent, "B", HashMap::new()).unwrap();
        b.add_edge("p1", "a", "b", None).unwrap();
        b.add_edge("p1", "b", "a", None).unwrap(); // cycle
        b.set_entry("p1", "a").unwrap();
        b.execute_step("p1").unwrap(); // step 1
        let err = b.execute_step("p1").unwrap_err();
        assert!(err.contains("Max steps"));
    }

    #[test]
    fn test_execute_step_completes_no_outgoing_edges() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        b.add_node("p1", "solo", NodeType::Agent, "Solo", HashMap::new()).unwrap();
        b.set_entry("p1", "solo").unwrap();
        b.execute_step("p1").unwrap();
        assert_eq!(b.get_pipeline("p1").unwrap().status, PipelineStatus::Completed);
    }

    #[test]
    fn test_execute_step_metrics() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap();
        b.execute_step(&pid).unwrap();
        assert_eq!(b.metrics.steps_executed, 2);
    }

    // -- Checkpoints --

    #[test]
    fn test_save_checkpoint() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap();
        b.save_checkpoint(&pid, "cp1", 1000).unwrap();
        let p = b.get_pipeline(&pid).unwrap();
        assert_eq!(p.checkpoints.len(), 1);
        assert_eq!(p.checkpoints[0].id, "cp1");
        assert_eq!(b.metrics.checkpoints_saved, 1);
    }

    #[test]
    fn test_save_checkpoint_preserves_state() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap();
        b.save_checkpoint(&pid, "cp1", 100).unwrap();
        let cp = &b.get_pipeline(&pid).unwrap().checkpoints[0];
        assert_eq!(cp.state.step_count, 1);
        assert!(cp.state.checkpoint_id.is_some());
    }

    #[test]
    fn test_restore_checkpoint() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap();
        b.save_checkpoint(&pid, "cp1", 100).unwrap();
        b.execute_step(&pid).unwrap(); // advance further
        b.restore_checkpoint(&pid, "cp1").unwrap();
        let p = b.get_pipeline(&pid).unwrap();
        assert_eq!(p.state.step_count, 1); // back to checkpoint state
        assert_eq!(p.status, PipelineStatus::Paused);
    }

    #[test]
    fn test_restore_checkpoint_not_found() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        let err = b.restore_checkpoint(&pid, "nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_save_checkpoint_missing_pipeline() {
        let mut b = make_bridge();
        let err = b.save_checkpoint("nope", "cp1", 0).unwrap_err();
        assert!(err.contains("not found"));
    }

    // -- List / Get --

    #[test]
    fn test_list_pipelines() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "Alpha").unwrap();
        b.create_pipeline("p2", "Beta").unwrap();
        let list = b.list_pipelines();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_get_pipeline_not_found() {
        let b = make_bridge();
        let err = b.get_pipeline("nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    // -- JSON export / import --

    #[test]
    fn test_export_graph_json() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        let json = b.export_graph_json(&pid).unwrap();
        assert!(json.contains("Test Pipeline"));
        assert!(json.contains("start"));
    }

    #[test]
    fn test_import_graph_json() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        let json = b.export_graph_json(&pid).unwrap();
        // Import into a fresh bridge
        let mut b2 = make_bridge();
        let imported_id = b2.import_graph_json(&json).unwrap();
        assert_eq!(imported_id, "p1");
        assert_eq!(b2.pipelines.len(), 1);
    }

    #[test]
    fn test_import_duplicate_fails() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        let json = b.export_graph_json(&pid).unwrap();
        let err = b.import_graph_json(&json).unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn test_import_invalid_json() {
        let mut b = make_bridge();
        let err = b.import_graph_json("not json").unwrap_err();
        assert!(err.contains("JSON parse failed"));
    }

    #[test]
    fn test_export_missing_pipeline() {
        let b = make_bridge();
        let err = b.export_graph_json("nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    // -- Event logging --

    #[test]
    fn test_events_logged_on_create() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "A").unwrap();
        assert!(!b.event_log.is_empty());
        assert_eq!(b.event_log[0].event_type, EventType::StateUpdated);
    }

    #[test]
    fn test_events_logged_on_step() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        let before = b.event_log.len();
        b.execute_step(&pid).unwrap();
        assert!(b.event_log.len() > before);
    }

    #[test]
    fn test_events_logged_metric() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap();
        assert!(b.metrics.events_logged > 0);
    }

    // -- State manipulation --

    #[test]
    fn test_state_values() {
        let mut state = AgentState::new();
        state.values.insert("key".to_string(), StateValue::Text("hello".to_string()));
        state.values.insert("num".to_string(), StateValue::Number(42.0));
        state.values.insert("flag".to_string(), StateValue::Bool(true));
        assert_eq!(state.values.len(), 3);
    }

    #[test]
    fn test_state_value_list() {
        let val = StateValue::List(vec!["a".to_string(), "b".to_string()]);
        if let StateValue::List(v) = &val {
            assert_eq!(v.len(), 2);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_state_value_map() {
        let mut m = HashMap::new();
        m.insert("k".to_string(), "v".to_string());
        let val = StateValue::Map(m);
        if let StateValue::Map(map) = &val {
            assert_eq!(map.get("k").unwrap(), "v");
        } else {
            panic!("Expected Map");
        }
    }

    // -- Metrics --

    #[test]
    fn test_metrics_initial() {
        let b = make_bridge();
        assert_eq!(b.metrics.pipelines_created, 0);
        assert_eq!(b.metrics.steps_executed, 0);
        assert_eq!(b.metrics.checkpoints_saved, 0);
        assert_eq!(b.metrics.events_logged, 0);
    }

    #[test]
    fn test_metrics_after_full_run() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap();
        b.save_checkpoint(&pid, "cp1", 100).unwrap();
        assert_eq!(b.metrics.pipelines_created, 1);
        assert!(b.metrics.steps_executed >= 1);
        assert_eq!(b.metrics.checkpoints_saved, 1);
        assert!(b.metrics.events_logged > 0);
    }

    // -- Edge cases --

    #[test]
    fn test_node_type_variants() {
        let types = vec![NodeType::Tool, NodeType::Agent, NodeType::Router, NodeType::Checkpoint, NodeType::End];
        assert_eq!(types.len(), 5);
        assert_ne!(types[0], types[1]);
    }

    #[test]
    fn test_pipeline_status_variants() {
        let s1 = PipelineStatus::Idle;
        let s2 = PipelineStatus::Running;
        let s3 = PipelineStatus::Failed("oops".to_string());
        assert_ne!(s1, s2);
        if let PipelineStatus::Failed(msg) = &s3 {
            assert_eq!(msg, "oops");
        }
    }

    #[test]
    fn test_event_type_variants() {
        let evts = vec![
            EventType::NodeEnter,
            EventType::NodeExit,
            EventType::EdgeTraversal,
            EventType::CheckpointSaved,
            EventType::StateUpdated,
            EventType::Error,
        ];
        assert_eq!(evts.len(), 6);
    }

    #[test]
    fn test_bridge_config_defaults() {
        let cfg = BridgeConfig::default();
        assert_eq!(cfg.api_port, 8765);
        assert_eq!(cfg.max_steps, 1000);
        assert!(!cfg.checkpoint_dir.is_empty());
    }

    #[test]
    fn test_multiple_checkpoints() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap();
        b.save_checkpoint(&pid, "cp1", 100).unwrap();
        b.execute_step(&pid).unwrap();
        b.save_checkpoint(&pid, "cp2", 200).unwrap();
        let p = b.get_pipeline(&pid).unwrap();
        assert_eq!(p.checkpoints.len(), 2);
        assert_eq!(b.metrics.checkpoints_saved, 2);
    }

    #[test]
    fn test_router_node_with_conditional_edges() {
        let mut b = make_bridge();
        b.create_pipeline("p1", "Router Test").unwrap();
        b.add_node("p1", "router", NodeType::Router, "Router", HashMap::new()).unwrap();
        b.add_node("p1", "path_a", NodeType::Tool, "Path A", HashMap::new()).unwrap();
        b.add_node("p1", "path_b", NodeType::Tool, "Path B", HashMap::new()).unwrap();
        b.add_edge("p1", "router", "path_a", Some("confidence > 0.9".to_string())).unwrap();
        b.add_edge("p1", "router", "path_b", Some("confidence <= 0.9".to_string())).unwrap();
        b.set_entry("p1", "router").unwrap();
        // Executes — takes first edge (path_a)
        b.execute_step("p1").unwrap();
        assert_eq!(
            b.get_pipeline("p1").unwrap().current_node,
            Some("path_a".to_string())
        );
    }

    #[test]
    fn test_roundtrip_serialization() {
        let mut b = make_bridge();
        let pid = setup_simple_pipeline(&mut b);
        b.execute_step(&pid).unwrap();
        b.save_checkpoint(&pid, "cp1", 100).unwrap();
        let json = serde_json::to_string(&b).unwrap();
        let b2: LangGraphBridge = serde_json::from_str(&json).unwrap();
        assert_eq!(b2.pipelines.len(), 1);
        assert_eq!(b2.metrics.pipelines_created, 1);
    }
}
