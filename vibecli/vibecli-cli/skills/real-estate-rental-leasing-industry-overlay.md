---
triggers: ["real estate, rental, and leasing", "real estate", "rental", "leasing"]
tools_allowed: ["read_file", "write_file"]
category: industry
---

# Real Estate, Rental, and Leasing

> **Industry ID:** IND-12 · **Accountable human owner:** property principal, licensed broker/appraiser, asset manager, or rental operations leader

This overlay composes OS 10, 11, 12, 16, 17, 19, and 20. Read the *Reference — Real-Estate and Rental Asset Models* section below for subsectors, regulated decisions, asset records, and exception scenarios.

## Mission

Match people and organizations to land, buildings, equipment, vehicles, and other usable rights while preserving truthful representation, fair access, safe condition, lawful contracting, reliable operations, and accountable stewardship across the asset lifecycle.

## Establishment archetypes

- Residential/commercial brokerage, appraisal, title, escrow, or closing service.
- Property, community, facility, or association manager.
- Developer, owner/operator, REIT, fund, or asset-management platform.
- Equipment, vehicle, tool, consumer-goods, or specialty rental fleet.
- Franchise, patent, trademark, copyright, or other intangible-right lessor.

## Core Jobs To Be Done

1. When acquiring or creating an asset, verify rights, condition, constraints, market, financing, development case, and lifecycle obligations.
2. When marketing, create accurate listings, disclosures, availability, pricing, media, and channel records without discrimination or manipulation.
3. When qualifying counterparties, verify identity, authority, need, eligibility, credit, insurance, sanctions where relevant, and fair-treatment controls.
4. When valuing or pricing, assemble comparable, income, cost, utilization, condition, and market evidence; disclose assumptions and uncertainty.
5. When contracting/closing, coordinate offers, leases, title, escrow, financing, inspections, disclosures, signatures, funds, possession, and recordation.
6. When occupancy or rental begins, onboard, inspect, document condition, provision access, collect deposits/rent, and explain responsibilities and redress.
7. When assets operate, schedule maintenance, utilities, safety, compliance, cleaning, reservations, dispatch, return, turnover, and vendor work.
8. When conditions change, manage renewals, escalations, vacancies, delinquency, modifications, damage, claims, disputes, and reasonable accommodation.
9. When optimizing portfolios/fleets, forecast demand, utilization, NOI, capex, residual value, risk, and disposition while protecting people and communities.
10. When ending use, inspect, settle fairly, revoke access, return deposits/collateral, transfer/record, refurbish, redeploy, sell, or decommission.

## AI and physical-AI allocation

- AI may perform listing enrichment, comparable retrieval, lease abstraction, document completeness, scheduling, maintenance triage, reservation/dispatch support, payment matching, utilization forecasting, inspection evidence organization, and routine communication.
- AI may recommend valuation ranges, pricing, tenant/renter qualification, renewal, maintenance priority, capex, collections, and disposition, but cannot make protected-class or rights-impacting decisions.
- Drones, inspection rovers, floor/roof scanners, autonomous yard movers, cleaning robots, key/access systems, and equipment telemetry may inspect, document, position, clean, or monitor bounded assets.
- Physical entry, eviction, repossession, unsafe inspection, lock changes, occupied-space monitoring, invasive testing, and high-consequence maintenance require direct human authority and safety controls.

## Human accountability boundary

Humans must own licensed brokerage/appraisal and professional opinions; disclosures and representations; fair-housing/equal-access decisions; final screening and adverse action; lending/credit decisions; contract and closing authority; escrow/client funds; rent increases and material lease terms; accommodation; eviction, repossession, lockout, or service restriction; safety/occupancy release; insurance claims; development entitlement; and regulator, court, owner, tenant, or community communication in disputes.

## Systems of record

Property/asset and unit master; GIS/title/parcel; CRM/listing channels; appraisal/comparable workfile; lease/contract management; applicant/renter screening; property/facilities/maintenance management; reservation/fleet/telematics; access control; utility and environmental data; accounting, escrow, deposits, rent, billing, collections; vendor/insurance; inspection/media/evidence; complaints, accommodation, incidents, claims, and legal holds.

## Controls

- Separate listing, valuation, approval, funds custody, maintenance verification, refunds/deposits, and write-offs as risk requires.
- Test models for geographic/protected-class proxies, unequal error, steering, price discrimination, and inaccessible appeal.
- Bind every listing, lease, inspection, charge, access event, and maintenance action to an authoritative asset/unit and effective date.
- Prevent autonomous access or surveillance beyond consent, purpose, place, and retention policy.
- Verify AI-generated comparables, abstracts, condition findings, charges, and notices before consequential use.
- Preserve title, disclosure, consent, inspection, funds, condition, maintenance, adverse-action, and redress evidence.

## Metrics

Occupancy/utilization, time to lease/rent, conversion, renewal, NOI/margin, rent/fee collection, DSO, maintenance response/first-time fix, downtime, turnover time, asset availability, residual value, disclosure accuracy, deposit disputes, complaints, fair-treatment outcomes, safety incidents, energy/water intensity, inspection defect escape, and automation correction/escalation rates.

## Failure modes and keep-warm

Discriminatory proxy scoring; fabricated or stale comparables; hidden defects; inaccurate lease abstracts; unauthorized surveillance/access; phantom availability; unsafe asset dispatch; deposit/fee abuse; automated eviction pressure; maintenance optimization that defers safety; title/entity mismatch; and sensor evidence accepted without calibration or context.

Preserve human appraisal, inspection, leasing, tenant communication, dispatch, maintenance diagnosis, key/access recovery, and emergency operation through sampled manual work and simulations.

## Operating procedure

1. Classify asset/right, jurisdiction, establishment model, lifecycle stage, occupancy, and protected-party impacts.
2. Name licensed, funds, safety, fair-treatment, asset, maintenance, and privacy owners.
3. Establish authoritative asset, party, contract, condition, access, money, and evidence records.
4. Map decision rights and appeal for listing, valuation, screening, pricing, access, maintenance, and termination.
5. Allocate reversible support to AI and bounded physical tasks to inspected machines.
6. Test discrimination, misrepresentation, unsafe condition, fraud, delinquency, disaster, cyber/access failure, and contested-evidence scenarios.
7. Deploy with human release gates, notices, appeal, incident response, and manual fallback.

## Reference — Real-Estate and Rental Asset Models

### Subsector modifiers

- **Residential:** fair housing, habitability, deposits, accommodation, privacy, eviction safeguards, vulnerable occupants.
- **Commercial:** tenant improvements, operating expenses/CAM, options, covenants, estoppel, insurance, business continuity.
- **Appraisal/title/escrow:** independence, workfile, comparable provenance, defects/encumbrances, funds custody, identity/wire fraud.
- **Development/investment:** entitlement, community impact, construction, leasing, capital stack, environmental liability, portfolio concentration.
- **Equipment/vehicle rental:** availability, reservations, inspection, training, damage, maintenance, telematics, retrieval, residual value.
- **IP/franchise leasing:** ownership, territory, quality control, royalties, audit rights, brand standards, infringement, termination.

### Critical exceptions

Identity/title mismatch; undisclosed beneficial owner; protected-class proxy; accommodation request; adverse action; valuation conflict; missing disclosure; unsafe/habitable condition; occupied-space access; lost key/credential; deposit dispute; unauthorized fee; delinquency; disaster displacement; suspected wire fraud; telematics/privacy complaint; damaged or recalled equipment; maintenance override; contested inspection; holdover; eviction/recovery; environmental contamination.

### Evidence model

Retain effective-dated asset/unit identity, ownership/authority, listing version, disclosures, comparable set, model/version, application inputs, screening reasons, consent, contract/lease, funds ledger, condition media, sensor calibration, work orders, access events, notices, communications, decisions, appeal, and disposition.

### Curated role composition

Property listing/valuation; lease abstraction; lease review; tenant screening/onboarding; facilities maintenance; code compliance; energy modeling; credit memo; reconciliation; KYC/AML; pricing; equipment rental fleet pricing; customer support; inspection drone and fleet safety roles.
