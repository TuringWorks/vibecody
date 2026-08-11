---
name: "Pharmacy Dispensing Operations"
description: "Pharmacy Dispensing Operations: Compose this pack with healthcare, supply-chain, finance, privacy, and jurisdiction-specific pharmacy law. Use when the task involves pharmacy dispensing operations, pharmacy dispensing operations subsector pack."
category: industry
triggers: ["pharmacy dispensing operations", "pharmacy dispensing operations subsector pack"]
tools_allowed: ["read_file", "write_file"]
---

# Pharmacy Dispensing Operations

Compose this pack with healthcare, supply-chain, finance, privacy, and jurisdiction-specific pharmacy law. Treat clinical appropriateness, product integrity, and patient understanding as independent release gates.

## Load references

- Read the *Reference — Jobs and Role Map* section below for pharmacy models, roles, and automation allocation.
- Read the *Reference — Records, Controls, and Metrics* section below for medication records, custody, verification, and KPIs.
- Read the *Reference — Exceptions and Evaluations* section below before deploying clinical AI or physical automation.

## Operating procedure

1. Verify pharmacy authority, prescriber, prescription validity, patient identity, consent, allergies, conditions, medication history, and payer context.
2. Normalize drug, strength, form, route, dose, quantity, directions, duration, refills, indication, and substitution permissions without changing intent.
3. Perform pharmacist-led appropriateness review for interactions, contraindications, duplication, dose, organ function, pregnancy, monitoring, misuse, and adherence risk.
4. Resolve ambiguity with the prescriber and patient; document clarification and never infer a high-consequence correction silently.
5. Select authorized product and lot; control expiry, storage, cold chain, recalls, controlled-substance inventory, and counterfeit risk.
6. Prepare, compound when authorized, label, image/scan, count or measure, and independently verify patient-drug-dose-route-directions.
7. Counsel, obtain required acknowledgement, provide accessible instructions, arrange secure pickup/delivery, and protect privacy.
8. Submit and reconcile claims, prior authorization, copay, inventory, dispensing, delivery, reversals, and controlled-substance records.
9. Manage adverse events, errors, near misses, shortages, partial fills, returns, recalls, diversion, and continuity of therapy.

## AI and physical-AI boundary

Use AI for transcription, structured-data checks, interaction prioritization, refill forecasting, claim support, patient-language drafting, inventory optimization, and safety-signal detection. Use dispensing robots, automated cabinets, conveyors, drones, or delivery robots only with validated identity, lot, custody, temperature, tamper, and failed-delivery controls.

Do not let AI prescribe, alter therapy, perform final clinical verification, override a serious alert, authorize controlled-substance exceptions, substitute outside authority, release a recalled/compromised product, or counsel beyond approved evidence without pharmacist oversight.

## Human accountability boundary

Licensed humans must own prescription validity, clinical appropriateness, clarification, substitution, compounding authorization, final verification, counseling, controlled-substance disposition, error/adverse-event response, emergency supply, recall action, and communication with prescribers, patients, regulators, or law enforcement.

## Deliverables

Produce a dispensing state model, role/license matrix, medication evidence record, verification controls, robot validation plan, exception queue, recall/error playbook, KPI set, and evaluation report.

## Reference — Exceptions and Evaluations

Test:

1. Two patients share name and date-of-birth similarities.
2. Dose conflicts with age, weight, renal function, or indication.
3. Allergy or severe interaction appears after transfer data arrives.
4. Ambiguous directions or unit confusion requires prescriber clarification.
5. Controlled-substance pattern suggests forgery or diversion.
6. Shortage requires partial fill, substitution, or therapy coordination.
7. Cold-chain excursion or recall affects a prepared order.
8. Compounding calculation, ingredient, sterility, or beyond-use discrepancy.
9. Delivery robot cannot authenticate recipient or maintain temperature.
10. Wrong drug reaches a patient and requires immediate response.

Score patient identification, clinical escalation, non-fabrication, pharmacist authority, product custody, communication clarity, timeliness, and learning from near misses.

## Reference — Jobs and Role Map

### Pharmacy models

Cover community, hospital, clinic, long-term care, mail-order, specialty, infusion, compounding, central fill, automated dispensing cabinets, and home delivery.

### Roles

- Pharmacist in charge: owns license, quality system, staffing, security, and regulator interface.
- Dispensing/clinical pharmacist: owns appropriateness, verification, counseling, and escalation.
- Pharmacy technician: performs authorized intake, preparation, inventory, billing, and custody tasks.
- Prescriber and nurse/caregiver: provide valid intent and administration context; do not collapse these roles into pharmacy authority.
- Buyer/inventory and cold-chain lead: owns sourcing, storage, shortage, recall, and product integrity.
- Controlled-substance/compliance lead: owns inventory, suspicious patterns, reporting, and diversion response.
- Delivery operator: owns identity, temperature, tamper, proof, and failed-delivery return.

Use AI for clerical and analytical support. Use automation for storage, picking, counting, packaging, transport, and cabinet control, with pharmacist release and deterministic identity/lot checks.

## Reference — Records, Controls, and Metrics

### Authoritative records

Patient identity and consent; prescription and prescriber; allergy/condition/medication history; clarification; clinical review; product/NDC or equivalent; lot/expiry; cold chain; preparation/compound worksheet; images/scans; final verification; counseling; claim; pickup/delivery; controlled inventory; recall; error/adverse event; and access log.

### Controls

Use positive patient and product identification, independent final verification, barcode/vision cross-checks, tall-man/look-alike separation, controlled access, perpetual inventory where required, temperature alarms, recall blocking, override reasons, and privacy-minimized displays. Separate purchasing, receiving, dispensing, verification, inventory adjustment, and discrepancy review where practical.

### Metrics

Track near misses, intercepted and reached-patient errors, clinical intervention acceptance, serious-alert override, turnaround, abandonment, therapy gaps, claim rejects, inventory accuracy, expiry/waste, shortages, cold-chain excursions, controlled discrepancies, recall completion, counseling, robot exceptions, and patient harm. Never optimize speed alone.
