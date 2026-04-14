Feature: TUI IME cursor marker and CJK width support
  tui_ime provides CURSOR_MARKER APC escape insertion/detection for IME
  candidate window positioning, and ANSI-aware CJK wide-character width
  calculation for correct terminal layout.

  Scenario: Insert and find cursor marker in ASCII text
    Given an ASCII line "hello world"
    When I insert the cursor marker at column 5
    Then the visible text is still "hello world"
    And find_cursor_marker returns column 5

  Scenario: Strip cursor marker from rendered output
    Given a rendered string with cursor markers at columns 0 and 6
    When I strip all cursor markers
    Then the result equals "foobar"

  Scenario: Visible width ignores ANSI escape sequences
    Given the string "\x1b[32mhello\x1b[0m"
    When I compute the visible width
    Then the visible width is 5

  Scenario: Visible width counts CJK wide characters as 2 columns
    Given the string "日本語"
    When I compute the visible width
    Then the visible width is 6

  Scenario: truncate_to_width respects wide character boundaries
    Given the string "你好世界" with max columns 5
    When I truncate to max columns
    Then the truncated visible width is at most 5
    And no wide character is split across the boundary

  Scenario: IME state machine full composition lifecycle
    Given a fresh ImeHandler
    When composition starts
    Then the IME state is Composing
    When composition updates to "中文"
    Then the preedit text is "中文"
    When composition ends with "中文输入"
    Then the IME state is Committed
    And the committed text is "中文输入"
    When I reset the IME handler
    Then the IME state is Idle
