---
triggers: ["e-commerce", "ecommerce", "retail", "shopping cart", "product catalog", "checkout", "POS", "point of sale", "order management", "inventory retail", "pricing engine", "promotions engine"]
tools_allowed: ["read_file", "write_file", "bash"]
category: retail
---

# Retail & E-Commerce Engineering

When working with retail and e-commerce systems:

1. Design product catalogs with a SKU-variant model: a parent product holds shared attributes (brand, description, images), while each variant (size, color, material) gets its own SKU, price, inventory count, and GTIN/UPC. Store variant option axes separately so new dimensions can be added without schema changes.

2. Manage shopping cart state with a hybrid approach: persist cart contents server-side keyed by session or user ID for durability, but cache the current cart in a short-lived client-side store (Redux, Zustand) for instant UI updates. Reconcile on login by merging anonymous and authenticated carts with conflict rules (higher quantity wins, most recent price applies).

3. Optimize checkout flow to minimize abandonment: collapse to a single-page or accordion checkout, pre-validate addresses with USPS/Google Address Validation, default to saved payment methods, show an order summary at every step, and implement idempotency keys on the submit endpoint so retries never create duplicate orders.

4. Orchestrate payments through an abstraction layer that wraps Stripe, Braintree, Adyen, or similar processors behind a unified PaymentGateway interface. Support authorize-then-capture for physical goods, immediate capture for digital, and partial captures for split shipments. Store only tokenized references -- never raw card data.

5. Build the Order Management System (OMS) as a state machine: Created -> PaymentAuthorized -> Fulfilled -> Shipped -> Delivered, with branching states for PartialShipment, ReturnInitiated, and Refunded. Emit domain events on every transition so downstream systems (warehouse, accounting, notifications) react asynchronously via a message bus.

6. Synchronize inventory across channels (web, mobile, POS, marketplace) using an event-sourced ledger. Every stock movement (receipt, sale, transfer, adjustment, reservation) is an immutable event. Derive current available-to-promise (ATP) quantities by projecting the event stream, and publish inventory snapshots to each channel on a cadence that balances freshness against load.

7. Implement pricing and promotion rules engines with a layered evaluation model: base price -> customer-group price -> volume tiers -> coupon/promo discount -> loyalty points. Evaluate rules in a defined precedence order, enforce stacking policies (e.g., max one coupon plus one automatic promotion), and log the full discount breakdown on every cart for audit and analytics.

8. Integrate POS terminals through a local gateway service that bridges the cloud catalog/pricing API with hardware peripherals (barcode scanner, receipt printer, card reader). Design for offline resilience: cache the product catalog and pricing rules locally, queue transactions when connectivity drops, and sync upstream when the connection restores with conflict detection.

9. Power product search with Elasticsearch or Algolia: index products with analyzed fields for full-text search, faceted fields for filtering (category, brand, price range, rating), and boosted fields for relevance tuning. Implement typo tolerance, synonym dictionaries, and searchable tags. Track click-through and conversion rates per query to feed a learning-to-rank model.

10. Build recommendation engines using collaborative filtering (users-who-bought-also-bought) for the product detail page and content-based filtering (attribute similarity) for the catalog browse page. Pre-compute recommendations in batch nightly, but layer a real-time session-based model for "recently viewed" and "frequently bought together" on the cart page.

11. Design the returns and refunds workflow as a separate bounded context: ReturnRequest -> Approved -> ItemReceived -> Inspected -> RefundIssued. Support return reasons taxonomy for analytics, automate return label generation via carrier APIs, handle restocking logic (re-sellable vs. damaged), and issue refunds to the original payment method or store credit based on policy rules.

12. Enable omnichannel fulfillment with a routing engine that evaluates each order line against fulfillment nodes (warehouse, store, drop-ship vendor) based on proximity, stock availability, shipping cost, and SLA. Support ship-from-store, buy-online-pickup-in-store (BOPIS), and curbside pickup as first-class fulfillment methods, each with their own status tracking and notification flows.

13. Implement rate limiting and bot protection on high-value endpoints (add-to-cart, checkout, coupon apply) to prevent inventory hoarding and promo abuse. Use CAPTCHA challenges on suspicious sessions, enforce per-user cart quantity limits, and set time-bound reservation windows on carted inventory so abandoned holds release back to the pool automatically.

14. Track e-commerce analytics events (product viewed, added to cart, checkout started, order completed) using a structured event schema that flows into a data warehouse. Build dashboards for conversion funnel analysis, average order value, cart abandonment rate, and customer lifetime value. Feed these metrics back into pricing, promotion, and recommendation tuning loops.
