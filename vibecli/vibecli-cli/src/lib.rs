pub mod diff_review;
pub mod cost_router;
pub mod agent_skills_compat;
pub mod quantum_computing;
pub mod spawn_agent;
#[allow(dead_code)]
pub mod browser_agent;
pub mod counsel;
pub mod superbrain;
pub mod a2a_protocol;
pub mod issue_triage;
pub mod web_grounding;
pub mod agent_host;
#[allow(dead_code)]
pub mod worktree_pool;
pub mod proactive_agent;
pub mod semantic_index;
pub mod doc_sync;
pub mod voice_local;
pub mod native_connectors;
pub mod agent_analytics;
pub mod agent_trust;
pub mod smart_deps;
pub mod rlcef_loop;
pub mod sketch_canvas;
pub mod mcp_streamable;
pub mod mcts_repair;
pub mod visual_verify;
pub mod next_task;
pub mod langgraph_bridge;
pub mod context_protocol;
pub mod health_score;
pub mod ai_code_review;
pub mod intent_refactor;
pub mod review_protocol;
pub mod skill_distillation;
pub mod self_improving_skills;
pub mod architecture_spec;
pub mod policy_engine;
pub mod company_store;
pub mod adapter_registry;
pub mod company_goals;
pub mod company_tasks;
pub mod company_cmd;
pub mod company_documents;
pub mod company_budget;
pub mod company_approvals;
pub mod company_secrets;
pub mod company_routines;
pub mod company_heartbeat;
pub mod company_workspace_config;
pub mod company_priority_map;
pub mod company_meeting_notes;
pub mod company_portability;
pub mod company_orchestrator;
pub mod profile_store;
pub mod workspace_store;
pub mod job_manager;
#[cfg(unix)]
pub mod subprocess_dispatch;
pub mod webhook;
// Phase 33-39: FIT-GAP v8
pub mod env_dispatch;
pub mod nested_agents;
pub mod mcp_governance;
pub mod msaf_compat;
pub mod agent_await;
pub mod thought_stream;
pub mod voice_vocab;
pub mod long_context;
pub mod design_mode;
pub mod ide_bridge;
pub mod on_device;
pub mod hard_problem;
pub mod auto_deploy;
pub mod clawcode_compat;
pub mod team_onboarding;
pub mod repro_agent;
pub mod test_gen;
pub mod polyglot_refactor;
pub mod supply_chain;
pub mod cost_predictor;
pub mod hybrid_search;
pub mod threat_model;
pub mod collab_session;
pub mod reasoning_video;
// FIT-GAP v9 — P2 modules
pub mod api_sketch;
pub mod a11y_agent;
pub mod perf_profiler;
pub mod temporal_debug;
pub mod symbolic_exec;
pub mod schema_migration;
// FIT-GAP v9 — P3 modules
pub mod federated_orchestrator;
pub mod incident_response;
pub mod local_embed_refresh;
pub mod workload_model_sel;
// Claw-code parity — Wave 1: correctness/reliability
pub mod workspace_fingerprint;
pub mod tool_pair_compaction;
pub mod session_health_probe;
// Claw-code parity — Wave 2: agent coordination
pub mod bash_classifier;
pub mod branch_lock;
pub mod worker_bootstrap;
pub mod recovery_recipe;
// Claw-code parity — Wave 3: governance
pub mod lane_events;
pub mod quality_gates;
pub mod stale_branch;
// Claw-code parity — Wave 4: config/hooks
pub mod trust_resolution;
pub mod config_layers;
pub mod hook_abort;
// FIT-GAP v10 — Phase 40: Execution Engine (P0)
pub mod parallel_tool_scheduler;
pub mod context_budget;
pub mod smart_diff;
// FIT-GAP v10 — Phase 41: Agent Intelligence (P1)
pub mod agent_state_machine;
pub mod file_watcher;
pub mod cost_estimator;
// FIT-GAP v10 — Phase 42: Reliability (P1)
pub mod rate_limit_backoff;
pub mod stream_patcher;
pub mod test_impact;
// FIT-GAP v10 — Phase 43: Developer Experience (P2)
pub mod conversation_branch;
pub mod dep_visualizer;
pub mod auto_stub;
// FIT-GAP v11 — Phase 45: Agent-OS (P0)
pub mod agent_registry;
pub mod agent_recruiter;
pub mod agent_quota;
pub mod agent_autoscale;
// FIT-GAP v11 — Phase 46: Context & Workspace (P1)
pub mod agent_persistence;
pub mod workspace_snapshot;
pub mod multi_repo_context;
pub mod inline_diff;
// FIT-GAP v11 — Phase 47: Developer Workflow (P2)
pub mod changelog_gen;
pub mod pr_description;
pub mod spec_to_test;
pub mod dep_update_advisor;
// FIT-GAP v10 — Phase 44: P3 Gaps (closed)
pub mod ai_merge;
pub mod symbol_rename;
pub mod code_templates;
pub mod cache_advisor;
pub mod voice_history;
pub mod agent_replay;
pub mod cursor_overlay;
pub mod plugin_marketplace;
// MemPalace techniques — LongMemEval benchmark
pub mod open_memory;
pub mod mem_benchmark;
// FIT-GAP v11 — Phase 48: P3 Gaps (closed)
pub mod token_dashboard;
pub mod session_export;
pub mod capability_discovery;
pub mod explain_depth;
pub mod perf_regression;
pub mod prompt_vcs;
pub mod repl_macros;
pub mod semantic_search_v2;
// Design platform — multi-provider (Pencil, Penpot, Draw.io, Figma, in-house)
pub mod design_providers;
pub mod drawio_connector;
pub mod pencil_connector;
pub mod penpot_connector;
pub mod diagram_generator;
pub mod design_system_hub;
// FIT-GAP v12 — P1: reasoning, memory, caching, exploration, RPC
pub mod reasoning_provider;
pub mod autodream;
pub mod prompt_cache;
pub mod alt_explore;
pub mod context_handoff;
pub mod app_server;
// FIT-GAP v12 — P2: desktop automation, scheduling, plugins
pub mod computer_use;
pub mod task_scheduler;
pub mod plugin_bundle;
// FIT-GAP v12 — P3: long sessions, Windows sandbox, dispatch, focus
pub mod long_session;
pub mod sandbox_windows;
pub mod dispatch_remote;
pub mod focus_view;
// FIT-GAP v12 — P0: auto-approval, bwrap sandbox, GitHub Actions, lazy MCP, ZDR
pub mod auto_approve;
pub mod sandbox_bwrap;
pub mod github_action;
pub mod mcp_tool_search;
pub mod zdr_mode;
// Phase B2: Pluggable tool I/O (SSH, Docker, local, dry-run backends)
pub mod tool_operations;
// Pi-mono gap bridge — Phases A1-D1
pub mod session_tree;
pub mod parallel_tools;
pub mod oauth_login;
pub mod message_queue;
pub mod stream_tool_args;
pub mod dual_log;
pub mod thinking_levels;
pub mod tui_images;
pub mod tui_ime;
pub mod session_share;
pub mod rpc_mode;
pub mod paste_guard;
pub mod event_bus;
pub mod pod_manager;
// Tailscale + ngrok connectivity (exposed for BDD test harnesses)
pub mod tailscale;
// Zero-config LAN discovery
pub mod mdns_announce;
// ngrok tunnel auto-detection and startup
pub mod ngrok;

// Apple Watch + Wear OS bridge
pub mod watch_auth;
pub mod watch_session_relay;
pub mod watch_bridge;
pub mod session_store;
