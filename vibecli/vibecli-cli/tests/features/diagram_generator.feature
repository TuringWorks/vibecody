Feature: AI diagram generation
  Generate Mermaid, PlantUML, C4 DSL, and draw.io XML diagrams
  from natural-language descriptions with AI assistance.

  Scenario: Build system prompt for Mermaid flowchart mentions format
    Given a diagram request for kind "flowchart" and format "mermaid_md"
    When I build the system prompt
    Then the prompt should contain "Mermaid"

  Scenario: Build user prompt includes description text
    Given a diagram request with description "Order processing system"
    When I build the user prompt
    Then the prompt should contain "Order processing system"

  Scenario: Valid Mermaid output passes post-processing
    Given raw LLM output "flowchart TD\n  A-->B"
    When I post-process for format "mermaid_md"
    Then the result should be OK
    And the output should contain "flowchart"

  Scenario: Output with markdown fences is stripped
    Given raw LLM output "```mermaid\nflowchart TD\n  A-->B\n```"
    When I post-process for format "mermaid_md"
    Then the result should be OK
    And the output should not contain "```"

  Scenario: Invalid Mermaid output returns error
    Given raw LLM output "this is not a diagram"
    When I post-process for format "mermaid_md"
    Then the result should be an error

  Scenario: Valid draw.io XML passes post-processing
    Given raw LLM output "<mxGraphModel><root></root></mxGraphModel>"
    When I post-process for format "draw_io_xml"
    Then the result should be OK

  Scenario: Valid PlantUML passes post-processing
    Given raw LLM output "@startuml\nA -> B\n@enduml"
    When I post-process for format "plant_uml"
    Then the result should be OK

  Scenario: PlantUML without closing tag gets appended
    Given raw LLM output "@startuml\nA -> B"
    When I post-process for format "plant_uml"
    Then the result should be OK
    And the output should contain "@enduml"

  Scenario: Microservices Mermaid template is valid flowchart
    When I get the microservices architecture Mermaid template
    Then the template should contain "flowchart"
    And the template should contain "Gateway"

  Scenario: ER diagram Mermaid template is valid erDiagram
    When I get the ER diagram Mermaid template
    Then the template should contain "erDiagram"
    And the template should contain "TENANT"

  Scenario: C4 context DSL template contains workspace
    Given a C4 context template for system "MyApp"
    Then the DSL should contain "workspace"
    And the DSL should contain "MyApp"
    And the DSL should contain "softwareSystem"

  Scenario: PlantUML component template includes startuml
    Given a PlantUML component template for "MySystem" with component "API" as "Rust"
    Then the template should contain "@startuml"
    And the template should contain "[API]"

  Scenario: DiagramDoc created with Mermaid provider
    Given I create a Mermaid diagram doc titled "Test Flow"
    Then the doc provider should be "mermaid"
    And the doc format should be "mermaid_md"

  Scenario: DiagramRequest with_format override works
    Given a diagram request for kind "flowchart"
    When I override format to "draw_io_xml"
    Then the request format should be "draw_io_xml"
