Feature: Long session management (7+ hour autonomous sessions)
  The SessionManager budgets tokens, turns, and wall-time, deciding
  whether to continue, compact, or halt each autonomous session.

  Scenario: Fresh session continues without compaction
    Given a new session started at time 0
    When I check the decision at time 100
    Then the decision should be Continue

  Scenario: 75% token usage triggers compaction
    Given a new session started at time 0
    And the session has used 1500000 tokens in 1 turn
    When I check the decision at time 0
    Then the decision should be CompactAndContinue

  Scenario: 100% token usage halts the session
    Given a new session started at time 0
    And the session has used 2000000 tokens in 1 turn
    When I check the decision at time 0
    Then the decision should be Halt

  Scenario: Budget remaining decreases with usage
    Given a new session started at time 0
    And the session has used 500000 tokens in 0 turns
    When I compute budget remaining at time 0
    Then remaining tokens should be 1500000
