Feature: Session HTML export and GitHub Gist sharing
  VibeCody can export sessions as self-contained HTML files with
  syntax-highlighted code blocks and optionally upload them to a private
  GitHub Gist to obtain a shareable link. Bridges the pi-mono /export
  and /share command gap (Phase C3).

  Scenario: HTML export contains all messages with correct CSS classes
    Given a session with a user message "What is Rust ownership?" and an assistant message "Ownership means each value has one owner."
    When I export the session as HTML with default options
    Then the output starts with "<!DOCTYPE html>"
    And the output contains the CSS class "msg-user"
    And the output contains the CSS class "msg-assistant"
    And the output contains the session title in a <title> element

  Scenario: Code fence highlighting wraps blocks with language class
    Given a markdown content block with a fenced Rust code block
    When I apply highlight_fences to the content
    Then the output contains a pre element with class "language-rust"
    And the output does not contain the raw triple-backtick fence markers

  Scenario: HTML escape neutralises dangerous characters
    Given a raw string containing HTML special characters '<', '>', '&', '"', and "'"
    When I call escape_html on the string
    Then the output contains "&lt;" instead of "<"
    And the output contains "&gt;" instead of ">"
    And the output contains "&amp;" instead of "&"
    And the output contains "&quot;" instead of '"'
    And the output contains "&#39;" instead of "'"

  Scenario: Gist payload is valid JSON with correct structure
    Given a session title "my-session" and HTML content "<html/>"
    And a GistOptions with description "Test share" and public false
    When I build the gist payload
    Then the payload contains the description "Test share"
    And the payload contains "public":false
    And the payload contains the filename "session-my-session.html"
    And the payload contains a "content" key

  Scenario: Gist response parsing extracts gist_id and html_url
    Given a mock GitHub API response JSON with id "gist42" and html_url "https://gist.github.com/user/gist42"
    When I parse the gist response
    Then the parsed gist_id equals "gist42"
    And the parsed html_url equals "https://gist.github.com/user/gist42"
    And no error is returned
