# CI/CD AI Status Checks

Run AI-powered checks as GitHub/GitLab status checks on every PR.

## Triggers
- "CI check", "status check", "PR review", "AI review"
- "GitHub check", "GitLab status", "code review CI"

## Usage
```
/ci create abc1234 main org/repo   # Create check suite
/ci check "Code Review"            # Add a check to suite
/ci annotate src/lib.rs 42 warning "unused import"
/ci complete                       # Finalize suite
/ci summary                        # Markdown summary
```

## Features
- CheckSuite with multiple StatusCheck runs per commit
- 7 check states: Pending, Running, Success, Failure, Error, Neutral, Skipped
- 8 check types: CodeReview, SecurityScan, TestCoverage, StyleCheck, DependencyAudit, PerformanceCheck, DocumentationCheck, Custom
- Annotations with Notice/Warning/Failure levels and optional suggestions
- GitHub Checks API JSON output format
- GitLab commit status JSON output format
- Markdown summary reports with annotation counts
- Configurable: required checks, auto-approve, annotation threshold
- Suite-level aggregation (worst state wins)
