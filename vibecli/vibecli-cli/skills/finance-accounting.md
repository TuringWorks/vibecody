---
triggers: ["accounting", "ledger", "double-entry", "journal entry", "chart of accounts", "GL", "general ledger", "accounts payable", "accounts receivable", "GAAP", "IFRS", "trial balance", "reconciliation"]
tools_allowed: ["read_file", "write_file", "bash"]
category: finance
---

# Finance - Accounting Systems

When working with accounting and bookkeeping systems:

1. Implement double-entry bookkeeping as an invariant: every transaction must produce balanced debits and credits. Enforce this at the database layer with CHECK constraints or triggers that reject unbalanced journal entries, never relying solely on application-level validation.

2. Design the chart of accounts with a hierarchical numbering scheme (e.g., 1xxx Assets, 2xxx Liabilities, 3xxx Equity, 4xxx Revenue, 5xxx Expenses). Use a parent-child tree structure with a `parent_account_id` foreign key so roll-ups and subtotals can be computed recursively without hard-coded aggregation logic.

3. Architect subledgers (AP, AR, inventory, fixed assets) as independent modules that post summarized entries to the general ledger on a scheduled or event-driven basis. Each subledger owns its detail records while the GL only holds control account totals, keeping the GL clean and auditable.

4. Validate journal entries before posting by enforcing business rules: debits equal credits, valid account codes, correct period assignment, required approval workflows for entries above a threshold, and rejection of postings to closed periods. Return structured error messages identifying every violated rule.

5. Automate period-end close with a multi-step pipeline: run preliminary trial balance, execute recurring entries, calculate accruals, post intercompany eliminations, perform automated reconciliations, generate variance reports, and lock the period. Each step should be idempotent so the close can be re-run safely after corrections.

6. Build reconciliation workflows that match transactions across systems (bank statements vs GL, subledger vs GL, intercompany) using deterministic matching rules first (exact amount + date + reference), then fuzzy matching (amount within tolerance, date within window), and surface unmatched items for manual review with clear aging.

7. Encode GAAP and IFRS compliance rules directly in code: revenue recognition timing (ASC 606 / IFRS 15 five-step model), lease accounting (ASC 842 / IFRS 16 right-of-use assets), and depreciation methods. Make the accounting standard configurable per entity so multi-GAAP reporting is supported without code duplication.

8. Maintain a complete audit trail by making all ledger tables append-only. Never update or delete posted journal entries; instead, post reversing entries. Store the user ID, timestamp, source system, approval chain, and originating document reference on every entry for SOX and regulatory compliance.

9. Handle multi-currency by storing every transaction in both the local (functional) currency and the originating currency, along with the exchange rate used. Run currency revaluation processes at period-end to adjust unrealized gains/losses on open balances, posting the differences to designated FX accounts.

10. Clearly separate accrual-basis and cash-basis logic. Default to accrual accounting for GAAP/IFRS compliance (recognize revenue when earned, expenses when incurred), but provide cash-basis views by filtering on payment/receipt dates. Support both reporting bases from the same underlying data without duplicating transactions.

11. Automate financial statement generation (balance sheet, income statement, cash flow statement) by mapping GL accounts to report line items via a configurable report definition layer. Support comparative periods, budget vs actual columns, and percentage-of-revenue analysis without hard-coding account numbers into report templates.

12. Design ERP integration patterns using event-driven architectures: publish journal entry events to a message queue so downstream systems (tax, consolidation, reporting) consume them asynchronously. Use idempotency keys to prevent duplicate postings when retrying failed integrations, and maintain a dead-letter queue for entries that fail validation in the target system.

13. Implement trial balance generation as a real-time or near-real-time query that aggregates all posted journal entries by account, producing opening balance, period activity (debits and credits separately), and closing balance columns. Ensure the sum of all debit closing balances equals the sum of all credit closing balances as a system health check.

14. Build accounts payable workflows with three-way matching (purchase order, goods receipt, invoice) before approving payment. Automate early-payment discount detection, payment scheduling based on terms, and batch payment file generation (ACH, wire, check) with proper approval gates and segregation of duties.

15. Build accounts receivable workflows that automate invoice generation from billing events, apply cash receipts using remittance advice matching, calculate aging buckets (current, 30, 60, 90+ days), trigger dunning letter sequences for overdue balances, and estimate allowance for doubtful accounts using historical loss rates.
