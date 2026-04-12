Feature: Tool-pair-preserving auto-compaction
  When compacting a conversation, the compaction boundary must not split
  a ToolUse/ToolResult pair. The compacted region is replaced by a
  structured summary and a synthetic assistant continuation message.

  Scenario: Compaction boundary avoids splitting a ToolUse/ToolResult pair
    Given a conversation ending with a ToolUse at position 9 and ToolResult at position 10
    And a raw compaction boundary of 10
    When I find the safe boundary
    Then it should be 9

  Scenario: Summary captures role counts correctly
    Given a conversation with 5 user, 3 assistant, and 1 system message
    When I generate a compaction summary
    Then user_count should be 5
    And assistant_count should be 3

  Scenario: Last 3 user requests are preserved in the summary
    Given a conversation with 6 user messages
    When I generate a compaction summary
    Then last_user_requests should contain exactly 3 entries

  Scenario: Synthetic continuation is an assistant message
    Given a compaction summary with 2 user and 1 assistant message
    When I create the synthetic continuation
    Then its role should be "assistant"

  Scenario: Full compaction round-trip preserves tool pairs
    Given a conversation with interleaved ToolUse and ToolResult messages
    When I compact the conversation with keep_recent 2
    Then no ToolUse message should be followed by a non-ToolResult message
