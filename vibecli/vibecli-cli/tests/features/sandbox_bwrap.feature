Feature: bwrap sandbox profile builder
  The BwrapProfile generates bwrap argv lists from a structured policy
  without performing any real syscalls.

  Scenario: Minimal profile unshares network and PID
    Given a minimal bwrap profile
    Then it should unshare network
    And it should unshare pid

  Scenario: Adding a read-only bind increases ro_count
    Given a minimal bwrap profile
    When I add a read-only bind from "/usr" to "/usr"
    Then the ro_count should increase by 1

  Scenario: with_network removes Net from unshare flags
    Given a minimal bwrap profile
    When I enable network access
    Then it should not unshare network
    And it should still unshare pid

  Scenario: Duplicate mount destination fails validation
    Given a minimal bwrap profile with an extra ro bind to "/usr"
    When I add a read-only bind from "/lib" to "/usr"
    Then validation should fail with a duplicate destination error
