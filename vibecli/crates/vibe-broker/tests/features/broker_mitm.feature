Feature: TLS MITM forwarding (slice B1.8)
  After CONNECT 200 the broker performs a rustls server handshake to
  the client (with a leaf cert minted for the host), opens TCP to the
  upstream and performs a client TLS handshake, then bidirectionally
  copies plaintext between the two encrypted halves.

  Scenario: end-to-end CONNECT through broker reaches a self-signed HTTPS upstream
    Given a self-signed HTTPS upstream that replies "pong"
    And a broker with TLS interception and the upstream cert in its trust store
    And a policy allowing the upstream host on CONNECT
    When the client performs CONNECT through the broker, then GET / over TLS
    Then the client receives status 200 and body "pong"
