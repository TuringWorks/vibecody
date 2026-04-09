# 03 — Undocumented Code

> Features/modules that exist in the codebase but have no documentation anywhere.

## Undocumented Tauri Commands — 422 commands (40% of total)

The largest undocumented command groups:

| Prefix | Count | Examples |
|--------|-------|---------|
| `get_*` (undocumented subset) | 69 | `get_acp_capabilities`, `get_adventure_names` |
| `company_*` | 44 | `company_agent_hire`, `company_budget_allocate` |
| `rl_*` | 20 | `rl_create_training_run`, `rl_deploy_environment` |
| `quantum_*` | 12 | `quantum_delete_circuit`, `quantum_estimate_cost` |
| `wm_*` | 11 | `wm_ai_generate_item`, `wm_delete_group` |
| `agile_*` | 10 | `agile_ai_estimate_points`, `agile_ai_retro_generate` |
| `cloud_*` | 10 | `cloud_oauth_complete`, `cloud_oauth_get_token` |
| `da_*` | 9 | `da_add_chart`, `da_execute_query` |
| `archspec_*` | 6 | `archspec_create_adr`, `archspec_generate` |
| `turboquant_*` | 5 | `turboquant_benchmark`, `turboquant_insert` |

**Ghost commands** (registered in `lib.rs` but function not defined): `inline_edit`, `record_purple_team_simulation`

## Undocumented Rust Modules — 41 modules (62,696 lines)

### RL-OS subsystem (8 modules, ~31,495 lines)
Despite `docs/RL-OS-ARCHITECTURE.md` existing, none of these filenames appear in it:
- `rl_env_os.rs` (4,202 lines)
- `rl_serve_os.rs` (4,490 lines)
- `rl_observe.rs` (4,401 lines)
- `rl_eval_os.rs` (3,819 lines)
- `rl_train_os.rs` (3,780 lines)
- `rl_model_hub.rs` (3,726 lines)
- `rl_rlhf.rs` (3,690 lines)
- `rl_opti_os.rs` (3,387 lines)

### Security/Vulnerability (2 modules, 4,248 lines)
- `vulnerability_db.rs` (3,427 lines)
- `security_hardening.rs` (821 lines)

### Quantum Computing (1 module, 2,489 lines)
- `quantum_computing.rs` (2,489 lines)

### Mobile/Cloud/IDE (3 modules, 3,588 lines)
- `mobile_gateway.rs` (1,904 lines)
- `cloud_ide.rs` (867 lines)
- `cloud_autofix.rs` (817 lines)

### Productivity Integrations (2 modules, 1,901 lines)
- `calendar_client.rs` (1,368 lines)
- `email_client.rs` (533 lines)

### CI/DevOps (3 modules, 2,878 lines)
- `ci_status_check.rs` (1,088 lines)
- `gh_actions_agent.rs` (1,067 lines)
- `docker_runtime.rs` (723 lines)

### IDE Integration (3 modules, 2,769 lines)
- `jetbrains_hooks.rs` (1,017 lines)
- `vscode_compat_ext.rs` (994 lines)
- `vscode_sessions.rs` (758 lines)

### Benchmarking/Testing (2 modules, 2,790 lines)
- `feature_demo.rs` (1,421 lines)
- `large_codebase_bench.rs` (1,369 lines)

### Remaining (17 modules, ~10,538 lines)
`debug_mode.rs`, `soul_generator.rs`, `resource_manager.rs`, `podman_runtime.rs`, `mermaid_ascii.rs`, `document_ingest.rs`, `distributed_training.rs`, `company_cmd.rs`, `agent_teams_v2_enhanced.rs`, `gpu_cluster.rs`, `render_optimize.rs`, `semantic_mcp.rs`, `company_meeting_notes.rs`, `company_priority_map.rs`, `company_workspace_config.rs`, `discussion_mode.rs`, `workspace_detect.rs`

## Undocumented VibeUI Panels — ~48 panels

### Company/Enterprise (15 panels)
`CompanyAdapterPanel`, `CompanyAgentDetailPanel`, `CompanyApprovalsPanel`, `CompanyBudgetPanel`, `CompanyDashboardPanel`, `CompanyDocumentsPanel`, `CompanyGoalsPanel`, `CompanyHeartbeatPanel`, `CompanyOrgChartPanel`, `CompanyPortabilityPanel`, `CompanyPriorityMapPanel`, `CompanyRoutinesPanel`, `CompanySecretsPanel`, `CompanyTaskBoardPanel`, `CompanyWorkspaceConfigPanel`

### RL-OS sub-panels (10 panels)
`RLDeploymentMonitor`, `RLEnvironmentViewer`, `RLEvalResults`, `RLHFAlignmentDashboard`, `RLModelLineage`, `RLMultiAgentView`, `RLOptimizationReport`, `RLPolicyComparison`, `RLRewardDecomposition`, `RLTrainingDashboard`

### Phase 32/AI Code Review (7 panels)
`AiCodeReviewPanel`, `ArchitectureSpecPanel`, `PolicyEnginePanel`, `HealthScorePanel`, `IntentRefactorPanel`, `ReviewProtocolPanel`, `SkillDistillationPanel`

### Miscellaneous (16 panels)
`AgentOSDashboard`, `AgentRecordingPanel`, `AgentUIRenderer`, `BugBotPanel`, `CidrPanel`, `CIReviewPanel`, `CiStatusPanel`, `CollabChatPanel`, `ColorConverterPanel`, `DatabasePanel`, `DataAnalysisPanel`, `DesignCanvasPanel`, `DesignMode`, `DocumentIngestPanel`, `GatewaySandboxPanel`, `SandboxChatPanel`, `ScreenshotToApp`, `TeamGovernancePanel`, `VisualEditor`, `VisualEditOverlay`

## Undocumented REPL Commands — 11 commands

| Command | Purpose |
|---------|---------|
| `/gitplatform` | Git platform integration |
| `/icontext` | Infinite context |
| `/legacymigrate` | Legacy migration |
| `/markers` | Code markers |
| `/quantum` | Quantum computing |
| `/smartdeps` | Smart dependencies |
| `/speculate` | Speculative execution |
| `/healthscore` | Code health scoring |
| `/intentrefactor` | Intent-based refactoring |
| `/reviewprotocol` | Review protocol |
| `/skilldistill` | Skill distillation |

## Undocumented Hooks and Utils

### Hooks (11, none documented)
`useApiKeyMonitor`, `useCollab`, `useDaemonMonitor`, `useEditorTheme`, `useModelRegistry`*, `useNotifications`, `usePanelSettings`, `usePersistentState`, `useSessionMemory`, `useToast`, `useVoiceInput`

*`useModelRegistry` is mentioned in CLAUDE.md for provider setup but has no API docs.

### Utils (5, none documented)
`DocsResolver.ts`, `fileUtils.tsx`, `FlowContext.ts`, `LinterIntegration.ts`, `SupercompleteEngine.ts`
