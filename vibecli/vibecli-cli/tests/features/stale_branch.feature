Feature: Stale branch detection with policies
  Branches are assessed as Fresh, Stale, or Diverged based on commit
  distance and last activity time. Configured policies determine the
  response to stale or diverged branches.

  Scenario: Fresh branch passes detection
    Given a branch 0 commits behind and active 1 hour ago
    When I assess freshness
    Then the state should be "fresh"

  Scenario: Stale branch detected after inactivity
    Given a branch 0 commits behind and active 30 days ago
    When I assess freshness
    Then the state should be "stale"

  Scenario: Diverged branch shows missing fixes message
    Given a branch 50 commits behind main
    When I assess freshness
    Then the state should be "diverged"
    And the missing_fixes_message should contain "50"
