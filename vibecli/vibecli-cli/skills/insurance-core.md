---
triggers: ["insurance", "underwriting", "claims processing", "actuarial", "policy administration", "premium calculation", "loss ratio", "risk scoring", "InsurTech", "ACORD", "reinsurance", "catastrophe modeling"]
tools_allowed: ["read_file", "write_file", "bash"]
category: insurance
---

# Insurance Core Systems

When working with insurance policy administration, underwriting, and claims systems:

1. Design policy administration systems (PAS) around a versioned policy lifecycle — quote, bind, issue, endorse, renew, cancel — with each transition producing an immutable policy transaction record and maintaining a full audit trail of every state change.

2. Implement underwriting rules engines using a declarative approach (decision tables or REGO/DMN) rather than hard-coded conditionals, separating business rules from application logic so actuaries and underwriters can update rating factors, eligibility criteria, and coverage limits without code deployments.

3. Model claims processing as an explicit workflow from First Notice of Loss (FNOL) through investigation, adjudication, reserving, and settlement, using a state machine with well-defined transitions, required validations at each gate, and SLA timers that trigger escalation when thresholds are breached.

4. Integrate actuarial models by exposing them behind a versioned API contract — accept policy and exposure data, return loss costs, expected loss ratios, and confidence intervals — so models can be updated independently of the core platform and A/B tested against prior versions in shadow mode.

5. Build premium rating algorithms as composable pipelines: base rate lookup, territory/class factor application, experience modification, schedule credits/debits, minimum premium floor, and tax/fee surcharge layers, with each step logged for regulatory rate filing justification and auditor review.

6. Implement risk scoring models with explainability — every score must produce a ranked list of contributing factors (e.g., SHAP values) so underwriters can justify decisions to regulators and policyholders, and so the model can be monitored for drift and bias over time.

7. Adopt ACORD data standards (XML/JSON schemas) for policy, claims, and party objects to ensure interoperability with brokers, reinsurers, and regulatory reporting systems; map internal domain models to ACORD canonical forms at integration boundaries rather than forcing ACORD structures into core domain logic.

8. Model reinsurance treaties (quota share, excess of loss, surplus, facultative) as first-class entities with their own lifecycle, linking ceded premiums and recoverable amounts back to underlying policies and claims so bordereau reporting and settlement calculations are automated and reconcilable.

9. Integrate catastrophe risk modeling outputs (AAL, PML, TVaR curves from AIR/RMS/CoreLogic) into underwriting and portfolio management workflows, storing model versions and run parameters alongside results so accumulation analyses and regulatory capital calculations are reproducible.

10. Detect claims fraud using a layered approach: rules-based red flags (e.g., claim filed within 60 days of policy inception, multiple claims at same address), network analysis (linking claimants, providers, and adjusters), and ML anomaly scoring — route flagged claims to SIU queues with full evidence packages rather than silently blocking.

11. Ensure regulatory compliance with Solvency II (or RBC in the US) by maintaining auditable capital adequacy calculations — own funds, SCR/MCR ratios, and risk margin — with data lineage from source policy and claims data through aggregation to final regulatory returns, supporting both standard formula and internal model approaches.

12. Automate document extraction for claims intake using OCR and NLP pipelines to parse police reports, medical records, invoices, and repair estimates into structured data, validating extracted fields against policy coverage terms and flagging discrepancies for adjuster review rather than manual data entry.

13. Calculate loss reserves using multiple methods (chain-ladder, Bornhuetter-Ferguson, Cape Cod) and expose results through a reserving dashboard that shows development triangles, method comparisons, and confidence ranges, with full version history so actuaries can track reserve adequacy over time and explain movements to finance.

14. Implement event-driven architecture for policy and claims state changes — publish domain events (PolicyIssued, ClaimOpened, PaymentApproved) to a message bus so downstream systems (billing, reinsurance, reporting, regulatory) react asynchronously without tight coupling, using idempotent consumers and dead-letter queues for reliability.
