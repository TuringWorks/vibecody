Feature: TUI inline image rendering
  VibeCLI can detect the terminal image protocol and render inline images
  using Kitty Graphics Protocol or iTerm2, falling back to a text placeholder
  when neither protocol is available.

  Scenario: Kitty protocol detected from KITTY_WINDOW_ID env var
    Given the environment variable "KITTY_WINDOW_ID" is set to "1"
    And the environment variable "TERM_PROGRAM" is unset
    When I detect the image protocol
    Then the protocol should be "kitty"
    And the protocol should be supported

  Scenario: iTerm2 protocol detected from TERM_PROGRAM env var
    Given the environment variable "KITTY_WINDOW_ID" is unset
    And the environment variable "TERM" is unset
    And the environment variable "TERM_PROGRAM" is set to "iTerm.app"
    When I detect the image protocol
    Then the protocol should be "iterm2"
    And the protocol should be supported

  Scenario: PNG dimensions are parsed from header bytes
    Given a synthetic 320x240 PNG header
    When I parse the image dimensions
    Then the width should be 320
    And the height should be 240

  Scenario: Kitty escape sequence has correct prefix and terminator
    Given raw image data "hello kitty"
    And the render protocol is "kitty"
    When I build the escape sequence with cols 40 and rows 10
    Then the escape sequence should start with "\x1b_G"
    And the escape sequence should end with "\x1b\\"
    And the escape sequence should contain "a=T"

  Scenario: iTerm2 escape sequence has correct prefix and terminator
    Given raw image data "hello iterm"
    And the render protocol is "iterm2"
    When I build the escape sequence with width 100 and height 50
    Then the escape sequence should start with "\x1b]1337;"
    And the escape sequence should contain "inline=1"
    And the escape sequence should end with "\x07"

  Scenario: Fallback placeholder when protocol is None
    Given raw image data "some bytes"
    And the render protocol is "none"
    When I render the image bytes
    Then the result should be a fallback
    And the placeholder text should contain "[image:"
    And the escape sequence should be empty
