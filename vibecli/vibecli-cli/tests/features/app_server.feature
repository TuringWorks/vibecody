Feature: Embedded app server lifecycle
  AppServer manages the lifecycle of a local HTTP server used to preview
  generated web apps. Start/stop transitions are enforced; double-start
  and double-stop both return errors.

  Scenario: Server starts successfully and reports running state
    Given a new app server
    When I start the server on port 3000 serving "/dist"
    Then the server should be running
    And the port should be 3000

  Scenario: Starting an already-running server returns an error
    Given a new app server
    When I start the server on port 3000 serving "/dist"
    And I start the server on port 3001 serving "/dist"
    Then the second start should fail with AlreadyRunning

  Scenario: Server stops successfully after running
    Given a new app server
    When I start the server on port 3000 serving "/dist"
    And I stop the server
    Then the server should not be running

  Scenario: Stopping an already-stopped server returns an error
    Given a new app server
    When I stop the server without starting
    Then the stop should fail with NotRunning
