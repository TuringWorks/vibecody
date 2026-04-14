Feature: Watch device authentication and JWT lifecycle
  The WatchAuthManager issues single-use challenges, signs short-lived JWTs,
  and enforces security boundaries: revocation, wrist-off suspension, and
  machine-ID binding prevent token misuse across devices and daemon instances.

  Scenario: Challenge nonce is a 32-character hex string
    Given a fresh WatchAuthManager
    When I issue a registration challenge
    Then the nonce should be 32 hex characters
    And the nonce expiry should be NONCE_TTL_SECS seconds from now

  Scenario: Challenge nonce is consumed on use
    Given a fresh WatchAuthManager
    When I issue a registration challenge
    And I consume the nonce
    Then the nonce should no longer be pending

  Scenario: Access token embeds device ID and machine ID
    Given a fresh WatchAuthManager with machine "daemon-01"
    When I issue an access token for device "watch-abc"
    Then the token claims should have sub "watch-abc"
    And the token claims should have machine_id "daemon-01"
    And the token kind should be "access"

  Scenario: Expired token is rejected
    Given a fresh WatchAuthManager
    When I create a token with expiry in the past
    Then verifying the token should fail with "expired"

  Scenario: Tampered signature is rejected
    Given a fresh WatchAuthManager
    When I issue an access token for device "watch-tamper"
    And I flip a byte in the signature
    Then decoding the token should fail

  Scenario: Refresh token has correct kind field
    Given a fresh WatchAuthManager
    When I issue a refresh token for device "watch-refresh"
    Then the token kind should be "refresh"

  Scenario: Token signed with wrong secret is rejected
    Given a WatchAuthManager with secret "secret-A-32-bytes-hmac-pad!!!!!"
    And another WatchAuthManager with secret "secret-B-32-bytes-hmac-pad!!!!!"
    When manager A issues an access token for device "watch-cross"
    Then manager B should reject the token as invalid

  Scenario: Wrist event with stale timestamp is rejected
    Given a fresh WatchAuthManager
    When I send a wrist-off event with a timestamp 60 seconds old
    Then the event should be rejected with "stale"

  Scenario: Ed25519 signature with wrong length is rejected
    Given a fresh WatchAuthManager
    When I verify an Ed25519 signature of 63 bytes
    Then verification should fail

  Scenario: WatchDevice serialises round-trip through JSON
    Given a WatchDevice with id "d1" and model "Watch7,1"
    When I serialise and deserialise the device
    Then the device_id should be "d1"
    And the model should be "Watch7,1"
    And wrist_suspended should be false
