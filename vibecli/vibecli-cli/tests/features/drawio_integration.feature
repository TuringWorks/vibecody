Feature: Draw.io deep integration
  Parse, generate, and transform draw.io XML diagrams.
  Provide MCP bridge commands and template-based generation.

  Scenario: Vertex cell renders XML with id and value
    Given a vertex cell with id "v1" value "Hello" at position 10,20 size 120x40
    When I render the cell to XML
    Then the XML should contain id "v1"
    And the XML should contain value "Hello"
    And the XML should contain 'vertex="1"'

  Scenario: Edge cell includes source and target
    Given an edge cell from "A" to "B" labeled "calls"
    When I render the cell to XML
    Then the XML should contain source "A"
    And the XML should contain target "B"
    And the XML should contain 'edge="1"'

  Scenario: Graph XML has root element
    Given an empty DrawioGraph named "TestGraph"
    When I render to XML
    Then the XML should contain "<root>"

  Scenario: DrawioFile wraps in mxfile envelope
    Given an empty DrawioGraph named "My Diagram"
    When I render to drawio file format
    Then the output should contain '<?xml version="1.0"'
    And the output should contain "<mxfile"

  Scenario: Flowchart template creates correct vertex and edge counts
    Given a flowchart with steps "Start" and "Validate?" and "End"
    When I generate the flowchart template
    Then the graph should have 3 vertices
    And the graph should have 2 edges

  Scenario: Parsing valid draw.io XML extracts pages and cells
    Given a valid draw.io XML string with 2 vertices and 1 edge
    When I parse the XML
    Then the parse result should have 1 page
    And the total vertex count should be 2
    And the total edge count should be 1

  Scenario: Parsing empty XML returns an error
    Given an empty XML string
    When I parse the XML
    Then a design error should be returned

  Scenario: MCP read_file command serialises to JSON
    Given a drawio MCP command to read file "/tmp/test.drawio"
    When I serialise to JSON
    Then the JSON should contain "drawio/read_file"
    And the JSON should contain "/tmp/test.drawio"

  Scenario: C4 context diagram generates persons and systems
    Given a C4 context with 1 person and 1 system and 1 relation
    When I generate the C4 context template
    Then the graph should have 2 vertices
    And the graph should have 1 edges

  Scenario: LLM arrow notation parses to flowchart
    Given LLM output "Start -> Validate -> Process -> End"
    When I parse as LLM flowchart
    Then the flowchart graph should have at least 4 vertices
