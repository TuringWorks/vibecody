Feature: A2A protocol speaks real HTTP + SSE (US-002)
  The A2aHttpServer hosts agent-card discovery, task submission, task status,
  and an SSE event stream. A2aHttpClient can interrogate a remote server over
  HTTP. Both sides use the existing AgentCard, TaskInput, TaskOutput, and
  TaskStatus types so in-process and across-the-wire A2A share one vocabulary.

  Scenario: Client discovers an agent card over HTTP
    Given an A2A HTTP server hosting an agent named "bot" on a random port
    When a client fetches the agent card from that server
    Then the fetched card name is "bot"
    And the fetched card has capability "code_generation"

  Scenario: Client submits a task and polls it to completion
    Given an A2A HTTP server hosting an agent named "worker" on a random port
    When a client submits a text task with content "refactor auth"
    Then the returned task id starts with "srv-task-"
    And the client can GET the task and its status is "Submitted"

  Scenario: SSE event stream emits TaskCreated for submitted tasks
    Given an A2A HTTP server hosting an agent named "notifier" on a random port
    When a client submits a text task with content "trigger event"
    And the client reads at most 3 SSE events from the server
    Then the received events include a "TaskCreated" event
