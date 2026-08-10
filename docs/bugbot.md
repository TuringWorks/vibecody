---
layout: page
title: BugBot
permalink: /bugbot/
---

> Automated diff review that ends in a **committable fix**, not just a comment.
> Run it locally before you push, or let the GitHub App run it on every pull request.

---

## Quick start

```bash
# Review your uncommitted changes
vibecli --bugbot

# Review just what's staged
vibecli --bugbot --staged

# Review and propose a committable fix for each finding
vibecli --bugbot --propose-fixes

# ...and write those fixes to the working tree
vibecli --bugbot --propose-fixes --apply-fixes

# Review a GitHub pull request and post the fixes as suggestions
vibecli --bugbot --pr 253 --propose-fixes --post-github
```

`--bugbot` exits **1** when any error-severity finding is reported, so it drops
straight into a pre-push hook or a CI step.

---

## What it checks

BugBot runs two passes over the diff:

1. **A static OWASP/CWE scan** — deterministic regex patterns over added lines.
   Runs first and needs no model, so critical issues still surface when the
   provider is down or unconfigured.
2. **An LLM pass** — logic errors, off-by-one mistakes, missing error handling,
   performance regressions, and test-coverage gaps.

Findings are `error`, `warning`, or `info`. Only `error` and `warning` findings
are eligible for a fix — `info` findings are observations, not defects.

BugBot is **provider-agnostic**: it uses whichever provider and model the CLI
resolved (`--provider` / `--model` / your config), never a hard-coded vendor.

---

## Coverage — what the review actually read

A finding count only means something if you know what was looked at. BugBot
splits the diff per file, packs the files into batches that each fit the
per-request budget, and reviews **every batch** — so coverage is a property of
the plan, not of how the diff happened to be ordered.

Every run reports it:

```
Reviewed 12/12 file(s) in 3 model call(s).
```

When coverage is not complete, it says so on stderr and names the files:

```
⚠ Incomplete coverage — 4 file(s) not reviewed (call budget). Review a smaller change (try --staged).
   · crates/big/src/generated.rs
```

The default plan is **8 calls × 8 000 characters**, so a diff up to roughly
64 KB is covered in full. A small diff still costs exactly one call.

The GitHub App carries the same caveat into the commit-status description and
returns a `coverage` object on the webhook response. **"0 issues" over a
partially reviewed diff is not the same claim as "0 issues" over all of it**, and
neither surface pretends otherwise.

### Multiple passes

A model's attention is not uniform across a long prompt: a defect in the last
file of a batch is likelier to be missed than one in the first. `--passes N`
reviews each batch `N` times, rotating which file leads:

```bash
vibecli --bugbot --passes 3
```

Rotation is **deterministic**, so two runs over the same diff issue the same
requests — reproducible in CI, unlike a randomised ordering. Findings are
deduplicated across passes by location plus a normalised message, keeping the
highest severity reported for each. Raising `--passes` scales the call ceiling
with it, so extra passes never cost coverage.

---

## Committable fixes

`--propose-fixes` asks the model for the smallest run of lines that resolves each
finding, then emits a GitHub suggestion block:

````text
❌ src/math.rs:2-2 — Division by zero when b is 0
```suggestion
    let q = a.checked_div(b).ok_or(Error::DivideByZero)?;
```
````

On a pull request (`--pr N --post-github`, or the GitHub App with `auto_fix` on)
each suggestion is attached to its inline review comment, so a reviewer applies
it with GitHub's **Commit suggestion** button.

### How anchoring works — and why it refuses

GitHub applies a suggestion by replacing the **exact lines the comment is
anchored to** in the head commit. An anchor that is off by one silently destroys
code. So a proposal is only ever built from lines BugBot can actually see in the
diff's post-image — never from a line number the model asserted.

A fix is withheld, with the reason printed, whenever:

| Refusal | Meaning |
|---|---|
| `lines N-M of <path> are not in the diff` | The target isn't in this diff's new side; nothing safe to anchor to. |
| `span of N lines exceeds the 20-line limit` | Too large to review as a suggestion, and likelier to drift against head. |
| `replacement was empty` | Deletions are never proposed automatically. |
| `replacement is identical to the original` | Nothing to apply. |
| `replacement contains a code fence` | Would break out of the suggestion block. |
| `model declined to propose a fix` | The model returned `{"skip": true}` or errored. |
| `model reply was not valid fix JSON` | Unparseable — no guess is substituted. |

A finding with no fix is still posted, as prose, exactly as before. **Absent
stays absent** — BugBot never invents an anchor to make the count look better.

### What "verified" means

Every proposal carries a verification level, and today there is exactly one:

**`AnchorVerified`** — the target lines were located in the diff's post-image,
and the replacement is non-empty and different from the original.

That is the whole claim. It does **not** mean the fix compiles, that tests pass,
or that the finding was reproduced. Every comment BugBot posts says so in the
footer. Review a suggestion before you commit it.

`--apply-fixes` adds one more guard at write time: a file whose contents no
longer match the reviewed diff is **skipped**, and both the written and skipped
counts are printed. It never reports success for a write it didn't make.

---

## GitHub App / CI

The daemon exposes `POST /webhook/github` (public — HMAC-verified, not bearer
authenticated). Point a GitHub App at it and BugBot reviews every
`pull_request` `opened` / `synchronize` / `reopened` event, posting inline
comments plus a `vibecody/review` commit status.

**A webhook secret is required.** The route is public, and a review is not a
read: it spends model budget and calls the GitHub API with your token against
whatever repository the payload names. An unsigned webhook is rejected rather
than acted on, with the `set-key` command to fix it in the error.

```toml
[github_app]
app_id = 12345
private_key_path = "path/to/key.pem"    # or GITHUB_APP_PRIVATE_KEY
webhook_secret = "your-webhook-secret"  # or GITHUB_APP_WEBHOOK_SECRET
auto_fix = true                         # attach committable suggestion blocks
severity_threshold = "high"             # critical | high | medium | low
```

`auto_fix` costs one extra model round-trip per actionable finding, bounded at
10 findings per review. It **never pushes a commit** and never opens a branch —
the reviewer stays in control of what lands.

The webhook response reports what actually happened:

```json
{
  "status": "failure",
  "findings": 5,
  "fixes_proposed": 3,
  "coverage": {
    "files_total": 12,
    "files_reviewed": 12,
    "llm_calls": 3,
    "files_truncated": [],
    "files_skipped": []
  },
  "summary": "VibeCody found 5 issue(s): 0 critical, 2 high, 3 medium, 0 low · 3 committable fix(es) proposed"
}
```

`fixes_proposed` counts fixes a reviewer can actually commit — findings the
fixer declined are not counted. `coverage` says what the review read; when it is
incomplete the caveat is appended to `summary` and to the commit-status
description too.

### Secrets

Both the webhook secret and the GitHub token resolve through the encrypted
[ProfileStore](./settings.md) first, per
[Zero-Config First](https://github.com/TuringWorks/vibecody/blob/main/AGENTS.md#zero-config-first--the-user-experience-contract):

```bash
vibecli set-key github gh_pat_...
vibecli set-key github_app_webhook_secret <secret>
```

Environment variables (`GITHUB_TOKEN`, `GH_TOKEN`,
`GITHUB_APP_WEBHOOK_SECRET`) remain as a compatibility fallback. Nothing is ever
written to a plaintext config file.

---

## Flags

| Flag | Effect |
|---|---|
| `--bugbot` | Review the diff and exit. 1 on any error-severity finding. |
| `--staged` | Review the staged index instead of all uncommitted changes. |
| `--pr N` | Review GitHub pull request N. Needs a `github.com` `origin` remote. |
| `--propose-fixes` | Ask for a committable fix per actionable finding. |
| `--apply-fixes` | With `--propose-fixes`: write the fixes to the working tree. |
| `--passes N` | Review each batch N times with rotated file order. Default 1. |
| `--post-github` | With `--pr`: post the review and suggestions to the PR. |
| `--provider` / `--model` | Which model authors the review and the fixes. |

`--pr` refuses to run against a remote that isn't GitHub rather than guessing a
slug — a GitLab or Bitbucket `origin` gets an error, not a review of some
unrelated repository.

---

## BugBot vs `--review`

Two different tools, kept separate on purpose:

| | `--bugbot` | `--review` |
|---|---|---|
| Input | A unified diff | Whole files across a ref range |
| Speed | Fast — one model pass over the diff | Slower — 7 detectors per file |
| Output | Findings + committable suggestions | Scored report, markdown or JSON |
| Best for | Pre-push and PR gating | Release readiness, architecture review |

See [Code Review & Analysis](./FEATURE-MATRIX.md#code-review--analysis) for the
full detector list behind `--review`.
