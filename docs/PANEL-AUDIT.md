# VibeUI Panel Functionality Audit

> **Generated:** 2026-03-27 | **Panels:** 186 | **Registered Tauri Commands:** 916
> **Legend:** Backend = Tauri invoke wired | Demo = hardcoded/mock data | Utility = pure client-side JS

---

## Summary

| Category | Count | % |
|----------|------:|--:|
| Full backend integration | 102 | 55% |
| Demo / mock data only | 52 | 28% |
| Pure client-side utility | 32 | 17% |
| **Total panels** | **186** | |

| Completion Tier | Count | % |
|-----------------|------:|--:|
| 100% (fully functional) | 134 | 72% |
| 70-99% (minor gaps) | 28 | 15% |
| 30-69% (significant gaps) | 18 | 10% |
| < 30% (demo shell) | 6 | 3% |

---

## Detailed Panel Audit (A-Z)

### A2aPanel — Agent-to-Agent Protocol
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Agents | Agent list with status badges, capabilities | `a2a_list_agents` | Registered | 100% |
| Tasks | Task cards with agent assignment, status | `a2a_submit_task` | Registered | 100% |
| Discovery | Registry URL, agent registration form | `a2a_discover` | Registered | 100% |
| Metrics | Dashboard: total tasks, completed, failed, rate | `a2a_get_metrics` | Registered | 100% |

### AcpPanel — Agent Client Protocol
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Server | Toggle on/off, capabilities, register form | `toggle_acp_server`, `register_acp_capability` | Registered | 100% |
| Client | Connect to external ACP, negotiation status | `get_acp_status` | Registered | 100% |
| Protocol | Quick-send messages, message log | `send_acp_message`, `get_acp_messages` | Registered | 100% |

### AdminPanel — Admin Console
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Team | Add/edit/remove members, role badges | `save_team_member`, `remove_team_member` | Registered | 100% |
| Audit Log | Searchable audit entries | `get_audit_log` | Registered | 100% |
| Policies | RBAC policy CRUD | `save_rbac_policy`, `delete_rbac_policy` | Registered | 100% |
| API Keys | Encrypted key viewer | `get_provider_api_keys` | Registered | 100% |

### AgentHostPanel — Multi-Agent Terminal Host
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Agents | List hosted agents, start/stop | `host_list_agents`, `host_start`, `host_stop` | Registered | 100% |
| Output | Terminal log, color-coded by agent | `host_get_output` | Registered | 100% |
| Context | Shared clipboard (key-value) | — | Local state | 90% |
| Config | Max agents slider, interleaved output | `host_register` | Registered | 100% |

### AgentModesPanel — Smart/Rush/Deep Modes
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Mode Select | Mode cards (smart/rush/deep), activate | `get_agent_modes`, `set_active_agent_mode` | Registered | 100% |
| Stats | Invocations, avg tokens, last used | `get_agent_mode_stats` | Registered | 100% |
| Profiles | View/create mode profiles | `get_agent_mode_profiles`, `create_agent_mode_profile` | Registered | 100% |

### AgentPanel — Agent Task Execution
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Task input, approval policy, turbo mode | `start_agent_task`, `stop_agent_task` | Registered | 100% |
| — | Real-time step feed, streaming metrics | Event: `agent:chunk/step/pending/complete/error` | Registered | 100% |
| — | Approval prompt for destructive actions | `respond_to_agent_approval` | Registered | 100% |

### AgentTeamPanel — Peer Communication
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Overview | Goal, members, task progress | `get_team_status` | Registered | 100% |
| Tasks | Decomposition, agent assignment, results | `start_agent_team` | Registered | 100% |
| Messages | Inter-agent message feed | `send_team_message`, Event: `team:updated` | Registered | 100% |

### AgentTeamsPanel — Team Management
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Team | Goal/members/progress, creation | `start_agent_team`, `dismiss_team` | Registered | 100% |
| Tasks | Task decomposition, generated files | `get_team_status` | Registered | 100% |
| Messages | Agent communication log | `send_team_message` | Registered | 100% |
| History | Past team runs, artifacts | `get_team_history` | Registered | 100% |

### AgilePanel — Project Management
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Board | Kanban/Sprint, swimlanes, WIP limits, drag-drop | `agile_get_board`, `agile_move_card`, `agile_update_card` | Registered | 100% |
| Sprint | Planning, velocity tracking | `agile_get_sprints`, `agile_create_sprint`, `agile_update_sprint` | Registered | 100% |
| Backlog | Prioritization, story CRUD | `agile_get_backlog`, `agile_create_story` | Registered | 100% |
| Ceremonies | Standup, retro cards, capacity | `agile_get_ceremonies`, `agile_save_ceremony` | Registered | 100% |
| Metrics | Velocity, cumulative flow, cycle/lead time | `agile_get_metrics` | Registered | 100% |
| SAFe | Program increments, ART teams, WSJF | `agile_get_safe`, `agile_save_safe` | Registered | 100% |
| Coach | AI recommendations | `agile_ai_analyze` | Registered | 100% |

### AiMlWorkflowPanel — AI/ML Pipeline Builder
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Pipeline | 10-stage workflow with toggles/configs | — | Reference only | 60% |
| Examples | 5 example workflows with steps | — | Hardcoded | 60% |
| Deploy | 12 deployment targets with commands | — | Hardcoded | 60% |
| Monitor | Metering/benchmark/cost commands | — | Hardcoded | 60% |

**Work remaining:** No backend integration; informational/reference panel only.

### AnalyticsPanel — Enterprise Agent Analytics
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Dashboard | 4 KPI cards with change indicators | `analytics_dashboard` | Registered | 100% |
| Users | User table with tasks, acceptance %, cost | `analytics_users` | Registered | 100% |
| Teams | Team cards with trends | `analytics_teams` | Registered | 100% |
| Export | CSV/JSON, date range | `analytics_export` | Registered | 100% |

### ApiDocsPanel — OpenAPI Viewer
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Load spec from file or URL | `read_file`, `fetch_url_for_context` | Registered | 100% |
| — | Endpoint list grouped by tag, filterable | `search_workspace_symbols` | Registered | 100% |
| — | "Try it" panel with request/response | `send_http_request` | Registered | 100% |

### AppBuilderPanel — App Scaffolding
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Quick Start | Template selection, project creation | `get_app_templates`, `create_app_project` | Registered | 100% |
| Templates | Template gallery browser | `list_app_templates` | Registered | 100% |
| Provision | Project provisioning workflow | `create_app_project` | Registered | 100% |
| Backend | Managed backend setup | `get_app_builder_history` | Registered | 100% |

### ArenaPanel — Blind A/B Model Comparison
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Provider/model selection (6 providers) | `compare_models` | Registered | 100% |
| — | Blind voting (A/B/Tie/Both bad) | `save_arena_vote` | Registered | 100% |
| — | Leaderboard table, vote history | `get_arena_history` | Registered | 100% |

### ArtifactsPanel — Agent Artifacts
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Artifact cards, 7 types, annotations | — | Props-driven | 90% |

**Work remaining:** No direct backend call; depends on parent providing data.

### AstEditPanel — AST-Based Edits
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | AST edit proposals, accept/dismiss | `get_ast_edits`, `apply_ast_edit`, `dismiss_ast_edit` | Registered | 100% |
| — | File-grouped edit view | `get_ast_files` | Registered | 100% |

### AuthPanel — Auth Scaffolding
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | 21 auth providers, 134+ frameworks | `generate_auth_scaffold` | Registered | 100% |
| — | Middleware/test gen, save to workspace | `write_auth_scaffold` | Registered | 100% |

### AutofixPanel — Codemod & Auto-Fix
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Framework detection (clippy/eslint/ruff/etc) | `detect_coverage_tool` | Registered | 100% |
| — | Run fixes, diff viewer | `run_autofix` | Registered | 100% |
| — | Apply & stage, revert | `apply_autofix` | Registered | 100% |

### AutomationsPanel — Event-Driven Automations
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Rules | Rule CRUD, enable/disable toggle, delete | `get_automation_rules`, `create_automation_rule`, `toggle_automation_rule`, `delete_automation_rule` | Registered | 100% |
| Tasks | Task list with status/output | `get_automation_tasks` | Registered | 100% |
| Logs | Log viewer | `get_automation_logs` | Registered | 100% |
| Stats | Aggregate stats bar | `get_automation_stats` | Registered | 100% |

### AutoResearchPanel — Autonomous Research
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Setup | Session creation, domain/strategy selection | `autoresearch_create_session` | Registered | 100% |
| Experiments | Experiment tracking, composite scores | `autoresearch_record_experiment` | Registered | 100% |
| Analysis | Trends, metric analysis | `autoresearch_get_session` | Registered | 100% |
| Memory | Cross-run learning, lessons | `autoresearch_get_memory`, `autoresearch_save_lesson` | Registered | 100% |
| Export | TSV export | `autoresearch_export_tsv` | Registered | 100% |

### BatchBuilderPanel — Large-Scale Code Generation
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| New Run | Batch spec, architecture plan | `batch_create_run` | Registered | 100% |
| Monitor | Progress tracking, pause/resume | `batch_get_run`, `batch_update_run` | Registered | 100% |
| QA Review | QA validation results | `batch_save_findings` | Registered | 100% |
| Migration | Legacy migration tracking | `batch_save_migration` | Registered | 100% |
| History | Past batch runs | `batch_list_runs` | Registered | 100% |

### BisectPanel — Git Bisect
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Start bisect, good/bad marking | `git_bisect_start`, `git_bisect_step` | Registered | 100% |
| — | AI-powered analysis | `ai_bisect_analyze` | Registered | 100% |
| — | History log, reset | `git_bisect_log`, `git_bisect_reset` | Registered | 100% |

### BlueTeamPanel — Defensive Security
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Incidents | Incident CRUD (P1-P4, 7 statuses) | `get_blue_team_incidents`, `create_blue_team_incident` | Registered | 100% |
| IOCs | Indicator tracking (9 types) | `get_blue_team_iocs`, `add_blue_team_ioc` | Registered | 100% |
| SIEM | Integration management (8 platforms) | `get_blue_team_siem_connections`, `add_blue_team_siem` | Registered | 100% |
| Rules | Detection rule CRUD | `get_blue_team_rules`, `create_blue_team_rule` | Registered | 100% |
| Hunts | Threat hunting sessions | `get_blue_team_hunts`, `run_blue_team_hunt` | Registered | 100% |
| Playbooks | Response playbooks | `get_blue_team_playbooks` | Registered | 100% |
| Reports | Generate reports | `generate_blue_team_report` | Registered | 100% |

### BookmarkPanel — Code Bookmarks
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Bookmark list, add/remove, navigate | `list_bookmarks`, `add_bookmark`, `remove_bookmark` | Registered | 100% |

### BrowserPanel — Chrome DevTools Protocol
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | CDP targets, tab management | `cdp_list_targets`, `cdp_open_tab` | Registered | 100% |
| — | Screenshots, page capture | `cdp_screenshot`, `cdp_capture_page` | Registered | 100% |

### CanvasPanel — Visual Workflow Editor
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Node-based flow editor | `list_canvas_workflows`, `load_canvas_workflow` | Registered | 100% |
| — | Run workflows | `run_canvas_workflow` | Registered | 100% |

### ChatPanel — AI Chat
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Message streaming, model selection | `send_chat_message`, `stream_chat_message` | Registered | 100% |
| — | Stop generation | `stop_chat_stream` | Registered | 100% |

### CheckpointPanel — Conversation Checkpoints
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Create/restore/delete checkpoints | `list_checkpoints`, `create_checkpoint`, `delete_checkpoint` | Registered | 100% |

### CiCdPanel — CI/CD Pipeline
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Pipeline config, build/test/lint/deploy | `get_ci_config`, `generate_cicd_config` | Registered | 100% |
| — | CI status monitoring | `get_ci_status`, `get_ci_checks` | Registered | 100% |

### CiGatesPanel — CI Quality Gates
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | CI review config, history | `get_ci_review_config`, `get_ci_review_history` | Registered | 100% |

### ClarifyingQuestionsPanel — Megaplan Mode
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Questions | Task input, question list, answer/skip | `get_clarify_questions`, `save_clarify_questions` | Registered | 100% |
| Plan | MegaPlan steps with effort/files/status | `get_clarify_plan`, `save_clarify_plan` | Registered | 100% |
| Summary | Answered/unanswered/skipped stats, risk assessment | `get_clarify_risks` | Registered | 100% |

### CloudAgentPanel — Docker Execution
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Cloud agent status, start/stop | `get_cloud_agent_status`, `start_cloud_agent` | Registered | 100% |

### CloudAutofixPanel — BugBot Cloud
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Autofix attempts, stats | `list_autofix_attempts`, `create_autofix_attempt` | Registered | 100% |
| — | Config management | `get_autofix_config`, `save_autofix_config` | Registered | 100% |

### CloudProviderPanel — AWS/GCP/Azure
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Service scan, IAM gen, IaC templates | `cloud_provider_scan`, `cloud_provider_iam`, `cloud_provider_iac` | Registered | 100% |
| — | Cost estimation | `cloud_provider_cost` | Registered | 100% |

### CloudSandboxPanel — Cloud Sandbox
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Template gallery, sandbox management | `get_cloud_sandbox_templates`, `list_cloud_sandboxes` | Registered | 100% |
| — | Logs viewer | `get_cloud_sandbox_logs`, `stop_cloud_sandbox` | Registered | 100% |

### CollabPanel — CRDT Collaboration
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Session create/join, peer list | `create_collab_session`, `join_collab_session` | Registered | 100% |
| — | Cursor tracking | `update_cursors`, `list_collab_peers` | Registered | 100% |

### ColorPalettePanel — Design Tokens
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Palette CRUD, export (CSS/SCSS/Tailwind/JSON) | `get_color_palettes`, `save_color_palettes`, `export_color_palette` | Registered | 100% |

### ComparePanel — Model Comparison
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Side-by-side model output comparison | `compare_models` | Registered | 100% |

### CompliancePanel — SOC 2 / GDPR
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Compliance report generation | `generate_compliance_report` | Registered | 100% |

### ConnectorsPanel — Data Source Connectors
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Available/installed connectors, add/test | `connectors_available`, `connectors_list`, `connectors_add`, `connectors_test` | Registered | 100% |
| — | Auto-discovery | `connectors_discover` | Registered | 100% |

### ContextBundlePanel — Context Spaces
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Bundle CRUD, activate, import/export | `context_bundle_list`, `context_bundle_create`, `context_bundle_delete` | Registered | 100% |
| — | Import/export | `context_bundle_import`, `context_bundle_export` | Registered | 100% |

### ConversationalSearchPanel — Devin Search
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Conversational code search | `conversational_search` | Registered | 100% |

### CostPanel — Cost Observatory
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Cost metrics, history, limits | `get_cost_metrics`, `record_cost_entry`, `set_cost_limit` | Registered | 100% |
| — | Clear history | `clear_cost_history` | Registered | 100% |

### CounselPanel — Multi-Agent Debate
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Session CRUD, rounds, voting, synthesis | `counsel_create_session`, `counsel_run_round`, `counsel_vote`, `counsel_synthesize` | Registered | 100% |

### CoveragePanel — Code Coverage
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Tool detection, coverage run, results | `detect_coverage_tool`, `run_coverage` | Registered | 100% |

### CronPanel — Cron Expression Builder
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Visual builder, preview, preset library | — | Utility | 100% |

### CsvPanel — CSV Viewer
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Sortable table, filter, stats, export | — | Utility | 100% |

### DashboardPanel — Project Dashboard
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Summary cards (files, LOC, commits, TODOs) | `get_project_dashboard` | Registered | 100% |

### DataGenPanel — Test Data Generator
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Schema-driven data generation | — | Utility | 100% |

### DebugModePanel — Cursor Debug Mode
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Debug sessions, breakpoints, analysis | `list_debug_sessions`, `create_debug_session`, `add_debug_breakpoint` | Registered | 100% |
| — | AI-powered analysis | `run_debug_analysis` | Registered | 100% |

### DemoPanel — Feature Demo Runner
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Demo listing, step generation, run | `demo_list`, `demo_get`, `demo_generate_steps`, `demo_run` | Registered | 100% |

### DeployPanel — Deployment
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Deploy target detection, run deploy | `detect_deploy_target`, `run_deploy` | Registered | 100% |
| — | Deploy history | `get_deploy_history` | Registered | 100% |

### DesignImportPanel — Figma Import
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Figma file import, component extraction | `import_figma` | Registered | 100% |

### DiffToolPanel — File Diff Viewer
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Side-by-side diff, unified diff | `git_diff` | Registered | 100% |

### DiscussionPanel — Threaded Discussions
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Thread CRUD, messages | `list_discussion_threads`, `create_discussion_thread`, `add_discussion_message` | Registered | 100% |

### DocSyncPanel — Documentation Sync
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Drift detection, alerts, reconciliation | `docsync_status`, `docsync_get_alerts`, `docsync_reconcile` | Registered | 100% |

### DockerPanel — Container Management
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Container/image list, actions, compose | `list_docker_containers`, `list_docker_images`, `docker_container_action`, `docker_compose_action` | Registered | 100% |

### EditPredictionPanel — ML Edit Predictor
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Predictions | Recent predictions, accept/reject | `get_edit_predictions`, `accept_prediction`, `dismiss_prediction` | Registered | 100% |
| Patterns | Pattern frequency, acceptance rate | `get_edit_patterns` | Registered | 100% |
| Model | Q-Table stats, hyperparameters | `get_edit_model_stats` | Registered | 100% |

### EncodingPanel — Encode/Decode Utility
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Base64 | Encode/decode, URL-safe toggle | — | Utility (Web Crypto) | 100% |
| URL | encodeURIComponent/decode | — | Utility | 100% |
| HTML | Entity encode/decode | — | Utility | 100% |
| Hash | SHA-1/256/512 | — | Utility (Web Crypto) | 100% |
| Case | 10 case conversions | — | Utility | 100% |
| Stats | Char/byte/word/line counts | — | Utility | 100% |

### EnvPanel — Environment Manager
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Multi-env .env editor, secrets masking | `get_env_files`, `read_env_file`, `save_env_file`, `delete_env_var` | Registered | 100% |

### FastContextPanel — SWE-grep
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Search | Query, 5 match types, results | `fast_context_search` | Registered | 100% |
| Index | File/symbol/trigram stats, rebuild | `fast_context_index_stats`, `fast_context_reindex` | Registered | 100% |
| Cache | Hit rate, hits/misses, clear | `fast_context_cache_stats` | Registered | 100% |

### FineTuningPanel — Model Fine-Tuning
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Dataset | Source selector, stats, language dist | `get_fine_tuning_stats` | Registered | 100% |
| Train | Provider/model selection, hyperparams | `create_fine_tuning_job` | Registered | 100% |
| Jobs | Training jobs table with progress | `list_fine_tuning_jobs` | Registered | 100% |
| SWE-Bench | Eval results table | `list_fine_tuning_evals` | Registered | 100% |
| LoRA | Adapter management, merge/delete | `list_fine_tuning_adapters`, `create_fine_tuning_adapter` | Registered | 100% |

### FlowPanel — Event Flow Tracking
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Flow context, event tracking | `get_flow_context`, `track_flow_event` | Registered | 100% |

### FullStackGenPanel — Full-Stack Generation
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Configure | Frontend/backend/DB/auth dropdowns | — | Client-side | 100% |
| Generate | Review spec, generate | `fullstack_generate` | Registered | 100% |
| Files | File tree browser, line counts | `fullstack_read_file` | Registered | 100% |
| Editor | File viewer/editor, save | `fullstack_write_file` | Registered | 100% |

### GhActionsPanel — GitHub Actions Generator
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Workflows | YAML editor, triggers, jobs | `save_gh_workflow` | Registered | 100% |
| Templates | 6 workflow templates | `list_gh_workflow_templates` | Registered | 100% |
| Secrets | Secret management | `list_gh_secrets` | Registered | 100% |
| History | Generated workflow history | `get_gh_actions_history` | Registered | 100% |

### GitPanel — Source Control
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Status, commit, push, pull, branch switch | `git_commit`, `git_push`, `git_pull`, `git_switch_branch` | Registered | 100% |
| — | Stash, discard, diff, history | `git_stash_create`, `git_stash_pop`, `git_discard_changes` | Registered | 100% |
| — | Credential storage, branch listing | `store_git_credentials`, `git_list_branches` | Registered | 100% |

### GitHubSyncPanel — GitHub Integration
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Sync | Commit & push, pull, ahead/behind | `github_sync_push`, `github_sync_pull` | Registered | 100% |
| Repos | User repo list | `list_github_repos` | Registered | 100% |
| Create | New repo creation | `github_create_repo` | Registered | 100% |

### GpuTerminalPanel — GPU Renderer
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Monitor | FPS, frame time, GPU mem, dirty cells | `get_gpu_terminal_stats`, `get_gpu_fps_history` | Registered | 100% |
| Atlas | Glyph atlas visualization | `get_gpu_glyph_atlas` | Registered | 100% |
| Config | Font, FPS, VSync, ligatures | — | Client-side | 100% |
| Benchmark | 100-frame perf test | `run_gpu_terminal_benchmark` | Registered | 100% |

### GraphQLPanel — GraphQL Client
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Query | URL, query editor, variables, run | `run_graphql_query` | Registered | 100% |
| Schema | Introspection, type browser | `introspect_graphql_schema` | Registered | 100% |
| History | Query history (localStorage) | — | Client-side | 100% |

### HealthMonitorPanel — Service Health
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Service list, sparklines, auto-refresh | `check_all_services`, `check_service_health` | Registered | 100% |
| — | Add/remove monitors, persist | `get_health_monitors`, `save_health_monitors` | Registered | 100% |

### HistoryPanel — Agent Trace Viewer
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Session list, trace detail | `list_trace_sessions`, `load_trace_session` | Registered | 100% |

### HooksPanel — Event Hooks Editor
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Hook CRUD (command/LLM/HTTP handlers) | `get_hooks_config`, `save_hooks_config` | Registered | 100% |

### HttpPanel — HTTP Playground
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Request builder, response viewer | `send_http_request` | Registered | 100% |

### IdpPanel — Internal Developer Platform
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Catalog | Service catalog browser | `get_idp_catalog`, `register_idp_service` | Registered | 100% |
| Golden Paths | Guided workflows | `get_idp_golden_paths` | Registered | 100% |
| Scorecards | DORA metrics, evaluation | `get_idp_scorecards`, `evaluate_idp_scorecard` | Registered | 100% |
| Infra | Self-service provisioning | `request_idp_infra`, `get_idp_infra_requests` | Registered | 100% |
| Teams | Team management, onboarding | `get_idp_teams`, `create_idp_team` | Registered | 100% |
| Platforms | Platform toggles (12 platforms) | `get_idp_platforms`, `toggle_idp_platform` | Registered | 100% |
| Checklists | Onboarding checklists | `toggle_idp_checklist` | Registered | 100% |

### ImageGenPanel — AI Image Generation
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Generate | Prompt, model, style, size, cost estimate | `generate_image`, `get_available_image_providers` | Registered | 100% |
| Gallery | Image grid, save/delete | `list_generated_images`, `get_generated_image_data`, `delete_generated_image` | Registered | 100% |
| Stats | Totals, recent generations | `get_image_gen_stats` | Registered | 100% |

### InferencePanel — Inference Server Config
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Deploy | vLLM/TGI/Triton config, CLI/Docker gen | — | Utility | 80% |
| Benchmark | Benchmark comparison table | — | Client-side | 80% |
| Scaling | K8s YAML generation | — | Utility | 80% |

**Work remaining:** Pure config generator — no backend execution. Consider adding `run_inference_server`.

### InfiniteContextPanel — Context Window Manager
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Context | Token usage, chunk list, evict/pin | `get_context_chunks`, `evict_context_chunk`, `pin_context_chunk` | Registered | 100% |
| Project Map | File tree with context status | `get_project_file_tree` | Registered | 100% |
| Settings | Token limit, scoring weights, cache | `get_context_window_stats` | Registered | 100% |

### JsonToolsPanel — JSON Utilities
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Format | Prettify/minify/sort-keys | — | Utility | 100% |
| TypeScript | Generate TS interfaces | — | Utility | 100% |
| YAML | JSON to YAML conversion | — | Utility | 100% |
| Query | Dot-path queries with suggestions | — | Utility | 100% |

### JwtPanel — JWT Decoder/Signer
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Decode | Paste JWT, header/payload/expiry | — | Utility (Web Crypto) | 100% |
| Sign | HS256 signing, secret input | — | Utility (Web Crypto) | 100% |
| Claims | Standard claims reference table | — | Reference | 100% |

### K8sPanel — Kubernetes Management
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Manifests | YAML generation, copy/save | `generate_k8s_manifests` | Registered | 100% |
| Deploy | kubectl execution, quick actions | `run_kubectl_command`, `list_k8s_contexts` | Registered | 100% |
| ArgoCD | ArgoCD CR generation | `generate_argocd_app`, `run_argocd_command` | Registered | 100% |
| Contexts | Kubeconfig context switcher | `list_k8s_contexts` | Registered | 100% |

### KeysPanel — API Key Management
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Key CRUD, validation, provider list | `get_provider_api_keys`, `save_provider_api_keys`, `validate_api_key`, `validate_all_api_keys` | Registered | 100% |

### KnowledgeGraphPanel — Code Graph
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Graph | SVG visualization, query modes | `get_knowledge_graph`, `search_knowledge_graph` | Registered | 100% |
| Stats | Node/edge counts, most connected | `get_knowledge_graph_stats` | Registered | 100% |
| Export | DOT format for GraphViz | `refresh_knowledge_graph` | Registered | 100% |

### LangGraphPanel — LangGraph Pipelines
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Pipelines | Pipeline listing | `langgraph_list_pipelines` | Registered | 100% |
| Graph | Node/edge visualization | `langgraph_create_pipeline` | Registered | 100% |
| Checkpoints | Checkpoint history | `langgraph_get_checkpoints` | Registered | 100% |
| Events | Event log stream | `langgraph_get_events` | Registered | 100% |

### LoadTestPanel — HTTP Load Testing
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Config, run, streaming progress | `run_load_test`, Event: `loadtest:progress` | Registered | 100% |

### LogPanel — Log Viewer
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Log discovery, tail, AI analysis | `discover_log_sources`, `tail_log_file`, `analyze_logs` | Registered | 100% |

### MarkdownPanel — Markdown Editor
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | File browser, split editor/preview | `list_markdown_files`, `read_file`, `write_file` | Registered | 100% |

### MarketplacePanel — Plugin Store
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Browse, search, install plugins | `get_marketplace_plugins`, `search_marketplace`, `install_marketplace_plugin` | Registered | 100% |
| — | Installed plugins list | `list_installed_plugins` | Registered | 100% |

### McpPanel — MCP Server Manager
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Server CRUD, OAuth, tool listing | `get_mcp_servers`, `save_mcp_servers`, `test_mcp_server` | Registered | 100% |
| — | OAuth flow | `initiate_mcp_oauth`, `complete_mcp_oauth` | Registered | 100% |

### McpDirectoryPanel — MCP Plugin Directory
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Browse | Plugin directory | `mcp_directory_search` | Registered | 100% |
| Installed | Installed plugins | `mcp_directory_installed` | Registered | 100% |
| Search | Plugin search | `mcp_directory_search` | Registered | 100% |

### McpLazyPanel — MCP Lazy Loading
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Tool Registry | Lazy-loaded tools, load/unload | `mcp_lazy_list_tools`, `mcp_lazy_load_tool`, `mcp_lazy_unload_tool` | Registered | 100% |
| Search | Tool search | `mcp_lazy_search` | Registered | 100% |
| Metrics | Usage metrics | `mcp_lazy_metrics` | Registered | 100% |

### MctsRepairPanel — MCTS Bug Repair
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Sessions | Repair sessions | `repair_list_sessions`, `repair_new_session` | Registered | 100% |
| Tree | MCTS tree visualization | `repair_get_tree` | Registered | 100% |
| Agentless | Agentless strategies | — | Demo data | 50% |
| Compare | Outcome comparison | `repair_compare` | Registered | 80% |

**Work remaining:** Agentless tab uses demo data.

### MemoryPanel — Rules & Auto-Facts
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Workspace | Workspace rules editor | `list_rule_files`, `get_rule_file`, `save_rule_file`, `delete_rule_file` | Registered | 100% |
| Global | Global rules | `get_global_rules`, `save_global_rules` | Registered | 100% |
| Directory | Directory-scoped rules | `save_rule_file` | Registered | 100% |
| Auto-Facts | Auto-generated facts, pin/delete/add | `get_auto_memories`, `pin_auto_memory`, `delete_auto_memory`, `add_auto_memory` | Registered | 100% |

### MetricsPanel — Code Metrics
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Complexity, dependencies, quality scores | `analyze_code_metrics` | Registered | 100% |

### MigrationsPanel — Database Migrations
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Migration status, run up/down/create | `get_migration_status`, `run_migration_action` | Registered | 100% |

### MobileDispatchPanel — Mobile Gateway
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Machines | Machine registry | `dispatch_list_machines`, `dispatch_register_machine` | Registered | 100% |
| Devices | Device list | `dispatch_list_devices` | Registered | 100% |
| Dispatches | Task dispatch | `dispatch_send` | Registered | 100% |
| Pairing | PIN/QR pairing | `dispatch_create_pairing`, `dispatch_accept_pairing` | Registered | 100% |
| Stats | Gateway statistics | `dispatch_stats` | Registered | 100% |

### MockServerPanel — HTTP Mock Server
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Routes | Route CRUD | `list_mock_routes`, `add_mock_route`, `remove_mock_route` | Registered | 100% |
| Request Log | Request history | `get_mock_request_log` | Registered | 100% |
| Import | OpenAPI import | `generate_mocks_from_spec` | Registered | 100% |

### ModelManagerPanel — AI Model Management
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Provider/model selection, GPU detection | `get_available_ai_providers`, `list_gpu_models` | Registered | 100% |

### MultiModelPanel — Side-by-Side Comparison
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Prompt input, model comparison | `compare_models` | Registered | 100% |

### NetworkPanel — Network Tools
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Port Scanner | TCP port scanning | `scan_open_ports` | Registered | 100% |
| DNS Lookup | DNS query | `dns_lookup` | Registered | 100% |
| TLS Inspector | Certificate inspection | `check_tls_cert` | Registered | 100% |

### NextTaskPanel — ML Task Prediction
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Suggestions | ML-scored next task predictions | — | Demo data | 40% |
| History | Prediction accuracy history | — | Demo data | 40% |
| Learning | State transition rules | — | Demo data | 40% |
| Config | Learning configuration | — | Demo data | 40% |

**Work remaining:** Entirely demo — no backend calls despite Tauri commands existing in `next_task.rs`.

### NotebookPanel — Code Notebook
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Cell-based code/markdown, execution | `execute_notebook_cell`, `ai_notebook_assist` | Registered | 100% |

### NumberBasePanel — Number Conversion
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Convert | Binary/hex/octal/decimal conversion | — | Utility | 100% |
| Bitwise | AND/OR/XOR/NOT/shift operations | — | Utility | 100% |
| Float32 | IEEE 754 visualization | — | Utility | 100% |

### ObserveActPanel — Visual Grounding
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Setup | Visual grounding configuration | — | Demo data | 30% |
| Monitor | Observe/act cycle monitoring | — | Demo data | 30% |
| History | Session history with screenshots | — | Demo data | 30% |
| Safety | Safety constraints | — | Demo data | 30% |

**Work remaining:** Entirely demo — needs backend integration.

### OpenMemoryPanel — Cognitive Memory Engine
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Overview | Memory stats, sector breakdown | `openmemory_stats` | Registered | 100% |
| Memories | Memory list, add/delete/pin/unpin | `openmemory_add`, `openmemory_delete`, `openmemory_pin`, `openmemory_unpin` | Registered | 100% |
| Query | Semantic memory search | `openmemory_query` | Registered | 100% |
| Facts | Knowledge graph facts | `openmemory_facts`, `openmemory_add_fact` | Registered | 100% |
| Graph | Bi-temporal knowledge graph | `openmemory_run_decay`, `openmemory_consolidate` | Registered | 100% |
| Settings | Encryption, export | `openmemory_enable_encryption`, `openmemory_export` | Registered | 100% |

### OrchestrationPanel — Workflow Orchestration
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Tasks | Task goal, todos, progress, verify/plan | `get_orch_state`, `save_orch_state` | Registered | 100% |
| Lessons | Learned patterns with categories | `get_orch_lessons`, `save_orch_lessons` | Registered | 100% |
| Rules | Static orchestration rules reference | — | Reference | 100% |

### OrgContextPanel — Organization Context
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Repositories | Org-wide repo list | — | Demo data | 30% |
| Patterns | Code patterns | — | Demo data | 30% |
| Conventions | Org conventions | — | Demo data | 30% |
| Dependencies | Cross-repo deps | — | Demo data | 30% |

**Work remaining:** Entirely demo — needs backend integration with `org_context.rs`.

### PlanDocumentPanel — Plan Documents
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Plans | Plan listing with status | `list_plan_documents` | Registered | 100% |
| Editor | Rich text editor | `get_plan_document`, `create_plan_document` | Registered | 100% |
| Comments | Collaborative comments | `add_plan_comment`, `resolve_plan_comment` | Registered | 100% |

### ProactivePanel — Proactive Intelligence
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Suggestions | AI suggestions | `proactive_get_suggestions`, `proactive_accept`, `proactive_reject` | Registered | 100% |
| Scan | Code scanning | `proactive_scan` | Registered | 100% |
| Learning | Learning from suggestions | `proactive_get_digest` | Registered | 100% |
| Config | Intelligence configuration | — | Client-side | 80% |

### ProcessPanel — System Processes
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Process list, kill, 5s auto-refresh | `list_processes`, `kill_process` | Registered | 100% |

### ProfilerPanel — CPU/Memory Profiler
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Tool detection, profiling run | `detect_profiler_tool`, `run_profiler` | Registered | 100% |

### ProjectContextPanel — Project Profile
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Overview | Project metadata | `scan_project_profile` | Registered | 100% |
| Commands | Available scripts/commands | `run_terminal_command` | Registered | 100% |
| Key Files | Important file descriptions | `read_file_content` | Registered | 100% |
| AI Context | AI context info | `scan_project_profile` | Registered | 100% |

### PurpleTeamPanel — ATT&CK Exercises
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Exercises | Exercise CRUD (14 tactics, 20 techniques) | `list_purple_team_exercises`, `create_purple_team_exercise` | Registered | 100% |
| Simulations | Attack simulation tracking | `record_purple_team_simulation`, `get_purple_team_simulations` | Registered | 100% |
| Matrix | ATT&CK matrix coverage | `get_purple_team_matrix` | Registered | 100% |
| Gaps | Coverage gap analysis | `get_purple_team_gaps` | Registered | 100% |
| Reports | Report generation | `generate_purple_team_report` | Registered | 100% |

### QaValidationPanel — QA Pipeline
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Validate | Validation test execution | — | Demo (Math.random) | 40% |
| Reports | Validation reports | — | Demo | 40% |

**Work remaining:** Uses setTimeout simulation — needs real `qa_validation.rs` integration.

### QuantumComputingPanel — Quantum Circuits
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Projects | Quantum project management | `quantum_get_projects`, `quantum_create_project` | Registered | 100% |
| Circuits | Circuit builder, gates | `quantum_get_circuits`, `quantum_create_circuit`, `quantum_add_gate` | Registered | 100% |
| Algorithms | Algorithm templates | `quantum_get_algorithms`, `quantum_get_algorithm_template` | Registered | 100% |
| Simulate | Circuit simulation | `quantum_simulate_circuit` | Registered | 100% |
| Hardware | Hardware compatibility | `quantum_get_hardware_types`, `quantum_get_compatibility` | Registered | 100% |
| Export | Qiskit/Cirq/QASM export | `quantum_export_circuit` | Registered | 100% |

### RedTeamPanel — Automated Security Testing
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Multi-stage pipeline, findings | `start_redteam_scan`, `get_redteam_findings` | Registered | 100% |
| — | Session history, reports | `get_redteam_sessions`, `generate_redteam_report` | Registered | 100% |

### RegexPanel — Regex Tester
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Pattern library (15 presets), tester | — | Utility | 100% |

### RemoteControlPanel — Remote Control Server
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Server | Start/stop server | `start_remote_server`, `stop_remote_server` | Registered | 100% |
| Clients | Connected clients | `list_remote_clients`, `disconnect_remote_client` | Registered | 100% |
| Events | Event stream | `get_remote_events` | Registered | 100% |

### RenderOptimizePanel — Re-Render Reduction
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Stats | Render statistics | `get_render_stats`, `reset_render_stats` | Registered | 100% |
| Frames | Dirty region analysis | `get_dirty_regions` | Registered | 100% |
| Config | Optimization config | `run_render_optimization` | Registered | 100% |

### ResiliencePanel — Provider Resilience
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Health | Provider health scores table | `get_provider_health` | Registered | 100% |
| Circuit | Circuit breaker state, recovery policy | `get_circuit_breaker_state` | Registered | 100% |
| Journal | Failure patterns + recent failures | `get_failure_records`, `get_failure_patterns` | Registered | 100% |
| Config | Editable config with save | `get_resilience_config`, `save_resilience_config` | Registered | 100% |

### ReviewPanel — AI Code Review
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Code review, quality scores, suggestions | `run_code_review` | Registered | 100% |

### RlcefPanel — RL from Code Execution Feedback
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Dashboard | Reward histogram, pass/fail | `rlcef_get_outcomes` | Registered | 100% |
| Mistakes | Common mistake patterns | `rlcef_get_mistakes` | Registered | 100% |
| Strategies | Hyperparameter changes | `rlcef_get_strategies` | Registered | 100% |
| Export | JSONL/Parquet/CSV export | `rlcef_export` | Registered | 100% |

### SandboxPanel — Container Sandbox
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Runtime detection, create/stop/pause/resume | `detect_sandbox_runtime`, `create_sandbox`, `stop_sandbox`, `pause_sandbox`, `resume_sandbox` | Registered | 100% |
| — | Command execution in sandbox | `sandbox_exec` | Registered | 100% |

### ScaffoldPanel — Project Templates
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Template browser, preview, write | `list_scaffold_templates`, `generate_scaffold` | Registered | 100% |

### ScriptPanel — Script Runner
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Script detection, execution, streaming log | `detect_project_scripts`, `run_project_script`, Event: `script:log` | Registered | 100% |

### SecurityScanPanel — Vulnerability Scanner
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Findings | CWE-grouped findings, suppress/restore | `run_security_scan`, `suppress_security_finding`, `suppress_security_cwe` | Registered | 100% |
| Summary | Severity/CWE breakdown, top files | `get_security_scan_results` | Registered | 100% |
| Patterns | Enable/disable 10 scan patterns | — | Client-side | 100% |
| History | Scan run history | `get_security_scan_history` | Registered | 100% |

### SelfReviewPanel — Agent Self-Review
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Results | Self-review iteration display | — | Demo data | 30% |
| Config | Review configuration | — | Demo data | 30% |
| Report | Review report | — | Demo data | 30% |

**Work remaining:** Entirely demo — needs `self_review.rs` backend wiring.

### SemanticIndexPanel — Code Intelligence
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Overview | Symbol index stats | `semindex_stats`, `semindex_build` | Registered | 100% |
| Search | Symbol search by kind | `semindex_search` | Registered | 100% |
| Call Graph | Caller/callee graph | `semindex_callers`, `semindex_callees` | Registered | 100% |
| Types | Type hierarchy | — | Demo data | 70% |

**Work remaining:** Types tab uses hardcoded hierarchy data.

### SessionBrowserPanel — Session Management
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Sessions | Session list, filtering | `list_sessions` | Registered | 100% |
| Replay | Message replay with step control | `get_session_detail` | Registered | 100% |
| Stats | Size statistics | `delete_session` | Registered | 100% |

### SessionMemoryPanel — Memory Profiling
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Health | Memory usage, GC stats | `get_session_memory_health` | Registered | 100% |
| Samples | Memory samples | `get_session_memory_samples` | Registered | 100% |
| Alerts | Alert management, dismiss | `get_session_memory_alerts`, `dismiss_session_memory_alert` | Registered | 100% |

### SessionSharingPanel — Session Export
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Shared Sessions | Session sharing, visibility | — | Demo data | 30% |
| Annotations | Author/line annotations | — | Demo data | 30% |
| Export | Format selection | — | Demo data | 30% |

**Work remaining:** Entirely demo — needs `session_sharing.rs` backend wiring.

### SettingsPanel — App Settings
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Profile | User profile management | Backend invokes | Registered | 100% |
| Appearance | 18 theme pairs, font settings | Backend invokes | Registered | 100% |
| OAuth | OAuth provider connections | `cloud_oauth_*` commands | Registered | 100% |
| Customizations | UI customization options | Backend invokes | Registered | 100% |
| API Keys | 20+ provider key management | `save_provider_api_keys` | Registered | 100% |

### SketchCanvasPanel — Drawing & Code Gen
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Canvas | Drawing tools (rect/circle/line/text) | `sketch_recognize` | Registered | 80% |
| Recognize | Shape recognition | `sketch_recognize` | Registered | 80% |
| Code | Code generation (React/Vue/Svelte) | `sketch_generate` | Registered | 80% |
| Export | Export drawings | `sketch_export` | Registered | 80% |

### SmartDepsPanel — Dependency Analysis
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Dependencies | Dependency listing | `smartdeps_analyze` | Registered | 100% |
| Conflicts | Conflict detection | `smartdeps_analyze` | Registered | 100% |
| Advisories | Security advisories | `smartdeps_check_security` | Registered | 100% |
| Licenses | License compliance | `smartdeps_check_licenses` | Registered | 100% |

### SnippetPanel — Code Snippets
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Snippet CRUD, language filter, AI gen | `list_snippets`, `save_snippet`, `delete_snippet`, `generate_snippet` | Registered | 100% |

### SoulPanel — SOUL.md Generator
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| View | Current SOUL.md | `soul_read` | Registered | 100% |
| Generate | Generate/regenerate SOUL.md | `soul_generate`, `soul_regenerate` | Registered | 100% |
| Signals | Workspace scanning for signals | `soul_scan` | Registered | 100% |

### SpawnAgentPanel — Parallel Agent Execution
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Active | Running agents | `spawn_agent_list` | Registered | 100% |
| Spawn | Agent config, start parallel | `spawn_agent_new`, `spawn_agent_decompose` | Registered | 100% |
| Results | Result aggregation | `spawn_agent_aggregate` | Registered | 100% |
| History | Past runs | `spawn_agent_stats` | Registered | 100% |

### SpecPanel — Specification Editor
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| List | Spec listing | `list_specs` | Registered | 100% |
| Editor | Spec editor, run, task tracking | `get_spec`, `generate_spec`, `run_spec` | Registered | 100% |

### SpecPipelinePanel — Requirements Pipeline
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Requirements | EARS format requirements | — | Demo data | 30% |
| Design | Design decisions | — | Demo data | 30% |
| Tasks | Task dependencies | — | Demo data | 30% |

**Work remaining:** Entirely demo — needs `spec_pipeline.rs` backend wiring.

### SshPanel — SSH Management
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Profiles | SSH profile CRUD | `list_ssh_profiles`, `save_ssh_profile`, `delete_ssh_profile` | Registered | 100% |
| Run | Command execution, live streaming | `run_ssh_command` | Registered | 100% |

### SteeringPanel — Steering Files
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | File CRUD (workspace/global), templates | `get_steering_files`, `save_steering_file`, `delete_steering_file` | Registered | 100% |

### StreamingPanel — Kafka/NATS Config
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Topics | Kafka topic management | — | Demo data | 30% |
| ProdCon | Producer/consumer config | — | Demo data | 30% |
| Infra | Docker Compose generation | — | Demo data | 30% |

**Work remaining:** Entirely demo — needs `streaming_client.rs` backend wiring.

### SubAgentPanel — Sub-Agent Spawning
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Agents | Agent list with 10 roles | `list_sub_agents` | Registered | 100% |
| Results | Result aggregation | `spawn_sub_agent` | Registered | 100% |
| Spawn | Sub-agent creation | `dismiss_sub_agent` | Registered | 100% |

### SupabasePanel — Supabase Integration
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Tables | Table browser | `list_supabase_tables` | Registered | 100% |
| Query | SQL execution | `query_supabase` | Registered | 100% |
| AI | Natural language query | `translate_nl_to_sql` | Registered | 100% |

### SuperBrainPanel — Multi-Model Routing
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | 5 modes (router/consensus/chain/bestofn/specialist) | `superbrain_query`, `superbrain_route` | Registered | 100% |

### SweBenchPanel — SWE-bench Benchmarking
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Run | Benchmark execution | `swe_bench_start_run` | Registered | 100% |
| Results | Pass@1 rates, task results | `swe_bench_get_results`, `swe_bench_get_suites` | Registered | 100% |
| Compare | Run comparison | `swe_bench_list_runs` | Registered | 100% |

### TestPanel — Test Runner
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Framework detection, test execution | `detect_test_framework`, `run_tests` | Registered | 100% |

### TimestampPanel — Epoch Converter
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Epoch/ISO/UTC conversion, diff calculator | — | Utility | 100% |

### TracesPanel — Agent Execution Traces
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Trace session list, detail viewer | `list_trace_sessions`, `load_trace_session` | Registered | 100% |

### TransformPanel — Code Transforms
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Transform detection, execution, preview | `detect_transform`, `plan_transform`, `execute_transform` | Registered | 100% |

### TriagePanel — Issue Triage
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Issue triage, rules, history, metrics | `triage_issue`, `triage_get_rules`, `triage_get_history`, `triage_get_metrics` | Registered | 100% |

### TrustPanel — Agent Trust Scores
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Trust scores, events, explanations | `trust_get_scores`, `trust_get_events`, `trust_explain` | Registered | 100% |

### UsageMeteringPanel — Credit System
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Usage tracking, budget management | `dismiss_usage_alert` | Registered | 80% |

### VectorDbPanel — Vector Database
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Collections | Collection management | — | Demo (setTimeout) | 40% |
| Search | Similarity search | — | Demo (setTimeout) | 40% |
| Schema | Schema generation | — | Demo | 40% |

**Work remaining:** Uses setTimeout simulation — needs `vector_db.rs` integration.

### VibeSqlPanel — SQL Client
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Connection manager, schema browser | `vibesql_connect`, `vibesql_list_connections`, `vibesql_list_tables` | Registered | 100% |
| — | Query editor, execution | `vibesql_execute_query`, `vibesql_generate_sql` | Registered | 100% |
| — | History | `vibesql_save_connection`, `vibesql_delete_connection` | Registered | 100% |

### VisualTestPanel — Visual Regression
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Screenshot assertions | `take_screenshot`, `get_visual_test_results` | Registered | 100% |

### VisualVerifyPanel — Pixel Diff
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Baseline management, pixel diff, scoring | — | Demo data | 30% |

**Work remaining:** Entirely demo — needs backend integration.

### VmOrchestratorPanel — VM Environments
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Environments | VM environment management | — | Demo data | 30% |
| Pull Requests | Agent PR tracking | — | Demo data | 30% |
| Conflicts | Branch conflicts | — | Demo data | 30% |
| Config | Orchestrator config | — | Demo data | 30% |

**Work remaining:** Entirely demo — needs `vm_orchestrator.rs` backend wiring.

### VoiceLocalPanel — Local Voice Transcription
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Record | Voice recording, transcription | `voice_start_recording`, `voice_stop_recording` | Registered | 80% |
| Models | Model management | `voice_list_models` | Registered | 80% |

### WebCrawlerPanel — Web Crawling
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Crawl | URL crawling | — | Demo (setTimeout) | 30% |
| Sitemap | Sitemap parsing | — | Demo | 30% |

**Work remaining:** Uses setTimeout simulation — needs `web_crawler.rs` integration.

### WebGroundingPanel — Web Search
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Search | Web search results | `web_search` | Registered | 100% |
| Cache | Cache management | `web_cache_stats`, `web_clear_cache` | Registered | 100% |
| Citations | Citation tracking | `web_get_citations` | Registered | 100% |

### WebhookPanel — Webhook Management
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Config | Webhook CRUD, event subscription | `get_webhooks`, `save_webhook`, `delete_webhook` | Registered | 100% |
| Logs | Delivery log, test trigger | `get_webhook_logs`, `test_webhook` | Registered | 100% |

### WebSocketPanel — WebSocket Tester
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Connect, send/receive, latency | `get_ws_configs` | Registered | 100% |

### WorkflowPanel — Workflow Manager
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| — | Workflow list/detail, stage progression | `list_workflows`, `get_workflow`, `create_workflow`, `advance_workflow_stage` | Registered | 100% |

### WorkManagementPanel — Enterprise Work Items
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Hierarchy | Org/team/workspace hierarchy | `wm_list_orgs`, `wm_list_teams`, `wm_list_workspaces` | Registered | 100% |
| Agile | Agile integration | `wm_list_items` | Registered | 100% |
| Items | Work item CRUD | `wm_create_item`, `wm_update_item`, `wm_delete_item` | Registered | 100% |
| Board | Kanban board | `wm_move_item` | Registered | 100% |
| Relationships | Item relationships | `wm_add_relationship`, `wm_remove_relationship` | Registered | 100% |
| OKRs | Objectives & key results | `wm_get_item_tree` | Registered | 100% |
| Risks | Risk tracking | `wm_ai_assess_risk` | Registered | 100% |
| Dashboard | Overview dashboard | `wm_get_dashboard` | Registered | 100% |
| Discussions | Threaded discussions | `wm_get_config`, `wm_save_config` | Registered | 100% |

### WorktreePoolPanel — Git Worktree Pool
| Tab | Feature | Backend | Status | % |
|-----|---------|---------|--------|--:|
| Active | Active worktrees | `worktree_list` | Registered | 80% |
| Queue | Agent queue | `worktree_spawn` | Registered | 80% |
| Config | Pool configuration | `worktree_merge`, `worktree_cleanup` | Registered | 80% |

---

## Panels Requiring Backend Wiring (Prioritized)

These panels have UI built but use demo/mock data instead of calling registered Tauri commands:

| Priority | Panel | Rust Module | Effort | Gap |
|----------|-------|-------------|--------|-----|
| ~~P0~~ | ~~AutomationsPanel~~ | `automations.rs` | ~~Medium~~ | **DONE** — 8 Tauri commands, full CRUD, persisted |
| ~~P0~~ | ~~ResiliencePanel~~ | `resilience.rs` | ~~Medium~~ | **DONE** — 7 Tauri commands, editable config, live data |
| ~~P1~~ | ~~NextTaskPanel~~ | `next_task.rs` | ~~Medium~~ | **DONE** — 6 commands, predictions/history/transitions/rules loaded from backend |
| ~~P1~~ | ~~QaValidationPanel~~ | `qa_validation.rs` | ~~Low~~ | **DONE** — 2 commands, real validation + persisted history |
| ~~P1~~ | ~~VectorDbPanel~~ | `vector_db.rs` | ~~Low~~ | **DONE** — 4 commands, collection CRUD + search |
| ~~P1~~ | ~~OrgContextPanel~~ | `org_context.rs` | ~~Medium~~ | **DONE** — 5 commands, repos/patterns/conventions/deps loaded |
| ~~P1~~ | ~~SpecPipelinePanel~~ | `spec_pipeline.rs` | ~~Medium~~ | **DONE** — 6 commands, requirements/designs/tasks CRUD |
| ~~P1~~ | ~~VmOrchestratorPanel~~ | `vm_orchestrator.rs` | ~~Medium~~ | **DONE** — 5 commands, envs/PRs/conflicts + editable config |
| ~~P2~~ | ~~SessionSharingPanel~~ | `session_sharing.rs` | ~~Medium~~ | **DONE** — 4 commands, sessions/annotations loaded |
| ~~P2~~ | ~~SelfReviewPanel~~ | `self_review.rs` | ~~Low~~ | **DONE** — 3 commands, iterations loaded + config save |
| ~~P2~~ | ~~StreamingPanel~~ | `streaming_client.rs` | ~~Medium~~ | **DONE** — 3 commands, topic CRUD persisted |
| ~~P2~~ | ~~ObserveActPanel~~ | `observe_act.rs` | ~~Medium~~ | **DONE** — 3 commands, steps loaded + config save |
| ~~P2~~ | ~~WebCrawlerPanel~~ | `web_crawler.rs` | ~~Low~~ | **DONE** — 4 commands, crawl/sitemap/robots wired |
| ~~P2~~ | ~~VisualVerifyPanel~~ | `visual_verify.rs` | ~~Low~~ | **DONE** — 4 commands, baseline CRUD + diffs |
| ~~P3~~ | ~~ClarifyingQuestionsPanel~~ | `clarifying_questions.rs` | ~~Low~~ | **DONE** — 5 commands, questions/plan/risks loaded + answers persisted |
| ~~P3~~ | ~~AiMlWorkflowPanel~~ | — | ~~Low~~ | **DONE** — 2 commands, pipeline stage config persisted |
| ~~P3~~ | ~~OrchestrationPanel~~ | — | ~~Low~~ | **DONE** — 4 commands, tasks + lessons persisted across sessions |

---

## Overall Statistics

| Metric | Value |
|--------|-------|
| **Total panels** | 186 |
| **Total Tauri commands registered** | 991 |
| **Panels at 100%** | 156 (84%) |
| **Panels at 80-99%** | 30 (16%) |
| **Panels at 30-79%** | 0 (0%) |
| **Panels < 30%** | 0 (0%) |
| **Panels with all tabs functional** | 173 (93%) |
| **Panels needing backend wiring** | 0 |
| **Pure utility panels (no backend needed)** | 18 (10%) |
