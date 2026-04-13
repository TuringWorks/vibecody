Feature: Design provider abstraction — multi-tool interop
  A provider-agnostic layer lets panels and agents work with
  Figma, Penpot, Pencil, Draw.io, and in-house design tools
  through a single interface.

  Scenario: Provider kind has a display name
    Given a provider kind "figma"
    Then its display name should be "Figma"

  Scenario: Draw.io provider supports editing
    Given a provider kind "draw_io"
    Then it should support editing

  Scenario: Mermaid provider does not require editing support
    Given a provider kind "mermaid"
    Then its display name should be "Mermaid"

  Scenario: Diagram format returns correct file extension
    Given a diagram format "mermaid_md"
    Then the file extension should be "md"

  Scenario: DrawIO format returns xml extension
    Given a diagram format "draw_io_xml"
    Then the file extension should be "drawio"

  Scenario: C4 diagram kind prefers C4 DSL format
    Given a diagram kind "c4_context"
    Then the preferred format should be "c4_dsl"

  Scenario: Flowchart prefers Mermaid format
    Given a diagram kind "flowchart"
    Then the preferred format should be "mermaid_md"

  Scenario: DiagramDoc created with id prefix
    Given I create a DiagramDoc titled "My Flow" of kind "flowchart" with content "flowchart TD\n  A-->B"
    Then the doc id should start with "diag-"
    And the doc title should be "My Flow"

  Scenario: Tokens convert to valid CSS custom properties
    Given a color token named "Primary Blue" with value "#3b82f6"
    When I export tokens to CSS
    Then the CSS should contain "--primary-blue: #3b82f6;"

  Scenario: Empty provider registry has no entries
    Given a fresh provider registry
    Then the available providers list should be empty

  Scenario: Design error formats with code and message
    Given a design error with code "NOT_FOUND" and message "file not found"
    Then the error string should equal "[NOT_FOUND] file not found"
