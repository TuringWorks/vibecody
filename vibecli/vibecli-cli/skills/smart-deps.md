# Smart Dependencies

Intelligent dependency management with conflict resolution, CVE patching, license compliance checking, and upgrade planning. Analyzes your dependency tree to find vulnerabilities, license violations, and safe upgrade paths.

## When to Use
- Resolving version conflicts in complex dependency trees
- Patching known CVEs in dependencies with minimal disruption
- Auditing license compliance across all direct and transitive deps
- Planning major dependency upgrades with impact analysis
- Detecting abandoned or unmaintained dependencies

## Commands
- `/deps audit` — Full audit: vulnerabilities, licenses, and health
- `/deps cve` — List known CVEs affecting current dependencies
- `/deps patch <cve-id>` — Auto-patch a specific CVE with minimal version bump
- `/deps licenses` — Show license breakdown and compliance status
- `/deps conflicts` — Detect and resolve version conflicts
- `/deps upgrade <package>` — Plan an upgrade with breaking change analysis
- `/deps health` — Check maintenance status of all dependencies
- `/deps tree <package>` — Show dependency tree for a specific package

## Examples
```
/deps audit
# Scanned 234 dependencies (89 direct, 145 transitive)
# CVEs: 3 critical, 2 high, 5 medium
# Licenses: 2 copyleft (GPL) in transitive deps — review needed
# Abandoned: 1 package (no commits in 2 years)

/deps patch CVE-2026-1234
# CVE-2026-1234 affects serde-json 1.0.108
# Safe upgrade: 1.0.108 -> 1.0.114 (patch only, no breaking changes)
# Applied. Run `cargo test` to verify.

/deps licenses
# MIT: 189 | Apache-2.0: 32 | ISC: 8 | BSD-3: 3 | GPL-2.0: 2
# WARNING: GPL-2.0 deps may conflict with MIT project license
# Affected: libfoo (via transitive dep bar -> baz -> libfoo)
```

## Best Practices
- Run deps audit weekly and before every release
- Address critical CVEs immediately, schedule high CVEs within a sprint
- Block GPL dependencies in MIT/Apache projects at the CI level
- Prefer packages with active maintenance and multiple contributors
- Test thoroughly after any dependency upgrade, even patch versions
