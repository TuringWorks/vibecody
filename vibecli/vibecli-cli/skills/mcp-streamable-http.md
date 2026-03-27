# MCP Streamable HTTP

MCP transport using Streamable HTTP with OAuth 2.1 enterprise authentication. Replaces legacy SSE transport with bidirectional streaming, supports token refresh, PKCE flows, and multi-tenant authorization for enterprise MCP deployments.

## When to Use
- Connecting to MCP servers that use the new Streamable HTTP transport
- Setting up OAuth 2.1 authentication for enterprise MCP endpoints
- Migrating from SSE-based MCP connections to Streamable HTTP
- Configuring multi-tenant MCP access with per-user token management
- Debugging MCP transport issues with connection diagnostics

## Commands
- `/mcp connect <url>` — Connect to an MCP server via Streamable HTTP
- `/mcp auth <url>` — Initiate OAuth 2.1 PKCE flow for an MCP server
- `/mcp token refresh <server>` — Manually refresh OAuth token
- `/mcp transport status` — Show active transports and connection health
- `/mcp migrate <server>` — Migrate an SSE connection to Streamable HTTP
- `/mcp headers <server> <key> <value>` — Set custom headers for a connection
- `/mcp debug <server>` — Enable verbose transport logging

## Examples
```
/mcp connect https://mcp.enterprise.com/v1
# Transport: Streamable HTTP (bidirectional)
# Auth required: OAuth 2.1 — initiating PKCE flow...
# Browser opened for authorization. Waiting for callback...
# Connected! 14 tools available, 3 resources.

/mcp transport status
# mcp.enterprise.com: Streamable HTTP, OAuth OK (expires 58m)
# localhost:3001: stdio, no auth
# tools.internal.co: Streamable HTTP, token refresh needed

/mcp migrate legacy-server
# Migrated legacy-server from SSE to Streamable HTTP. Testing... OK.
```

## Best Practices
- Use OAuth 2.1 with PKCE for all production MCP connections
- Set token refresh buffers to avoid mid-request expiration
- Test Streamable HTTP connections with debug mode before production use
- Store OAuth client credentials in system keychain, not config files
- Use custom headers for correlation IDs in multi-tenant environments
