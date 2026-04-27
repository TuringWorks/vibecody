Feature: Bearer + Basic credential injection (slice B2.1)
  When the matched policy rule has an Inject directive, the broker
  resolves the SecretRef against its SecretStore and adds the matching
  `Authorization` header on the request it forwards to the real
  upstream. The sandbox-supplied `Authorization` header (if any) is
  dropped before forwarding — the sandbox never sees the real secret.

  Scenario: Bearer injection adds Authorization on outbound request
    Given a self-signed HTTPS upstream that echoes the Authorization header
    And the broker holds a secret "@profile.openai_key" with value "sk-test-token-123"
    And a policy that allows the upstream on CONNECT and bearer-injects "@profile.openai_key" on GET
    When the client performs CONNECT through the broker, then GET on root over TLS, sending its own Authorization "Bearer sandbox-fake"
    Then the upstream observed Authorization "Bearer sk-test-token-123"
