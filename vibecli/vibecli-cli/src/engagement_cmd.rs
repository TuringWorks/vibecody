//! `vibecli --engagement …` — the engagement lifecycle without a GUI.
//!
//! The panel is the comfortable way to run an engagement; this is the way to
//! *check* one from a pipeline. `--engagement gate <id>` exits non-zero while a
//! phase still has blockers, which is what lets "a security review that happens
//! during the build rather than after it" be a build step rather than a promise.
//!
//! Exit codes are part of the contract:
//!
//! | Code | Meaning |
//! |---|---|
//! | 0 | The command succeeded; for `gate`, the phase is clean. |
//! | 1 | The phase has blockers, or the operation failed. |
//! | 2 | Usage error — unknown subcommand, missing argument, bad value. |
//!
//! `1` and `2` are kept apart on purpose: a pipeline that treats "you typed the
//! phase name wrong" as "the engagement is not ready" fails for a reason nobody
//! can act on.

use crate::engagement::{
    render_handover_markdown, render_report_markdown, DeliverableStatus, EngagementStore,
    EvidenceKind, GateVerdict, Phase,
};

const USAGE: &str = r#"vibecli --engagement <command> [args]

  list                                  List engagements
  new <name> [--client C] [--workspace P] [--summary S]
                                        Create one, seeded with all four phases
  show <id>                             Phase-by-phase readiness table
  report <id>                           Status report (markdown, stdout)
  handover <id>                         Handover pack (markdown, stdout)
  deliverables <id> [--phase P]         List deliverables
  set <id> <key> <status>               Set a deliverable's status
                                        (not_started|in_progress|ready|accepted|waived)
  evidence <id> <key> <reference> [--kind K] [--label L]
                                        Attach evidence (file|url|run|metric|note)
  gates <id> [--phase P]                List gates
  judge <id> <gate-id> <verdict> [--observed O] [--rationale R] [--by W]
                                        Record a verdict
                                        (not_measured|pending|pass|fail|waived)
  advance <id> [--force]                Close the current phase
  gate <id> [--phase P]                 CI check: exit 1 while blockers remain
  scan <id> [--path P] [--attach]       Propose workspace files as evidence.
                                        Never changes a deliverable's status.

  P = discover|prove|build|operate

Exit codes: 0 ok / clean · 1 blocked or failed · 2 usage error.
"#;

const EXIT_OK: i32 = 0;
const EXIT_BLOCKED: i32 = 1;
const EXIT_USAGE: i32 = 2;

/// Read `--name value` from the argument list.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

/// Positional arguments, i.e. everything that is not a flag or a flag's value.
fn positionals(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for a in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a.starts_with("--") {
            // `--force` takes no value; every other flag here does.
            if a != "--force" {
                skip_next = true;
            }
            continue;
        }
        out.push(a.clone());
    }
    out
}

fn parse_phase_flag(args: &[String]) -> Result<Option<Phase>, String> {
    match flag(args, "--phase") {
        None => Ok(None),
        Some(p) => Phase::from_str(&p)
            .map(Some)
            .ok_or_else(|| format!("unknown phase '{p}'; expected discover|prove|build|operate")),
    }
}

/// `n/a`, never `0%`. Same rule as the panel and the markdown report.
fn pct(v: Option<f64>) -> String {
    match v {
        None => "n/a".to_string(),
        Some(f) => format!("{:.0}%", f * 100.0),
    }
}

pub fn run_engagement_command(args: &[String]) -> i32 {
    let store = match EngagementStore::open_default() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("engagement store unavailable: {e:#}");
            return EXIT_BLOCKED;
        }
    };

    let Some(cmd) = args.first().map(String::as_str) else {
        print!("{USAGE}");
        return EXIT_USAGE;
    };
    let rest = &args[1..];
    let pos = positionals(rest);

    match cmd {
        "list" => match store.list() {
            Ok(items) if items.is_empty() => {
                println!("No engagements. Create one with: vibecli --engagement new <name>");
                EXIT_OK
            }
            Ok(items) => {
                for e in items {
                    println!(
                        "{}  {}  [{}]  phase={}  {}",
                        e.id,
                        e.name,
                        e.status.as_str(),
                        e.current_phase.as_str(),
                        e.client
                    );
                }
                EXIT_OK
            }
            Err(e) => fail(e),
        },

        "new" => {
            let Some(name) = pos.first() else {
                eprintln!("usage: vibecli --engagement new <name> [--client C]");
                return EXIT_USAGE;
            };
            match store.create(
                name,
                &flag(rest, "--client").unwrap_or_default(),
                flag(rest, "--workspace").as_deref(),
                &flag(rest, "--summary").unwrap_or_default(),
            ) {
                Ok(e) => {
                    println!("{}", e.id);
                    eprintln!(
                        "Created '{}' seeded with all four phases, every promised deliverable, \
                         and its gates — unmeasured until measured.",
                        e.name
                    );
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "show" => {
            let Some(id) = pos.first() else {
                return usage("show <id>");
            };
            match store.report(id) {
                Ok(r) => {
                    println!("{}  [{}]", r.engagement.name, r.engagement.status.as_str());
                    println!(
                        "Current phase: {} ({} of 4)\n",
                        r.engagement.current_phase.title(),
                        r.engagement.current_phase.index() + 1
                    );
                    println!(
                        "{:<20} {:<16} {:>8} {:>10} {:>12} EXIT",
                        "PHASE", "CADENCE", "ACCEPTED", "COMPLETE", "GATES PASS"
                    );
                    for p in &r.phases {
                        let in_scope = p.deliverables.total - p.deliverables.waived;
                        println!(
                            "{:<20} {:<16} {:>8} {:>10} {:>12} {}",
                            p.title,
                            // A dash, not an invented duration.
                            p.cadence.as_deref().unwrap_or("—"),
                            format!("{}/{}", p.deliverables.accepted, in_scope),
                            pct(p.completion),
                            format!("{}/{}", p.gates.pass, p.gates.total),
                            if p.can_exit {
                                "ready".to_string()
                            } else {
                                format!("{} blocker(s)", p.blockers.len())
                            }
                        );
                    }
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "report" | "handover" => {
            let Some(id) = pos.first() else {
                return usage(&format!("{cmd} <id>"));
            };
            let rendered = if cmd == "report" {
                render_report_markdown(&store, id)
            } else {
                render_handover_markdown(&store, id)
            };
            match rendered {
                Ok(md) => {
                    print!("{md}");
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "deliverables" => {
            let Some(id) = pos.first() else {
                return usage("deliverables <id> [--phase P]");
            };
            let phase = match parse_phase_flag(rest) {
                Ok(p) => p,
                Err(e) => return usage(&e),
            };
            match store.deliverables(id, phase) {
                Ok(items) => {
                    for d in items {
                        println!(
                            "{:<10} {:<38} {:<12} ev={:<3} {}",
                            d.phase.as_str(),
                            d.key,
                            d.status.as_str(),
                            d.evidence_count,
                            d.tool_hint.as_deref().unwrap_or("—")
                        );
                    }
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "set" => {
            let (Some(id), Some(key), Some(status)) = (pos.first(), pos.get(1), pos.get(2)) else {
                return usage("set <id> <key> <status>");
            };
            // `DeliverableStatus::from_str` falls back to `not_started`, which
            // is right for a database column and wrong here: a typo would
            // silently reset a deliverable the client had accepted.
            let parsed = match status.as_str() {
                "not_started" => DeliverableStatus::NotStarted,
                "in_progress" => DeliverableStatus::InProgress,
                "ready" => DeliverableStatus::Ready,
                "accepted" => DeliverableStatus::Accepted,
                "waived" => DeliverableStatus::Waived,
                other => {
                    return usage(&format!(
                        "unknown status '{other}'; expected \
                         not_started|in_progress|ready|accepted|waived"
                    ))
                }
            };
            match store.deliverable_by_key(id, key) {
                Ok(Some(d)) => match store.update_deliverable(
                    &d.id,
                    Some(parsed),
                    flag(rest, "--owner").as_deref(),
                    flag(rest, "--notes").as_deref(),
                ) {
                    Ok(()) => {
                        println!("{key} = {}", parsed.as_str());
                        if matches!(
                            parsed,
                            DeliverableStatus::Ready | DeliverableStatus::Accepted
                        ) && d.evidence_count == 0
                        {
                            eprintln!(
                                "warning: '{key}' is now {} with no evidence attached; \
                                 it will block the phase until something backs it.",
                                parsed.as_str()
                            );
                        }
                        EXIT_OK
                    }
                    Err(e) => fail(e),
                },
                Ok(None) => {
                    eprintln!("no deliverable '{key}' on that engagement");
                    EXIT_BLOCKED
                }
                Err(e) => fail(e),
            }
        }

        "evidence" => {
            let (Some(id), Some(key), Some(reference)) = (pos.first(), pos.get(1), pos.get(2))
            else {
                return usage("evidence <id> <key> <reference> [--kind K] [--label L]");
            };
            let kind = match flag(rest, "--kind").as_deref() {
                None => EvidenceKind::Note,
                Some("file") => EvidenceKind::File,
                Some("url") => EvidenceKind::Url,
                Some("run") => EvidenceKind::Run,
                Some("metric") => EvidenceKind::Metric,
                Some("note") => EvidenceKind::Note,
                Some(other) => {
                    return usage(&format!(
                        "unknown evidence kind '{other}'; expected file|url|run|metric|note"
                    ))
                }
            };
            match store.deliverable_by_key(id, key) {
                Ok(Some(d)) => {
                    let label = flag(rest, "--label").unwrap_or_else(|| d.title.clone());
                    match store.add_evidence(&d.id, kind, &label, reference) {
                        Ok(_) => {
                            println!("attached {} evidence to {key}", kind.as_str());
                            EXIT_OK
                        }
                        Err(e) => fail(e),
                    }
                }
                Ok(None) => {
                    eprintln!("no deliverable '{key}' on that engagement");
                    EXIT_BLOCKED
                }
                Err(e) => fail(e),
            }
        }

        "gates" => {
            let Some(id) = pos.first() else {
                return usage("gates <id> [--phase P]");
            };
            let phase = match parse_phase_flag(rest) {
                Ok(p) => p,
                Err(e) => return usage(&e),
            };
            match store.gates(id, phase) {
                Ok(items) => {
                    for g in items {
                        println!(
                            "{:<10} {:<14} {}\n           observed: {}",
                            g.phase.as_str(),
                            g.verdict.as_str(),
                            g.title,
                            // Absent stays absent.
                            g.observed.as_deref().unwrap_or("(not recorded)")
                        );
                        println!("           id: {}", g.id);
                    }
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "judge" => {
            let (Some(_id), Some(gid), Some(verdict)) = (pos.first(), pos.get(1), pos.get(2))
            else {
                return usage("judge <id> <gate-id> <verdict> [--observed O]");
            };
            let parsed = match verdict.as_str() {
                "not_measured" => GateVerdict::NotMeasured,
                "pending" => GateVerdict::Pending,
                "pass" => GateVerdict::Pass,
                "fail" => GateVerdict::Fail,
                "waived" => GateVerdict::Waived,
                other => {
                    return usage(&format!(
                        "unknown verdict '{other}'; expected \
                         not_measured|pending|pass|fail|waived"
                    ))
                }
            };
            let observed = flag(rest, "--observed");
            // Same rule the HTTP route enforces: a pass with nothing observed
            // is an assertion, not a measurement.
            if parsed == GateVerdict::Pass
                && observed
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
            {
                eprintln!("a passing gate must record what was observed (--observed \"…\")");
                return EXIT_USAGE;
            }
            match store.judge_gate(
                gid,
                parsed,
                observed.as_deref(),
                &flag(rest, "--rationale").unwrap_or_default(),
                flag(rest, "--by").as_deref(),
            ) {
                Ok(()) => {
                    println!("{gid} = {}", parsed.as_str());
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "advance" => {
            let Some(id) = pos.first() else {
                return usage("advance <id> [--force]");
            };
            match store.advance_phase(id, has_flag(rest, "--force")) {
                Ok(o) => {
                    println!("{}", o.reason);
                    for b in &o.blockers {
                        println!("  - {}", b.detail);
                    }
                    if o.advanced {
                        EXIT_OK
                    } else {
                        EXIT_BLOCKED
                    }
                }
                Err(e) => fail(e),
            }
        }

        // The pipeline entry point.
        "gate" => {
            let Some(id) = pos.first() else {
                return usage("gate <id> [--phase P]");
            };
            let engagement = match store.get(id) {
                Ok(Some(e)) => e,
                Ok(None) => {
                    eprintln!("no engagement '{id}'");
                    return EXIT_BLOCKED;
                }
                Err(e) => return fail(e),
            };
            let phase = match parse_phase_flag(rest) {
                Ok(Some(p)) => p,
                Ok(None) => engagement.current_phase,
                Err(e) => return usage(&e),
            };
            match store.phase_readiness(id, phase) {
                Ok(r) => {
                    if r.can_exit {
                        println!(
                            "{}: clean — {} of {} deliverables accepted, {} gate(s) pass.",
                            r.title,
                            r.deliverables.accepted,
                            r.deliverables.total - r.deliverables.waived,
                            r.gates.pass
                        );
                        EXIT_OK
                    } else {
                        println!("{}: {} blocker(s)", r.title, r.blockers.len());
                        for b in &r.blockers {
                            println!("  - {}", b.detail);
                        }
                        // Reported separately so a reader can tell "nobody
                        // looked" from "we looked and it failed".
                        if r.gates.not_measured > 0 {
                            println!(
                                "\n{} gate(s) are unmeasured. That is not a failure — it is an \
                                 absence of measurement, and it blocks for that reason.",
                                r.gates.not_measured
                            );
                        }
                        EXIT_BLOCKED
                    }
                }
                Err(e) => fail(e),
            }
        }

        "scan" => {
            let Some(id) = pos.first() else {
                return usage("scan <id> [--path P] [--attach]");
            };
            // `--path`, then the engagement's bound workspace, then the cwd.
            // No silent guess: whichever it resolves to is printed.
            let root = match flag(rest, "--path") {
                Some(p) => p,
                None => match store.get(id) {
                    Ok(Some(e)) => e.workspace_path.unwrap_or_else(|| ".".to_string()),
                    Ok(None) => {
                        eprintln!("no engagement '{id}'");
                        return EXIT_BLOCKED;
                    }
                    Err(e) => return fail(e),
                },
            };
            let report = match crate::engagement_scan::scan(&root) {
                Ok(r) => r,
                Err(e) => return fail(e),
            };
            println!(
                "Scanned {} ({} files) — {} candidate(s).",
                report.root,
                report.files_scanned,
                report.candidates.len()
            );
            for c in &report.candidates {
                println!("  {:<38} {}  [{}]", c.deliverable_key, c.path, c.rule);
            }
            for n in &report.notes {
                println!("\n{n}");
            }
            if !report.undetectable.is_empty() {
                println!("No rule exists for: {}", report.undetectable.join(", "));
            }
            if has_flag(rest, "--attach") {
                match crate::engagement_scan::attach(&store, id, &report) {
                    Ok(n) => println!(
                        "\nAttached {n} evidence item(s). No deliverable status was changed — \
                         a detected file is a candidate, not a completed deliverable."
                    ),
                    Err(e) => return fail(e),
                }
            }
            EXIT_OK
        }

        "--help" | "-h" | "help" => {
            print!("{USAGE}");
            EXIT_OK
        }

        other => {
            eprintln!("unknown subcommand '{other}'\n");
            print!("{USAGE}");
            EXIT_USAGE
        }
    }
}

fn fail(e: anyhow::Error) -> i32 {
    eprintln!("{e:#}");
    EXIT_BLOCKED
}

fn usage(msg: &str) -> i32 {
    eprintln!("usage: vibecli --engagement {msg}");
    EXIT_USAGE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn positionals_skip_flags_and_their_values() {
        let args = v(&["abc", "--phase", "prove", "key", "--force", "third"]);
        assert_eq!(positionals(&args), v(&["abc", "key", "third"]));
    }

    #[test]
    fn force_is_a_bare_flag_and_does_not_swallow_the_next_argument() {
        // `--force` taking a value would eat the id and turn "advance anyway"
        // into a usage error the operator cannot explain.
        let args = v(&["engagement-id", "--force"]);
        assert_eq!(positionals(&args), v(&["engagement-id"]));
        assert!(has_flag(&args, "--force"));
    }

    #[test]
    fn flag_reads_the_following_value() {
        let args = v(&["--client", "Acme Corp", "--phase", "build"]);
        assert_eq!(flag(&args, "--client").as_deref(), Some("Acme Corp"));
        assert_eq!(flag(&args, "--phase").as_deref(), Some("build"));
        assert_eq!(flag(&args, "--missing"), None);
    }

    #[test]
    fn a_trailing_flag_with_no_value_is_none_not_a_panic() {
        let args = v(&["--client"]);
        assert_eq!(flag(&args, "--client"), None);
    }

    #[test]
    fn phase_flag_rejects_a_typo() {
        assert!(parse_phase_flag(&v(&["--phase", "discovery"])).is_err());
        assert_eq!(
            parse_phase_flag(&v(&["--phase", "operate"])),
            Ok(Some(Phase::Operate))
        );
        assert_eq!(parse_phase_flag(&v(&[])), Ok(None));
    }

    #[test]
    fn exit_codes_separate_blocked_from_usage() {
        // A pipeline that cannot tell "you typed the phase wrong" from "the
        // engagement is not ready" fails for a reason nobody can act on.
        assert_ne!(EXIT_BLOCKED, EXIT_USAGE);
        assert_eq!(EXIT_OK, 0);
    }

    #[test]
    fn pct_reports_na_for_an_empty_denominator() {
        assert_eq!(pct(None), "n/a");
        assert_eq!(pct(Some(0.0)), "0%");
        assert_eq!(pct(Some(1.0)), "100%");
    }

    // ── Dispatcher, end to end ────────────────────────────────────────────
    //
    // These drive `run_engagement_command` against a scratch database, so the
    // exit-code contract a pipeline depends on is measured rather than
    // asserted in prose.
    //
    // `TestDb` is shared with `engagement_routes`' tests on purpose: it carries
    // the single lock over the process-global `VIBECLI_ENGAGEMENT_DB`. When
    // each module had its own lock, a routes test cleared the variable while a
    // command test was mid-run and the command reported "no engagement" — a
    // wrong exit code with no hint that isolation was the cause.
    use crate::engagement::test_support::TestDb;

    /// Create an engagement and return its id, by going through the store the
    /// command uses (the command prints the id to stdout, which a test cannot
    /// capture without redirecting the process's own handle).
    fn seed_engagement() -> String {
        let store = EngagementStore::open_default().expect("store");
        store
            .create("Acme platform", "Acme Corp", None, "")
            .expect("create")
            .id
    }

    #[test]
    fn no_subcommand_is_a_usage_error() {
        let _db = TestDb::new();
        assert_eq!(run_engagement_command(&[]), EXIT_USAGE);
        assert_eq!(run_engagement_command(&v(&["nonsense"])), EXIT_USAGE);
        assert_eq!(run_engagement_command(&v(&["help"])), EXIT_OK);
    }

    #[test]
    fn gate_blocks_a_fresh_engagement_and_clears_once_satisfied() {
        let _db = TestDb::new();
        let id = seed_engagement();

        // Nothing measured yet — the pipeline must fail.
        assert_eq!(
            run_engagement_command(&v(&["gate", &id])),
            EXIT_BLOCKED,
            "a fresh engagement has measured nothing, so its phase cannot pass"
        );

        let store = EngagementStore::open_default().expect("store");
        for d in store
            .deliverables(&id, Some(Phase::Discover))
            .expect("deliverables")
        {
            store
                .update_deliverable(&d.id, Some(DeliverableStatus::Accepted), None, None)
                .expect("accept");
            store
                .add_evidence(&d.id, EvidenceKind::Note, "done", "seen")
                .expect("evidence");
        }
        for g in store.gates(&id, Some(Phase::Discover)).expect("gates") {
            store
                .judge_gate(&g.id, GateVerdict::Pass, Some("checked"), "", Some("rb"))
                .expect("judge");
        }

        assert_eq!(run_engagement_command(&v(&["gate", &id])), EXIT_OK);
    }

    #[test]
    fn a_bad_phase_is_a_usage_error_not_a_blocked_gate() {
        let _db = TestDb::new();
        let id = seed_engagement();
        // The distinction a pipeline depends on: "you typed it wrong" must not
        // look like "the engagement is not ready".
        assert_eq!(
            run_engagement_command(&v(&["gate", &id, "--phase", "discovery"])),
            EXIT_USAGE
        );
        assert_eq!(
            run_engagement_command(&v(&["gate", &id, "--phase", "build"])),
            EXIT_BLOCKED
        );
    }

    #[test]
    fn a_misspelt_status_is_refused_rather_than_resetting_a_deliverable() {
        let _db = TestDb::new();
        let id = seed_engagement();
        let store = EngagementStore::open_default().expect("store");
        let key = "risk-register";
        store
            .update_deliverable(
                &store
                    .deliverable_by_key(&id, key)
                    .expect("query")
                    .expect("present")
                    .id,
                Some(DeliverableStatus::Accepted),
                None,
                None,
            )
            .expect("accept");

        assert_eq!(
            run_engagement_command(&v(&["set", &id, key, "acccepted"])),
            EXIT_USAGE
        );
        // Still accepted — a typo must not silently downgrade a signed-off
        // deliverable to not_started.
        assert_eq!(
            store
                .deliverable_by_key(&id, key)
                .expect("query")
                .expect("present")
                .status,
            DeliverableStatus::Accepted
        );
    }

    #[test]
    fn judging_a_pass_without_an_observation_is_refused() {
        let _db = TestDb::new();
        let id = seed_engagement();
        let store = EngagementStore::open_default().expect("store");
        let gid = store
            .gates(&id, Some(Phase::Prove))
            .expect("gates")
            .into_iter()
            .next()
            .expect("a prove gate")
            .id;

        assert_eq!(
            run_engagement_command(&v(&["judge", &id, &gid, "pass"])),
            EXIT_USAGE
        );
        assert_eq!(
            store.gate(&gid).expect("get").expect("present").verdict,
            GateVerdict::NotMeasured,
            "the refused pass must leave the gate unmeasured"
        );

        assert_eq!(
            run_engagement_command(&v(&[
                "judge",
                &id,
                &gid,
                "pass",
                "--observed",
                "ran on the client's cluster against their data"
            ])),
            EXIT_OK
        );
        assert_eq!(
            store.gate(&gid).expect("get").expect("present").verdict,
            GateVerdict::Pass
        );
    }

    #[test]
    fn advance_reports_blocked_then_succeeds_when_forced() {
        let _db = TestDb::new();
        let id = seed_engagement();
        assert_eq!(run_engagement_command(&v(&["advance", &id])), EXIT_BLOCKED);
        assert_eq!(
            run_engagement_command(&v(&["advance", &id, "--force"])),
            EXIT_OK
        );
        let store = EngagementStore::open_default().expect("store");
        assert_eq!(
            store.get(&id).expect("get").expect("present").current_phase,
            Phase::Prove
        );
    }

    #[test]
    fn evidence_and_report_run_against_a_real_engagement() {
        let _db = TestDb::new();
        let id = seed_engagement();
        assert_eq!(
            run_engagement_command(&v(&[
                "evidence",
                &id,
                "threat-model",
                "docs/threat-model.md",
                "--kind",
                "file"
            ])),
            EXIT_OK
        );
        assert_eq!(
            run_engagement_command(&v(&["evidence", &id, "no-such-key", "x"])),
            EXIT_BLOCKED
        );
        assert_eq!(
            run_engagement_command(&v(&[
                "evidence",
                &id,
                "threat-model",
                "x",
                "--kind",
                "psychic"
            ])),
            EXIT_USAGE
        );
        for cmd in [
            "show",
            "report",
            "handover",
            "deliverables",
            "gates",
            "list",
        ] {
            assert_eq!(
                run_engagement_command(&v(&[cmd, &id])),
                EXIT_OK,
                "'{cmd}' failed"
            );
        }
    }

    #[test]
    fn usage_text_documents_every_subcommand_the_dispatcher_handles() {
        for cmd in [
            "list",
            "new",
            "show",
            "report",
            "handover",
            "deliverables",
            "set",
            "evidence",
            "gates",
            "judge",
            "advance",
            "gate",
            "scan",
        ] {
            assert!(
                USAGE.contains(cmd),
                "subcommand '{cmd}' is dispatched but undocumented in --help"
            );
        }
    }
}
