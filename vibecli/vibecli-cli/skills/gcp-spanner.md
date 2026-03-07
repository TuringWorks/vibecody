---
triggers: ["Spanner", "cloud spanner", "gcp spanner", "spanner interleave", "spanner query", "google spanner", "globally distributed database"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP Cloud Spanner

When working with Cloud Spanner:

1. Design primary keys to avoid hotspots; never use monotonically increasing integers as the first key column. Instead, use UUIDv4 (`GENERATE_UUID()`), bit-reversed sequences, or composite keys with a hash prefix to distribute writes evenly across splits.
2. Use interleaved tables (`CREATE TABLE OrderItems ... INTERLEAVE IN PARENT Orders ON DELETE CASCADE`) to co-locate parent and child rows on the same split; this dramatically reduces join latency and is essential for hierarchical data models.
3. Prefer stale reads for read-heavy workloads with `transaction_options=TransactionOptions(read_only=ReadOnly(max_staleness=timedelta(seconds=15)))` to allow reads from any replica, reducing latency and contention versus strong reads.
4. Batch DML operations with `transaction.batch_update([stmt1, stmt2, ...])` to group up to 20 DML statements in a single RPC; this reduces round trips and improves throughput for bulk mutations.
5. Use commit timestamps (`OPTIONS (allow_commit_timestamp=true)`) on timestamp columns with `spanner.commit_timestamp()` as the value to get TrueTime-based ordering; this enables efficient change-tracking queries with `WHERE updated_at > @last_sync`.
6. Right-size instances by monitoring CPU utilization via `spanner.googleapis.com/instance/cpu/utilization`; keep multi-region instances under 45% and single-region under 65% CPU to maintain latency SLOs. Use autoscaler for dynamic workloads.
7. Query execution statistics are available via `SPANNER_SYS.QUERY_STATS_TOP_10MINUTE`; regularly review this table to identify slow queries, missing indexes, and full table scans that waste resources.
8. Create secondary indexes with `STORING` clauses (`CREATE INDEX idx ON Orders(customer_id) STORING (total, status)`) to cover queries and avoid expensive back-joins to the base table.
9. Use the Spanner emulator (`gcloud emulators spanner start`) for local development and CI testing; set `SPANNER_EMULATOR_HOST=localhost:9010` in your environment to redirect client library calls.
10. For multi-region configurations, choose the nearest leader region for write-heavy workloads and use `gcloud spanner instances create --config=nam6` (or equivalent) to balance global read latency against write commit latency.
11. Implement retry logic with the client library's built-in transaction runner which automatically retries aborted transactions; never cache read results across retry attempts as the library replays the entire transaction function.
12. Control costs by using provisioned compute with autoscaling (`--autoscaling-min-nodes=1 --autoscaling-max-nodes=5`) rather than fixed node counts, and archive cold data to BigQuery via scheduled Dataflow exports to keep the Spanner dataset lean.
