---
triggers: ["Cognito", "aws cognito", "user pool", "identity pool", "cognito trigger", "cognito JWT", "cognito hosted UI"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS Cognito Authentication

When working with AWS Cognito:

1. Create user pools with strong password policies (`MinimumLength: 12`, require symbols and numbers) and enable MFA (`MfaConfiguration: "OPTIONAL"` or `"ON"`) with TOTP as the preferred method; SMS MFA is less secure due to SIM-swapping risks.
2. Validate Cognito JWTs on the server by verifying the signature against the JWKS endpoint (`https://cognito-idp.{region}.amazonaws.com/{userPoolId}/.well-known/jwks.json`), checking `iss`, `aud` (or `client_id`), `token_use` (`access` or `id`), and `exp` claims.
3. Use Lambda triggers for custom auth flows: `PreSignUp` for auto-verification, `PreTokenGeneration` to inject custom claims, `PostConfirmation` to provision downstream resources, and `DefineAuthChallenge` + `CreateAuthChallenge` + `VerifyAuthChallengeResponse` for passwordless flows.
4. Implement RBAC by adding users to Cognito groups that map to IAM roles; access the `cognito:groups` claim in the ID token to enforce permissions in your API layer with middleware checks.
5. Configure the Hosted UI for OAuth2/OIDC flows (`response_type=code`) with PKCE for SPAs; set callback URLs precisely and use `allowed_oauth_scopes` to limit token scope to what each client needs.
6. Federate with social providers (Google, Apple, Facebook) and SAML/OIDC enterprise IdPs by configuring identity providers on the user pool; use attribute mapping to normalize claims (`email`, `name`) across providers.
7. Use identity pools (federated identities) to vend temporary AWS credentials scoped by authenticated/unauthenticated roles; set trust policies with `cognito-identity.amazonaws.com` and condition on `aud` to lock to your identity pool.
8. Call `AdminInitiateAuth` server-side (with `ADMIN_NO_SRP_AUTH` flow) only from trusted backends; use `USER_SRP_AUTH` or `USER_PASSWORD_AUTH` from clients with the Amplify SDK to avoid exposing credentials.
9. Configure token expiry appropriately: short-lived access tokens (15-60 min), longer refresh tokens (7-30 days); call `InitiateAuth` with `REFRESH_TOKEN_AUTH` grant type to rotate access tokens without re-prompting users.
10. Enable advanced security features (`UserPoolAddOns: {AdvancedSecurityMode: "ENFORCED"}`) for adaptive authentication that detects compromised credentials, anomalous sign-ins, and automated bot attempts.
11. Use custom scopes in the resource server (`aws cognito-idp create-resource-server`) to define API-level permissions (e.g., `api/read`, `api/write`); request specific scopes in the OAuth2 authorize call and validate them in your API authorizer.
12. Store the refresh token securely (HttpOnly cookie or OS keychain) and never expose it to JavaScript; implement token revocation via `RevokeToken` API on logout and configure the user pool client to enable token revocation.
