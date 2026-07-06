//! Golden test: every shipped `skills/*.md` file must parse without error.
//!
//! The skills dir lives in the vibecli crate (monorepo). When this crate is
//! built standalone (published, no monorepo), the dir is absent and the test
//! skips rather than fails.

use std::path::PathBuf;

use skilllensai::Skill;

fn skills_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vibecli/vibecli-cli/skills");
    dir.is_dir().then_some(dir)
}

#[test]
fn all_shipped_skills_parse() {
    let Some(dir) = skills_dir() else {
        eprintln!("skills dir not found — skipping golden test (standalone build)");
        return;
    };

    let mut count = 0usize;
    let mut with_triggers = 0usize;
    let mut with_frontmatter = 0usize;

    for entry in std::fs::read_dir(&dir).expect("read skills dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let skill = Skill::from_file(&path)
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));

        assert!(!skill.name.is_empty(), "empty name for {}", path.display());
        assert!(
            !skill.category.is_empty(),
            "empty category for {}",
            path.display()
        );
        count += 1;
        if !skill.triggers.is_empty() {
            with_triggers += 1;
        }
        if !skill.category.eq("uncategorized") {
            with_frontmatter += 1;
        }
    }

    eprintln!(
        "parsed {count} skills ({with_triggers} with triggers, {with_frontmatter} categorised)"
    );
    assert!(count >= 700, "expected ~710 skills, got {count}");
    // The library is majority-frontmatter; sanity-check the parser actually
    // extracted triggers rather than silently returning empties everywhere.
    assert!(
        with_triggers >= 500,
        "only {with_triggers} skills had triggers — parser likely broken"
    );
}
