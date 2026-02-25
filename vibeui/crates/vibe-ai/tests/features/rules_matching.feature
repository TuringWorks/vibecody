Feature: Rules directory loading and path-aware matching
  As an AI assistant user
  I want rules to apply only to relevant files
  So that I get targeted, context-specific AI guidance

  Background:
    Given a workspace directory exists

  Scenario: Empty rules directory loads zero rules
    When I load rules from the workspace
    Then 0 rules are loaded

  Scenario: A rule without path_pattern always matches
    Given a rule file "always.md" with content "Always be safe"
    When I load rules from the workspace
    Then 1 rule is loaded
    And the rule "always" matches an empty file list
    And the rule "always" matches the file "anything.py"

  Scenario: A rule with path_pattern only matches designated files
    Given a rule file "rust.md" with path_pattern "**/*.rs" and content "No unwrap"
    When I load rules from the workspace
    Then the rule "rust" matches the file "src/main.rs"
    And the rule "rust" does not match the file "src/main.ts"
    And the rule "rust" does not match an empty file list

  Scenario: Multiple rules load and filter independently
    Given a rule file "always.md" with content "Be safe"
    And a rule file "rust.md" with path_pattern "**/*.rs" and content "No unwrap"
    And a rule file "ts.md" with path_pattern "**/*.ts" and content "Use strict"
    When I load rules from the workspace
    Then 3 rules are loaded
    And the rule "always" matches the file "anything.go"
    And the rule "rust" matches the file "lib/util.rs"
    And the rule "ts" does not match the file "lib/util.rs"

  Scenario: Rule name is derived from filename when no frontmatter
    Given a rule file "my-custom-rule.md" with content "Some guidance"
    When I load rules from the workspace
    Then the rule "my-custom-rule" matches an empty file list

  Scenario: Rule name from frontmatter overrides filename
    Given a rule file "generic.md" with frontmatter name "overridden" and content "Body"
    When I load rules from the workspace
    Then 1 rule is loaded
    And the rule "overridden" matches an empty file list
