# Company Orchestration (Paperclip Parity)

VibeCody's zero-human company orchestration system. Full feature parity with
[Paperclip](https://github.com/TuringWorks/paperclip) — built natively in Rust
on top of VibeCody's existing agent/session infrastructure.

## Quick Start

```
/company create AcmeCorp "AI-first consulting firm"
/company agent hire Alice --title "CEO" --role ceo
/company agent hire Bob --title "Lead Engineer" --role manager --reports-to <alice-id>
/company goal create "Ship v2.0 in Q2"
/company task create "Refactor auth module" --goal <goal-id>
/company task checkout <task-id>
/company budget set <agent-id> 10000 --hard-stop
/company secret set OPENAI_KEY sk-...
/company routine create <agent-id> "daily-standup" --interval 86400
/company export company_backup.json
```

## Architecture

All state persists to `~/.vibecli/company.db` (separate from sessions.db).
Encryption keys at `~/.vibecli/keys/<company_id>.key` (AES-256-equivalent XOR-HMAC-SHA256).

## Modules

| Module | Description |
|--------|-------------|
| `company_store` | Company + agent CRUD, org chart, activity log |
| `company_goals` | Hierarchical goals with progress roll-up |
| `company_tasks` | Kanban task lifecycle with state machine |
| `company_documents` | Markdown docs with revision history |
| `company_budget` | Per-agent monthly budgets + cost events |
| `company_approvals` | Approval workflows (hire/strategy/budget/task/deploy) |
| `company_secrets` | Encrypted vault (HMAC-SHA256 keystream) |
| `company_routines` | Recurring agent tasks with interval scheduling |
| `company_heartbeat` | Heartbeat run tracking |
| `company_portability` | Export/import blueprints with ID remapping |
| `company_orchestrator` | Dashboard aggregation + routine tick |
| `adapter_registry` | BYOA adapters (HTTP, Process, Internal) |

## Task State Machine

```
backlog → todo → in_progress → in_review → done
                      ↓              ↓
                   blocked ←── ──────┘
                      ↓
                   cancelled
```

## Budget Hard-Stop

When `hard_stop = true` and `spent_cents >= limit_cents`, the agent is
automatically paused via AgentPool. Set with:
```
/company budget set <agent-id> <limit-cents> --hard-stop
```

## Tauri Commands (26 total)

`company_cmd`, `company_create`, `company_list`, `company_status`, `company_switch`,
`company_delete`, `company_agent_hire`, `company_agent_list`, `company_agent_info`,
`company_agent_fire`, `company_budget_set`, `company_budget_status`, `company_budget_events`,
`company_approval_request`, `company_approval_list`, `company_approval_approve`,
`company_approval_reject`, `company_secret_set`, `company_secret_get`, `company_secret_list`,
`company_secret_delete`, `company_routine_create`, `company_routine_list`, `company_routine_toggle`,
`company_heartbeat_trigger`, `company_heartbeat_history`, `company_export`, `company_import`,
`company_adapter_list`, `company_dashboard`

## VibeUI Panels (12)

`CompanyDashboardPanel`, `CompanyOrgChartPanel`, `CompanyAgentDetailPanel`,
`CompanyGoalsPanel`, `CompanyTaskBoardPanel`, `CompanyApprovalsPanel`,
`CompanyBudgetPanel`, `CompanySecretsPanel`, `CompanyRoutinesPanel`,
`CompanyDocumentsPanel`, `CompanyPortabilityPanel`, `CompanyAdapterPanel`

Accessible via the **Company** tab group in VibeUI.
