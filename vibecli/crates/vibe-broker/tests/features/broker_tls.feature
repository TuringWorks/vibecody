Feature: TLS interception (slice B1.7)
  The broker mints a per-broker root CA at startup, mints ephemeral leaf
  certs per origin on demand, terminates TLS broker-side, inspects each
  HTTPS request through policy + SSRF, then re-encrypts to the upstream.
  Clients trust the broker via env-injected CA bundle (the host system
  CA store is unchanged).

  Scenario: CA is generated on first construction and stable across leaf mints
    Given a fresh BrokerCa
    When I read the CA cert PEM
    Then the PEM starts with "-----BEGIN CERTIFICATE-----"
    And the PEM ends with a "-----END CERTIFICATE-----" block

  Scenario: leaf_for returns a cert valid for the requested SAN
    Given a fresh BrokerCa
    When I mint a leaf for "api.example.com"
    Then the leaf cert SAN list contains "api.example.com"
    And the leaf cert is signed by the broker CA

  Scenario: leaf_for caches per-origin
    Given a fresh BrokerCa
    When I mint a leaf for "api.example.com"
    And I mint a leaf for "api.example.com" again
    Then both leaf certs share the same serial number

  Scenario: CONNECT to a denied host is refused before any TLS bytes
    Given a running broker with TLS interception and empty policy
    When the client sends "CONNECT api.openai.com:443"
    Then the broker response status is 403
    And the broker response header "x-vibe-broker-reason" is "policy_denied"

  Scenario: CONNECT to an allowed host returns 200 then does MITM
    Given a running broker with TLS interception and a rule allowing "api.example.com" on CONNECT
    When the client sends "CONNECT api.example.com:443"
    Then the broker response status is 200
