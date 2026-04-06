//! Non-interactive `/company` command handler for `vibecli --cmd "/company <args>"`.
//!
//! Called by main.rs when the binary is invoked as:
//!   vibecli --cmd "/company status"
//!   vibecli --cmd "/company agent list"
//!   vibecli --cmd "/company goal list"
//!   etc.

use crate::company_approvals::{ApprovalStore, ApprovalRequestType};
use crate::company_goals::{GoalStore, print_goal_tree};
use crate::company_secrets::SecretStore;
use crate::company_documents::DocumentStore;
use crate::company_budget::BudgetStore;
use crate::company_routines::RoutineStore;
use crate::company_heartbeat::HeartbeatStore;
use crate::adapter_registry::AdapterRegistry;
use crate::company_store::{AdapterType, CompanyRole, CompanyStore, get_active_company_id};
use crate::company_tasks::{TaskPriority, TaskStatus, TaskStore};

/// Entry point — `args` is everything after "/company " (may be empty).
pub async fn handle_company_cmd_once(args: &str) -> String {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let sub = parts.first().copied().unwrap_or("help");

    match sub {
        "status"    => cmd_status(),
        "list"      => cmd_list(),
        "create"    => cmd_create(&parts[1..]),
        "switch"    => cmd_switch(parts.get(1).copied().unwrap_or("")),
        "delete"    => cmd_delete(parts.get(1).copied().unwrap_or("")),
        "agent"     => cmd_agent(&parts[1..]),
        "goal"      => cmd_goal(&parts[1..]),
        "task"      => cmd_task(&parts[1..]),
        "approval"  => cmd_approval(&parts[1..]),
        "secret"    => cmd_secret(&parts[1..]),
        "doc"       => cmd_doc(&parts[1..]),
        "budget"    => cmd_budget(&parts[1..]),
        "routine"   => cmd_routine(&parts[1..]),
        "heartbeat" => cmd_heartbeat(&parts[1..]),
        "adapter"   => cmd_adapter(&parts[1..]),
        _ => help(),
    }
}

fn help() -> String {
    "Company Commands:\n\
     \n  status                      — Active company dashboard\
     \n  list                        — List all companies\
     \n  create <name> [desc]        — Create a company\
     \n  switch <name|id>            — Set active company\
     \n  delete <name|id>            — Archive a company\
     \n  agent list|tree|hire|info   — Manage agents\
     \n  goal  list|tree|create      — Manage goals\
     \n  task  list|create|transition — Manage tasks\
    ".into()
}

// ── Company ───────────────────────────────────────────────────────────────────

fn cmd_status() -> String {
    match CompanyStore::open_default() {
        Err(e) => format!("Error opening store: {e}"),
        Ok(store) => match store.list_companies() {
            Err(e) => format!("Error: {e}"),
            Ok(companies) if companies.is_empty() => {
                "No companies yet.\nUse: /company create <name>".into()
            }
            Ok(companies) => {
                let active = get_active_company_id();
                companies.iter().map(|c| {
                    let marker = if active.as_deref() == Some(c.id.as_str()) { "▶ " } else { "  " };
                    let label = if !c.description.is_empty() { &c.description } else { &c.mission };
                    format!("{}[{}] {}  {}", marker, c.status.as_str(), c.name, label)
                }).collect::<Vec<_>>().join("\n")
            }
        },
    }
}

fn cmd_list() -> String { cmd_status() }

fn cmd_create(parts: &[&str]) -> String {
    let name = match parts.first().copied() {
        Some(n) => n,
        None => return "Usage: /company create <name> [description]".into(),
    };
    let desc = parts[1..].join(" ");
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => match store.create_company(name, &desc, "") {
            Ok(c) => format!("✓ Company created: {} [{}]", c.name, &c.id[..8.min(c.id.len())]),
            Err(e) => format!("Error: {e}"),
        },
    }
}

fn cmd_switch(name_or_id: &str) -> String {
    if name_or_id.is_empty() { return "Usage: /company switch <name|id>".into(); }
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => match store.list_companies() {
            Err(e) => format!("Error: {e}"),
            Ok(companies) => {
                match companies.iter().find(|c| c.name == name_or_id || c.id.starts_with(name_or_id)) {
                    None => format!("Company '{}' not found.", name_or_id),
                    Some(c) => match crate::company_store::set_active_company_id(&c.id) {
                        Ok(()) => format!("✓ Switched to: {}", c.name),
                        Err(e) => format!("Error: {e}"),
                    },
                }
            }
        },
    }
}

fn cmd_delete(name_or_id: &str) -> String {
    if name_or_id.is_empty() { return "Usage: /company delete <name|id>".into(); }
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => match store.list_companies() {
            Err(e) => format!("Error: {e}"),
            Ok(companies) => {
                match companies.iter().find(|c| c.name == name_or_id || c.id.starts_with(name_or_id)) {
                    None => format!("Company '{}' not found.", name_or_id),
                    Some(c) => match store.delete_company(&c.id) {
                        Ok(()) => format!("✓ Archived: {}", c.name),
                        Err(e) => format!("Error: {e}"),
                    },
                }
            }
        },
    }
}

// ── Agent ─────────────────────────────────────────────────────────────────────

fn cmd_agent(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("list");
    let active = get_active_company_id();

    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => {
            let cid = match &active { Some(id) => id.clone(), None => return "No active company. Use: /company switch <name>".into() };
            match sub {
                "list" => match store.list_agents(&cid) {
                    Err(e) => format!("Error: {e}"),
                    Ok(agents) if agents.is_empty() => "No agents. Use: /company agent hire <name>".into(),
                    Ok(agents) => agents.iter().map(|a| format!(
                        "  [{}] {} — {} ({})",
                        a.status.as_str(), a.name, a.title, a.role.as_str()
                    )).collect::<Vec<_>>().join("\n"),
                },
                "tree" => match store.list_agents(&cid) {
                    Err(e) => format!("Error: {e}"),
                    Ok(agents) if agents.is_empty() => "No agents yet.".into(),
                    Ok(agents) => build_agent_tree(&agents),
                },
                "hire" => {
                    let name = match parts.get(1).copied() {
                        Some(n) => n,
                        None => return "Usage: /company agent hire <name> [--title <t>] [--role <r>]".into(),
                    };
                    let title = flag_value(parts, "--title").unwrap_or("Agent");
                    let role_str = flag_value(parts, "--role").unwrap_or("agent");
                    let role = CompanyRole::from_str(role_str);
                    match store.hire_agent(&cid, name, title, role, None, &[], AdapterType::Internal, 0) {
                        Ok(a) => format!("✓ Hired: {} [{}] — {}", a.name, a.role.as_str(), &a.id[..8.min(a.id.len())]),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                "info" => {
                    let id = parts.get(1).copied().unwrap_or("");
                    if id.is_empty() { return "Usage: /company agent info <id|name>".into(); }
                    match store.list_agents(&cid) {
                        Err(e) => format!("Error: {e}"),
                        Ok(agents) => match agents.iter().find(|a| a.id.starts_with(id) || a.name == id) {
                            None => format!("Agent '{id}' not found."),
                            Some(a) => format!(
                                "Name:    {}\nTitle:   {}\nRole:    {}\nStatus:  {}\nID:      {}",
                                a.name, a.title, a.role.as_str(), a.status.as_str(), a.id
                            ),
                        },
                    }
                }
                _ => "Agent subcommands: list | tree | hire <name> [--title <t>] [--role <r>] | info <id>".into(),
            }
        }
    }
}

fn build_agent_tree(agents: &[crate::company_store::CompanyAgent]) -> String {
    fn print_tree(all: &[crate::company_store::CompanyAgent], parent_id: &str, depth: usize, out: &mut String) {
        for a in all.iter().filter(|a| a.reports_to.as_deref() == Some(parent_id)) {
            out.push_str(&format!("{}└─ [{}] {}\n", "  ".repeat(depth), a.role.as_str(), a.name));
            print_tree(all, &a.id, depth + 1, out);
        }
    }
    let mut out = String::new();
    for root in agents.iter().filter(|a| a.reports_to.is_none()) {
        out.push_str(&format!("● [{}] {}\n", root.role.as_str(), root.name));
        print_tree(agents, &root.id, 1, &mut out);
    }
    out
}

// ── Goal ──────────────────────────────────────────────────────────────────────

fn cmd_goal(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("list");
    let active = get_active_company_id();

    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => {
            let cid = match &active { Some(id) => id.clone(), None => return "No active company.".into() };
            let gs = GoalStore::new(store.conn());
            if let Err(e) = gs.ensure_schema() { return format!("Schema error: {e}"); }
            match sub {
                "list" => match gs.list(&cid) {
                    Err(e) => format!("Error: {e}"),
                    Ok(goals) if goals.is_empty() => "No goals. Use: /company goal create <title>".into(),
                    Ok(goals) => goals.iter().map(|g| format!(
                        "  [{}] {} ({}%)", g.status.as_str(), g.title, g.progress_pct
                    )).collect::<Vec<_>>().join("\n"),
                },
                "tree" => match gs.build_tree(&cid) {
                    Err(e) => format!("Error: {e}"),
                    Ok(tree) if tree.is_empty() => "No goals yet.".into(),
                    Ok(tree) => print_goal_tree(&tree, 0),
                },
                "create" => {
                    let title = parts[1..].join(" ");
                    if title.is_empty() { return "Usage: /company goal create <title>".into(); }
                    match gs.create(&cid, &title, "", None, None, 1) {
                        Ok(g) => format!("✓ Goal created: {} [{}]", g.title, &g.id[..8.min(g.id.len())]),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                _ => "Goal subcommands: list | tree | create <title>".into(),
            }
        }
    }
}

// ── Task ──────────────────────────────────────────────────────────────────────

fn cmd_task(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("list");
    let active = get_active_company_id();

    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => {
            let cid = match &active { Some(id) => id.clone(), None => return "No active company.".into() };
            let ts = TaskStore::new(store.conn());
            if let Err(e) = ts.ensure_schema() { return format!("Schema error: {e}"); }
            match sub {
                "list" => {
                    let status_filter = parts.get(1).copied();
                    match ts.list(&cid, status_filter) {
                        Err(e) => format!("Error: {e}"),
                        Ok(tasks) if tasks.is_empty() => "No tasks. Use: /company task create <title>".into(),
                        Ok(tasks) => tasks.iter().map(|t| t.summary_line()).collect::<Vec<_>>().join("\n"),
                    }
                }
                "create" => {
                    let title = parts[1..].join(" ");
                    if title.is_empty() { return "Usage: /company task create <title>".into(); }
                    match ts.create(&cid, &title, "", None, None, None, TaskPriority::Medium) {
                        Ok(t) => format!("✓ Task created: {} [{}]", t.title, &t.id[..8.min(t.id.len())]),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                "transition" => {
                    let id = parts.get(1).copied().unwrap_or("");
                    let status_str = parts.get(2).copied().unwrap_or("");
                    if id.is_empty() || status_str.is_empty() {
                        return "Usage: /company task transition <id> <status>".into();
                    }
                    match ts.transition(id, TaskStatus::from_str(status_str)) {
                        Ok(t) => format!("✓ {}", t.summary_line()),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                _ => "Task subcommands: list [<status>] | create <title> | transition <id> <status>".into(),
            }
        }
    }
}

// ── Approval ──────────────────────────────────────────────────────────────────

fn cmd_approval(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("list");
    let active = get_active_company_id();
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => {
            let cid = match &active { Some(id) => id.clone(), None => return "No active company.".into() };
            let ap = ApprovalStore::new(store.conn());
            if let Err(e) = ap.ensure_schema() { return format!("Schema error: {e}"); }
            match sub {
                "list" => {
                    let pending_only = !parts.contains(&"--all") && !parts.contains(&"all");
                    let status_filter = if pending_only { Some("pending") } else { None };
                    match ap.list(&cid, status_filter) {
                        Err(e) => format!("Error: {e}"),
                        Ok(approvals) if approvals.is_empty() => "No approvals.".into(),
                        Ok(approvals) => approvals.iter().map(|a| a.summary_line()).collect::<Vec<_>>().join("\n"),
                    }
                }
                "decide" => {
                    let id = parts.get(1).copied().unwrap_or("");
                    let decision = parts.get(2).copied().unwrap_or("");
                    if id.is_empty() || decision.is_empty() {
                        return "Usage: /company approval decide <id> <approve|reject> [reason]".into();
                    }
                    let approved = decision == "approve";
                    let decider = parts.get(3).copied().unwrap_or("system");
                    match ap.decide(id, approved, decider) {
                        Ok(a) => format!("✓ {}", a.summary_line()),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                "request" => {
                    let req_type_str = parts.get(1).copied().unwrap_or("");
                    let subject_id = parts.get(2).copied().unwrap_or("");
                    if req_type_str.is_empty() || subject_id.is_empty() {
                        return "Usage: /company approval request <type> <subject-id>".into();
                    }
                    let req_type = ApprovalRequestType::from_str(req_type_str);
                    let requester = parts.get(3).copied().unwrap_or("system");
                    let reason = if parts.len() > 4 { parts[4..].join(" ") } else { String::new() };
                    match ap.request(&cid, req_type, subject_id, requester, &reason) {
                        Ok(a) => format!("✓ Requested: {}", a.summary_line()),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                _ => "Approval subcommands: list [--all] | decide <id> <approve|reject> | request <type> <subject-id>".into(),
            }
        }
    }
}

// ── Secret ────────────────────────────────────────────────────────────────────

fn cmd_secret(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("list");
    let active = get_active_company_id();
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => {
            let cid = match &active { Some(id) => id.clone(), None => return "No active company.".into() };
            let ss = SecretStore::new(store.conn());
            if let Err(e) = ss.ensure_schema() { return format!("Schema error: {e}"); }
            match sub {
                "list" => match ss.list(&cid) {
                    Err(e) => format!("Error: {e}"),
                    Ok(secrets) if secrets.is_empty() => "No secrets.".into(),
                    Ok(secrets) => secrets.iter().map(|s| format!(
                        "  {} [{}]", s.key_name, s.created_at
                    )).collect::<Vec<_>>().join("\n"),
                },
                "set" => {
                    let key = parts.get(1).copied().unwrap_or("");
                    let value = if parts.len() > 2 { parts[2..].join(" ").trim_matches('"').to_string() } else { String::new() };
                    if key.is_empty() || value.is_empty() {
                        return "Usage: /company secret set <key> <value>".into();
                    }
                    match ss.set(&cid, key, &value, None) {
                        Ok(s) => format!("✓ Secret set: {}", s.summary_line()),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                "get" => {
                    let key = parts.get(1).copied().unwrap_or("");
                    if key.is_empty() { return "Usage: /company secret get <key>".into(); }
                    match ss.get(&cid, key) {
                        Err(e) => format!("Error: {e}"),
                        Ok(None) => format!("Secret '{key}' not found."),
                        Ok(Some(s)) => format!("{} [encrypted — use CLI to decrypt]", s.key_name),
                    }
                }
                "delete" => {
                    let key = parts.get(1).copied().unwrap_or("");
                    if key.is_empty() { return "Usage: /company secret delete <key>".into(); }
                    match ss.delete(&cid, key) {
                        Ok(_) => format!("✓ Deleted: {key}"),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                _ => "Secret subcommands: list | set <key> <value> | get <key> | delete <key>".into(),
            }
        }
    }
}

// ── Document ──────────────────────────────────────────────────────────────────

fn cmd_doc(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("list");
    let active = get_active_company_id();
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => {
            let cid = match &active { Some(id) => id.clone(), None => return "No active company.".into() };
            let ds = DocumentStore::new(store.conn());
            if let Err(e) = ds.ensure_schema() { return format!("Schema error: {e}"); }
            match sub {
                "list" => match ds.list(&cid) {
                    Err(e) => format!("Error: {e}"),
                    Ok(docs) if docs.is_empty() => "No documents. Use: /company doc create <title>".into(),
                    Ok(docs) => docs.iter().map(|d| d.summary_line()).collect::<Vec<_>>().join("\n"),
                },
                "create" => {
                    let title = parts[1..].join(" ").trim_matches('"').to_string();
                    if title.is_empty() { return "Usage: /company doc create <title>".into(); }
                    match ds.create(&cid, &title, "", None, None, None) {
                        Ok(d) => format!("✓ Document created: {}", d.summary_line()),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                "show" => {
                    let id = parts.get(1).copied().unwrap_or("");
                    if id.is_empty() { return "Usage: /company doc show <id>".into(); }
                    match ds.get(id) {
                        Err(e) => format!("Error: {e}"),
                        Ok(None) => "Document not found.".into(),
                        Ok(Some(d)) => format!("# {}\n\n{}", d.title, d.content),
                    }
                }
                "history" => {
                    let id = parts.get(1).copied().unwrap_or("");
                    if id.is_empty() { return "Usage: /company doc history <id>".into(); }
                    match ds.list_revisions(id) {
                        Err(e) => format!("Error: {e}"),
                        Ok(revs) if revs.is_empty() => "No revisions.".into(),
                        Ok(revs) => revs.iter().map(|r| format!(
                            "  v{} — {} chars  [{}]", r.revision, r.content.len(), r.created_at
                        )).collect::<Vec<_>>().join("\n"),
                    }
                }
                _ => "Doc subcommands: list | create <title> | show <id> | history <id>".into(),
            }
        }
    }
}

// ── Budget ────────────────────────────────────────────────────────────────────

fn cmd_budget(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("status");
    let active = get_active_company_id();
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => {
            let cid = match &active { Some(id) => id.clone(), None => return "No active company.".into() };
            let bs = BudgetStore::new(store.conn());
            if let Err(e) = bs.ensure_schema() { return format!("Schema error: {e}"); }
            match sub {
                "status" | "list" => {
                    let _agent_filter = parts.get(1).copied();
                    match bs.list(&cid) {
                        Err(e) => format!("Error: {e}"),
                        Ok(budgets) if budgets.is_empty() => "No budgets set.".into(),
                        Ok(budgets) => budgets.iter().map(|b| b.summary_line()).collect::<Vec<_>>().join("\n"),
                    }
                }
                "set" => {
                    let agent_id = parts.get(1).copied().unwrap_or("");
                    let limit: i64 = parts.get(2).copied().unwrap_or("0").parse().unwrap_or(0);
                    if agent_id.is_empty() { return "Usage: /company budget set <agent-id> <limit-cents>".into(); }
                    let hard_stop = parts.contains(&"--hard-stop");
                    let month = flag_value(parts, "--month").unwrap_or("");
                    let month_str = if month.is_empty() { BudgetStore::current_month_static() } else { month.to_string() };
                    match bs.set_budget(&cid, agent_id, &month_str, limit, hard_stop, 80) {
                        Ok(b) => format!("✓ Budget set: {}", b.summary_line()),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                "events" => {
                    let agent_id = parts.get(1).copied().unwrap_or("");
                    match bs.list_events(&cid, if agent_id.is_empty() { None } else { Some(agent_id) }) {
                        Err(e) => format!("Error: {e}"),
                        Ok(events) if events.is_empty() => "No cost events.".into(),
                        Ok(events) => events.iter().map(|e| format!(
                            "  {} ¢{} — {}", e.agent_id, e.amount_cents, e.description
                        )).collect::<Vec<_>>().join("\n"),
                    }
                }
                _ => "Budget subcommands: status [agent-id] | set <agent-id> <cents> | events [agent-id]".into(),
            }
        }
    }
}

// ── Routine ───────────────────────────────────────────────────────────────────

fn cmd_routine(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("list");
    let active = get_active_company_id();
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => {
            let cid = match &active { Some(id) => id.clone(), None => return "No active company.".into() };
            let rs = RoutineStore::new(store.conn());
            if let Err(e) = rs.ensure_schema() { return format!("Schema error: {e}"); }
            match sub {
                "list" => match rs.list(&cid) {
                    Err(e) => format!("Error: {e}"),
                    Ok(routines) if routines.is_empty() => "No routines.".into(),
                    Ok(routines) => routines.iter().map(|r| r.summary_line()).collect::<Vec<_>>().join("\n"),
                },
                "create" => {
                    let agent_id = parts.get(1).copied().unwrap_or("");
                    let name = parts.get(2).copied().unwrap_or("");
                    if agent_id.is_empty() || name.is_empty() {
                        return "Usage: /company routine create <agent-id> <name> [--interval <secs>]".into();
                    }
                    let interval: i64 = flag_value(parts, "--interval").and_then(|v| v.parse().ok()).unwrap_or(3600);
                    let prompt = flag_value(parts, "--prompt").unwrap_or("");
                    match rs.create(&cid, agent_id, name, prompt, interval) {
                        Ok(r) => format!("✓ Routine created: {}", r.summary_line()),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                "toggle" => {
                    let id = parts.get(1).copied().unwrap_or("");
                    if id.is_empty() { return "Usage: /company routine toggle <id>".into(); }
                    match rs.toggle(id) {
                        Ok(r) => format!("✓ {}", r.summary_line()),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                _ => "Routine subcommands: list | create <agent-id> <name> [--interval <secs>] | toggle <id>".into(),
            }
        }
    }
}

// ── Heartbeat ─────────────────────────────────────────────────────────────────

fn cmd_heartbeat(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("history");
    let active = get_active_company_id();
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(store) => {
            let cid = match &active { Some(id) => id.clone(), None => return "No active company.".into() };
            let hs = HeartbeatStore::new(store.conn());
            if let Err(e) = hs.ensure_schema() { return format!("Schema error: {e}"); }
            match sub {
                "trigger" => {
                    let agent_id = parts.get(1).copied().unwrap_or("");
                    if agent_id.is_empty() { return "Usage: /company heartbeat trigger <agent-id>".into(); }
                    match hs.trigger_manual(&cid, agent_id) {
                        Ok(hb) => format!("✓ Heartbeat triggered for {} [{}]", agent_id, hb.id),
                        Err(e) => format!("Error: {e}"),
                    }
                }
                "history" => {
                    let agent_id = parts.get(1).copied().unwrap_or("");
                    let limit: i64 = parts.get(2).copied().and_then(|v| v.parse().ok()).unwrap_or(10);
                    let result = if agent_id.is_empty() {
                        hs.company_history(&cid, limit)
                    } else {
                        hs.history(agent_id, limit)
                    };
                    match result {
                        Err(e) => format!("Error: {e}"),
                        Ok(hbs) if hbs.is_empty() => "No heartbeats.".into(),
                        Ok(hbs) => hbs.iter().map(|h| h.summary_line()).collect::<Vec<_>>().join("\n"),
                    }
                }
                _ => "Heartbeat subcommands: trigger <agent-id> | history [agent-id] [limit]".into(),
            }
        }
    }
}

// ── Adapter ───────────────────────────────────────────────────────────────────

fn cmd_adapter(parts: &[&str]) -> String {
    let sub = parts.first().copied().unwrap_or("list");
    let active = get_active_company_id();
    match CompanyStore::open_default() {
        Err(e) => format!("Error: {e}"),
        Ok(_store) => {
            let _cid = match &active { Some(id) => id.clone(), None => return "No active company.".into() };
            let ar = AdapterRegistry::new();
            match sub {
                "list" => {
                    let adapters = ar.list();
                    if adapters.is_empty() {
                        "No custom adapters registered. Built-in: internal.".into()
                    } else {
                        adapters.iter().map(|a| format!(
                            "  [{}] {}  {}", a.adapter_type, a.name, a.label
                        )).collect::<Vec<_>>().join("\n")
                    }
                }
                "register" => {
                    let name = parts.get(1).copied().unwrap_or("");
                    if name.is_empty() { return "Usage: /company adapter register <name> --type <http|process> --url <url>".into(); }
                    let type_str = flag_value(parts, "--type").unwrap_or("http");
                    let url = flag_value(parts, "--url").unwrap_or("");
                    let cmd = flag_value(parts, "--command").unwrap_or("");
                    use crate::adapter_registry::{HttpAdapterConfig, ProcessAdapterConfig};
                    use std::collections::HashMap;
                    if type_str == "process" {
                        ar.register_process(name, ProcessAdapterConfig {
                            command: cmd.to_string(),
                            args: vec![],
                            env: HashMap::new(),
                            working_dir: None,
                        });
                    } else {
                        ar.register_http(name, HttpAdapterConfig {
                            url: url.to_string(),
                            method: "POST".to_string(),
                            headers: HashMap::new(),
                            timeout_secs: 30,
                        });
                    }
                    format!("✓ Adapter registered: {name} [{type_str}]")
                }
                "remove" => {
                    let name = parts.get(1).copied().unwrap_or("");
                    if name.is_empty() { return "Usage: /company adapter remove <name>".into(); }
                    if ar.unregister(name) {
                        format!("✓ Removed: {name}")
                    } else {
                        format!("Adapter '{name}' not found.")
                    }
                }
                _ => "Adapter subcommands: list | register <name> --type <http|process> --url <url> | remove <name>".into(),
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn flag_value<'a>(parts: &[&'a str], flag: &str) -> Option<&'a str> {
    parts.iter().position(|p| *p == flag)
        .and_then(|i| parts.get(i + 1))
        .copied()
}
