//! Third-party datasets: fetching them, and converting them into tasks.
//!
//! Public benchmarks are how VibeCody's coding ability gets compared to
//! anything outside this repository, so they matter — but they are not
//! vendored here. Two reasons, and both are load-bearing:
//!
//! * **Licensing.** SWE-bench, HumanEval and MBPP each carry their own terms.
//!   Copying rows into an MIT repository would relicense someone else's work
//!   by accident. Manifests record the licence and the upstream URL; the data
//!   is downloaded to `~/.vibecli/evals/datasets/` at the operator's request.
//! * **Contamination.** A benchmark checked into the repository a coding agent
//!   is trained and tested on is a benchmark with a short life.
//!
//! ## Honesty about scores
//!
//! Tasks produced here are *derived from* their datasets, not scored by the
//! upstream harness. Official SWE-bench numbers come from per-instance Docker
//! images with pinned dependency sets; this crate clones the repo at the base
//! commit and runs the declared tests in whatever environment it finds. That
//! is useful for tracking VibeCody against itself and it is **not** a
//! leaderboard-comparable number. Every imported task carries
//! [`crate::task::TaskSource::Imported`] so a report can say which is which,
//! and [`Registry::comparability_note`] states it in words.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::grade::{CommandStep, Grader};
use crate::task::{Capability, Difficulty, EvalTask, Fixture, Limits, Surface, TaskSource};

/// How the downloaded file is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetFormat {
    /// One JSON object per line.
    JsonLines,
    /// A single JSON array of objects.
    JsonArray,
}

/// Which conversion turns rows into tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Importer {
    /// SWE-bench family: clone the repo at `base_commit`, let the agent fix
    /// the issue, then apply the held-out `test_patch` and run the declared
    /// FAIL_TO_PASS / PASS_TO_PASS tests.
    SweBench,
    /// HumanEval: a function stub plus a hidden test harness.
    HumanEval,
    /// MBPP: a natural-language description plus assert statements.
    Mbpp,
    /// Anything else, driven entirely by the manifest's field mapping.
    Generic,
}

/// A dataset this harness knows how to obtain and convert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetManifest {
    pub id: String,
    pub name: String,
    pub homepage: String,
    /// SPDX identifier where one exists, or the upstream's own wording.
    pub license: String,
    /// Anything an operator must know before downloading — attribution
    /// requirements, non-commercial clauses, terms-of-use links.
    #[serde(default)]
    pub license_note: String,
    pub url: String,
    pub format: DatasetFormat,
    pub importer: Importer,
    /// Expected digest of the download.
    ///
    /// `None` is allowed but noisy on purpose: without it the harness cannot
    /// tell you that the benchmark under your feet changed between runs, and
    /// a benchmark that changes silently makes every historical comparison a
    /// guess. The fetcher prints the observed digest so it can be pinned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub capability: Capability,
    pub difficulty: Difficulty,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Field mapping for [`Importer::Generic`].
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

/// Datasets that are *not* wired up, and precisely why.
///
/// Listing them is the point. An eval harness that silently covers only what
/// was easy leaves the reader assuming the gaps are deliberate; naming them
/// turns each into a decision someone can revisit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownGap {
    pub id: String,
    pub name: String,
    pub homepage: String,
    /// What it measures that nothing currently wired does.
    pub measures: String,
    /// What it would take to support it here.
    pub blocked_on: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    #[error("dataset `{0}` is not in the registry")]
    Unknown(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("checksum mismatch for `{id}`: manifest says {expected}, download is {actual}")]
    Checksum {
        id: String,
        expected: String,
        actual: String,
    },
    #[error("cannot read {path}: {reason}")]
    Io { path: PathBuf, reason: String },
    #[error("row {index} of `{id}` is missing required field `{field}`")]
    MissingField {
        id: String,
        index: usize,
        field: String,
    },
    #[error("no cache directory available (HOME is unset)")]
    NoCacheDir,
}

/// The built-in catalogue.
pub struct Registry;

impl Registry {
    /// Datasets with a working importer.
    ///
    /// URLs point at the datasets-server / raw exports rather than a Parquet
    /// snapshot, because this crate deliberately has no Parquet reader: adding
    /// one to pull a benchmark would be a large dependency in service of a
    /// file format, not of the measurement.
    pub fn manifests() -> Vec<DatasetManifest> {
        vec![
            DatasetManifest {
                id: "humaneval".to_string(),
                name: "OpenAI HumanEval".to_string(),
                homepage: "https://github.com/openai/human-eval".to_string(),
                license: "MIT".to_string(),
                license_note: "Attribution required; see upstream LICENSE.".to_string(),
                url: "https://raw.githubusercontent.com/openai/human-eval/master/data/HumanEval.jsonl.gz"
                    .to_string(),
                format: DatasetFormat::JsonLines,
                importer: Importer::HumanEval,
                sha256: None,
                capability: Capability::CodeGeneration,
                difficulty: Difficulty::Easy,
                tags: vec!["external".into(), "python".into(), "function-level".into()],
                fields: BTreeMap::new(),
            },
            DatasetManifest {
                id: "mbpp".to_string(),
                name: "Mostly Basic Python Problems".to_string(),
                homepage: "https://github.com/google-research/google-research/tree/master/mbpp"
                    .to_string(),
                license: "CC-BY-4.0".to_string(),
                license_note: "Attribution required.".to_string(),
                url: "https://raw.githubusercontent.com/google-research/google-research/master/mbpp/mbpp.jsonl"
                    .to_string(),
                format: DatasetFormat::JsonLines,
                importer: Importer::Mbpp,
                sha256: None,
                capability: Capability::CodeGeneration,
                difficulty: Difficulty::Easy,
                tags: vec!["external".into(), "python".into(), "function-level".into()],
                fields: BTreeMap::new(),
            },
            DatasetManifest {
                id: "swebench_verified".to_string(),
                name: "SWE-bench Verified".to_string(),
                homepage: "https://www.swebench.com".to_string(),
                license: "MIT (harness); task repositories keep their own licences".to_string(),
                license_note:
                    "Human-verified subset of 500 instances. Scores produced here are NOT \
                     leaderboard-comparable — see the module docs."
                        .to_string(),
                url: "https://datasets-server.huggingface.co/rows?dataset=princeton-nlp%2FSWE-bench_Verified&config=default&split=test&offset=0&length=100"
                    .to_string(),
                format: DatasetFormat::JsonArray,
                importer: Importer::SweBench,
                sha256: None,
                capability: Capability::CodeRepair,
                difficulty: Difficulty::Hard,
                tags: vec!["external".into(), "python".into(), "repo-level".into()],
                fields: BTreeMap::new(),
            },
        ]
    }

    /// Benchmarks worth having that this harness does not yet run, with the
    /// reason each is blocked.
    pub fn gaps() -> Vec<KnownGap> {
        vec![
            KnownGap {
                id: "terminal_bench".to_string(),
                name: "Terminal-Bench".to_string(),
                homepage: "https://www.tbench.ai".to_string(),
                measures: "End-to-end terminal competence: long shell sessions, \
                           environment setup, recovering from its own mistakes."
                    .to_string(),
                blocked_on: "Per-task Docker images and its own Python runner. Wiring it \
                             means shelling out to the upstream harness and importing its \
                             results, not re-implementing its graders."
                    .to_string(),
            },
            KnownGap {
                id: "aider_polyglot".to_string(),
                name: "Aider polyglot benchmark".to_string(),
                homepage: "https://aider.chat/docs/leaderboards/".to_string(),
                measures: "Editing ability across six languages, on Exercism problems \
                           chosen for being hard to one-shot."
                    .to_string(),
                blocked_on: "Ships as a git repository of exercise directories rather than \
                             a row-oriented file. Needs a directory importer plus per-language \
                             toolchains (go, rust, java, javascript, c++, python)."
                    .to_string(),
            },
            KnownGap {
                id: "gaia".to_string(),
                name: "GAIA".to_string(),
                homepage: "https://huggingface.co/datasets/gaia-benchmark/GAIA".to_string(),
                measures: "General assistant tasks needing web browsing, multi-modal \
                           input and tool composition — the closest public proxy for the \
                           knowledge-work half of this harness."
                    .to_string(),
                blocked_on: "Gated dataset requiring accepted terms and a HuggingFace token; \
                             answers are held out for the public split. Needs an \
                             authenticated fetch path and an exact-match scorer."
                    .to_string(),
            },
            KnownGap {
                id: "tau_bench".to_string(),
                name: "τ-bench".to_string(),
                homepage: "https://github.com/sierra-research/tau-bench".to_string(),
                measures: "Tool-agent-user interaction against domain APIs with policy \
                           constraints — multi-turn work with a simulated user."
                    .to_string(),
                blocked_on: "Needs a simulated-user model in the loop and its own domain \
                             environments. Closest fit for VibeCody's `work_task` \
                             capability; the vendored work suites approximate it locally."
                    .to_string(),
            },
            KnownGap {
                id: "swe_bench_multimodal".to_string(),
                name: "SWE-bench Multimodal".to_string(),
                homepage: "https://www.swebench.com/multimodal.html".to_string(),
                measures: "Repository-level fixes where the issue includes screenshots \
                           or video."
                    .to_string(),
                blocked_on: "Requires a vision-capable provider and image transport through \
                             the harness; the task model has no attachment field yet."
                    .to_string(),
            },
        ]
    }

    pub fn find(id: &str) -> Option<DatasetManifest> {
        Self::manifests().into_iter().find(|m| m.id == id)
    }

    /// The sentence that must accompany any imported score.
    pub fn comparability_note() -> &'static str {
        "Imported tasks are derived from their upstream datasets and graded by this \
         harness, not by the upstream evaluation. Treat them as a signal for tracking \
         VibeCody against itself over time; they are not leaderboard-comparable numbers."
    }
}

/// Where a dataset's raw download lives.
pub fn cache_path(id: &str) -> Result<PathBuf, DatasetError> {
    crate::datasets_dir()
        .map(|dir| dir.join(format!("{}.data", id)))
        .ok_or(DatasetError::NoCacheDir)
}

/// Outcome of a fetch, including the digest actually observed.
#[derive(Debug, Clone, PartialEq)]
pub struct Fetched {
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: usize,
    /// True when the file was already cached and no download happened.
    pub from_cache: bool,
}

/// Download a dataset into the cache, verifying its digest when one is pinned.
pub async fn fetch(manifest: &DatasetManifest) -> Result<Fetched, DatasetError> {
    let path = cache_path(&manifest.id)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DatasetError::Io {
            path: parent.to_path_buf(),
            reason: e.to_string(),
        })?;
    }

    if path.exists() {
        let bytes = std::fs::read(&path).map_err(|e| DatasetError::Io {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        let digest = sha256_hex(&bytes);
        // A cached file that no longer matches the pin is a changed benchmark,
        // not a cache to silently reuse.
        if let Some(expected) = &manifest.sha256 {
            if &digest != expected {
                return Err(DatasetError::Checksum {
                    id: manifest.id.clone(),
                    expected: expected.clone(),
                    actual: digest,
                });
            }
        }
        return Ok(Fetched {
            path,
            sha256: digest,
            bytes: bytes.len(),
            from_cache: true,
        });
    }

    let response = reqwest::get(&manifest.url)
        .await
        .map_err(|e| DatasetError::Download(e.to_string()))?;
    if !response.status().is_success() {
        return Err(DatasetError::Download(format!(
            "{} returned {}",
            manifest.url,
            response.status()
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| DatasetError::Download(e.to_string()))?;
    let digest = sha256_hex(&bytes);
    if let Some(expected) = &manifest.sha256 {
        if &digest != expected {
            return Err(DatasetError::Checksum {
                id: manifest.id.clone(),
                expected: expected.clone(),
                actual: digest,
            });
        }
    }
    std::fs::write(&path, &bytes).map_err(|e| DatasetError::Io {
        path: path.clone(),
        reason: e.to_string(),
    })?;
    Ok(Fetched {
        path,
        sha256: digest,
        bytes: bytes.len(),
        from_cache: false,
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Parse a downloaded file into rows.
pub fn parse_rows(
    text: &str,
    format: DatasetFormat,
) -> Result<Vec<serde_json::Value>, DatasetError> {
    match format {
        DatasetFormat::JsonLines => Ok(text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()),
        DatasetFormat::JsonArray => {
            let value: serde_json::Value = serde_json::from_str(text)
                .map_err(|e| DatasetError::Download(format!("response is not JSON: {}", e)))?;
            // The HuggingFace datasets-server wraps rows as
            // `{"rows":[{"row":{…}}]}`; a plain export is a bare array.
            let rows = match &value {
                serde_json::Value::Array(items) => items.clone(),
                serde_json::Value::Object(map) => map
                    .get("rows")
                    .and_then(|r| r.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| item.get("row").cloned().unwrap_or_else(|| item.clone()))
                            .collect()
                    })
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            Ok(rows)
        }
    }
}

fn field<'a>(row: &'a serde_json::Value, name: &str) -> Option<&'a str> {
    row.get(name).and_then(|v| v.as_str())
}

fn string_list(row: &serde_json::Value, name: &str) -> Vec<String> {
    match row.get(name) {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        // SWE-bench ships these as JSON-encoded strings in some exports and as
        // real arrays in others.
        Some(serde_json::Value::String(s)) => {
            serde_json::from_str::<Vec<String>>(s).unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

/// Convert rows into tasks.
pub fn import(
    manifest: &DatasetManifest,
    rows: &[serde_json::Value],
    limit: Option<usize>,
) -> (Vec<EvalTask>, Vec<DatasetError>) {
    let mut tasks = Vec::new();
    let mut errors = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        if limit.is_some_and(|n| tasks.len() >= n) {
            break;
        }
        let converted = match manifest.importer {
            Importer::HumanEval => import_humaneval(manifest, row, index),
            Importer::Mbpp => import_mbpp(manifest, row, index),
            Importer::SweBench => import_swebench(manifest, row, index),
            Importer::Generic => import_generic(manifest, row, index),
        };
        match converted {
            Ok(task) => tasks.push(task),
            // One malformed row must not lose the other 499.
            Err(e) => errors.push(e),
        }
    }
    (tasks, errors)
}

fn base_task(
    manifest: &DatasetManifest,
    id: String,
    title: String,
    prompt: String,
    instance_id: String,
) -> EvalTask {
    EvalTask {
        id,
        title,
        capability: manifest.capability,
        difficulty: manifest.difficulty,
        surfaces: vec![Surface::Cli],
        prompt,
        fixture: Fixture::default(),
        grader: Grader::AlwaysSkip {
            reason: "importer produced no grader".to_string(),
        },
        limits: Limits::default(),
        tags: manifest.tags.clone(),
        source: TaskSource::Imported {
            dataset: manifest.id.clone(),
            instance_id,
            license: manifest.license.clone(),
        },
        requires: Vec::new(),
        workspace: crate::task::WorkspaceMode::Temp,
    }
}

fn import_humaneval(
    manifest: &DatasetManifest,
    row: &serde_json::Value,
    index: usize,
) -> Result<EvalTask, DatasetError> {
    let missing = |f: &str| DatasetError::MissingField {
        id: manifest.id.clone(),
        index,
        field: f.to_string(),
    };
    let task_id = field(row, "task_id").ok_or_else(|| missing("task_id"))?;
    let prompt_src = field(row, "prompt").ok_or_else(|| missing("prompt"))?;
    let test = field(row, "test").ok_or_else(|| missing("test"))?;
    let entry_point = field(row, "entry_point").ok_or_else(|| missing("entry_point"))?;

    let slug = task_id.replace('/', "-").to_lowercase();
    let mut task = base_task(
        manifest,
        slug,
        format!("HumanEval {}", task_id),
        format!(
            "Complete the function in `solution.py`. The file contains a signature and \
             docstring; implement the body so the described behaviour holds. Do not \
             change the signature, and do not create or modify any other file.\n\n\
             Run `python3 -c \"import solution\"` to check it at least parses."
        ),
        task_id.to_string(),
    );

    task.fixture
        .files
        .insert("solution.py".to_string(), prompt_src.to_string());
    // The test file is written at grade time, not into the fixture: leaving it
    // in the workspace would hand the agent the answer key.
    task.grader = Grader::Command {
        steps: vec![CommandStep {
            cmd: "python3".to_string(),
            args: vec![
                "-c".to_string(),
                format!(
                    "import solution\n\
                     {test}\n\
                     check(solution.{entry})\n\
                     print('HUMANEVAL_OK')",
                    test = test,
                    entry = entry_point
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
            expect_exit: Some(0),
            stdout_contains: Some("HUMANEVAL_OK".to_string()),
            stdout_not_contains: None,
            timeout_secs: Some(120),
        }],
    };
    task.requires = vec!["python3".to_string()];
    Ok(task)
}

fn import_mbpp(
    manifest: &DatasetManifest,
    row: &serde_json::Value,
    index: usize,
) -> Result<EvalTask, DatasetError> {
    let missing = |f: &str| DatasetError::MissingField {
        id: manifest.id.clone(),
        index,
        field: f.to_string(),
    };
    let task_id = row
        .get("task_id")
        .map(|v| v.to_string().trim_matches('"').to_string())
        .ok_or_else(|| missing("task_id"))?;
    let text = field(row, "text")
        .or_else(|| field(row, "prompt"))
        .ok_or_else(|| missing("text"))?;
    let tests = string_list(row, "test_list");
    if tests.is_empty() {
        return Err(missing("test_list"));
    }

    let mut task = base_task(
        manifest,
        format!("mbpp-{}", task_id),
        format!("MBPP {}", task_id),
        format!(
            "Write Python code in `solution.py` that satisfies this requirement:\n\n{}\n\n\
             Define the function at module level so it can be imported. \
             Here is one example of how it will be called:\n\n    {}",
            text,
            // One assert is shown so the agent knows the expected function
            // name and signature; the rest stay hidden, which is what keeps
            // the task from degenerating into transcribing the tests.
            tests.first().map(String::as_str).unwrap_or("")
        ),
        task_id.clone(),
    );

    task.fixture.files.insert(
        "solution.py".to_string(),
        "# Implement the requested function here.\n".to_string(),
    );
    let body = tests
        .iter()
        .map(|t| format!("    {}", t))
        .collect::<Vec<_>>()
        .join("\n");
    task.grader = Grader::Command {
        steps: vec![CommandStep {
            cmd: "python3".to_string(),
            args: vec![
                "-c".to_string(),
                format!(
                    "from solution import *\n\
                     def _check():\n{body}\n\
                     _check()\n\
                     print('MBPP_OK')",
                    body = body
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
            expect_exit: Some(0),
            stdout_contains: Some("MBPP_OK".to_string()),
            stdout_not_contains: None,
            timeout_secs: Some(120),
        }],
    };
    task.requires = vec!["python3".to_string()];
    Ok(task)
}

fn import_swebench(
    manifest: &DatasetManifest,
    row: &serde_json::Value,
    index: usize,
) -> Result<EvalTask, DatasetError> {
    let missing = |f: &str| DatasetError::MissingField {
        id: manifest.id.clone(),
        index,
        field: f.to_string(),
    };
    let instance_id = field(row, "instance_id").ok_or_else(|| missing("instance_id"))?;
    let repo = field(row, "repo").ok_or_else(|| missing("repo"))?;
    let base_commit = field(row, "base_commit").ok_or_else(|| missing("base_commit"))?;
    let problem = field(row, "problem_statement").ok_or_else(|| missing("problem_statement"))?;
    let test_patch = field(row, "test_patch").unwrap_or_default();
    let fail_to_pass = string_list(row, "FAIL_TO_PASS");
    let pass_to_pass = string_list(row, "PASS_TO_PASS");

    if fail_to_pass.is_empty() {
        // Without FAIL_TO_PASS there is nothing that distinguishes a fix from
        // a no-op, and the task would score whatever the repo already did.
        return Err(missing("FAIL_TO_PASS"));
    }

    let mut task = base_task(
        manifest,
        instance_id.to_lowercase().replace(['/', '.'], "-"),
        format!("{} — {}", repo, instance_id),
        format!(
            "You are working in a checkout of `{repo}` at commit {commit}. \
             Fix the following issue by editing the source. Do not edit any test file — \
             the tests used to grade this task are held out and will be applied after \
             you finish.\n\n---\n\n{problem}",
            repo = repo,
            commit = &base_commit[..base_commit.len().min(12)],
            problem = problem
        ),
        instance_id.to_string(),
    );

    // Clone shallowly around the base commit rather than the full history:
    // SWE-bench repos are large, and a full clone per task dominates runtime.
    task.fixture.setup = vec![
        CommandStep {
            cmd: "git".to_string(),
            args: vec![
                "clone".to_string(),
                "--filter=blob:none".to_string(),
                "--no-checkout".to_string(),
                format!("https://github.com/{}.git", repo),
                ".".to_string(),
            ],
            cwd: None,
            env: BTreeMap::new(),
            expect_exit: Some(0),
            stdout_contains: None,
            stdout_not_contains: None,
            timeout_secs: Some(900),
        },
        CommandStep {
            cmd: "git".to_string(),
            args: vec!["checkout".to_string(), base_commit.to_string()],
            cwd: None,
            env: BTreeMap::new(),
            expect_exit: Some(0),
            stdout_contains: None,
            stdout_not_contains: None,
            timeout_secs: Some(900),
        },
    ];

    task.grader = Grader::PatchAndTest {
        test_patch: test_patch.to_string(),
        fail_to_pass,
        pass_to_pass,
        runner: CommandStep {
            cmd: "python3".to_string(),
            args: vec![
                "-m".to_string(),
                "pytest".to_string(),
                "-x".to_string(),
                "-q".to_string(),
                "{test}".to_string(),
            ],
            cwd: None,
            env: BTreeMap::new(),
            expect_exit: Some(0),
            stdout_contains: None,
            stdout_not_contains: None,
            timeout_secs: Some(900),
        },
    };
    task.requires = vec!["git".to_string(), "python3".to_string()];
    task.limits.timeout_secs = Some(2400);
    Ok(task)
}

fn import_generic(
    manifest: &DatasetManifest,
    row: &serde_json::Value,
    index: usize,
) -> Result<EvalTask, DatasetError> {
    let mapped =
        |key: &str| -> Option<&str> { manifest.fields.get(key).and_then(|name| field(row, name)) };
    let missing = |f: &str| DatasetError::MissingField {
        id: manifest.id.clone(),
        index,
        field: manifest
            .fields
            .get(f)
            .cloned()
            .unwrap_or_else(|| f.to_string()),
    };
    let id = mapped("id").ok_or_else(|| missing("id"))?;
    let prompt = mapped("prompt").ok_or_else(|| missing("prompt"))?;
    let expected = mapped("expected_answer");

    let mut task = base_task(
        manifest,
        id.to_lowercase().replace(['/', '.', ' '], "-"),
        mapped("title").unwrap_or(id).to_string(),
        prompt.to_string(),
        id.to_string(),
    );
    task.grader = match expected {
        // Exact-answer datasets grade on the final message.
        Some(answer) => Grader::Transcript {
            assertions: vec![crate::grade::TranscriptAssertion::FinalContains {
                text: answer.to_string(),
            }],
        },
        // Without an expected answer there is nothing to check. Say so rather
        // than emit a grader that would pass on anything.
        None => Grader::AlwaysSkip {
            reason: format!(
                "dataset `{}` declares no `expected_answer` field mapping, so rows \
                 cannot be graded automatically",
                manifest.id
            ),
        },
    };
    Ok(task)
}

/// Write imported tasks out as a suite file.
pub fn to_suite_yaml(
    manifest: &DatasetManifest,
    tasks: &[EvalTask],
) -> Result<String, serde_yaml::Error> {
    let suite = crate::suite::Suite {
        id: format!("imported-{}", manifest.id),
        title: format!("{} (imported)", manifest.name),
        description: format!(
            "Imported from {} ({}). Licence: {}. {} {}",
            manifest.name,
            manifest.homepage,
            manifest.license,
            manifest.license_note,
            Registry::comparability_note()
        ),
        default_surfaces: vec![Surface::Cli],
        defaults: Limits::default(),
        tasks: tasks.to_vec(),
        base_dir: PathBuf::new(),
    };
    serde_yaml::to_string(&suite)
}

/// Suggested path for an imported suite, kept out of the vendored tree so a
/// `git status` after an import is obviously separate from authored work.
pub fn imported_suite_path(suites_dir: &Path, id: &str) -> PathBuf {
    suites_dir.join("imported").join(format!("{}.yaml", id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_manifest_declares_a_licence_and_a_homepage() {
        // A dataset without provenance is one nobody can safely redistribute
        // or reproduce.
        for m in Registry::manifests() {
            assert!(!m.license.trim().is_empty(), "{} has no licence", m.id);
            assert!(m.homepage.starts_with("http"), "{} has no homepage", m.id);
            assert!(!m.url.trim().is_empty(), "{} has no url", m.id);
        }
    }

    #[test]
    fn gaps_explain_themselves() {
        let gaps = Registry::gaps();
        assert!(!gaps.is_empty());
        for g in gaps {
            assert!(!g.blocked_on.trim().is_empty(), "{} has no reason", g.id);
            assert!(!g.measures.trim().is_empty(), "{} says nothing", g.id);
        }
    }

    #[test]
    fn humaneval_row_becomes_a_task_that_hides_its_tests() {
        let row = serde_json::json!({
            "task_id": "HumanEval/0",
            "prompt": "def has_close_elements(numbers, threshold):\n    \"\"\"docstring\"\"\"\n",
            "canonical_solution": "    return True\n",
            "test": "def check(candidate):\n    assert candidate([1.0], 0.5) == False\n",
            "entry_point": "has_close_elements"
        });
        let manifest = Registry::find("humaneval").expect("manifest");
        let (tasks, errors) = import(&manifest, &[row], None);
        assert!(errors.is_empty(), "{:?}", errors);
        let task = &tasks[0];
        assert_eq!(task.id, "humaneval-0");
        assert!(task.fixture.files.contains_key("solution.py"));
        // The answer key must not be in the workspace the agent can read.
        let fixture_text = task.fixture.files.values().cloned().collect::<String>();
        assert!(
            !fixture_text.contains("assert candidate"),
            "tests leaked into the fixture"
        );
        assert!(
            !fixture_text.contains("return True"),
            "solution leaked into the fixture"
        );
        assert!(matches!(task.source, TaskSource::Imported { .. }));
        assert_eq!(task.requires, vec!["python3".to_string()]);
    }

    #[test]
    fn mbpp_row_becomes_a_task() {
        let row = serde_json::json!({
            "task_id": 2,
            "text": "Write a function to find the shared elements from two lists.",
            "code": "def similar_elements(a,b): pass",
            "test_list": [
                "assert similar_elements((3,4,5),(5,7,4)) == (4,5)",
                "assert similar_elements((1,2),(2,3)) == (2,)"
            ]
        });
        let manifest = Registry::find("mbpp").expect("manifest");
        let (tasks, errors) = import(&manifest, &[row], None);
        assert!(errors.is_empty(), "{:?}", errors);
        let task = &tasks[0];
        assert_eq!(task.id, "mbpp-2");
        // Exactly one assert is revealed, for the signature.
        assert!(task.prompt.contains("similar_elements((3,4,5)"));
        assert!(
            !task.prompt.contains("(1,2),(2,3)"),
            "held-out test leaked into the prompt"
        );
    }

    #[test]
    fn swebench_row_becomes_a_patch_and_test_task() {
        let row = serde_json::json!({
            "instance_id": "django__django-12345",
            "repo": "django/django",
            "base_commit": "abcdef1234567890",
            "problem_statement": "ORM does the wrong thing",
            "test_patch": "--- a/tests/x.py\n+++ b/tests/x.py\n",
            "FAIL_TO_PASS": "[\"tests/x.py::test_a\"]",
            "PASS_TO_PASS": ["tests/x.py::test_b"]
        });
        let manifest = Registry::find("swebench_verified").expect("manifest");
        let (tasks, errors) = import(&manifest, &[row], None);
        assert!(errors.is_empty(), "{:?}", errors);
        let task = &tasks[0];
        match &task.grader {
            Grader::PatchAndTest {
                fail_to_pass,
                pass_to_pass,
                ..
            } => {
                // FAIL_TO_PASS arrives JSON-encoded in some exports and as a
                // real array in others; both must parse.
                assert_eq!(fail_to_pass, &vec!["tests/x.py::test_a".to_string()]);
                assert_eq!(pass_to_pass, &vec!["tests/x.py::test_b".to_string()]);
            }
            other => panic!("wrong grader: {:?}", other),
        }
        assert!(task.prompt.contains("Do not edit any test file"));
        assert!(
            !task.prompt.contains("test_a"),
            "held-out test names leaked into the prompt"
        );
    }

    #[test]
    fn a_swebench_row_without_fail_to_pass_is_rejected() {
        // Such a task would score the repo's existing state, not a fix.
        let row = serde_json::json!({
            "instance_id": "x__y-1",
            "repo": "x/y",
            "base_commit": "abc",
            "problem_statement": "p",
            "FAIL_TO_PASS": []
        });
        let manifest = Registry::find("swebench_verified").expect("manifest");
        let (tasks, errors) = import(&manifest, &[row], None);
        assert!(tasks.is_empty());
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn one_bad_row_does_not_lose_the_good_ones() {
        let good = serde_json::json!({
            "task_id": "HumanEval/1", "prompt": "def f(): pass\n",
            "test": "def check(c): pass\n", "entry_point": "f"
        });
        let bad = serde_json::json!({"task_id": "HumanEval/2"});
        let manifest = Registry::find("humaneval").expect("manifest");
        let (tasks, errors) = import(&manifest, &[good, bad], None);
        assert_eq!(tasks.len(), 1);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn generic_import_without_an_expected_answer_is_ungraded_not_passing() {
        let mut manifest = Registry::find("mbpp").expect("manifest");
        manifest.importer = Importer::Generic;
        manifest.fields = BTreeMap::from([
            ("id".to_string(), "qid".to_string()),
            ("prompt".to_string(), "question".to_string()),
        ]);
        let row = serde_json::json!({"qid": "q1", "question": "what is 2+2"});
        let (tasks, errors) = import(&manifest, &[row], None);
        assert!(errors.is_empty());
        assert!(matches!(tasks[0].grader, Grader::AlwaysSkip { .. }));
    }

    #[test]
    fn import_respects_the_limit() {
        let manifest = Registry::find("humaneval").expect("manifest");
        let rows: Vec<serde_json::Value> = (0..10)
            .map(|i| {
                serde_json::json!({
                    "task_id": format!("HumanEval/{}", i),
                    "prompt": "def f(): pass\n",
                    "test": "def check(c): pass\n",
                    "entry_point": "f"
                })
            })
            .collect();
        let (tasks, _) = import(&manifest, &rows, Some(3));
        assert_eq!(tasks.len(), 3);
    }

    #[test]
    fn parses_json_lines_and_the_datasets_server_envelope() {
        let lines = "{\"a\":1}\n\n{\"a\":2}\n";
        assert_eq!(
            parse_rows(lines, DatasetFormat::JsonLines)
                .expect("parse")
                .len(),
            2
        );

        let envelope = r#"{"rows":[{"row":{"a":1}},{"row":{"a":2}}]}"#;
        let rows = parse_rows(envelope, DatasetFormat::JsonArray).expect("parse");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["a"], 1);

        let bare = r#"[{"a":1}]"#;
        assert_eq!(
            parse_rows(bare, DatasetFormat::JsonArray)
                .expect("parse")
                .len(),
            1
        );
    }

    #[test]
    fn imported_suites_round_trip_and_carry_their_licence() {
        let manifest = Registry::find("humaneval").expect("manifest");
        let row = serde_json::json!({
            "task_id": "HumanEval/0", "prompt": "def f(): pass\n",
            "test": "def check(c): pass\n", "entry_point": "f"
        });
        let (tasks, _) = import(&manifest, &[row], None);
        let yaml = to_suite_yaml(&manifest, &tasks).expect("serialize");
        assert!(yaml.contains("MIT"));
        assert!(yaml.contains("not leaderboard-comparable"));
        let suite = crate::suite::Suite::from_yaml(&yaml, Path::new("/tmp/x.yaml"))
            .expect("imported suite must be valid");
        assert_eq!(suite.tasks.len(), 1);
    }

    #[test]
    fn sha256_matches_a_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
