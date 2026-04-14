Feature: RPC mode JSONL framing and transport
  The RPC mode module exchanges JSON objects as LF-terminated lines over
  stdin/stdout. Each line carries a "type" field plus a payload. The
  transport must use LF-only framing (never CRLF) and must reject frames
  that are missing the required "type" field.

  Scenario: Serialised frame ends with LF not CRLF
    Given a token_delta frame with text "hello world"
    When I serialise the frame to a JSONL line
    Then the line ends with LF
    And the line does not end with CRLF
    And the line contains the text "hello world"

  Scenario: Deserialise a well-formed JSONL line
    Given a JSONL line with type "ping" and id "req-1"
    When I parse the line into an RPC frame
    Then the frame type is "ping"
    And the frame field "id" is "req-1"

  Scenario: Missing type field returns an error
    Given a JSONL line without a "type" field
    When I attempt to parse the line
    Then parsing fails with an error containing "type"

  Scenario: RpcReader collects multiple frames
    Given a byte stream containing 3 valid JSONL frames
    When I collect all frames with an RpcReader
    Then I receive exactly 3 frames
    And the first frame type is "send_message"
    And the second frame type is "interrupt"
    And the third frame type is "shutdown"

  Scenario: Writer and MemoryTransport roundtrip
    Given an empty MemoryTransport
    When I write a pong frame with id "p-99" through the transport writer
    And I flush the transport writer
    Then the transport outbound count is 1
    And the first popped outbound frame has type "pong"
    And the first popped outbound frame field "id" is "p-99"
