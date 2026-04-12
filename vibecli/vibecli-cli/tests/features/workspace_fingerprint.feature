Feature: Workspace fingerprinting for session isolation
  Hashing workspace paths with FNV-1a produces stable 16-char hex
  namespaces that isolate sessions across concurrent workspaces.

  Scenario: Fingerprint is a 16-character hex string
    Given a workspace path "/home/user/project"
    When I compute the fingerprint
    Then it should be exactly 16 characters of hex digits

  Scenario: Same path always produces the same fingerprint
    Given workspace paths "/home/user/project" and "/home/user/project"
    When I compute both fingerprints
    Then they should be identical

  Scenario: Different paths produce different fingerprints
    Given workspace paths "/home/user/alpha" and "/home/user/beta"
    When I compute both fingerprints
    Then they should differ

  Scenario: Trailing slash is normalized before hashing
    Given workspace paths "/home/user/project" and "/home/user/project/"
    When I compute both fingerprints
    Then they should be identical
