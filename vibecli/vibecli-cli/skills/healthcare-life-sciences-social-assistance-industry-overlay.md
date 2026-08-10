---
triggers: ["healthcare, life sciences, and social assistance", "healthcare", "life sciences", "social assistance"]
tools_allowed: ["read_file", "write_file"]
category: industry
---

# Healthcare, Life Sciences, and Social Assistance

> **Industry ID:** IND-18 · **Accountable human owner:** licensed clinician, care/service leader, qualified laboratory/manufacturing authority, ethics owner, or regulated-product executive

This overlay composes OS 05, 12, 13, 15, 16, 20, 21, and 23. Read the *Reference — Care and Life-Science Models* section below.

## Mission

Prevent harm, improve health and daily functioning, produce safe therapies and evidence, and support people with dignity, consent, equity, continuity, and accountable professional judgment.

## Core Jobs To Be Done

1. Design accessible services/products/research, capacity, eligibility, pathways, safety/quality systems, workforce, supply, finance, and emergency continuity.
2. Verify identity, consent/authority, need, history, eligibility/coverage, safeguarding, language/accessibility, and urgent risk.
3. Assess, diagnose/support diagnosis, plan care/service/research, explain options/uncertainty, and obtain informed consent.
4. Deliver treatment, medication, procedure, monitoring, rehabilitation, daily support, referral, social assistance, or emergency response.
5. Order/collect/transport/process samples, images, data, and supplies with identity, condition, calibration, custody, and result controls.
6. Discover/develop/test/manufacture/release/distribute drugs, biologics, diagnostics, and devices under validated quality and safety systems.
7. Coordinate teams, beds/appointments, home visits, caregivers, pharmacies, labs, payers, community services, and transitions.
8. Document, code, authorize, bill/pay, reconcile, communicate, protect privacy, and provide complaint/appeal/redress.
9. Detect deterioration, outbreak, adverse event, interaction, abuse/neglect, product defect, trial deviation, fraud, and inequitable outcomes.
10. Respond, report, recall/contain, investigate, support affected people, restore service, and improve clinical/product/social controls.

## AI and physical-AI allocation

AI may support documentation, imaging/lab triage, diagnostic options, interaction checks, care gaps, trial matching, surveillance, authorization, scheduling, logistics, literature, experiment planning, and data analysis. Robots/vehicles/drones may move supplies, pharmacy items, samples, linen, waste, or equipment; lab assistants may handle validated low-risk steps; care robots may fetch, remind, or monitor with consent.

## Human accountability boundary

Licensed humans must own diagnosis, treatment, prescribing, procedure, clinical triage, capacity/consent, restraint, safeguarding, scarce-resource allocation, discharge, death determination, research ethics, protocol approval/deviation, laboratory result release, product batch/release, adverse-event causality, recall, benefit/coverage denial and appeal, intimate care, and communications to patients/families, regulators, ethics bodies, insurers, or the public.

## Systems, controls, and metrics

EHR/care/social-service case; scheduling/bed/workforce; LIS/PACS/pharmacy; medication/device; trial/EDC/eTMF; LIMS/QMS/manufacturing/batch; supply/cold chain/serialization; payer/authorization/claims; consent/identity; safeguarding/incident; public health; robot/logistics telemetry.

Enforce patient/participant/product/sample identity, consent, order/protocol, professional scope, dose/range, allergies/interactions, specimen custody, calibrated/validated methods, segregation/release, privacy purpose, billing integrity, and appeal. Learned systems cannot silently modify records, orders, protocols, or release status.

Measure mortality/morbidity/function, safety/adverse events, diagnostic/result quality, medication errors, access/wait, continuity/readmission, patient/caregiver experience, equity, safeguarding, trial integrity, product yield/deviation/recall, claim/appeal, workforce safety/burnout, logistics temperature/custody, robot intervention, and manual readiness.

## Failure modes and operating procedure

Watch for identity mismatch, automation bias, hallucinated evidence, missed deterioration, unequal performance, consent erosion, privacy leakage, alert fatigue, wrong sample/medication, protocol drift, batch contamination, denial optimization, dehumanized care, unsafe robot proximity, and supply-chain counterfeit or temperature loss.

1. Classify care/product/social-service/research setting, patient/participant vulnerability, professional scope, product risk, and consequence tier.
2. Name clinical/care, ethics, safeguarding, laboratory/product quality, privacy, payer, logistics, safety, and incident owners.
3. Establish authoritative identity, consent, order/protocol, observation/result, medication/product, custody, decision, release, billing, and incident records.
4. Test deterioration, wrong identity/sample/drug, adverse event, safeguarding, outbreak, contamination, cold-chain loss, cyber outage, robot failure, and manual recovery.
5. Deploy through silent validation and supervised assistance with licensed release, consent, appeal, emergency stop, incident reporting, and humane fallback.

## Reference — Care and Life-Science Models

- Acute/outpatient/home/residential/behavioral: clinical authority, consent/capacity, medication, deterioration, safeguarding, transitions.
- Diagnostics/labs/imaging/pharmacy: order, identity, specimen, method, calibration, result release, interaction and counseling.
- Biotech/pharma/device/CRO/CDMO: protocol/design controls, GxP, validation, batch, deviations, pharmacovigilance, recall.
- Payer/plan: enrollment, network, authorization, claims, medical necessity, adverse decision, appeal, fraud.
- Childcare/disability/community: safeguarding, dignity, accommodation, family/guardian authority, continuity, least restrictive support.

Critical exceptions: patient/sample/product mismatch, incapacity/no consent, suicide/violence/abuse risk, rapid deterioration, allergy/interaction, contaminated batch, protocol deviation, unblinding, adverse event, product counterfeit/recall, coverage denial, privacy breach, outbreak, supply shortage, and robot contact or delivery error.
