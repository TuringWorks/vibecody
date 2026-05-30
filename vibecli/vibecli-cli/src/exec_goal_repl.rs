//! REPL handlers for the `/goal` slash command.
//!
//! Each handler is a pure-side-effect function that prints to stdout
//! and operates against `SessionStore::open_default()` directly — same
//! convention as `/recap` session at `main.rs:5121-5191`. The daemon
//! HTTP routes in `serve.rs` exist for non-REPL clients (VibeUI,
//! mobile, watch, VS Code, SDK).

use crate::exec_goal::{Goal, GoalLink, GoalLinkKind, GoalStatus, TITLE_MAX_LEN};
use crate::session_store::{GoalListFilter, GoalPatch, SessionStore};

/// Resolve a `<id-prefix>` to a full goal id by scanning the most
/// recent N goals. Returns `None` and prints a clear message on miss.
fn resolve_goal_id(store: &SessionStore, prefix: &str) -> Option<String> {
    if prefix.is_empty() {
        println!("Usage requires a goal id prefix.\n");
        return None;
    }
    let goals = match store.list_goals(&GoalListFilter {
        limit: 200,
        ..Default::default()
    }) {
        Ok(g) => g,
        Err(e) => {
            println!("Failed to list goals: {e}\n");
            return None;
        }
    };
    match goals.into_iter().find(|g| g.id.starts_with(prefix)) {
        Some(g) => Some(g.id),
        None => {
            println!("No goal matched prefix {prefix:?}. Try `/goal list`.\n");
            None
        }
    }
}

fn short(id: &str) -> &str {
    &id[..id.len().min(8)]
}

/// `/goal new <title…>` — create a new goal at default scope. The
/// statement and other fields can be edited via VibeUI or `/goal edit`.
pub fn handle_goal_new(args: &str) {
    let title = args.trim();
    if title.is_empty() {
        println!("Usage: /goal new <title>\n");
        return;
    }
    if title.chars().count() > TITLE_MAX_LEN {
        println!(
            "Title exceeds {TITLE_MAX_LEN} chars ({} chars given). Pick a tighter headline.\n",
            title.chars().count()
        );
        return;
    }
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let mut goal = Goal::new(title);
    // Default the workspace to cwd. The daemon route leaves this
    // nullable; the REPL is always invoked from a workspace, so we
    // attach it for cleaner /goal list filtering.
    goal.workspace = std::env::current_dir().ok();
    match store.insert_goal(&goal) {
        Ok(stored) => {
            println!("Created goal {} — {}", short(&stored.id), stored.title);
            println!("  status: {}", stored.status.as_str());
            if let Some(ws) = &stored.workspace {
                println!("  workspace: {}", ws.display());
            }
            println!(
                "  next: /goal show {} | /goal plan {} (requires daemon) | /goal start {}\n",
                short(&stored.id),
                short(&stored.id),
                short(&stored.id),
            );
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("UNIQUE constraint failed") {
                println!(
                    "A goal with that title already exists in this workspace. \
                     Use `/goal list` to find it, or pick a different title.\n"
                );
            } else {
                println!("Failed to insert goal: {e}\n");
            }
        }
    }
}

/// `/goal list [status]` — list goals, optionally filtered.
pub fn handle_goal_list(args: &str) {
    let arg = args.trim();
    let status = if arg.is_empty() {
        None
    } else {
        match GoalStatus::from_str(arg) {
            Some(s) => Some(s),
            None => {
                println!("Unknown status {arg:?}. Valid: active | paused | done | abandoned\n");
                return;
            }
        }
    };
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let goals = match store.list_goals(&GoalListFilter {
        status,
        limit: 50,
        ..Default::default()
    }) {
        Ok(g) => g,
        Err(e) => {
            println!("Failed to list goals: {e}\n");
            return;
        }
    };
    if goals.is_empty() {
        if let Some(s) = status {
            println!("No goals with status `{}`.\n", s.as_str());
        } else {
            println!("No goals yet. Try `/goal new <title>`.\n");
        }
        return;
    }
    println!("\n{} goals\n", goals.len());
    for g in &goals {
        let scope = g
            .workspace
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("global");
        println!(
            "  {} [{}] [{}] {}",
            short(&g.id),
            g.status.as_str(),
            scope,
            g.title,
        );
    }
    println!();
}

/// `/goal show <id-prefix>` — full detail incl. plan + links.
pub fn handle_goal_show(args: &str) {
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let Some(id) = resolve_goal_id(&store, args.trim()) else {
        return;
    };
    let goal = match store.get_goal_by_id(&id) {
        Ok(Some(g)) => g,
        Ok(None) => {
            println!("Goal vanished between list and read?\n");
            return;
        }
        Err(e) => {
            println!("Failed to load goal: {e}\n");
            return;
        }
    };
    let links = store.list_goal_links(&id).unwrap_or_default();

    println!("\n# {}\n", goal.title);
    println!("id:        {}", goal.id);
    println!("status:    {}", goal.status.as_str());
    if let Some(ws) = &goal.workspace {
        println!("workspace: {}", ws.display());
    } else {
        println!("workspace: (global)");
    }
    if !goal.tags.is_empty() {
        println!("tags:      {}", goal.tags.join(", "));
    }
    println!("created:   {}", goal.created_at.to_rfc3339());
    println!("updated:   {}", goal.updated_at.to_rfc3339());
    if !goal.statement.trim().is_empty() {
        println!("\n## Statement\n{}", goal.statement);
    }
    if !goal.success_criteria.is_empty() {
        println!("\n## Success criteria");
        for c in &goal.success_criteria {
            println!("  - {c}");
        }
    }
    if let Some(plan) = &goal.current_plan {
        println!("\n## Current plan\n{}", plan.display());
    } else {
        println!(
            "\n## Plan\n  (none yet — run /goal plan {} via daemon to generate)",
            short(&goal.id)
        );
    }
    if !links.is_empty() {
        println!("\n## Links ({})", links.len());
        for l in &links {
            let note = l
                .note
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("  — {s}"))
                .unwrap_or_default();
            println!("  [{}] {}{}", l.kind.as_str(), short(&l.target_id), note,);
        }
    }
    println!();
}

/// `/goal status <id> <status>` — flip status. Allows any transition;
/// the audit trail is `updated_at`.
pub fn handle_goal_status(args: &str) {
    let mut parts = args.trim().splitn(2, char::is_whitespace);
    let id_prefix = parts.next().unwrap_or("").trim();
    let status_arg = parts.next().unwrap_or("").trim();
    if id_prefix.is_empty() || status_arg.is_empty() {
        println!("Usage: /goal status <id-prefix> <active|paused|done|abandoned>\n");
        return;
    }
    let status = match GoalStatus::from_str(status_arg) {
        Some(s) => s,
        None => {
            println!("Unknown status {status_arg:?}. Valid: active | paused | done | abandoned\n");
            return;
        }
    };
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let Some(id) = resolve_goal_id(&store, id_prefix) else {
        return;
    };
    let patch = GoalPatch {
        status: Some(status),
        ..Default::default()
    };
    match store.update_goal(&id, &patch) {
        Ok(Some(g)) => println!("Goal {} → {}\n", short(&g.id), g.status.as_str(),),
        Ok(None) => println!("Goal not found.\n"),
        Err(e) => println!("Failed to update goal: {e}\n"),
    }
}

/// `/goal link <id> <session|job|recap|note> <target-id> [note…]`
/// Attach an existing session/job/recap to a goal.
pub fn handle_goal_link(args: &str) {
    let mut parts = args.trim().splitn(4, char::is_whitespace);
    let id_prefix = parts.next().unwrap_or("").trim();
    let kind_str = parts.next().unwrap_or("").trim();
    let target_id = parts.next().unwrap_or("").trim();
    let note = parts
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if id_prefix.is_empty() || kind_str.is_empty() || target_id.is_empty() {
        println!("Usage: /goal link <goal-id> <session|job|recap|note> <target-id> [note]\n");
        return;
    }
    let kind = match GoalLinkKind::from_str(kind_str) {
        Some(k) => k,
        None => {
            println!("Unknown link kind {kind_str:?}. Valid: session | job | recap | note\n");
            return;
        }
    };
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let Some(goal_id) = resolve_goal_id(&store, id_prefix) else {
        return;
    };
    let link = GoalLink {
        id: crate::exec_goal::new_goal_link_id(),
        goal_id: goal_id.clone(),
        kind,
        target_id: target_id.to_string(),
        linked_at: chrono::Utc::now(),
        note,
    };
    match store.insert_goal_link(&link) {
        Ok(_) => println!(
            "Linked {} → {}({})\n",
            short(&goal_id),
            kind.as_str(),
            short(target_id),
        ),
        Err(e) => println!("Failed to link: {e}\n"),
    }
}

/// `/goal start <id> [task…]` — create a new session bound to this
/// goal and print its id. Daemon-side /v1/goals/:id/start does the
/// same; this is the REPL convenience entry-point.
pub fn handle_goal_start(args: &str) {
    let mut parts = args.trim().splitn(2, char::is_whitespace);
    let id_prefix = parts.next().unwrap_or("").trim();
    let task_override = parts.next().map(str::to_string);
    if id_prefix.is_empty() {
        println!("Usage: /goal start <goal-id> [task description]\n");
        return;
    }
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let Some(goal_id) = resolve_goal_id(&store, id_prefix) else {
        return;
    };
    let goal = match store.get_goal_by_id(&goal_id) {
        Ok(Some(g)) => g,
        _ => {
            println!("Goal vanished mid-start.\n");
            return;
        }
    };

    let session_id = uuid::Uuid::new_v4().simple().to_string();
    let task = task_override.unwrap_or_else(|| format!("Goal: {}", goal.title));
    let project_path = goal.workspace.as_ref().and_then(|p| p.to_str());
    let insert_result = match project_path {
        Some(ws) => store.insert_session_with_project(&session_id, &task, "", "", ws),
        None => store.insert_session(&session_id, &task, "", ""),
    };
    if let Err(e) = insert_result {
        println!("Failed to create session: {e}\n");
        return;
    }
    let link = GoalLink {
        id: crate::exec_goal::new_goal_link_id(),
        goal_id: goal_id.clone(),
        kind: GoalLinkKind::Session,
        target_id: session_id.clone(),
        linked_at: chrono::Utc::now(),
        note: Some("started via /goal start".to_string()),
    };
    if let Err(e) = store.insert_goal_link(&link) {
        println!("Session created but link failed: {e}\n");
        return;
    }
    println!(
        "Started session {} for goal {}\n  task: {}\n",
        short(&session_id),
        short(&goal_id),
        task,
    );
}

/// `/goal children <id>` — list direct children of a goal (one level).
pub fn handle_goal_children(args: &str) {
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let Some(parent_id) = resolve_goal_id(&store, args.trim()) else {
        return;
    };
    let parent = match store.get_goal_by_id(&parent_id) {
        Ok(Some(g)) => g,
        _ => {
            println!("Parent vanished mid-lookup.\n");
            return;
        }
    };
    let kids = match store.list_children_goals(&parent_id) {
        Ok(k) => k,
        Err(e) => {
            println!("Failed to list children: {e}\n");
            return;
        }
    };
    if kids.is_empty() {
        println!(
            "Goal {} ({}) has no children. Use `/goal reparent` to attach one.\n",
            short(&parent_id),
            parent.title,
        );
        return;
    }
    println!("\n{} children of {}\n", kids.len(), parent.title);
    for g in &kids {
        println!("  {} [{}] {}", short(&g.id), g.status.as_str(), g.title,);
    }
    println!();
}

/// `/goal reparent <id> <parent-id>` — set a goal's parent. Use the
/// literal `none` (or `-`) as `<parent-id>` to promote the goal to a
/// root.
pub fn handle_goal_reparent(args: &str) {
    let mut parts = args.trim().splitn(2, char::is_whitespace);
    let id_prefix = parts.next().unwrap_or("").trim();
    let parent_arg = parts.next().unwrap_or("").trim();
    if id_prefix.is_empty() || parent_arg.is_empty() {
        println!("Usage: /goal reparent <child-id> <parent-id|none>\n");
        return;
    }
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let Some(child_id) = resolve_goal_id(&store, id_prefix) else {
        return;
    };
    let new_parent: Option<String> = if parent_arg == "none" || parent_arg == "-" {
        None
    } else {
        match resolve_goal_id(&store, parent_arg) {
            Some(p) if p == child_id => {
                println!("A goal cannot be its own parent.\n");
                return;
            }
            Some(p) => Some(p),
            None => return,
        }
    };
    let patch = crate::session_store::GoalPatch {
        parent_goal_id: Some(new_parent.clone()),
        ..Default::default()
    };
    match store.update_goal(&child_id, &patch) {
        Ok(Some(_)) => match &new_parent {
            Some(p) => println!("Reparented {} under {}\n", short(&child_id), short(p),),
            None => println!("Promoted {} to a root.\n", short(&child_id)),
        },
        Ok(None) => println!("Child goal not found.\n"),
        Err(e) => println!("Failed to reparent: {e}\n"),
    }
}

/// `/goal pin <id-prefix>` — mark a goal as the "current" pin for this
/// workspace (cwd) so the daemon can auto-link new sessions to it.
/// Passing the literal `--global` flag pins to the cross-workspace
/// slot used by mobile and watch clients.
pub fn handle_goal_pin(args: &str) {
    let mut global = false;
    let mut rest = args.trim().to_string();
    if let Some(stripped) = rest
        .strip_prefix("--global")
        .map(|s| s.trim_start().to_string())
    {
        global = true;
        rest = stripped;
    }
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let Some(id) = resolve_goal_id(&store, rest.trim()) else {
        return;
    };
    let ws_str = if global {
        None
    } else {
        match std::env::current_dir() {
            Ok(p) => Some(p.to_string_lossy().into_owned()),
            Err(_) => None,
        }
    };
    match store.pin_goal(ws_str.as_deref(), &id) {
        Ok(()) => {
            let scope = ws_str
                .as_deref()
                .map(|s| format!("workspace {s}"))
                .unwrap_or_else(|| "global slot".to_string());
            println!("Pinned {} as current goal ({scope}).\n", short(&id));
        }
        Err(e) => println!("Failed to pin: {e}\n"),
    }
}

/// `/goal unpin` — clear the current pin for the cwd workspace (or the
/// global slot with `--global`).
pub fn handle_goal_unpin(args: &str) {
    let global = args.trim() == "--global";
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let ws_str = if global {
        None
    } else {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    };
    match store.unpin_goal(ws_str.as_deref()) {
        Ok(true) => println!("Cleared pinned goal.\n"),
        Ok(false) => println!("No goal was pinned for that scope.\n"),
        Err(e) => println!("Failed to unpin: {e}\n"),
    }
}

/// `/goal current` — print the currently pinned goal for the cwd
/// workspace, or `--global` for the cross-workspace slot.
pub fn handle_goal_current(args: &str) {
    let global = args.trim() == "--global";
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let ws_str = if global {
        None
    } else {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    };
    match store.get_pinned_goal(ws_str.as_deref()) {
        Ok(Some((goal_id, pinned_at))) => match store.get_goal_by_id(&goal_id) {
            Ok(Some(goal)) => {
                println!(
                    "Current goal: {} — {} ({}, pinned {})\n",
                    short(&goal.id),
                    goal.title,
                    goal.status.as_str(),
                    pinned_at,
                );
            }
            Ok(None) => println!("Pinned goal {goal_id} no longer exists.\n"),
            Err(e) => println!("Failed to load pinned goal: {e}\n"),
        },
        Ok(None) => println!("No goal is currently pinned.\n"),
        Err(e) => println!("Failed to read pin: {e}\n"),
    }
}

/// `/goal delete <id>` — hard-delete a goal and (via FK cascade) its
/// links. Confirms by id-prefix only; the user must paste an unambiguous
/// prefix.
pub fn handle_goal_delete(args: &str) {
    let store = match SessionStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open session store: {e}\n");
            return;
        }
    };
    let Some(id) = resolve_goal_id(&store, args.trim()) else {
        return;
    };
    match store.delete_goal(&id) {
        Ok(true) => println!("Deleted goal {}.\n", short(&id)),
        Ok(false) => println!("Goal vanished before delete.\n"),
        Err(e) => println!("Failed to delete: {e}\n"),
    }
}
