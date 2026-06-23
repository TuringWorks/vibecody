Feature: MCP Tasks extension + stateless _meta (gap C3)
  The Tasks extension turns a tools/call into a task handle the client drives
  with tasks/get, tasks/update, tasks/cancel; the stateless model carries
  protocol/capabilities in a per-request _meta field instead of a session.

  Scenario: A created task starts in the working state
    Given a fresh task registry
    When I create a task for tool "build_project"
    Then the task state is "working"

  Scenario: A task can progress and complete with a result
    Given a fresh task registry
    When I create a task for tool "build_project"
    And I update the task progress to 50
    And I complete the task with a result
    Then the task state is "completed"
    And the task progress is 100

  Scenario: Completing a task without a result is rejected
    Given a fresh task registry
    When I create a task for tool "x"
    Then completing it without a result fails

  Scenario: A cancelled task cannot be updated further
    Given a fresh task registry
    When I create a task for tool "x"
    And I cancel the task
    Then the task state is "cancelled"
    And updating it afterward fails

  Scenario: Stateless _meta advertises the Tasks extension
    Given a request whose _meta advertises the tasks extension
    Then the request supports tasks
