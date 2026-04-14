Feature: Streaming tool call argument accumulation
  As the VibeCody TUI layer,
  I want to accumulate partial JSON tool-call argument fragments
  so that I can surface real-time UI hints before the full payload arrives.

  Scenario: Single delta is accumulated and partial keys are detected
    Given a new accumulator for call "call_1" and tool "write_file"
    When I push the fragment '{"path": "src/main.rs", "con'
    Then the buffer should contain "src/main.rs"
    And the sequence should be 1
    And the extractable keys should include "path"

  Scenario: Key extraction from a partial JSON object
    Given a new accumulator for call "call_2" and tool "edit_file"
    When I push the fragment '{"file_path": "lib.rs", "content": "fn foo() {'
    Then the extractable keys should include "file_path"
    And the extractable keys should not include "content"

  Scenario: Finalization returns a parsed JSON value
    Given a new accumulator for call "call_3" and tool "bash"
    When I push the fragment '{"command": "cargo build --release"}'
    Then finalizing should succeed
    And the finalized value at "command" should be "cargo build --release"

  Scenario: File path hint is detected and surfaced
    Given a new accumulator for call "call_4" and tool "write_file"
    When I push the fragment '{"path": "src/lib.rs"}'
    Then the hint should be a FilePath hint
    And the hint file path should be "src/lib.rs"
    And the rendered hint for tool "write_file" should contain "src/lib.rs"

  Scenario: Manager tracks multiple concurrent tool calls
    Given a new streaming tool call manager
    When I send a delta for call "call_a" tool "write_file" with fragment '{"path": "a.rs"}'
    And I send a delta for call "call_b" tool "bash" with fragment '{"command": "ls"}'
    Then the manager should have 2 active calls
    And completing call "call_a" should succeed
    And completing call "call_b" should succeed
