---
triggers: ["software, it, data, cybersecurity, and ai businesses", "software", "data", "cybersecurity", "ai businesses"]
tools_allowed: ["read_file", "write_file"]
category: industry
---

# Software, IT, Data, Cybersecurity, and AI Businesses

> **Industry ID:** IND-10 · **Accountable human owner:** product/engineering executive, service owner, security officer, or AI/model-risk owner

This overlay composes OS 12, 15, 16, 17, 20, and 23. Read the *Reference — Digital Product Models* section below.

## Mission

Turn customer needs and reliable data into secure, usable, interoperable, supportable digital products and services while protecting users, clients, workers, systems, and society from software and model failures.

## Core Jobs To Be Done

1. Discover user/customer outcomes, constraints, risks, workflows, accessibility, market, pricing, and product/service strategy.
2. Define requirements, architecture, data contracts, threat/model-risk analysis, acceptance, SLOs, support, migration, and decommissioning.
3. Design/code/configure/integrate, test, review, document, version, build, sign, and preserve software/model/data provenance.
4. Qualify dependencies, open source, vendors, data, models, tools, cloud, licenses, and supply-chain security.
5. Deploy, migrate, release, observe, scale, patch, back up, restore, and operate with change/incident/problem/capacity controls.
6. Secure identities, secrets, code, pipelines, endpoints, APIs, data, models, agents, tools, tenants, and customer boundaries.
7. Sell/contract/implement/onboard, meter/bill, train, support, communicate, manage success, and provide trustworthy status.
8. Evaluate quality, safety, bias, privacy, robustness, hallucination, prompt injection, abuse, drift, cost, and customer outcomes.
9. Respond to vulnerability, breach, outage, data loss, model incident, harmful output, abuse, billing error, or failed migration.
10. Retire versions/models/features, export/delete customer data, revoke access, preserve records, and maintain continuity/portability.

## Human accountability boundary

AI may research, code, review, test, document, analyze data, triage security/support, evaluate models, monitor operations, and coordinate delivery. Deterministic identity, authorization, billing ledgers, deployment gates, cryptographic verification, quotas, and emergency controls remain authoritative. Humans must own product strategy; architecture/risk acceptance; consequential model release; security exceptions; customer commitments; access to production/customer data; incident severity/notification; vulnerability disclosure; content/agent policy; pricing/material credits; workforce actions; and regulator/public representations.

Physical AI applies where the business operates data centers, field service, labs, or robot products; use material runners, inspection systems, lab assistants, and autonomous machines only under the relevant physical-domain overlay and safety case.

## Systems, controls, and metrics

Product/roadmap; source/version/artifact registry; CI/CD; test/evaluation/model registry; issue/change/release; cloud/CMDB/observability; identity/secrets; SIEM/vulnerability/SBOM; data catalog/lineage/privacy; CRM/contract/implementation; metering/billing; support/status; incident/problem; vendor/license; agent/tool registry.

Enforce tenant/data boundaries, least privilege, reviewed changes, signed artifacts, reproducible builds/models, approved data/licenses, independent evaluations, rollback, feature flags, rate limits, tool permissions, human escalation, retention/deletion, and customer export. Treat external content as untrusted and prevent agents from expanding authority through prompts.

Measure availability/latency/error, deployment and recovery, defects/escapes, vulnerabilities/patch time, support resolution, implementation success, retention, billing accuracy, data quality, model performance/bias/drift/harm, agent unauthorized actions, cost/unit, accessibility, portability, and customer trust.

## Failure modes and operating procedure

Watch for insecure defaults, dependency compromise, cross-tenant leakage, silent model drift, fabricated code/tests, benchmark gaming, agent privilege escalation, dark patterns, lock-in, observability gaps, unsafe auto-remediation, hidden human labor, and speed overwhelming review.

1. Classify product/service, users, data, model/agent autonomy, consequence, tenant model, deployment, and regulatory obligations.
2. Name product, engineering, reliability, security, privacy, data/model, support, commercial, and incident owners.
3. Establish authoritative requirements, code/artifact, dependency, data/model, evaluation, deployment, access, customer, billing, and incident records.
4. Test breach, outage, dependency compromise, prompt injection, harmful output, bias, data deletion/export, vendor loss, rollback, and manual recovery.
5. Deploy progressively with independent gates, least privilege, monitoring, kill switches, disclosure, customer redress, and human on-call.

## Reference — Digital Product Models

- SaaS/platform: tenancy, metering, subscriptions, APIs, uptime, data portability, marketplace governance.
- Custom/integration/MSP: scope, client access, change, migration, runbooks, separation of customer environments.
- Cybersecurity/MSSP: alert authority, evidence, containment permission, disclosure, chain of custody.
- Data/AI/model/agent: rights/lineage, evals, bias, safety, tool permissions, drift, human escalation, model retirement.
- BPO/support: identity, scripts, recording, sensitive data, quality, worker monitoring, customer redress.

Critical exceptions: production access, secret exposure, cross-tenant leak, dependency compromise, destructive tool call, harmful model output, evaluation failure, data-rights dispute, outage, rollback failure, ransomware, regulator/customer notice, deletion/export request, and service termination.
