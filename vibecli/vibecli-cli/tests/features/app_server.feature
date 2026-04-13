Feature: JSON-RPC 2.0 dispatcher
  AppServer routes incoming JSON-RPC 2.0 requests to registered handlers
  and returns well-formed responses. Unknown methods produce METHOD_NOT_FOUND
  errors; invalid JSON produces PARSE_ERROR responses.

  Scenario: Registered method returns a result
    Given an app server with an "echo" handler
    When I dispatch a request for method "echo" with params "hello"
    Then the response should have no error
    And the result should equal "hello"

  Scenario: Unknown method returns METHOD_NOT_FOUND error
    Given an app server with an "echo" handler
    When I dispatch a request for method "unknown"
    Then the response should have an error with code -32601

  Scenario: Raw JSON for known method round-trips correctly
    Given an app server with an "echo" handler
    When I handle raw JSON '{"jsonrpc":"2.0","id":1,"method":"echo","params":"ping"}'
    Then the raw response should contain "ping"

  Scenario: Malformed JSON produces a parse error
    Given an app server with an "echo" handler
    When I handle raw JSON 'not-json'
    Then the raw response should contain "-32700"
