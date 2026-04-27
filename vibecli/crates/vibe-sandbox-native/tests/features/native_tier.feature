Feature: native() tier construction
  vibe_sandbox_native::native() returns the OS-appropriate Tier-0 sandbox
  with tier() == Native, regardless of which platform we're built on.

  Scenario: native() builds a Native-tier sandbox
    When I call vibe_sandbox_native::native
    Then I get a sandbox whose tier is "Native"
