---
layout: page
title: "Tutorial: AI-Powered Code Review"
permalink: /tutorials/code-review/
---

# AI-Powered Code Review

Use VibeCody to get instant, AI-powered feedback on code changes -- uncommitted work, branches, or GitHub pull requests.

**Prerequisites:**
- VibeCody installed with a working provider (see [First Provider Tutorial](/tutorials/first-provider/))
- A Git repository with some changes to review

---

## Review Uncommitted Changes

The simplest use case: you have been editing files and want AI feedback before you commit.

### From the Command Line

```bash
vibecli --review
```

### From the REPL

```bash
vibecli
```

```
vibecli> /review
```

### Expected Output

```
[review] Analyzing diff (4 files, +83 -21 lines)...

## Code Review Summary

### src/auth.rs (2 issues)
  [HIGH] Line 42: SQL query built with string concatenation.
         This is vulnerable to SQL injection.
         Suggestion: Use parameterized queries with bind variables.

  [MED]  Line 78: Function returns a generic String error.
         Suggestion: Define a custom AuthError enum for typed errors.

### src/handlers/login.rs (1 issue)
  [MED]  Line 15: Password logged at debug level.
         Suggestion: Remove or redact sensitive data from log output.

### src/models/user.rs (0 issues)
  Looks good. Clean struct definitions with appropriate derives.

### tests/auth_test.rs (1 issue)
  [LOW]  Line 30: Test uses hardcoded sleep(2) for async wait.
         Suggestion: Use tokio::time::timeout or a condition variable.

4 issues found (1 high, 2 medium, 1 low).
```

---

## Review a Specific Branch

Compare a feature branch against your main branch:

```
vibecli> /review --branch feature/auth-refactor
```

This diffs `feature/auth-refactor` against `main` (or your default branch) and reviews all the changes.

---

## Review a GitHub Pull Request

VibeCody can review a PR directly and optionally post comments on GitHub:

```
vibecli> /review --pr 42
```

### Expected Output

```
[review] Fetching PR #42: "Add OAuth2 support"
[review] Analyzing diff (8 files, +312 -45 lines)...

## Code Review: PR #42 — Add OAuth2 support

### Overall Assessment
Solid implementation of OAuth2 authorization code flow.
Two security issues should be addressed before merge.

### src/oauth.rs (2 issues)
  [HIGH] Line 89: State parameter is not validated on callback.
         This allows CSRF attacks on the OAuth flow.
         Suggestion: Generate a random state, store in session,
         and verify it matches on callback.

  [HIGH] Line 134: Access token stored in localStorage.
         Suggestion: Use httpOnly cookies or in-memory storage.

### src/routes/callback.rs (1 issue)
  [MED]  Line 22: Error from token exchange is swallowed.
         Suggestion: Log the error and return a user-friendly message.

### src/models/session.rs (0 issues)
  Clean implementation with proper expiry handling.

### tests/ (0 issues)
  Good coverage of happy path and token refresh scenarios.

5 issues found (2 high, 1 medium).

Post comments to GitHub? [y/n]:
```

Type `y` to post inline comments directly on the PR.

---

## Understanding the Review Report

### Severity Levels

| Level | Meaning | Action |
|-------|---------|--------|
| **HIGH** | Security vulnerabilities, data loss risks, crashes | Fix before merging |
| **MED** | Code quality, error handling, maintainability | Should fix |
| **LOW** | Style, naming, minor improvements | Nice to have |

### What the Review Checks

The AI reviewer analyzes changes for:

- **Security:** SQL injection, XSS, authentication flaws, secret exposure
- **Error handling:** Panics, unwraps, swallowed errors, missing validation
- **Correctness:** Logic errors, off-by-one, race conditions
- **Performance:** Unnecessary allocations, N+1 queries, blocking in async
- **Maintainability:** Code clarity, naming, documentation, test coverage
- **Best practices:** Language idioms, library usage, API design

---

## Customizing Review Focus

You can guide the review with additional context:

```
vibecli> /review Focus on security and error handling only
```

```
vibecli> /review This is a performance-critical path -- check for allocations
```

```
vibecli> /review We are migrating from sync to async -- check for blocking calls
```

The AI incorporates your guidance into its analysis and adjusts severity accordingly.

---

## Integrating with CI

Use `--exec` mode to run reviews in CI pipelines with structured output:

```bash
vibecli --review --output-format json --output review-report.json
```

The JSON output includes structured fields for each issue:

```json
{
  "summary": {
    "files_reviewed": 4,
    "issues": 3,
    "high": 1,
    "medium": 1,
    "low": 1
  },
  "issues": [
    {
      "file": "src/auth.rs",
      "line": 42,
      "severity": "high",
      "message": "SQL query built with string concatenation",
      "suggestion": "Use parameterized queries with bind variables"
    }
  ]
}
```

### Example GitHub Actions Step

```yaml
- name: AI Code Review
  run: |
    vibecli --review \
      --provider claude \
      --output-format json \
      --output review-report.json

    # Fail the build if any HIGH severity issues
    HIGH_COUNT=$(jq '.summary.high' review-report.json)
    if [ "$HIGH_COUNT" -gt 0 ]; then
      echo "::error::Code review found $HIGH_COUNT high-severity issues"
      cat review-report.json
      exit 1
    fi
```

### Example GitLab CI Step

```yaml
code_review:
  stage: test
  script:
    - vibecli --review --provider claude --output-format json --output review.json
    - |
      HIGH=$(jq '.summary.high' review.json)
      if [ "$HIGH" -gt 0 ]; then
        echo "High severity issues found"
        exit 1
      fi
  artifacts:
    paths:
      - review.json
```

---

## Tips for Better Reviews

1. **Commit often, review often.** Smaller diffs get more focused, actionable feedback.

2. **Use a strong model.** Code review benefits from reasoning power. Claude and GPT-4o produce the best reviews.

3. **Add context.** Telling the reviewer "this handles payments" triggers more thorough security analysis than reviewing the same code without context.

4. **Combine with tests.** Run `/review` first, then `/agent fix the issues found in the review` to auto-fix.

5. **Review your own code.** AI review is not just for PRs -- run it on your uncommitted changes as a pre-commit habit.

---

## Next Steps

- [Agent Workflow Tutorial](/tutorials/agent-workflow/) -- let the agent fix review findings automatically
- [Setting Up Your First Provider](/tutorials/first-provider/) -- try Claude for higher-quality reviews
- [Tutorials Index](./) -- browse all tutorials
