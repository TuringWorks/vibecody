Feature: Tailscale integration for remote access
  The daemon reads Tailscale status and surfaces the 100.x.x.x IP and
  optionally a Funnel HTTPS URL in the beacon response.

  Scenario: TailscaleInfo can be constructed and accessed
    Given a connected TailscaleInfo with ip "100.64.1.2" and hostname "my-mac"
    Then connected should be true
    And tailscale_ip should be "100.64.1.2"
    And hostname should be "my-mac"

  Scenario: TailscaleInfo disconnected state has no IP
    Given a disconnected TailscaleInfo
    Then connected should be false
    And tailscale_ip should be None

  Scenario: TailscaleInfo serialises to JSON correctly
    Given a connected TailscaleInfo with ip "100.1.2.3" and hostname "dev"
    When I serialise the TailscaleInfo to JSON
    Then the JSON should contain "100.1.2.3"
    And the JSON should contain "connected"

  Scenario: TailscaleInfo deserialises from JSON correctly
    Given JSON '{"connected":false,"tailscale_ip":null,"hostname":null,"tailnet":null}'
    When I deserialise it as TailscaleInfo
    Then connected should be false
    And tailscale_ip should be None

  Scenario: TailscaleInfo roundtrips through serde
    Given a connected TailscaleInfo with ip "100.64.0.1" and hostname "box"
    When I serialise and deserialise the TailscaleInfo
    Then the roundtripped ip should be "100.64.0.1"
    And the roundtripped hostname should be "box"

  Scenario: tailscale_funnel_url returns None when tailscale binary absent
    Given the tailscale binary is not on PATH
    When I call tailscale_funnel_url for port 7878
    Then the result should be None

  Scenario: Parse funnel status JSON — funnel active on port 443
    Given a tailscale status JSON with DNSName "my-mac.tailnet.ts.net." and FunnelPorts [443]
    When I parse the funnel URL
    Then the funnel URL should be "https://my-mac.tailnet.ts.net"

  Scenario: Parse funnel status JSON — funnel not active
    Given a tailscale status JSON with DNSName "my-mac.tailnet.ts.net." and FunnelPorts []
    When I parse the funnel URL
    Then the funnel URL should be None

  Scenario: Parse funnel status JSON — missing DNSName
    Given a tailscale status JSON with no DNSName and FunnelPorts [443]
    When I parse the funnel URL
    Then the funnel URL should be None
