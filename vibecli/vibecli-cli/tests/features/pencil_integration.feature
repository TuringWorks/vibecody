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

  Scenario Outline: Every offered template produces a document that parses back
    Given I generate the "<template>" template titled "Round Trip"
    Then the template should have at least 1 page
    And every page should have at least one shape
    And the EP XML should round-trip through the parser

    Examples:
      | template      |
      | landing_page  |
      | dashboard     |
      | mobile_app    |
      | login_form    |
      | settings_page |
      | data_table    |

  Scenario: An unknown template id is refused rather than substituted
    Given I generate the "not_a_template" template titled "X"
    Then a template error should be returned

  Scenario: The .ep export is a ZIP containing content.xml
    Given I generate the "login_form" template titled "Sign in"
    When I package the document as a .ep archive
    Then the archive should be a ZIP
    And the archive should contain "content.xml"

  Scenario: The HTML export renders one block per shape
    Given I generate the "data_table" template titled "Records"
    When I render the document as HTML
    Then the HTML should contain one block per shape
