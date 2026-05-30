// Library exposes modules also declared in main.rs (see CLAUDE.md "Module declaration pattern").
// Many items are reachable from the binary but not from any library pub API, which trips the
// per-crate dead_code lint. Silence at crate level rather than per-item.
#![allow(dead_code)]

pub mod a2a_http;
pub mod a2a_protocol;
pub mod agent_analytics;
pub mod agent_host;
pub mod agent_skills_compat;
pub mod agent_trust;
pub mod auth_util;
#[allow(dead_code)]
pub mod browser_agent;
pub mod cost_router;
pub mod counsel;
pub mod diff_review;
pub mod doc_sync;
pub mod issue_triage;
pub mod mcp_http;
pub mod mcp_streamable;
pub mod mcp_taint;
pub mod native_connectors;
pub mod proactive_agent;
pub mod proactive_scanner;
pub mod quantum_computing;
pub mod rag_taint;
pub mod redact;
pub mod rlcef_loop;
pub mod semantic_index;
pub mod signed_agent_card;
pub mod sketch_canvas;
pub mod smart_deps;
pub mod spawn_agent;
pub mod superbrain;
pub mod tainted;
pub mod tainted_http_bridge;
pub mod tainted_prompter;
pub mod voice_local;
pub mod voice_whisper;
pub mod web_grounding;
pub mod web_grounding_backend;
pub mod worktree_git;
#[allow(dead_code)]
pub mod worktree_pool;
// A5 — async subagent state machine.
pub mod async_subagent;
// A1 — MCP Apps payload parser/validator (SEP-1865).
pub mod mcp_apps_payload;
// A3 — /.well-known/mcp.json capability discovery.
pub mod mcp_well_known;
// A2 — MCPB bundle format pack/extract/verify.
pub mod mcpb_bundle;
pub mod mcts_repair;
pub mod visual_verify;
// A9 — cloud-agent session resume protocol.
pub mod session_resume_protocol;
// A8 — self-verifying agent loop with bounded iterations.
pub mod adapter_registry;
pub mod ai_code_review;
pub mod architecture_spec;
pub mod company_approvals;
pub mod company_budget;
pub mod company_cmd;
pub mod company_documents;
pub mod company_goals;
pub mod company_heartbeat;
pub mod company_meeting_notes;
pub mod company_orchestrator;
pub mod company_portability;
pub mod company_priority_map;
pub mod company_routines;
pub mod company_secrets;
pub mod company_store;
pub mod company_tasks;
pub mod company_workspace_config;
pub mod context_protocol;
pub mod health_score;
pub mod intent_refactor;
pub mod job_manager;
pub mod langgraph_bridge;
pub mod next_task;
pub mod policy_engine;
pub mod profile_store;
pub mod review_protocol;
pub mod self_improving_skills;
pub mod skill_distillation;
#[cfg(unix)]
pub mod subprocess_dispatch;
pub mod verify_loop;
pub mod webhook;
pub mod workspace_store;
// Phase 33-39: FIT-GAP v8
pub mod agent_await;
pub mod auto_deploy;
pub mod clawcode_compat;
pub mod collab_session;
pub mod cost_predictor;
pub mod design_mode;
pub mod env_dispatch;
pub mod hard_problem;
pub mod hybrid_search;
pub mod ide_bridge;
pub mod long_context;
pub mod mcp_governance;
pub mod msaf_compat;
pub mod nested_agents;
pub mod on_device;
pub mod polyglot_refactor;
pub mod reasoning_video;
pub mod repro_agent;
pub mod supply_chain;
pub mod team_onboarding;
pub mod test_gen;
pub mod thought_stream;
pub mod threat_model;
pub mod voice_vocab;
// FIT-GAP v9 — P2 modules
pub mod a11y_agent;
pub mod api_sketch;
pub mod perf_profiler;
pub mod schema_migration;
pub mod symbolic_exec;
pub mod temporal_debug;
// FIT-GAP v9 — P3 modules
pub mod federated_orchestrator;
pub mod incident_response;
pub mod local_embed_refresh;
pub mod workload_model_sel;
// Claw-code parity — Wave 1: correctness/reliability
pub mod session_health_probe;
pub mod tool_pair_compaction;
pub mod workspace_fingerprint;
// Claw-code parity — Wave 2: agent coordination
pub mod bash_classifier;
pub mod branch_lock;
pub mod recovery_recipe;
pub mod worker_bootstrap;
// Claw-code parity — Wave 3: governance
pub mod lane_events;
pub mod quality_gates;
pub mod stale_branch;
// Claw-code parity — Wave 4: config/hooks
pub mod config;
pub mod config_layers;
pub mod container_runtime;
pub mod hook_abort;
pub mod trust_resolution;
// FIT-GAP v10 — Phase 40: Execution Engine (P0)
pub mod context_budget;
pub mod parallel_tool_scheduler;
pub mod smart_diff;
// FIT-GAP v10 — Phase 41: Agent Intelligence (P1)
pub mod agent_state_machine;
pub mod cost_estimator;
pub mod file_watcher;
// FIT-GAP v10 — Phase 42: Reliability (P1)
pub mod rate_limit_backoff;
pub mod stream_patcher;
pub mod test_impact;
// FIT-GAP v10 — Phase 43: Developer Experience (P2)
pub mod auto_stub;
pub mod conversation_branch;
pub mod dep_visualizer;
// FIT-GAP v11 — Phase 45: Agent-OS (P0)
pub mod agent_autoscale;
pub mod agent_quota;
pub mod agent_recruiter;
pub mod agent_registry;
// FIT-GAP v11 — Phase 46: Context & Workspace (P1)
pub mod agent_persistence;
pub mod inline_diff;
pub mod multi_repo_context;
pub mod workspace_snapshot;
// FIT-GAP v11 — Phase 47: Developer Workflow (P2)
pub mod changelog_gen;
pub mod dep_update_advisor;
pub mod pr_description;
pub mod spec_to_test;
// FIT-GAP v10 — Phase 44: P3 Gaps (closed)
pub mod agent_replay;
pub mod ai_merge;
pub mod cache_advisor;
pub mod code_templates;
pub mod cursor_overlay;
pub mod plugin_marketplace;
pub mod symbol_rename;
pub mod voice_history;
// MemPalace techniques — LongMemEval benchmark
pub mod open_memory;
// A6 — multi-root workspace permission resolver.
pub mod workspace_roots;
// A4 — ACP server mode (Zed/JetBrains/Neovim, JSON-RPC over stdio).
pub mod acp_stdio;
// A10 — skills hot-reload watcher (companion to B1 SkillCatalog).
pub mod skill_watcher;
// Memory-as-infrastructure redesign — Phase 2: single context assembler
// (depends on memory, workflow_orchestration, project_init below).
pub mod context_assembler;
pub mod memory;
pub mod project_init;
pub mod workflow_orchestration;
// Memory-as-infrastructure redesign — Phase 6: USER.md / MEMORY.md
// projections rendered from OpenMemory state.
pub mod memory_projections;
// Recap & Resume — Phase F1.1 foundation. Cross-cutting `Recap` shape
// + Session-only heuristic generator. See docs/design/recap-resume/.
pub mod recap;
// Recap & Resume — Phase F1.3 resume surface (in-memory handle registry
// + pure helpers consumed by the /v1/resume routes in serve.rs).
pub mod resume;
// /goal — durable execution intent. See docs/design/goal/.
pub mod exec_goal;
// /goal — REPL handlers (display + direct-DB CRUD).
pub mod exec_goal_repl;
// Recap & Resume — Phase D1.1: diffcomplete chain types + encrypted
// store on workspace.db. Patent re-audit: PASS (1–5 unchanged).
pub mod diff_chain;
pub mod diff_chain_store;
pub mod mem_benchmark;
// FIT-GAP v11 — Phase 48: P3 Gaps (closed)
pub mod capability_discovery;
pub mod explain_depth;
pub mod perf_regression;
pub mod prompt_vcs;
pub mod repl_macros;
pub mod semantic_search_v2;
pub mod session_export;
pub mod token_dashboard;
// Design platform — multi-provider (Pencil, Penpot, Draw.io, Figma, in-house)
pub mod design_providers;
pub mod design_system_hub;
pub mod diagram_generator;
pub mod drawio_connector;
pub mod pencil_connector;
pub mod penpot_connector;
// FIT-GAP v12 — P1: reasoning, memory, caching, exploration, RPC
pub mod alt_explore;
pub mod app_server;
pub mod autodream;
pub mod context_handoff;
pub mod prompt_cache;
pub mod reasoning_provider;
// FIT-GAP v12 — P2: desktop automation, scheduling, plugins
pub mod computer_use;
pub mod plugin_bundle;
pub mod task_scheduler;
// B2.1 — `vibecli-plugin.toml` inner manifest carried inside an MCPB
// bundle. Outer container is `mcpb_bundle.rs` (A2); this defines the
// VibeCody-specific schema with publisher JWK + component lists.
pub mod plugin_manifest;
// B2.2 — detached P-256 ECDSA signing for `vibecli-plugin.toml`.
// Signature lives in a sibling `vibecli-plugin.sig` next to the
// manifest inside the extracted bundle. Verifier uses the publisher
// key embedded in the manifest itself (TOFU).
pub mod plugin_signing;
// B2.4 — core install function for signed MCPB plugin bundles.
// Brings B2.1 (manifest), B2.2 (verify), B2.3 (policy) together.
pub mod plugin_install;
// B2.5 — runtime view: walks the install dir, applies per-plugin
// policy, returns only the components that should actually load.
// Consumed by skill_catalog / mcp_governance / hook_abort / rules
// loader as a filtered input list (per-loader wiring is a follow-up).
pub mod plugin_runtime;
// FIT-GAP v12 — P3: long sessions, Windows sandbox, dispatch, focus
pub mod dispatch_remote;
pub mod focus_view;
pub mod long_session;
pub mod sandbox_windows;
// FIT-GAP v12 — P0: auto-approval, bwrap sandbox, GitHub Actions, lazy MCP, ZDR
pub mod auto_approve;
pub mod sandbox_bwrap;
// Sandbox-tiers (S0/N0+): vibe-sandbox stack entry point.
pub mod github_action;
pub mod mcp_tool_search;
pub mod sandbox_entry;
pub mod zdr_mode;
// Phase B2: Pluggable tool I/O (SSH, Docker, local, dry-run backends)
pub mod tool_operations;
// Pi-mono gap bridge — Phases A1-D1
pub mod dual_log;
pub mod message_queue;
pub mod oauth_login;
pub mod parallel_tools;
pub mod paste_guard;
pub mod path_guard;
pub mod rpc_mode;
pub mod session_share;
pub mod session_tree;
pub mod stream_tool_args;
pub mod thinking_levels;
pub mod tui_images;
pub mod tui_ime;
// Security Posture — unified scanner aggregator + finding shape +
// adapters for existing scanners + new scanners. See
// `docs/design/security-posture/`.
pub mod event_bus;
pub mod pod_manager;
pub mod security_posture;
pub mod security_posture_adapters;
pub mod security_posture_license;
pub mod security_posture_secrets;
pub mod security_posture_store;
pub mod security_posture_taint;
// Tailscale + ngrok connectivity (exposed for BDD test harnesses)
pub mod tailscale;
// Zero-config LAN discovery
pub mod mdns_announce;
// ngrok tunnel auto-detection and startup
pub mod ngrok;

// Apple Watch + Wear OS bridge
pub mod session_store;
pub mod watch_auth;
pub mod watch_bridge;
pub mod watch_session_relay;

// Modules previously declared only in main.rs — exposed here so library consumers (e.g. vibe-ui Tauri shell) can use them. See CLAUDE.md "Module declaration pattern".
pub mod acp;
pub mod acp_protocol;
pub mod agent_modes;
pub mod agent_teams_v2;
pub mod agent_teams_v2_enhanced;
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
pub mod ci;
pub mod ci_status_check;
pub mod clarifying_questions;
pub mod cloud_agent;
pub mod cloud_autofix;
pub mod cloud_ide;
pub mod cloud_providers;
pub mod cloud_sandbox;
pub mod code_replay;
pub mod code_review_agent;
pub mod compliance;
pub mod compliance_controls;
pub mod compressed_hnsw;
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
pub mod migrate;
pub mod mobile_gateway;
pub mod multimodal_agent;
pub mod next_edit;
pub mod notebook;
pub mod observe_act;
pub mod opensandbox_client;
pub mod otel_init;
pub mod pairing;
pub mod plan_document;
pub mod plugin;
pub mod plugin_lifecycle;
pub mod plugin_registry;
pub mod plugin_sdk;
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
pub mod rl_advanced;
pub mod rl_deploy;
pub mod rl_env_os;
pub mod rl_envs;
pub mod rl_eval;
pub mod rl_eval_os;
pub mod rl_executor;
pub mod rl_model_hub;
pub mod rl_observe;
pub mod rl_opti_os;
pub mod rl_policies;
pub mod rl_rlhf;
pub mod rl_runs;
pub mod rl_runtime;
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
pub mod skill_catalog;
pub mod soul_generator;
pub mod spec;
pub mod spec_pipeline;
pub mod speculative_exec;
pub mod streaming_client;
pub mod sub_agent_roles;
pub mod sub_agents;
pub mod swe_bench;
pub mod syntax;
pub mod team;
pub mod team_governance;
pub mod tool_executor;
pub mod transform;
pub mod tui;
pub mod usage_metering;
pub mod v1_messages;
pub mod vector_db;
pub mod verification;
pub mod vm_orchestrator;
pub mod voice;
pub mod vscode_compat_ext;
pub mod vscode_sessions;
pub mod vulnerability_db;
pub mod web_client;
pub mod web_crawler;
pub mod workflow;
