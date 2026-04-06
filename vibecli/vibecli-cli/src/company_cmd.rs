//! Non-interactive `/company` command handler for `vibecli --cmd "/company <args>"`.
//!
//! Called by main.rs when the binary is invoked as:
//!   vibecli --cmd "/company status"
//!   vibecli --cmd "/company agent list"
//!   vibecli --cmd "/company goal list"
//!   etc.

use crate::company_goals::{GoalStore, print_goal_tree};
use crate::company_store::{AdapterType, CompanyRole, CompanyStore, get_active_company_id};
use crate::company_tasks::{TaskPriority, TaskStatus, TaskStore};

/// Entry point — `args` is everything after "/company " (may be empty).
pub async fn handle_company_cmd_once(args: &str) -> String {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let sub = parts.first().copied().unwrap_or("help");

    match sub {
        "status" => cmd_status(),
        "list"   => cmd_list(),
        "create" => cmd_create(&parts[1..]),
        "switch" => cmd_switch(parts.get(1).copied().unwrap_or("")),
        "delete" => cmd_delete(parts.get(1).copied().unwrap_or("")),
        "agent"  => cmd_agent(&parts[1..]),
        "goal"   => cmd_goal(&parts[1..]),
        "task"   => cmd_task(&parts[1..]),
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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn flag_value<'a>(parts: &[&'a str], flag: &str) -> Option<&'a str> {
    parts.iter().position(|p| *p == flag)
        .and_then(|i| parts.get(i + 1))
        .copied()
}
