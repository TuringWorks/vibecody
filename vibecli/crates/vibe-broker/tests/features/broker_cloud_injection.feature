Feature: GCP IAM + Azure MSI credential injection (slice B2.3)
  When the matched rule has Inject::GcpIam or Inject::AzureMsi, the
  broker resolves the SecretRef into a pre-minted access token (the
  daemon refreshes those out-of-band, slice B2.4) and injects it as
  Authorization: Bearer on the outbound HTTPS request. The sandbox
  never holds the underlying GCP service-account key or Azure client
  secret.

  Scenario: GCP IAM injection adds Authorization: Bearer
    Given a self-signed HTTPS upstream that echoes the Authorization header
    And the broker holds a GCP access token at "@workspace.gcp_default" with value "ya29.gcp-test-token"
    And a policy that allows the upstream on CONNECT and GCP-IAM-injects "@workspace.gcp_default" on GET
    When the client performs CONNECT through the broker, then GET on root over TLS, sending its own Authorization "Bearer sandbox-fake"
    Then the upstream observed Authorization "Bearer ya29.gcp-test-token"

  Scenario: Azure MSI injection adds Authorization: Bearer
    Given a self-signed HTTPS upstream that echoes the Authorization header
    And the broker holds an Azure access token at "@workspace.azure_default" with value "eyJ.azure-test-token"
    And a policy that allows the upstream on CONNECT and Azure-MSI-injects "@workspace.azure_default" on GET
    When the client performs CONNECT through the broker, then GET on root over TLS, sending its own Authorization "Bearer sandbox-fake"
    Then the upstream observed Authorization "Bearer eyJ.azure-test-token"
