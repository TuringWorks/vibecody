---
name: "Subsector Pack Router"
description: "Subsector Pack Router: Use this router after selecting the parent industry overlay. Use when the task involves subsector pack router."
category: agent
triggers: ["subsector pack router"]
tools_allowed: ["read_file", "write_file"]
---

# Subsector Pack Router

Use this router after selecting the parent industry overlay. The `*-subsector-pack` skills are the canonical registry.

## Routing procedure

1. Identify the establishment, regulated activity, jurisdiction, license/authority, work product, physical system, and accountable owner.
2. Select every pack whose triggers match the activity; use multiple packs for cross-domain work.
3. Load the parent industry overlay, selected pack, and only the pack references needed for the request.
4. Apply the strictest human-accountability boundary, evidence requirement, release gate, and safe-stop rule across composed skills.
5. Add jurisdiction-specific law, standards, licenses, SOPs, systems, and records before operational use.
6. Distinguish advisory design from real execution. Never imply authorization, certification, release, filing, prescribing, trading, or machine operation.
7. Test one normal scenario, one ambiguity, one system failure, one malicious/adversarial case, and one emergency/manual-recovery case.

## Composition rules

- Use `customs-brokerage-clearance` with international trade, transportation, wholesale, or manufacturing for border declarations.
- Use `commercial-aviation-operations` with transportation, tourism, trade, and resilience for passenger or cargo aviation.
- Use `pharmacy-dispensing-operations` with healthcare, commerce, and logistics for medication fulfillment and delivery.
- Use `nuclear-facility-operations` with utilities, manufacturing, environment, security, and resilience for nuclear or radiological facilities.
- Use `securities-market-operations` with finance, software/cybersecurity, and professional services for market activity.
- Use `professional-attestation-engagements` with the subject industry and professional services for independent assurance.
- Use `autonomous-farm-deployment` with agriculture and machine-specific skills for field, barn, orchard, or aerial systems.
- Use `autonomous-freight-corridor-deployment` with transportation, trade, public safety, and machine-specific skills for driverless freight.

## Output contract

Return selected skills, assumptions, accountable humans, authoritative records, AI/physical-AI allocation, release gates, exceptions, metrics, evaluation scenarios, and unresolved jurisdictional requirements.

## Reference — Routing Examples

- Imported pharmaceutical cold chain: customs, pharmacy, transportation, healthcare, trade, and cold-chain machine controls.
- Autonomous harvester crossing a public road: autonomous farm, agriculture, road/public-safety rules, and harvester skill.
- Driverless truck carrying regulated medicine through a border: autonomous freight, customs, pharmacy, transportation, and dangerous-goods controls.
- Airline issuing audited sustainability claims: commercial aviation, professional attestation, transportation, environment, and finance.
- Nuclear operator using an inspection robot: nuclear facility, utility, security/cyber, environmental, and inspection-machine skills.
- Broker-dealer using an LLM for surveillance: securities markets, finance, software/cybersecurity, privacy, and professional oversight.

When no pack matches, use the parent overlay and record a candidate only if licensing, evidence, exception handling, or machine ODD requirements materially differ from existing guidance.
