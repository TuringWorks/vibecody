Feature: Watch bridge — Axum router state and response structures
  The WatchBridgeState is the standalone state type for all /watch/* handlers.
  It decouples the lib crate from binary-only modules (session_store, serve).
  The router handles 11 routes with auth, replay prevention, and SSE streaming.

  Scenario: Dispatch response includes session ID in streaming URL
    Given a WatchDispatchResponse with session_id "sess-xyz"
    When I serialise the response
    Then the streaming_url should contain "sess-xyz"
    And the streaming_url should start with "/watch/stream/"

  Scenario: WatchEventStreams map is empty on initialisation
    Given a new WatchEventStreams map
    Then the map should be empty

  Scenario: WatchEventStreams accepts a broadcast sender for a session
    Given a new WatchEventStreams map
    When I insert a broadcast sender for session "session-1"
    Then the map should contain key "session-1"
    And the map size should be 1

  Scenario: Nonce replay in bridge context is rejected
    Given a NonceRegistry used by the bridge
    And the current Unix timestamp
    When I record nonce "bridge-dispatch-001"
    Then recording the same nonce again should fail with "Nonce"

  Scenario: Different dispatch nonces are both accepted
    Given a NonceRegistry used by the bridge
    And the current Unix timestamp
    When I record nonce "dispatch-A"
    And I record nonce "dispatch-B"
    Then both should succeed

  Scenario: SandboxControlRequest serialises action field
    Given a WatchSandboxControlRequest with action "pause"
    When I serialise it to JSON
    Then the JSON should contain "pause"
    And deserialising should produce action "pause"

  Scenario: Dispatch request without session_id starts a new session
    Given a WatchDispatchRequest with no session_id
    When I serialise and deserialise the request
    Then session_id should be null

  Scenario: Dispatch request with session_id continues existing session
    Given a WatchDispatchRequest with session_id "existing-session-abc"
    When I serialise and deserialise the request
    Then session_id should be "existing-session-abc"

  Scenario: WatchBridgeState is a valid sized type
    When I check the size of WatchBridgeState
    Then the size should be greater than zero

  Scenario: WatchDispatchResponse serialises all required fields
    Given a WatchDispatchResponse with session_id "s1" and message_id 99
    When I serialise the response
    Then the JSON should contain "s1"
    And the JSON should contain "99"
