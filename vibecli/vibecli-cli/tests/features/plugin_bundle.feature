Feature: Plugin bundle validation
  PluginBundle validates that all declared dependency IDs are present
  in the bundle and that no plugin ID appears twice.

  Scenario: Bundle with no dependencies is valid
    Given a bundle with plugin "core" version "1.0"
    When I validate the bundle
    Then the bundle should be valid

  Scenario: Plugin with satisfied dependency is valid
    Given a bundle with plugin "core" version "1.0"
    And a plugin "ui" version "1.0" that requires "core"
    When I validate the bundle
    Then the bundle should be valid

  Scenario: Plugin with missing dependency is invalid
    Given a bundle with plugin "ui" version "1.0" that requires "core"
    When I validate the bundle
    Then the bundle should be invalid
    And there should be 1 missing dependency

  Scenario: Bundle with duplicate plugin IDs is invalid
    Given a bundle with plugin "core" version "1.0"
    And a bundle with plugin "core" version "2.0"
    When I validate the bundle
    Then the bundle should be invalid
    And there should be 1 duplicate id
