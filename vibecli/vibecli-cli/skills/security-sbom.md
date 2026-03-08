---
triggers: ["SBOM", "software bill of materials", "CycloneDX", "SPDX", "Syft", "sbom generation", "software composition", "VEX", "dependency inventory"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# SBOM Generation and Management

When working with SBOMs:

1. Generate SBOMs from source directories using Syft with CycloneDX format: `syft dir:. -o cyclonedx-json > sbom.cdx.json` for JSON or `syft dir:. -o spdx-json > sbom.spdx.json` for SPDX; for container images use `syft docker:myimage:latest -o cyclonedx-json > image-sbom.cdx.json` to capture all installed packages.

2. Choose between CycloneDX and SPDX based on your use case: CycloneDX excels at vulnerability correlation with native VEX support and is preferred for security workflows; SPDX is the ISO/IEC 5962:2021 standard preferred for license compliance and government procurement; generate both with `syft . -o cyclonedx-json,spdx-json`.

3. Analyze dependency trees to identify transitive dependencies that introduce risk: `syft . -o json | jq '[.artifacts[] | {name, version, type, locations: [.locations[].path]}] | sort_by(.name)'` and cross-reference against vulnerability databases with `grype sbom:sbom.cdx.json` to find vulnerable transitives hidden deep in the dependency graph.

4. Track transitive dependencies explicitly by generating SBOMs from lockfiles: `syft file:package-lock.json`, `syft file:Cargo.lock`, or `syft file:poetry.lock` to capture the full resolved dependency tree including pinned transitive versions that may differ from what manifest files declare.

5. Sign and attest SBOMs using cosign and in-toto attestations: `cosign attest --predicate sbom.cdx.json --type cyclonedx myregistry/myimage:v1.0` creates a signed attestation linking the SBOM to a specific image digest, enabling consumers to verify provenance with `cosign verify-attestation --type cyclonedx myregistry/myimage:v1.0`.

6. Ensure NTIA minimum elements compliance by validating your SBOM contains: supplier name, component name, component version, unique identifier, dependency relationships, author of SBOM, and timestamp; validate with `sbom-tool validate -b sbom.cdx.json -o validation-report.json` or use the CycloneDX CLI `cyclonedx validate --input-file sbom.cdx.json`.

7. Create VEX (Vulnerability Exploitability eXchange) documents to communicate vulnerability status: generate with `vexctl create --product myapp --vuln CVE-2024-XXXX --status not_affected --justification component_not_present > vex.json` to declare that a flagged CVE does not actually affect your product, reducing noise in downstream consumers' scans.

8. Share SBOMs with customers and partners through standardized distribution channels: publish SBOMs alongside releases in OCI registries with `oras push myregistry/myimage:v1.0-sbom sbom.cdx.json:application/vnd.cyclonedx+json`, attach to GitHub releases via `gh release upload v1.0 sbom.cdx.json`, or serve from a dedicated SBOM API endpoint.

9. Perform license compliance analysis from SBOMs by extracting license data: `jq '[.components[] | {name, version, licenses: [.licenses[].license.id]}] | group_by(.licenses) | map({license: .[0].licenses, count: length})' sbom.cdx.json` to identify copyleft (GPL, AGPL) vs permissive (MIT, Apache-2.0) licenses and flag policy violations.

10. Meet EO 14028 (Executive Order on Cybersecurity) requirements by generating SBOMs for all software delivered to federal agencies: produce SBOMs in both CycloneDX and SPDX formats, include all NTIA minimum elements, sign with cryptographic attestation, and deliver with each software release or update per NIST SP 800-218 guidelines.

11. Integrate SBOM generation into CI/CD pipelines by adding a build step: `syft . -o cyclonedx-json > sbom.cdx.json && grype sbom:sbom.cdx.json --fail-on high` generates the SBOM and immediately scans it for high-severity vulnerabilities, failing the build if critical issues exist in declared dependencies.

12. Maintain SBOM freshness by regenerating on every build and storing historical versions: archive SBOMs with timestamps in a dedicated repository or artifact store, diff successive SBOMs with `cyclonedx diff sbom-v1.cdx.json sbom-v2.cdx.json` to detect added, removed, or updated components, and trigger re-scanning when dependency changes introduce new risk.
