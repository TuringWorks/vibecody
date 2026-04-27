// Library exposes modules also declared in main.rs (see CLAUDE.md "Module declaration pattern").
// Many items are reachable from the binary but not from any library pub API, which trips the
// per-crate dead_code lint. Silence at crate level rather than per-item.
#![allow(dead_code)]

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
pub mod a2a_http;
pub mod issue_triage;
pub mod web_grounding;
pub mod web_grounding_backend;
pub mod agent_host;
#[allow(dead_code)]
pub mod worktree_pool;
pub mod worktree_git;
pub mod proactive_agent;
pub mod proactive_scanner;
pub mod semantic_index;
pub mod doc_sync;
pub mod voice_local;
pub mod voice_whisper;
pub mod native_connectors;
pub mod agent_analytics;
pub mod agent_trust;
pub mod smart_deps;
pub mod rlcef_loop;
pub mod sketch_canvas;
pub mod mcp_streamable;
pub mod mcp_http;
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
pub mod container_runtime;
pub mod config;
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
// Memory-as-infrastructure redesign — Phase 2: single context assembler
// (depends on memory, workflow_orchestration, project_init below).
pub mod memory;
pub mod workflow_orchestration;
pub mod project_init;
pub mod context_assembler;
// Memory-as-infrastructure redesign — Phase 6: USER.md / MEMORY.md
// projections rendered from OpenMemory state.
pub mod memory_projections;
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
// Sandbox-tiers (S0/N0+): vibe-sandbox stack entry point.
pub mod sandbox_entry;
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

// Modules previously declared only in main.rs — exposed here so library consumers (e.g. vibe-ui Tauri shell) can use them. See CLAUDE.md "Module declaration pattern".
pub mod acp_protocol;
pub mod acp;
pub mod agent_modes;
pub mod agent_teams_v2_enhanced;
pub mod agent_teams_v2;
pub mod api_key_monitor;
pub mod app_builder;
pub mod ast_edit;
pub mod audio_output;
pub mod auto_research;
pub mod automations;
pub mod background_agents;
pub mod batch_builder;
pub mod blue_team;
pub mod branch_agent;
pub mod bugbot;
pub mod calendar_client;
pub mod ci_status_check;
pub mod ci;
pub mod clarifying_questions;
pub mod cloud_agent;
pub mod cloud_autofix;
pub mod cloud_ide;
pub mod cloud_providers;
pub mod cloud_sandbox;
pub mod code_replay;
pub mod code_review_agent;
pub mod compliance_controls;
pub mod compliance;
pub mod container_tool_executor;
pub mod context_bundles;
pub mod conversational_search;
pub mod database_client;
pub mod debug_mode;
pub mod design_import;
pub mod desktop_agent;
pub mod diff_viewer;
pub mod discovery;
pub mod discussion_mode;
pub mod distributed_training;
pub mod docgen;
pub mod docker_runtime;
pub mod document_ingest;
pub mod edit_prediction;
pub mod email_client;
pub mod explainable_agent;
pub mod fast_context;
pub mod feature_demo;
pub mod fine_tuning;
pub mod fullstack_gen;
pub mod gateway;
pub mod gh_actions_agent;
pub mod git_platform;
pub mod github_app;
pub mod gpu_cluster;
pub mod gpu_terminal;
pub mod handoff;
pub mod home_assistant;
pub mod idp;
pub mod image_gen_agent;
pub mod inference;
pub mod inference_routes;
pub mod inference_server;
pub mod infinite_context;
pub mod jetbrains_hooks;
pub mod knowledge_graph;
pub mod large_codebase_bench;
pub mod legacy_migration;
pub mod linear;
pub mod marketplace;
pub mod mcp_apps;
pub mod mcp_directory;
pub mod mcp_lazy;
pub mod mcp_server;
pub mod memory_recorder;
pub mod mermaid_ascii;
pub mod mobile_gateway;
pub mod multimodal_agent;
pub mod next_edit;
pub mod notebook;
pub mod observe_act;
pub mod opensandbox_client;
pub mod otel_init;
pub mod pairing;
pub mod plan_document;
pub mod plugin_lifecycle;
pub mod plugin_registry;
pub mod plugin_sdk;
pub mod plugin;
pub mod podman_runtime;
pub mod productivity;
pub mod profile;
pub mod purple_team;
pub mod qa_validation;
pub mod redteam;
pub mod remote_control;
pub mod render_optimize;
pub mod repl;
pub mod resource_manager;
pub mod review;
pub mod rl_env_os;
pub mod rl_eval_os;
pub mod rl_model_hub;
pub mod rl_observe;
pub mod rl_opti_os;
pub mod rl_rlhf;
pub mod rl_serve_os;
pub mod rl_train_os;
pub mod scheduler;
pub mod schema;
pub mod screen_recorder;
pub mod security_hardening;
pub mod security_scan;
pub mod security_scanning;
pub mod self_review;
pub mod semantic_mcp;
pub mod serve;
pub mod session_memory;
pub mod session_sharing;
pub mod setup;
pub mod soul_generator;
pub mod spec_pipeline;
pub mod spec;
pub mod speculative_exec;
pub mod streaming_client;
pub mod sub_agent_roles;
pub mod sub_agents;
pub mod swe_bench;
pub mod syntax;
pub mod team_governance;
pub mod team;
pub mod tool_executor;
pub mod transform;
pub mod tui;
pub mod usage_metering;
pub mod vector_db;
pub mod compressed_hnsw;
pub mod verification;
pub mod vm_orchestrator;
pub mod voice;
pub mod vscode_compat_ext;
pub mod vscode_sessions;
pub mod vulnerability_db;
pub mod web_client;
pub mod web_crawler;
pub mod workflow;
