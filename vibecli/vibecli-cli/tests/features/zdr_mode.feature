Feature: Zero Data Retention (ZDR) mode
  ZDR mode ensures stateless operation: no session logging, full history
  included in every request, and automatic PII/key scrubbing.

  Scenario: Strict policy is ZDR compliant
    Given a strict ZDR policy
    Then the policy should be ZDR compliant

  Scenario: Permissive policy has compliance violations
    Given a permissive ZDR policy
    Then the policy should not be ZDR compliant
    And there should be at least 1 compliance violation

  Scenario: Session builds request with full history
    Given a ZDR session with strict policy
    When I add a "user" message "hello world"
    And I add a "assistant" message "hi there"
    Then the built request should contain 2 messages

  Scenario: PII scrubbing removes email addresses
    Given the text "send results to bob@secret.org immediately"
    When I apply PII scrubbing
    Then the output should not contain "bob@secret.org"
    And the output should contain "[REDACTED]"

  Scenario: clear() empties the session
    Given a ZDR session with strict policy
    When I add a "user" message "temporary thought"
    And I clear the session
    Then the session should have 0 messages
