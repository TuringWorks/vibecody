Feature: Branch lock collision detection
  Prevents parallel agents from conflicting on the same branch by
  detecting exact, parent-child, and nested-module collisions.

  Scenario: Exact collision is detected when same branch is re-locked
    Given branch "feature/auth" is locked by lane "lane-1" for write
    When lane "lane-2" tries to lock "feature/auth" for write
    Then the acquisition should fail with a collision

  Scenario: Parent-child collision is detected
    Given branch "feature" is locked by lane "lane-1" for write
    When lane "lane-2" tries to lock "feature/login" for write
    Then the acquisition should fail with a collision

  Scenario: Read locks do not collide with each other
    Given branch "main" is locked by lane "lane-1" for read
    When lane "lane-2" tries to lock "main" for read
    Then the acquisition should succeed

  Scenario: Releasing a lock allows re-acquisition by another lane
    Given branch "develop" is locked by lane "lane-1" for write
    When lane "lane-1" releases "develop"
    And lane "lane-2" tries to lock "develop" for write
    Then the acquisition should succeed
