Feature: Design System Hub — cross-provider token registry and audit
  Normalise, merge, export, and audit design tokens from multiple providers.

  Scenario: CSS export wraps tokens in :root block
    Given a design system with a color token "primary" value "#3b82f6"
    When I export to CSS
    Then the CSS should contain ":root"
    And the CSS should contain "--primary: #3b82f6"

  Scenario: Tailwind export includes extend colors block
    Given a design system with a color token "blue-500" value "#3b82f6"
    When I export to Tailwind config
    Then the output should contain "colors"
    And the output should contain "blue-500"

  Scenario: Style Dictionary export produces valid JSON
    Given a design system "brand" namespace with token "primary" value "#000"
    When I export to Style Dictionary format
    Then the JSON should be parseable
    And the JSON should contain "primary"

  Scenario: Audit of empty system produces warnings
    Given an empty design system named "Empty"
    When I audit the design system
    Then the report should have at least 1 issue
    And the score should be less than 100

  Scenario: Audit detects duplicate token names
    Given a design system with two namespaces both containing token "primary"
    When I audit the design system
    Then the report should contain an error with code "DUPLICATE_TOKENS"

  Scenario: No color tokens triggers warning
    Given a design system with no tokens
    When I audit the design system
    Then the report should contain a warning with code "NO_COLORS"

  Scenario: Token drift detects value changes between versions
    Given a baseline design system with token "primary" value "#000000"
    And a current design system with token "primary" value "#3b82f6"
    When I detect token drift
    Then 1 drift should be reported
    And the drifted token should be "primary"

  Scenario: No drift when tokens are identical
    Given a baseline design system with token "primary" value "#000000"
    And a current design system with token "primary" value "#000000"
    When I detect token drift
    Then 0 drifts should be reported

  Scenario: Merge prefers the specified provider
    Given provider "figma" has token "primary" value "#000"
    And provider "penpot" has token "primary" value "#fff" and "secondary" value "#888"
    When I merge with preferred provider "figma"
    Then the merged list should have 2 tokens
    And the "primary" token value should be "#000"

  Scenario: VibeCody default design system has color tokens
    When I load the VibeCody default design system
    Then it should have color tokens
    And it should contain a token named "accent-blue"

  Scenario: TypeScript export contains const tokens object
    Given a design system with a color token "brand-primary" value "#1a202c"
    When I export to TypeScript
    Then the output should contain "export const tokens"
