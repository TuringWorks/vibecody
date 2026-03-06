---
triggers: ["GDPR", "HIPAA", "SOC2", "PCI-DSS", "compliance", "data privacy", "regulation", "audit trail"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Compliance & Regulatory

When implementing compliance requirements:

1. **GDPR**: collect minimal data, get explicit consent, support data export and deletion (right to erasure)
2. **Data classification**: identify PII, PHI, financial data — apply protection based on sensitivity
3. **Audit trail**: log who did what, when, from where — immutable, tamper-evident logs
4. **Encryption**: encrypt PII at rest (AES-256) and in transit (TLS 1.2+)
5. **Access control**: RBAC with least privilege — regular access reviews and deprovisioning
6. **Data retention**: define retention periods — auto-delete data after expiry
7. **HIPAA**: encrypt PHI, implement BAA with vendors, access logging, breach notification
8. **SOC 2**: implement controls for security, availability, processing integrity, confidentiality, privacy
9. **PCI-DSS**: never store full card numbers, use tokenization, segment cardholder data environment
10. **Breach response**: have a documented incident response plan — 72-hour GDPR notification
11. **Vendor management**: assess third-party data processors — DPAs, security questionnaires
12. **Documentation**: maintain policies, procedures, risk assessments — evidence for auditors
