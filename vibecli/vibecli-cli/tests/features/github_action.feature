Feature: GitHub Action workflow generation
  ActionGenerator produces valid workflow configurations for common CI/CD
  patterns and generates the required vibecody-action scaffold files.

  Scenario: PR review workflow has correct trigger and job structure
    Given a PR review workflow
    Then the workflow should have trigger "pull_request"
    And the workflow should have 1 job
    And the workflow YAML should contain "name: VibeCLI PR Review"

  Scenario: Issue workflow has @vibecli comment trigger
    Given an issue handler workflow
    Then the workflow should have trigger "issue_comment"
    And the workflow YAML should contain "issue_comment"
    And the workflow YAML should contain "@vibecli"

  Scenario: Empty workflow validation returns warnings
    Given an empty workflow
    Then validation should return at least 1 warning
    And at least one warning should mention "jobs"

  Scenario: Generated action.yml contains required fields
    Given the generated action.yml content
    Then the content should contain "inputs:"
    And the content should contain "prompt:"
    And the content should contain "using: docker"
