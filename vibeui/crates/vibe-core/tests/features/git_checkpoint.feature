Feature: Git checkpoint management
  As a developer using VibeUI
  I want to save and restore checkpoints of my work
  So that I can safely experiment and roll back changes

  Background:
    Given a fresh git repository with an initial commit

  Scenario: Creating a checkpoint saves uncommitted changes
    Given the file "work.txt" exists with content "original"
    And the file "work.txt" is modified to contain "new work"
    When I create a checkpoint named "before-refactor"
    Then 1 checkpoint exists in the stash list
    And the checkpoint message contains "before-refactor"
    And the file "work.txt" contains "original"

  Scenario: Listing checkpoints on a clean repo returns nothing
    When I list all checkpoints
    Then 0 checkpoints exist in the stash list

  Scenario: Restoring a checkpoint reapplies saved changes
    Given the file "work.txt" exists with content "original"
    And the file "work.txt" is modified to contain "modified content"
    And I create a checkpoint named "restore-test"
    When I restore the checkpoint at index 0
    Then the file "work.txt" contains "modified content"

  Scenario: Deleting a checkpoint removes it permanently
    Given the file "work.txt" exists with content "original"
    And the file "work.txt" is modified to contain "temp change"
    And I create a checkpoint named "to-delete"
    When I delete the checkpoint at index 0
    Then 0 checkpoints exist in the stash list

  Scenario: Creating multiple checkpoints and dropping one by index
    Given the file "work.txt" exists with content "original"
    And the file "work.txt" is modified to contain "change one"
    And I create a checkpoint named "first"
    And the file "work.txt" is modified to contain "change two"
    And I create a checkpoint named "second"
    When I delete the checkpoint at index 0
    Then 1 checkpoint exists in the stash list
