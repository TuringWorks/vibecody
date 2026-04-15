Feature: Watch P256 ECDSA authentication
  The daemon verifies Apple Watch registrations using P256 ECDSA signatures
  produced by the Secure Enclave. Ed25519 / wrong-length keys are rejected.

  Background:
    Given a fresh WatchAuthManager

  Scenario: Valid P256 signature is accepted during registration
    Given a P256 signing key is generated
    And a registration challenge is issued
    When the watch signs the challenge with the P256 key
    Then register_device succeeds
    And the returned device_id matches the request

  Scenario: Wrong public key length is rejected
    Given a registration challenge is issued
    When register_device is called with a 32-byte public key
    Then register_device fails with "64 bytes"

  Scenario: Invalid signature bytes are rejected
    Given a P256 signing key is generated
    And a registration challenge is issued
    When register_device is called with zeroed signature bytes
    Then register_device fails with "signature"

  Scenario: Signature over wrong message is rejected
    Given a P256 signing key is generated
    And a registration challenge is issued
    When the watch signs a tampered message with the P256 key
    Then register_device fails with "signature"

  Scenario: Nonce can only be used once
    Given a P256 signing key is generated
    And a registration challenge is issued
    When the watch signs the challenge with the P256 key
    And register_device succeeds
    And the same request is replayed
    Then register_device fails with "already-used"
