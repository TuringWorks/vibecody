---
triggers: ["health insurance", "medical claims", "HL7", "FHIR", "EDI 837", "CPT code", "ICD-10", "prior authorization", "formulary", "pharmacy benefit", "utilization review"]
tools_allowed: ["read_file", "write_file", "bash"]
category: insurance
---

# Health Insurance Systems

When working with health insurance, medical claims, and healthcare interoperability:

1. Integrate with HL7 FHIR R4 APIs by modeling resources (Patient, Coverage, Claim, ExplanationOfBenefit, MedicationRequest) as typed domain objects, using FHIR search parameters for queries, and implementing SMART on FHIR OAuth2 scopes for authorization — treat FHIR as the canonical interoperability layer while keeping internal domain models optimized for your processing needs.

2. Process EDI 837 (claim submission) and 835 (remittance advice) transactions by building robust parsers that handle segment/loop hierarchies (ISA/GS/ST envelopes, 2000A-2400 loops), validate segment counts and control numbers, and map to internal claim objects — always store the raw EDI alongside parsed data for dispute resolution and compliance audits.

3. Validate CPT (Current Procedural Terminology) and ICD-10 (diagnosis) codes at claim intake using the current code year's master files, checking code validity, gender/age appropriateness, and procedure-to-diagnosis consistency — flag claims with retired codes, unbundling violations (NCCI edits), or medically unlikely units for review before adjudication.

4. Automate prior authorization workflows by encoding payer-specific clinical criteria (InterQual, MCG guidelines) as executable rules, integrating with provider EHR systems via FHIR CRD/DTR hooks to collect required clinical data at the point of care, and returning real-time determinations where possible to reduce administrative burden and care delays.

5. Build formulary management systems that maintain a versioned drug database with tier assignments, quantity limits, step therapy requirements, and prior authorization flags — expose formulary data via FHIR MedicationKnowledge resources and integrate with pharmacy point-of-sale systems to enable real-time benefit checks at the pharmacy counter.

6. Implement pharmacy benefit processing (PBM integration) using NCPDP SCRIPT standards for e-prescribing and NCPDP Telecom for real-time pharmacy claims — handle DAW (Dispense as Written) codes, MAC (Maximum Allowable Cost) pricing, copay/coinsurance calculations, and accumulator tracking for deductibles and out-of-pocket maximums across medical and pharmacy benefits.

7. Design utilization review rules engines that evaluate medical necessity, level of care appropriateness, and length of stay against clinical guidelines — implement concurrent review triggers (e.g., inpatient stay exceeding expected days), retrospective review sampling, and appeal workflows with clinical peer-to-peer review scheduling and regulatory turnaround time compliance.

8. Verify member eligibility in real-time using EDI 270/271 (eligibility inquiry/response) transactions, caching active coverage spans and benefit details (copays, deductibles, network status) while respecting retroactive enrollment changes — always re-verify at claim adjudication time since eligibility can change between service date and claim receipt.

9. Automate provider credentialing by integrating with CAQH ProView, NPPES (NPI registry), state license verification APIs, and DEA databases — track credentialing status, re-credentialing cycles, and sanctions/exclusions (OIG/SAM), and propagate provider network status changes to claims adjudication and member directory systems in near real-time.

10. Enforce HIPAA compliance in code by implementing PHI access controls (minimum necessary standard), audit logging for all PHI access with user/purpose/timestamp, encryption at rest (AES-256) and in transit (TLS 1.2+), BAA tracking for all data sharing partners, and automated de-identification (Safe Harbor or Expert Determination) for analytics datasets — never log PHI in application logs or error messages.

11. Generate Explanation of Benefits (EOB) documents by assembling adjudication results — allowed amounts, member responsibility (copay, coinsurance, deductible), provider write-offs, and remark codes — into compliant, readable formats (paper and digital via FHIR ExplanationOfBenefit resource), including appeal rights, balance billing protections (No Surprises Act), and multilingual support where required by state regulation.

12. Integrate clinical data from HIEs (Health Information Exchanges) and provider EHRs using FHIR Bulk Data Access for population health analytics, C-CDA documents for care history, and ADT (Admit/Discharge/Transfer) feeds for care coordination — normalize clinical data into a longitudinal member health record that supports risk adjustment (HCC coding), care gap identification, and quality measure reporting (HEDIS/Stars).

13. Implement risk adjustment and HCC (Hierarchical Condition Category) coding workflows that analyze clinical encounter data to identify and validate diagnosis codes that map to HCC categories, calculate RAF (Risk Adjustment Factor) scores, and support retrospective chart review and prospective coding programs — ensure full audit trails for CMS RADV (Risk Adjustment Data Validation) compliance.

14. Design claims adjudication engines as rule pipelines: eligibility verification, benefit plan application, provider contract terms (fee schedules, case rates, DRG), COB (Coordination of Benefits) with other payers, clinical edits (NCCI, MUE), and payment calculation — make each step independently testable and auditable, with configurable rule precedence and override capabilities for manual adjuster intervention.
