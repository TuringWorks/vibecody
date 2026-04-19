Feature: MCP Streamable HTTP + OAuth 2.1 PKCE over real HTTP (US-004)
  McpHttpClient exchanges authorization codes for tokens against a real
  token endpoint, refreshes access tokens, and opens SSE streams to an
  MCP server with Bearer auth. PKCE challenges use SHA-256 and CSPRNG
  randomness (no hand-rolled SHA or xorshift).

  Scenario: PKCE challenge has the correct shape
    When a PKCE S256 challenge is generated
    Then the verifier is at least 43 base64url characters
    And the challenge is the base64url of SHA-256(verifier)

  Scenario: Authorization URL carries client_id, redirect, state, and PKCE
    Given an OAuth config with client "app-1", redirect "https://x/cb", scopes "read write"
    When the client builds an authorization URL with state "s-42" and a fresh PKCE challenge
    Then the URL contains "client_id=app-1"
    And the URL contains "redirect_uri=https%3A%2F%2Fx%2Fcb"
    And the URL contains "state=s-42"
    And the URL contains "code_challenge_method=S256"
    And the URL contains "scope=read+write"

  Scenario: Exchanging an authorization code posts code_verifier and returns tokens
    Given a mock OAuth token server that requires code_verifier "v-1" and issues access "at-1" refresh "rt-1"
    When the client exchanges code "code-1" with verifier "v-1"
    Then the received access token is "at-1"
    And the received refresh token is "rt-1"

  Scenario: Token refresh posts refresh_token grant and returns new access token
    Given a mock OAuth token server that accepts refresh "rt-1" and issues access "at-2"
    When the client refreshes with refresh token "rt-1"
    Then the received access token is "at-2"

  Scenario: MCP stream opens with Bearer token and delivers SSE messages
    Given a mock MCP server that requires bearer "at-1" and emits 2 SSE messages
    When the client opens a stream with token "at-1" and reads at most 2 messages
    Then the stream yields 2 messages
    And message 1 contains "hello"
    And message 2 contains "world"

  Scenario: MCP stream rejects missing or wrong Bearer token
    Given a mock MCP server that requires bearer "good" and emits 1 SSE messages
    When the client opens a stream with token "wrong" and reads at most 1 messages
    Then opening the stream returns an authorization error
