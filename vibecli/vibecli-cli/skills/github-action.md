---
triggers: ["github action", "github actions", "workflow yaml", "CI workflow", "vibecody-action", "PR review workflow", "action.yml", "entrypoint.sh"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# GitHub Action Workflow Generation

When generating or validating VibeCLI GitHub Actions workflows:

1. **Workflow Structure** — Use `ActionGenerator::pr_review_workflow()` for PR review automation and `ActionGenerator::issue_to_task_workflow()` for issue-comment-triggered automation. Both return a fully populated `WorkflowConfig` with the correct triggers and steps.

2. **Trigger Types** — Four trigger variants are supported: `PullRequest`, `IssueComment { pattern }`, `Push { branch }`, and `WorkflowDispatch`. Use `has_trigger()` to verify a trigger is configured before serializing.

3. **YAML Serialization** — Call `workflow.to_yaml()` to produce a human-readable YAML-like string. The output always includes `name:`, `on:`, and `jobs:` sections with step-level `name:`, `uses:`, and `run:` fields.

4. **Validation** — Call `workflow.validate()` before writing to disk. An empty return vec means the workflow is valid. Common warnings: "Workflow has no triggers", "Workflow has no jobs", "Job '{id}' has no steps".

5. **Step Helpers** — Use `ActionStep::checkout()` for the standard checkout step and `ActionStep::vibecli_run(prompt)` to inject a `vibecli -p "..."` run step.

6. **vibecody-action Scaffold** — `ActionGenerator::generate_action_yml()` produces the `action.yml` content declaring `prompt` and `api_key` inputs with `runs.using: docker`. `ActionGenerator::generate_entrypoint_sh()` produces the `entrypoint.sh` that calls `vibecli -p "$INPUT_PROMPT"`.

7. **Dockerfile** — The `vibecody-action/Dockerfile` should use `ubuntu:22.04` as a base, install dependencies, and copy the pre-built `vibecli` binary to `/usr/local/bin/`. The `ENTRYPOINT` should point to `entrypoint.sh`.

8. **Job/Step Counting** — Use `job_count()` and `step_count()` for quick summary stats. `step_count()` sums steps across all jobs.

9. **Testing** — The nine unit tests in `github_action.rs` cover trigger presence, checkout step presence, YAML content, validation warnings, and scaffold file content. Run with `cargo test -p vibecli --lib -- github_action`.

10. **Integration Pattern** — In CI pipelines, combine with `agentic_cicd` for full PR-to-deploy automation. The generated workflow calls VibeCLI which can then trigger nested agents via `nested_agents` or spawn work via `SpawnAgent`.
