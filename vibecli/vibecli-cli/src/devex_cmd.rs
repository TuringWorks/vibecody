//! `vibecli --devex …` — Developer Excellence measurement without a GUI.
//!
//! The panel is the comfortable way to read these numbers; this is the way to
//! *enforce* them from a pipeline. `--devex gate` exits non-zero when delivery
//! performance falls below a band the team agreed to hold, which is what turns
//! "we care about lead time" into a build step rather than a slide.
//!
//! Exit codes are part of the contract:
//!
//! | Code | Meaning |
//! |---|---|
//! | 0 | The command succeeded; for `gate`, every required band was met. |
//! | 1 | A required band was missed, or the operation failed. |
//! | 2 | Usage error — unknown subcommand, missing argument, bad value. |
//! | 3 | `gate` could not decide: a required metric was **unmeasurable** here. |
//!
//! `3` exists because "we could not measure lead time" is not "lead time is
//! bad". A pipeline that conflates them either blocks releases for a tooling
//! gap or, worse, passes them because an absent metric defaulted to fine. The
//! separate code lets a team choose which one their pipeline treats as fatal.

use std::path::PathBuf;

use crate::devex_metrics::{
    compute_dora, compute_space, render_scorecard_markdown, render_space_markdown,
    render_survey_markdown, scan_onboarding, scan_practices, scorecard, Band, DoraOptions,
    ReleaseMarker, DEFAULT_WINDOW_DAYS,
};

const USAGE: &str = r#"vibecli --devex <command> [args]

  dora [--path P] [--window D] [--marker tags|merges] [--branch B]
                                  DORA four keys from git history
  practices [--path P]            Engineering-practice maturity (detected)
  onboarding [--path P] [--window D]
                                  Bootstrap readiness + new contributors
  space [--path P] [--window D]   SPACE frame: the dimensions this repository can
                                  answer, and the system that holds each it cannot
  survey                          Print the quarterly experience-survey instrument
  scorecard [--path P] [...]      Delivery + practices in one view
  report [--path P] [...]         The scorecard as markdown, on stdout
  gate [--path P] [--require-deploy-frequency BAND] [--require-lead-time BAND]
       [--require-change-failure-rate BAND] [--require-time-to-restore BAND]
       [--unmeasured-is-failure]
                                  CI check: exit 1 when a required band is missed

  --json                          Machine-readable output for any command
  --path P                        Repository or workspace (default: cwd)
  --window D                      Measurement window in days (default: 90)
  BAND                            elite|high|medium|low — the *worst* acceptable

Exit codes: 0 met · 1 missed or failed · 2 usage error · 3 required metric
unmeasurable (see --unmeasured-is-failure).
"#;

const EXIT_OK: i32 = 0;
const EXIT_MISSED: i32 = 1;
const EXIT_USAGE: i32 = 2;
const EXIT_UNMEASURABLE: i32 = 3;

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

/// Repository to measure. Defaults to the working directory — unlike the HTTP
/// route, which refuses to guess. The difference is deliberate: a CLI *is* run
/// from the directory the user means, while the daemon's cwd is unrelated to
/// any caller's workspace.
fn resolve_path(args: &[String]) -> PathBuf {
    flag(args, "--path")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn resolve_opts(args: &[String]) -> Result<DoraOptions, String> {
    let window_days = match flag(args, "--window") {
        None => DEFAULT_WINDOW_DAYS,
        Some(w) => w
            .parse::<u32>()
            .ok()
            .filter(|w| *w > 0)
            .ok_or_else(|| format!("--window must be a positive number of days, got '{w}'"))?,
    };
    let release_marker = match flag(args, "--marker") {
        None => ReleaseMarker::VersionTags,
        Some(m) => ReleaseMarker::from_str(&m)
            .ok_or_else(|| format!("--marker must be tags|merges, got '{m}'"))?,
    };
    Ok(DoraOptions {
        window_days,
        release_marker,
        release_branch: flag(args, "--branch").unwrap_or_else(|| "HEAD".to_string()),
    })
}

/// Parse a band name for the gate thresholds.
fn parse_band(s: &str) -> Option<Band> {
    match s.trim().to_ascii_lowercase().as_str() {
        "elite" => Some(Band::Elite),
        "high" => Some(Band::High),
        "medium" => Some(Band::Medium),
        "low" => Some(Band::Low),
        _ => None,
    }
}

/// Rank a band so "at least high" is a comparison rather than a match arm.
/// Higher is better.
fn rank(b: Band) -> u8 {
    match b {
        Band::Elite => 4,
        Band::High => 3,
        Band::Medium => 2,
        Band::Low => 1,
    }
}

fn emit_json<T: serde::Serialize>(value: &T) -> i32 {
    match serde_json::to_string_pretty(value) {
        Ok(s) => {
            println!("{s}");
            EXIT_OK
        }
        Err(e) => {
            eprintln!("failed to serialize result: {e}");
            EXIT_MISSED
        }
    }
}

/// The four gate flags, in the order a report reads.
const GATE_FLAGS: &[(&str, &str)] = &[
    ("--require-deploy-frequency", "deployment_frequency"),
    ("--require-lead-time", "lead_time_for_changes"),
    ("--require-change-failure-rate", "change_failure_rate"),
    ("--require-time-to-restore", "time_to_restore"),
];

pub fn run_devex_command(args: &[String]) -> i32 {
    let Some(cmd) = args.first().map(String::as_str) else {
        print!("{USAGE}");
        return EXIT_USAGE;
    };
    let rest = &args[1..];
    let json = has_flag(rest, "--json");
    let path = resolve_path(rest);

    match cmd {
        "dora" => {
            let opts = match resolve_opts(rest) {
                Ok(o) => o,
                Err(e) => return usage(&e),
            };
            match compute_dora(&path, &opts) {
                Ok(r) if json => emit_json(&r),
                Ok(r) => {
                    println!(
                        "DORA — {} · {} day window · deployments = {}",
                        r.repo, r.window_days, r.release_marker_description
                    );
                    for (key, m) in r.measured() {
                        println!(
                            "  {key:<24} {:>10.2} {:<24} band={:<7} n={}",
                            m.value,
                            m.unit,
                            m.band.as_str(),
                            m.sample_size
                        );
                    }
                    for u in &r.unmeasured {
                        println!("  {:<24} UNMEASURED — {}", u.metric, u.reason);
                        println!("  {:<24}   to measure it: {}", "", u.to_measure_this);
                    }
                    for n in &r.notes {
                        println!("  note: {n}");
                    }
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "practices" => match scan_practices(&path) {
            Ok(r) if json => emit_json(&r),
            Ok(r) => {
                println!("{}", r.scope_note);
                for p in &r.practices {
                    println!(
                        "  {:<28} level {} ({:<8}) {}/{}  [{}]",
                        p.title, p.level, p.level_name, p.found, p.expected, p.pillar
                    );
                    for s in p.signals.iter().filter(|s| !s.found) {
                        println!("      missing: {}", s.name);
                    }
                    // Printed with the misses, not in a footnote: a reader who
                    // sees "missing: test directory" needs the caveat in the
                    // same glance, or they have already drawn the conclusion.
                    if let Some(c) = &p.detection_caveat {
                        println!("      caveat: {c}");
                    }
                }
                println!("  mean detected level: {:.2}/{}", r.mean_level, r.max_detectable_level);
                EXIT_OK
            }
            Err(e) => fail(e),
        },

        "onboarding" => {
            let window = match flag(rest, "--window") {
                None => DEFAULT_WINDOW_DAYS,
                Some(w) => match w.parse::<u32>().ok().filter(|w| *w > 0) {
                    Some(w) => w,
                    None => return usage(&format!("--window must be a positive number, got '{w}'")),
                },
            };
            match scan_onboarding(&path, window) {
                Ok(r) if json => emit_json(&r),
                Ok(r) => {
                    println!(
                        "Onboarding readiness: {}/{} signals present",
                        r.readiness_found, r.readiness_expected
                    );
                    for s in &r.readiness {
                        let mark = if s.found { "yes" } else { "NO " };
                        println!("  [{mark}] {} {}", s.name, s.path.clone().unwrap_or_default());
                    }
                    println!(
                        "New contributors in the last {} days: {}",
                        r.window_days,
                        r.new_contributors.len()
                    );
                    for c in r.new_contributors.iter().take(20) {
                        match c.hours_to_second_commit {
                            Some(h) => println!(
                                "  {:<28} {} commits · {:.1}h to second commit",
                                c.author, c.commits_in_window, h
                            ),
                            None => println!(
                                "  {:<28} {} commits · no second commit yet",
                                c.author, c.commits_in_window
                            ),
                        }
                    }
                    for u in &r.not_measured {
                        println!("  {} NOT MEASURED — {}", u.metric, u.reason);
                        println!("      to measure it: {}", u.to_measure_this);
                    }
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "space" => {
            let opts = match resolve_opts(rest) {
                Ok(o) => o,
                Err(e) => return usage(&e),
            };
            // DORA is computed first and handed in, so SPACE Performance
            // *references* the stability pair rather than recomputing and
            // restating it — the double counting the SPACE guidance warns about.
            let dora = match compute_dora(&path, &opts) {
                Ok(d) => d,
                Err(e) => return fail(e),
            };
            match compute_space(&path, opts.window_days, &dora) {
                Ok(sp) if json => emit_json(&sp),
                Ok(sp) if has_flag(rest, "--markdown") => {
                    print!("{}", render_space_markdown(&sp));
                    EXIT_OK
                }
                Ok(sp) => {
                    println!("{}", sp.scope_note);
                    println!();
                    for d in &sp.dimensions {
                        println!("{}", d.title);
                        for m in &d.measures {
                            println!(
                                "    {:<44} {:>10.2} {:<28} [{}] n={}",
                                m.name, m.value, m.unit, m.source, m.sample_size
                            );
                        }
                        for u in &d.unmeasured {
                            println!("    {:<44} NOT MEASURED HERE", u.metric);
                            println!("        {}", u.reason);
                            println!("        to measure it: {}", u.to_measure_this);
                        }
                        if d.measures.is_empty() && d.unmeasured.is_empty() {
                            println!("    (nothing)");
                        }
                    }
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "survey" => {
            print!("{}", render_survey_markdown());
            EXIT_OK
        }

        "scorecard" | "report" => {
            let opts = match resolve_opts(rest) {
                Ok(o) => o,
                Err(e) => return usage(&e),
            };
            match scorecard(&path, &opts) {
                Ok(sc) if json => emit_json(&sc),
                Ok(sc) if cmd == "report" => {
                    print!("{}", render_scorecard_markdown(&sc));
                    EXIT_OK
                }
                Ok(sc) => {
                    println!("{}", sc.headline);
                    println!(
                        "DORA coverage: {:.0}%  ·  delivery grade: {}",
                        sc.dora_coverage * 100.0,
                        sc.delivery_grade.clone().unwrap_or_else(|| "n/a".into())
                    );
                    EXIT_OK
                }
                Err(e) => fail(e),
            }
        }

        "gate" => {
            let opts = match resolve_opts(rest) {
                Ok(o) => o,
                Err(e) => return usage(&e),
            };
            // Parse thresholds before doing any work, so a typo fails in a
            // second rather than after a full history walk.
            let mut required: Vec<(&str, Band)> = Vec::new();
            for (flag_name, metric) in GATE_FLAGS {
                if let Some(raw) = flag(rest, flag_name) {
                    match parse_band(&raw) {
                        Some(b) => required.push((metric, b)),
                        None => {
                            return usage(&format!(
                                "{flag_name} must be elite|high|medium|low, got '{raw}'"
                            ))
                        }
                    }
                }
            }
            if required.is_empty() {
                return usage(
                    "gate needs at least one --require-* threshold; a gate with no criterion \
                     passes everything and measures nothing",
                );
            }
            let unmeasured_fails = has_flag(rest, "--unmeasured-is-failure");

            let report = match compute_dora(&path, &opts) {
                Ok(r) => r,
                Err(e) => return fail(e),
            };
            let measured: std::collections::HashMap<&str, _> =
                report.measured().into_iter().collect();

            let mut missed = Vec::new();
            let mut unmeasurable = Vec::new();
            for (metric, floor) in &required {
                match measured.get(metric) {
                    Some(m) if rank(m.band) >= rank(*floor) => {
                        println!(
                            "PASS  {metric}: {:.2} {} — band {} ≥ required {}",
                            m.value,
                            m.unit,
                            m.band.as_str(),
                            floor.as_str()
                        );
                    }
                    Some(m) => {
                        println!(
                            "MISS  {metric}: {:.2} {} — band {} < required {}",
                            m.value,
                            m.unit,
                            m.band.as_str(),
                            floor.as_str()
                        );
                        missed.push(*metric);
                    }
                    None => {
                        let why = report
                            .unmeasured
                            .iter()
                            .find(|u| u.metric == *metric)
                            .map(|u| u.reason.clone())
                            .unwrap_or_else(|| "not present in the report".to_string());
                        println!("N/A   {metric}: unmeasurable — {why}");
                        unmeasurable.push(*metric);
                    }
                }
            }

            if !missed.is_empty() {
                eprintln!("devex gate: {} metric(s) below the agreed band", missed.len());
                return EXIT_MISSED;
            }
            if !unmeasurable.is_empty() {
                eprintln!(
                    "devex gate: {} required metric(s) could not be measured here. \
                     This is a tooling gap, not a performance result.",
                    unmeasurable.len()
                );
                return if unmeasured_fails {
                    EXIT_MISSED
                } else {
                    EXIT_UNMEASURABLE
                };
            }
            println!("devex gate: every required band met");
            EXIT_OK
        }

        "-h" | "--help" | "help" => {
            print!("{USAGE}");
            EXIT_OK
        }

        other => usage(&format!("unknown --devex command '{other}'")),
    }
}

fn usage(msg: &str) -> i32 {
    eprintln!("{msg}\n");
    eprint!("{USAGE}");
    EXIT_USAGE
}

fn fail(e: anyhow::Error) -> i32 {
    eprintln!("{e:#}");
    EXIT_MISSED
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn no_command_is_a_usage_error() {
        assert_eq!(run_devex_command(&[]), EXIT_USAGE);
    }

    #[test]
    fn unknown_command_is_a_usage_error_not_a_failure() {
        // Kept apart so a pipeline never reads "you typed it wrong" as
        // "delivery performance regressed".
        assert_eq!(run_devex_command(&argv(&["dorra"])), EXIT_USAGE);
    }

    #[test]
    fn help_exits_clean() {
        assert_eq!(run_devex_command(&argv(&["--help"])), EXIT_OK);
    }

    #[test]
    fn a_gate_with_no_criterion_is_refused() {
        // The cheapest way to a green gate must not be to remove its criteria.
        let dir = tempfile::tempdir().expect("tempdir");
        let code = run_devex_command(&argv(&["gate", "--path", &dir.path().display().to_string()]));
        assert_eq!(code, EXIT_USAGE);
    }

    #[test]
    fn a_bad_band_name_fails_before_any_measurement() {
        let dir = tempfile::tempdir().expect("tempdir");
        let code = run_devex_command(&argv(&[
            "gate",
            "--path",
            &dir.path().display().to_string(),
            "--require-lead-time",
            "excellent",
        ]));
        assert_eq!(code, EXIT_USAGE);
    }

    #[test]
    fn bands_are_ordered_best_first() {
        assert!(rank(Band::Elite) > rank(Band::High));
        assert!(rank(Band::High) > rank(Band::Medium));
        assert!(rank(Band::Medium) > rank(Band::Low));
    }

    #[test]
    fn band_names_round_trip() {
        for b in [Band::Elite, Band::High, Band::Medium, Band::Low] {
            assert_eq!(parse_band(b.as_str()), Some(b));
        }
        assert_eq!(parse_band("ELITE"), Some(Band::Elite));
        assert_eq!(parse_band("great"), None);
    }

    #[test]
    fn every_gate_flag_names_a_real_dora_key() {
        const KEYS: &[&str] = &[
            "deployment_frequency",
            "lead_time_for_changes",
            "change_failure_rate",
            "time_to_restore",
        ];
        for (_, metric) in GATE_FLAGS {
            assert!(KEYS.contains(metric), "{metric} is not a DORA key");
        }
        assert_eq!(GATE_FLAGS.len(), KEYS.len(), "every key needs a gate flag");
    }

    #[test]
    fn window_must_be_positive() {
        assert!(resolve_opts(&argv(&["--window", "0"])).is_err());
        assert!(resolve_opts(&argv(&["--window", "abc"])).is_err());
        assert_eq!(
            resolve_opts(&argv(&[])).expect("defaults").window_days,
            DEFAULT_WINDOW_DAYS
        );
    }

    #[test]
    fn exit_codes_are_distinct() {
        // The whole point of code 3 is that it is not code 1.
        let codes = [EXIT_OK, EXIT_MISSED, EXIT_USAGE, EXIT_UNMEASURABLE];
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len());
    }
}
