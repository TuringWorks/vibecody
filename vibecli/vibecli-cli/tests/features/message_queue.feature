Feature: Agent message queues for steering and follow-up injection
  The MessageQueue and AgentMessageQueues types provide thread-safe FIFO
  queues with configurable drain modes. The steering queue injects guidance
  between tool calls; the follow-up queue injects after a turn ends.

  Scenario: OneAtATime mode drains one message per opportunity
    Given a OneAtATime message queue
    And I enqueue user messages "alpha", "beta", "gamma"
    When I drain the queue
    Then I should receive exactly 1 message
    And the message content should be "alpha"
    And the queue should have 2 messages remaining

  Scenario: All mode drains every message at once
    Given an All mode message queue
    And I enqueue user messages "one", "two", "three"
    When I drain the queue
    Then I should receive exactly 3 messages
    And the queue should have 0 messages remaining

  Scenario: Max size is enforced and excess messages are rejected
    Given a OneAtATime message queue with max size 2
    And I enqueue user message "first"
    And I enqueue user message "second"
    When I try to enqueue user message "third"
    Then the enqueue should fail with a capacity error
    And the queue should have 2 messages remaining

  Scenario: Steering and follow-up queues are independent
    Given an agent message queues pair with OneAtATime drain mode
    When I steer with "use bullet points"
    And I steer with "be concise"
    And I follow up with "now summarise"
    And I drain the steering queue
    Then I should receive exactly 1 steering message
    And the follow-up queue should have 1 message remaining
    And the steering queue should have 1 message remaining

  Scenario: is_idle reports true only when both queues are empty
    Given an agent message queues pair with OneAtATime drain mode
    Then the agent queues should be idle
    When I steer with "redirect focus"
    Then the agent queues should not be idle
    When I drain the steering queue
    Then the agent queues should be idle

  Scenario: Concurrent enqueue from multiple threads stays consistent
    Given a OneAtATime message queue with max size 1000
    When 8 threads each enqueue 10 messages concurrently
    Then the queue should have 80 messages remaining
