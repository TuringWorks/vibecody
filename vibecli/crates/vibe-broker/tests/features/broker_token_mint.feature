Feature: Token-mint flow for GCP + Azure (slice B2.4)
  The daemon mints OAuth2 access tokens for cloud providers out-of-band
  and stashes them in the SecretStore for the broker's hot path. v1
  ships the minter primitives + a refresh-aware cache wrapper; the
  daemon-side scheduler that drives them is a follow-on slice.

  Scenario: Azure client_credentials mint exchanges form-encoded POST for an access token
    Given a stub Azure OAuth endpoint at /tenant42/oauth2/v2.0/token returning access_token "azu-test-tok-1" with expires_in 3600
    And an AzureClientCredentialsMinter pointing at the stub with tenant "tenant42" client_id "abc" client_secret "shh" scope "https://graph.microsoft.com/.default"
    When I mint via the Azure minter
    Then the minted access_token is "azu-test-tok-1"
    And the minted token expires at least 3500 seconds from now

  Scenario: cached minter reuses tokens until close to expiry
    Given a stub Azure OAuth endpoint at /tenant42/oauth2/v2.0/token returning access_token "azu-cached-1" with expires_in 3600
    And an AzureClientCredentialsMinter pointing at the stub with tenant "tenant42" client_id "abc" client_secret "shh" scope "default"
    And a CachedMinter wrapping it with a 300-second refresh buffer
    When I mint via the cached minter
    And I mint via the cached minter
    Then the cached minter underlying mint count is 1

  Scenario: cached minter refreshes when within the buffer
    Given a stub Azure OAuth endpoint at /tenant42/oauth2/v2.0/token returning access_token "azu-fresh" with expires_in 60
    And an AzureClientCredentialsMinter pointing at the stub with tenant "tenant42" client_id "abc" client_secret "shh" scope "default"
    And a CachedMinter wrapping it with a 300-second refresh buffer
    When I mint via the cached minter
    And I mint via the cached minter
    Then the cached minter underlying mint count is 2
