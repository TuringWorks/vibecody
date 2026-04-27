Feature: Sandbox trait + tier selection
  The vibe-sandbox crate exposes a unified Sandbox trait, a SandboxTier
  enum identifying which backend is active, and a select() function that
  picks the OS-appropriate native tier. Stub implementations refuse
  unsupported tiers cleanly with TierUnsupported.

  Scenario: select with Native tier returns a sandbox advertising Native tier
    Given a request for the Native tier
    When I call select on it
    Then I get a sandbox whose tier is "Native"

  Scenario: SandboxTier round-trips via Display + FromStr
    Given a tier name "Firecracker"
    When I parse it via FromStr
    Then I get a tier whose Display is "Firecracker"

  Scenario: NetPolicy default is None
    Given a fresh NetPolicy default
    Then the policy variant is "None"

  Scenario: ResourceLimits default has no caps set
    Given a fresh ResourceLimits default
    Then memory_bytes is unset
    And cpu_quota_ms_per_sec is unset
    And wall_clock is unset

  Scenario: select with Firecracker tier on a non-Linux host downgrades to Native
    Given a host that does not support Firecracker
    When I call select on the Firecracker tier
    Then the returned tier is "Native"
    And a downgrade event was recorded

  Scenario: BindMode is one of Rw or Ro
    Given a BindMode "Rw"
    Then the mode allows writes
    And the mode allows reads
    Given a BindMode "Ro"
    Then the mode does not allow writes
    And the mode allows reads
