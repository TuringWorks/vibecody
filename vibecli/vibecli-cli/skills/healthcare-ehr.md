---
triggers: ["EHR", "electronic health record", "EMR", "patient record", "clinical workflow", "CDSS", "clinical decision support", "e-prescribing", "patient portal", "health informatics"]
tools_allowed: ["read_file", "write_file", "bash"]
category: healthcare
---

# Healthcare EHR Systems

When working with electronic health record (EHR) systems and clinical software:

1. Model patient data using established standards — adopt FHIR resources (Patient, Encounter, Observation, Condition, MedicationRequest) as your canonical data model, mapping internal schemas to FHIR R4 for interoperability and future-proofing against vendor lock-in.

2. Design clinical workflow engines as finite state machines — model each clinical process (admission, discharge, referral) as explicit states with guard conditions, ensuring every transition is auditable, reversible where clinically safe, and tied to role-based permissions.

3. Integrate CDSS rules using a layered architecture — separate clinical knowledge (rule definitions in CQL or Arden Syntax) from the inference engine and the EHR presentation layer, enabling clinical teams to update rules without code deployments and supporting version-controlled rule sets.

4. Implement e-prescribing with NCPDP SCRIPT standard — use NCPDP SCRIPT 2017071 or later for NewRx, RxRenewal, and CancelRx messages, validate against formulary and drug interaction databases before transmission, and maintain a complete audit trail of prescription lifecycle events.

5. Build patient matching algorithms with probabilistic scoring — combine deterministic matching (SSN, MRN) with probabilistic methods (Jaro-Winkler on names, phonetic encoding, date-of-birth fuzzy matching) to achieve high sensitivity while minimizing duplicate records; always surface potential matches for human review.

6. Generate clinical documents using CDA (Clinical Document Architecture) — produce C-CDA compliant documents (Continuity of Care, Discharge Summary, Progress Notes) with proper section templates, coded entries using SNOMED-CT and LOINC, and human-readable narrative blocks.

7. Enforce comprehensive audit logging for PHI access — log every create, read, update, and delete operation on protected health information with timestamp, user identity, patient ID, data accessed, and reason for access; implement break-the-glass workflows with mandatory justification for emergency overrides.

8. Implement role-based access control with clinical context — define access policies per role (attending physician, nurse, pharmacist, billing admin) scoped to care team assignment, patient consent directives, and minimum necessary standard; enforce at both API and UI layers.

9. Support interoperability through HL7v2, FHIR, and C-CDA — implement HL7v2 ADT/ORM/ORU message parsing for legacy system integration, FHIR REST APIs for modern interop, and C-CDA document exchange for transitions of care; use integration engines (Mirth, HAPI) to mediate between formats.

10. Build medication reconciliation workflows — compare medication lists across care settings (ambulatory, inpatient, pharmacy claims), flag discrepancies (duplicates, interactions, omissions), present a unified reconciliation view to providers, and persist the reconciled list as a MedicationStatement resource.

11. Design CPOE (Computerized Provider Order Entry) with safety checks — validate orders against allergy lists, drug-drug interactions, dose range checks, and duplicate order detection in real-time; present alerts with severity tiers (hard stops vs. soft warnings) and track alert override rates for clinical quality improvement.

12. Secure patient portal authentication with MFA and proxy access — implement multi-factor authentication for patient-facing portals, support authorized representative (proxy) access for minors and dependents, enforce session timeouts, and provide audit trails visible to patients showing who accessed their records.

13. Use LOINC and SNOMED-CT for clinical data coding — map lab results to LOINC codes and clinical findings to SNOMED-CT concepts at the point of data capture, enabling semantic interoperability, clinical decision support rule matching, and standardized quality measure reporting.

14. Design for regulatory compliance from the start — build HIPAA safeguards (encryption at rest AES-256, TLS 1.2+ in transit, BAA-compliant hosting), support Meaningful Use / Promoting Interoperability measures, and implement 21st Century Cures Act information blocking provisions including patient access APIs.

15. Implement clinical data archival and retention policies — define retention periods per document type aligned with state and federal regulations (typically 7-10 years for adults, age of majority plus retention period for minors), support legal hold overrides, and ensure archived data remains queryable for continuity of care.
