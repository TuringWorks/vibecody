---
triggers: ["legacy code", "modernize", "migration strategy", "adapter pattern", "incremental rewrite", "legacy system"]
tools_allowed: ["read_file", "write_file", "bash"]
category: review
---

# Legacy Code Modernization

When modernizing legacy systems:

1. Strangler Fig: build new functionality alongside old — gradually route traffic to new system
2. Write characterization tests first: capture existing behavior before changing anything
3. Anti-Corruption Layer: adapter between old and new systems — isolate legacy from modern code
4. Incremental migration: replace one module/service at a time — never big-bang rewrite
5. Feature flags: deploy new code behind toggles — switch between old and new at runtime
6. Data migration: dual-write to old and new stores → validate → cutover → decomission old
7. API versioning: maintain old API contract while building new — migrate clients gradually
8. Dependency upgrade path: update one major version at a time, fixing breaks incrementally
9. Document tribal knowledge: extract undocumented business rules into tests and docs
10. Monitoring: compare old and new system outputs — detect behavioral differences
11. Rollback plan: every migration step must be reversible — test rollback before cutover
12. Timeline: plan for months, not weeks — legacy modernization is marathon, not sprint
