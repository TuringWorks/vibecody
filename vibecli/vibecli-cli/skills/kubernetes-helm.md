---
triggers: ["Helm", "helm chart", "helm template", "helm values", "helm dependency", "helm hooks", "helmfile"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["helm"]
category: devops
---

# Helm Charts and Package Management

When working with Helm charts and package management:

1. Structure charts with clear separation: `templates/` for K8s manifests, `values.yaml` for defaults, `values-prod.yaml` for overrides. Always include `Chart.yaml` with proper `appVersion` and SemVer `version`.

2. Use `helm template` to render manifests locally before deploying — pipe through `kubectl apply --dry-run=server -f -` to catch both template errors and API validation issues.

3. Define sensible defaults in `values.yaml` and document every value with comments. Use `{{ required "msg" .Values.field }}` for mandatory values that have no safe default.

4. Manage subcharts with `Chart.yaml` dependencies and `condition` fields to toggle them: `dependencies: [{name: redis, version: "17.x", repository: "https://charts.bitnami.com/bitnami", condition: redis.enabled}]`. Run `helm dependency update` after changes.

5. Use named templates in `_helpers.tpl` for labels, selectors, and names: `{{- define "app.labels" -}}app.kubernetes.io/name: {{ include "app.name" . }}{{- end }}` — keeps templates DRY and consistent.

6. Implement Helm hooks for lifecycle management: `"helm.sh/hook": pre-install,pre-upgrade` for database migrations, `"helm.sh/hook-weight"` to order them, and `"helm.sh/hook-delete-policy": hook-succeeded` to clean up.

7. Use `helm diff upgrade` (via the diff plugin) before every production upgrade to review exactly what will change — integrate this into CI pipelines as a required review step.

8. Pin chart versions in `helmfile.yaml` or CI scripts — never use `latest` or unpinned ranges in production. Lock with `helm dependency build` and commit `Chart.lock`.

9. Leverage Helmfile for multi-chart deployments: define environments, use `{{ requiredEnv "VAR" }}` for secrets, and organize releases across multiple `helmfile.d/*.yaml` files for large stacks.

10. Test charts with `helm unittest` plugin — write test cases in `tests/` that assert rendered YAML matches expectations: `asserts: [{equal: {path: spec.replicas, value: 3}}]`.

11. Use `lookup` function sparingly to read existing cluster state in templates (e.g., check if a Secret exists before creating one), but always provide a fallback for `helm template` which runs without cluster access.

12. Secure Helm releases by enabling `--atomic` (auto-rollback on failure), `--timeout 10m` for large deployments, and `--cleanup-on-fail` to remove new resources on failed upgrades.
