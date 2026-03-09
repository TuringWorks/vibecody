---
triggers: ["qa validation", "quality assurance", "multi-qa", "cross validation", "qa pipeline", "code review agents", "qa agents", "quality gate", "automated review"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Multi-QA Agent Cross-Validation

When validating code through the QA pipeline:

1. Spawn all 8 QA agent types for comprehensive validation: CompileChecker (syntax + type errors), TestRunner (unit + integration), SecurityAuditor (OWASP + secrets + dependencies), StyleEnforcer (formatting + naming + patterns), DocValidator (missing docs + outdated comments), PerformanceAnalyzer (N+1 queries + memory leaks + blocking I/O), DependencyAuditor (vulnerabilities + outdated + unused), IntegrationTester (API contracts + data flow).
2. Run QA in rounds (default max 3): after each round, assess findings. If critical issues remain or score is below threshold (default 80%), run another round focusing on unresolved findings.
3. Cross-validate between agents: compare findings from different agent types on the same files. High agreement (>80%) increases confidence; low agreement flags files for manual review.
4. Classify findings by severity: Critical (breaks compilation/security), High (logic bugs, data loss risk), Medium (code smell, missing error handling), Low (style, naming), Info (suggestions).
5. Auto-fix where possible: mark findings as auto_fixable when a deterministic fix exists (formatting, unused imports, missing null checks). Apply fixes between rounds to reduce noise.
6. Generate a QA report with recommendation: Approve (score ≥90, zero critical), Approve with Warnings (score ≥80, zero critical), Request Changes (score <80 or critical findings), Reject (score <50 or unresolvable critical issues).
7. Track QA history across runs: compare scores over time to identify improving or degrading code quality trends.
8. Configure per-project: enable/disable categories, set severity threshold, adjust pass score, toggle cross-validation and auto-fix.
