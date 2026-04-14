Feature: Bracketed paste safety guard
  PasteGuard wraps terminal input processing to detect bracketed paste
  sequences (ESC[200~ ... ESC[201~). Small pastes pass through verbatim;
  large pastes are collapsed to a compact marker and stored in a ring buffer.

  Background:
    Given a PasteGuard with line threshold 5 and max stored pastes 3

  Scenario: Small paste passes through verbatim
    Given a paste containing 3 lines
    When I process the bracketed input
    Then the result was_paste flag is true
    And the result was_collapsed flag is false
    And the processed output contains the original lines

  Scenario: Large paste is collapsed to a marker
    Given a paste containing 20 lines
    When I process the bracketed input
    Then the result was_paste flag is true
    And the result was_collapsed flag is true
    And the processed output contains a marker matching "[paste #1 +20 lines]"
    And the processed output does not contain "line 10"

  Scenario: expand_marker retrieves the original content
    Given a paste containing 12 lines
    When I process the bracketed input
    Then the result was_collapsed flag is true
    When I expand the marker in the processed output
    Then the expanded content matches the original paste

  Scenario: Ring buffer eviction drops the oldest paste
    Given a paste containing 10 lines labeled "first"
    When I process the bracketed input
    Given a paste containing 10 lines labeled "second"
    When I process the bracketed input
    Given a paste containing 10 lines labeled "third"
    When I process the bracketed input
    Given a paste containing 10 lines labeled "fourth"
    When I process the bracketed input
    Then the store contains 3 pastes
    And paste id 1 is no longer in the store
    And paste id 2 is in the store

  Scenario: extract_paste_content returns the inner text from raw input
    Given a raw string with bracketed paste sequences wrapping "hello world"
    When I call extract_paste_content on it
    Then the extracted content is "hello world"
