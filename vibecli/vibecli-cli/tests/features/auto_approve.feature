Feature: Auto-approval scorer for tool calls
  The AutoApprover assigns a risk score (0.0–1.0) and emits
  AutoApprove / AskUser / AutoDeny based on heuristic signals.

  Scenario: Known-safe command is auto-approved
    Given a tool named "ls" with input "-la /tmp"
    When I evaluate the approval
    Then the decision should be "AutoApprove"
    And the score should be below 0.2

  Scenario: Destructive command is auto-denied
    Given a tool named "bash" with input "rm -rf /"
    When I evaluate the approval
    Then the decision should be "AutoDeny"
    And the score should be above 0.8

  Scenario: Privilege escalation raises score to AskUser
    Given a tool named "bash" with input "sudo systemctl restart nginx"
    When I evaluate the approval
    Then the decision should not be "AutoApprove"
    And the score should be above 0.2

  Scenario: always_allow list overrides a high-risk score
    Given a tool named "nuclear_tool" with input "rm -rf /"
    And "nuclear_tool" is in the always_allow list
    When I evaluate the approval
    Then the decision should be "AutoApprove"
