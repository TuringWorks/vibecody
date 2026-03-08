---
triggers: ["payment gateway", "Stripe", "PayPal", "payment processing", "PCI DSS", "tokenization", "3D Secure", "payment orchestration", "checkout", "recurring billing"]
tools_allowed: ["read_file", "write_file", "bash"]
category: fintech
---

# Fintech Payments Integration

When working with payment gateway integration and processing:

1. Never handle raw card numbers in your backend; use client-side tokenization (Stripe Elements, Braintree Drop-in) so that PAN data never touches your servers, reducing PCI DSS scope to SAQ-A or SAQ-A-EP.

2. Make every charge request idempotent by generating a unique idempotency key (UUIDv4) on the client and passing it with the API call; this prevents duplicate charges on retries caused by network timeouts or server restarts.

3. Implement 3D Secure 2.0 (3DS2) for all card transactions in SCA-regulated regions; use the payment gateway's built-in 3DS2 flow (e.g., Stripe PaymentIntents with `payment_method_options.card.request_three_d_secure = 'automatic'`) rather than rolling your own challenge flow.

4. Always verify webhook signatures using the PSP-provided signing secret before processing events; reject requests with invalid or expired signatures and return 200 immediately after queuing the event for async processing to avoid timeout-induced retries.

5. Store all monetary amounts as integers in the smallest currency unit (cents, paise, fen) to avoid floating-point rounding errors; use the currency's ISO 4217 exponent to determine the divisor for display.

6. Implement a payment orchestration layer that abstracts multiple PSPs behind a unified interface; route transactions based on currency, region, card BIN, or failure rate to maximize authorization rates and minimize processing costs.

7. For recurring/subscription billing, store payment method tokens (not card details) with the customer record; implement dunning logic with configurable retry schedules (e.g., retry failed charges at 1, 3, 5, 7 days) and send pre-charge notifications to reduce involuntary churn.

8. Handle refunds and disputes programmatically by listening for `charge.dispute.created` and `charge.refunded` webhook events; automatically gather transaction metadata, delivery proof, and customer communication logs to build dispute evidence packages.

9. Support multi-currency by determining the presentment currency from the customer's locale or explicit selection; use the PSP's automatic currency conversion or maintain exchange rate snapshots for reconciliation, and always display the exact amount that will be charged.

10. Integrate fraud scoring before authorizing payments; use a combination of PSP-native fraud tools (Stripe Radar, Braintree fraud protection) and custom rules (velocity checks, geolocation mismatches, device fingerprinting) to flag or block suspicious transactions.

11. Implement Strong Customer Authentication (SCA) compliance for European transactions by defaulting to 3DS2 and correctly applying exemptions (low-value, trusted beneficiary, TRA) only when the acquirer and issuer support them.

12. Log every payment lifecycle event (created, authorized, captured, refunded, disputed) with correlation IDs to an append-only audit trail; ensure PCI DSS Requirement 10 compliance by retaining logs for at least one year with three months immediately accessible.

13. Design checkout flows to handle partial captures and auth-and-capture separately; authorize at checkout time and capture only upon fulfillment to reduce refund volume and improve cash flow management.

14. Implement circuit breakers around PSP API calls so that sustained gateway failures trigger automatic failover to a secondary processor rather than cascading errors to customers.
