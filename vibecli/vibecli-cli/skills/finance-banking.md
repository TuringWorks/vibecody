---
triggers: ["banking", "core banking", "payment processing", "wire transfer", "ACH", "SWIFT", "KYC", "AML", "loan origination", "credit scoring", "deposit", "open banking"]
tools_allowed: ["read_file", "write_file", "bash"]
category: finance
---

# Finance - Banking & Payments

When working with banking and payment processing systems:

1. Architect core banking systems around an immutable ledger with double-entry accounting at its foundation. Every account balance is derived from the sum of its ledger entries, never stored as a mutable running total. Separate the ledger of record (source of truth) from materialized balance views (optimized for queries), and rebuild views from the ledger whenever discrepancies are detected.

2. Implement payment rails with protocol-specific message formatting and validation: ACH (NACHA file format with batch headers, detail entries, and control totals), SWIFT (MT and MX/ISO 20022 messages with BIC routing), Fedwire (immediate gross settlement with specific formatting), and SEPA (pain.001 for credit transfers, pain.008 for direct debits). Validate every field against the rail's specification before submission.

3. Ensure idempotent transaction processing by assigning a unique idempotency key to every payment request at the point of origin. Store the key with the transaction outcome so that retries (from network failures, timeouts, or duplicate submissions) return the original result rather than processing the payment again. Implement this at the API gateway level and enforce it at the ledger level as a unique constraint.

4. Build KYC (Know Your Customer) screening pipelines as a multi-stage workflow: identity verification (document OCR + liveness check), address verification, PEP (Politically Exposed Person) screening against government lists, sanctions screening against OFAC/EU/UN lists, and adverse media screening. Score each check, apply risk-based thresholds (simplified, standard, enhanced due diligence), and store all evidence for regulatory examination.

5. Implement AML (Anti-Money Laundering) monitoring with both rule-based and model-based detection: rules for structuring (multiple deposits just below reporting thresholds), rapid movement of funds, geographic risk (high-risk jurisdictions), and unusual patterns; ML models trained on labeled SAR (Suspicious Activity Report) data for anomaly detection. Generate alerts with full transaction context and route them to compliance analysts through a case management workflow.

6. Build credit scoring models using both traditional features (credit bureau data, income, employment, debt-to-income ratio) and alternative data (bank transaction history, rental payments). Implement scorecards as logistic regression for interpretability, or gradient-boosted trees for performance, but always produce adverse action reason codes as required by ECOA/Reg B. Validate models for disparate impact across protected classes.

7. Design loan origination workflows as a state machine: application intake, identity verification, credit pull, income verification, automated underwriting decision, pricing, document generation, e-signature, funding, and boarding to the servicing system. Each state transition should be auditable with timestamps, decision rationale, and the ability to handle exceptions (manual review, counter-offers, document re-requests).

8. Implement interest calculation engines that support multiple methods: simple interest (principal times rate times time), compound interest (daily, monthly, quarterly compounding), actual/360, actual/365, 30/360 day-count conventions, and amortization schedules (fixed payment, interest-only, balloon). Use arbitrary-precision decimal arithmetic (never floating-point) to ensure cent-accurate calculations across all loan products.

9. Calculate regulatory capital requirements under Basel III: Common Equity Tier 1 (CET1), Additional Tier 1, and Tier 2 capital ratios. Implement risk-weighted asset (RWA) calculations using the standardized approach (risk weights by asset class) or internal ratings-based approach (PD, LGD, EAD models). Monitor capital buffers (conservation, countercyclical, systemic) and generate capital adequacy reports.

10. Build open banking APIs compliant with PSD2 and regional standards: Account Information Service (AIS) endpoints for balance and transaction access, Payment Initiation Service (PIS) for third-party-initiated payments, and Confirmation of Funds (CoF) for balance checks. Implement strong customer authentication (SCA) with multi-factor flows, consent management with granular scope and expiry, and TPP (Third Party Provider) certificate validation.

11. Implement real-time fraud detection as an inline decision engine in the payment authorization path. Score every transaction within milliseconds using features like device fingerprint, geolocation, velocity (transaction frequency and cumulative amount), merchant category risk, and behavioral biometrics. Apply step-up authentication (OTP, biometric) for medium-risk scores and hard-decline for high-risk, with feedback loops to continuously retrain models.

12. Build account reconciliation processes that run daily: reconcile the bank's internal ledger against nostro/vostro accounts at correspondent banks, payment processor settlement files, card network settlement files, and central bank reserve account statements. Automatically match items using reference numbers and amounts, flag breaks with aging, and escalate unresolved items based on materiality thresholds.

13. Design ledger immutability patterns using append-only tables with cryptographic chaining: each entry includes a hash of the previous entry, creating a tamper-evident chain. Implement point-in-time queries so auditors can reconstruct the exact state of any account at any historical moment. Prohibit physical deletion of ledger records; use logical reversal entries for corrections.

14. Handle multi-currency accounts by maintaining separate ledger entries per currency, with real-time FX rate lookups for cross-currency transfers. Apply the bank's spread to the mid-market rate, disclose the effective rate to the customer before confirmation, and settle the FX component through the bank's treasury desk. Post realized FX gains/losses to the appropriate GL accounts.

15. Implement deposit products (checking, savings, CD, money market) with configurable interest accrual rules, tiered rate structures, maturity handling (auto-renewal, grace periods), early withdrawal penalties, and regulatory hold policies (Reg CC for check holds). Calculate and report interest for 1099-INT generation, and enforce FDIC/NCUA insurance limit monitoring across related accounts.
