Feature: Pencil wireframe integration
  Parse Evolus Pencil .ep XML format, generate wireframe templates,
  and bridge to the TuringWorks Pencil MCP server.

  Scenario: PencilDocument serialises to valid EP XML
    Given a PencilDocument named "TestDoc" with one page "Page1" of size 1280x800
    When I serialise to EP XML
    Then the XML should contain "TestDoc"
    And the XML should contain "Page1"

  Scenario: Parsing valid EP XML extracts document structure
    Given a valid EP XML string with document "MyDoc" and page "Page1"
    When I parse the EP XML
    Then the document name should be "MyDoc"
    And the page count should be 1

  Scenario: Parsing empty EP XML returns an error
    Given an empty string
    When I parse the EP XML
    Then a design error should be returned

  Scenario: Parsing EP XML extracts shapes
    Given a valid EP XML with one rectangle shape
    When I parse the EP XML
    Then the first page should have 1 shape

  Scenario: Landing page template has navigation bar
    Given I generate a landing page template titled "MyProduct"
    Then the template should have 1 page
    And the page should contain a shape with id "nav"
    And the page should contain a shape with id "hero"

  Scenario: Dashboard template has sidebar shape
    Given I generate a dashboard template with sections "Overview" and "Analytics"
    Then the template should have 1 page
    And the page should contain a shape with id "sidebar"

  Scenario: Mobile app template creates one page per screen
    Given I generate a mobile app with screens "Home" and "Profile" and "Settings"
    Then the template should have 3 pages

  Scenario: Converting document to DesignFile maps frames
    Given a PencilDocument with 2 pages
    When I convert to a DesignFile
    Then the DesignFile should have 2 frames
    And the DesignFile provider should be "pencil"

  Scenario: Color shapes are extracted as design tokens
    Given a PencilDocument with a shape having fill color "#3b82f6"
    When I convert to a DesignFile
    Then the DesignFile should have at least 1 token

  Scenario: Pencil MCP op serialises get_editor_state
    Given a Pencil MCP operation for get_editor_state
    When I serialise to JSON
    Then the JSON should contain "get_editor_state"
