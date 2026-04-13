Feature: Penpot design integration
  Connect to self-hosted or cloud Penpot instances via REST API,
  extract design tokens and components, and generate framework code.

  Scenario: Config API URL is correctly formed
    Given a Penpot config with host "https://design.penpot.app" and token "tok123"
    When I request the API URL for command "get-profile"
    Then the URL should be "https://design.penpot.app/api/rpc/command/get-profile"

  Scenario: Config trims trailing slash from host
    Given a Penpot config with host "https://design.penpot.app/" and token "tok"
    Then the host should not end with "/"

  Scenario: Validation fails for empty host
    Given a Penpot config with host "" and token "tok"
    When I validate the config
    Then validation should fail

  Scenario: Validation fails for host without http schema
    Given a Penpot config with host "design.penpot.app" and token "tok"
    When I validate the config
    Then validation should fail

  Scenario: Validation fails for empty token
    Given a Penpot config with host "https://design.penpot.app" and token ""
    When I validate the config
    Then validation should fail

  Scenario: Validation passes for valid config
    Given a Penpot config with host "https://design.penpot.app" and token "abc123"
    When I validate the config
    Then validation should pass

  Scenario: Get-profile request includes authorization header
    Given a Penpot config with host "https://example.penpot.app" and token "mytoken"
    When I build a get-profile request
    And I convert to curl
    Then the curl command should contain "mytoken"

  Scenario: Colors export to CSS custom properties
    Given a Penpot color named "Primary Blue" with hex "#3b82f6"
    When I export colors to CSS
    Then the CSS should contain "--primary-blue: #3b82f6;"

  Scenario: Component exports to React function component
    Given a Penpot component named "Card" with id "comp-1"
    When I export to React
    Then the code should contain "function Card"
    And the code should contain "CardProps"

  Scenario: Component exports to Vue template
    Given a Penpot component named "Button" with id "btn-1"
    When I export to Vue
    Then the code should contain "<template>"

  Scenario: Parsing malformed JSON returns error
    Given a Penpot file response JSON string "not-json"
    When I parse the file response
    Then a parse error should be returned

  Scenario: Parsing minimal JSON extracts id and name
    Given a Penpot file response JSON with id "file-1" and name "My Design"
    When I parse the file response
    Then the file id should be "file-1"
    And the file name should be "My Design"
    And the provider should be "penpot"
