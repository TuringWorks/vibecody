Feature: AutoDream memory consolidation
  AutoDream merges duplicates, prunes stale entries, evicts on overflow,
  and ranks surviving entries by access frequency and recency.

  Scenario: Duplicate keys are merged into one entry
    Given a memory store with two entries for key "topic"
    When I consolidate the memory
    Then the result should have 1 kept entry

  Scenario: Stale entries are pruned
    Given a memory store with an entry created 30 days ago
    And the max age policy is 7 days
    When I consolidate the memory
    Then the result should have 0 kept entries

  Scenario: Overflow entries are evicted by lowest access count
    Given a memory store with 3 entries and a max_entries limit of 2
    And the entry with key "rare" has access_count 1
    When I consolidate the memory
    Then the result should have 2 kept entries
    And the kept entries should not include key "rare"

  Scenario: Entries are ranked by access count descending
    Given a memory store with two entries
    And the entry with key "popular" has access_count 50
    And the entry with key "rare" has access_count 1
    When I rank the entries
    Then the first entry should have key "popular"
