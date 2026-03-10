# GitHub Actions Agent

Run VibeCLI as a CI/CD agent in GitHub Actions workflows.

## Triggers
- "github actions", "CI agent", "actions workflow", "GH actions"
- "workflow yaml", "CI/CD agent", "actions integration"

## Usage
```
/actions generate review                  # Generate code review workflow
/actions generate autofix                 # Generate autofix workflow
/actions generate test                    # Generate test suite workflow
/actions generate security                # Generate security scan workflow
/actions step "Review this PR" --model claude # Generate agent step
/actions validate workflow.yml            # Validate workflow YAML
/actions secrets                          # List required secrets
/actions estimate review-workflow         # Estimate CI minutes
```

## Workflow Templates
- **CodeReview** — Agent reviews PRs on pull_request events
- **AutoFix** — Agent auto-fixes failing tests
- **TestSuite** — Agent generates and runs tests
- **SecurityScan** — Agent runs security analysis
- **Deploy** — Agent manages deployments
- **Custom** — User-defined workflow

## Features
- 6 trigger types: Push, PullRequest, Schedule, WorkflowDispatch, IssueComment, Release
- YAML workflow generation with proper formatting
- Agent step generation (vibecli as action step)
- Secret management (API keys, tokens)
- Workflow validation with line-level issue reporting
- CI minutes estimation
