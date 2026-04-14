Feature: Session Tree

  Scenario: Linear append builds a chain
    Given a new session tree
    When I append a user message "hello"
    And I append an assistant message "hi" as child of the last entry
    Then the tree has 2 entries
    And the last entry is a leaf

  Scenario: Branching from a midpoint creates two leaves
    Given a new session tree
    When I append a user message "start"
    And I append an assistant message "branch-A" as child of "start"
    And I branch from "start" with an assistant message "branch-B"
    Then the tree has 3 entries
    And the branch count is 2

  Scenario: path_to returns the correct ancestor chain
    Given a new session tree
    When I append a user message "root"
    And I append an assistant message "middle" as child of "root"
    And I append a user message "leaf" as child of "middle"
    Then the path to "leaf" has length 3
    And the first entry in the path is "root"

  Scenario: JSONL roundtrip preserves all entries
    Given a new session tree
    When I append a user message "first"
    And I append an assistant message "second" as child of "first"
    And I append a compaction entry under "second"
    And I serialize the tree to JSONL
    And I deserialize the JSONL into a new tree
    Then the restored tree has 3 entries

  Scenario: Folding a subtree hides it
    Given a new session tree
    When I append a user message "root"
    And I append an assistant message "side-branch" as child of "root"
    And I append a user message "side-leaf" as child of "side-branch"
    And I append an assistant message "main-leaf" as child of "root"
    And I fold the subtree at "side-branch"
    Then only 2 entries are visible after folding

  Scenario: Labelling an entry persists through serialization
    Given a new session tree
    When I append a user message "checkpoint"
    And I label that entry "release-v1"
    And I serialize the tree to JSONL
    And I deserialize the JSONL into a new tree
    Then the restored entry has label "release-v1"
