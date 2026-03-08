---
triggers: ["container scanning", "Trivy", "Grype", "image scanning", "container vulnerability", "cosign", "Sigstore", "distroless", "chainguard", "container security"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["docker"]
category: security
---

# Container and Image Vulnerability Scanning

When working with container scanning:

1. Scan container images with Trivy for vulnerabilities, misconfigurations, and secrets in a single pass: `trivy image --severity HIGH,CRITICAL --format json myregistry/myapp:latest > trivy-results.json` or use `trivy image --exit-code 1 --severity CRITICAL myapp:latest` in CI to fail builds on critical findings.

2. Use Grype as an alternative or complementary scanner: `grype myregistry/myapp:latest -o json > grype-results.json` and compare results against Trivy since different scanners use different vulnerability databases; run both in CI with `grype myapp:latest --fail-on high` to increase detection coverage.

3. Scan container registries continuously by configuring scheduled scans: `trivy image --server http://trivy-server:4954 myregistry/myapp:latest` using Trivy in client-server mode for centralized scanning, or configure Harbor/ACR/ECR built-in scanning to automatically scan images on push with vulnerability reports accessible via registry UI.

4. Select minimal base images to reduce attack surface: prefer `gcr.io/distroless/static-debian12` for Go/Rust binaries (zero shell, zero package manager), `cgr.dev/chainguard/python:latest` for Python workloads, or `gcr.io/distroless/java21-debian12` for Java; verify with `trivy image gcr.io/distroless/static-debian12 --severity HIGH,CRITICAL` showing zero or near-zero CVEs.

5. Build with Chainguard images for continuously updated, FIPS-compliant, and SBOM-included base images: `FROM cgr.dev/chainguard/node:latest` in your Dockerfile, then verify signatures with `cosign verify cgr.dev/chainguard/node:latest --certificate-identity-regexp='.*chainguard.*'` to ensure supply chain integrity.

6. Implement admission controllers in Kubernetes to block unscanned or vulnerable images: deploy Kyverno with a policy `spec.rules[].validate.image.verify[].attestors` requiring signed scan results, or use OPA Gatekeeper with a rego policy checking that images have been scanned within the last 24 hours with no critical vulnerabilities.

7. Enable runtime scanning to detect vulnerabilities introduced after deployment: deploy Falco or Sysdig for runtime threat detection with `helm install falco falcosecurity/falco --set falcosidekick.enabled=true`, and run periodic re-scans of running images with `trivy image $(kubectl get pods -o jsonpath='{.items[*].spec.containers[*].image}' | tr ' ' '\n' | sort -u)`.

8. Sign container images using cosign and Sigstore keyless signing: `cosign sign --yes myregistry/myapp@sha256:abc123` uses OIDC-based keyless signing via Fulcio and records to Rekor transparency log; verify with `cosign verify myregistry/myapp@sha256:abc123 --certificate-identity=user@example.com --certificate-oidc-issuer=https://accounts.google.com`.

9. Enforce image signing policies in CI by adding verification gates: `cosign verify myregistry/myapp:latest --key cosign.pub || exit 1` before deployment, and configure Kubernetes admission with Sigstore policy-controller to reject unsigned images: `kubectl apply -f https://github.com/sigstore/policy-controller/releases/latest/download/policy-controller.yaml`.

10. Configure scan policies in CI/CD with severity thresholds and exception lists: create `.trivy.yaml` with `severity: [HIGH, CRITICAL]`, `ignore-unfixed: true`, and `.trivyignore` for accepted risks with expiration dates; in GitHub Actions use `aquasecurity/trivy-action@master` with `exit-code: 1` and `severity: CRITICAL`.

11. Scan Dockerfiles for misconfigurations before building: `trivy config Dockerfile` or `hadolint Dockerfile` to catch issues like running as root, using `latest` tags, missing health checks, exposing unnecessary ports, and storing secrets in layers; integrate with `hadolint --format json Dockerfile > hadolint-results.json` in pre-commit hooks.

12. Generate and attach SBOMs to container images for full transparency: `syft myregistry/myapp:latest -o cyclonedx-json > sbom.json && cosign attest --predicate sbom.json --type cyclonedx myregistry/myapp@sha256:abc123` creates a signed attestation, then scan the SBOM with `grype sbom:sbom.json` and share with consumers who can verify with `cosign verify-attestation`.
