---
triggers: ["Apache Spark", "PySpark", "Spark SQL", "Spark Streaming", "distributed processing"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data-engineering
---

# Apache Spark Best Practices

When working with Apache Spark:

1. Prefer DataFrames and Datasets over RDDs for most workloads — the Catalyst optimizer and Tungsten execution engine deliver significant performance gains through predicate pushdown, column pruning, and whole-stage code generation that raw RDDs cannot leverage.
2. Design partitioning strategies around data skew and downstream operations — use `repartition()` for balanced shuffles before wide transformations and `coalesce()` for reducing partitions without a full shuffle; aim for 128-256 MB per partition as a starting point.
3. Optimize Spark SQL queries by analyzing physical plans with `explain(true)`, enabling adaptive query execution (`spark.sql.adaptive.enabled=true`), and leveraging partition pruning on date/region columns to minimize data scanned.
4. Use broadcast joins (`broadcast()` hint or `spark.sql.autoBroadcastJoinThreshold`) when one side of a join fits in memory (typically under 10 MB default, tunable up to ~1 GB) to eliminate expensive shuffle joins entirely.
5. Choose caching and persistence levels deliberately — `MEMORY_AND_DISK` for iterative ML pipelines, `MEMORY_ONLY_SER` for memory-constrained clusters with large datasets, and always `unpersist()` when cached data is no longer needed to free executor memory.
6. Prefer Structured Streaming over the legacy Spark Streaming (DStreams) for new projects — it provides exactly-once guarantees, event-time processing with watermarks, and a unified batch/streaming API built on the DataFrame engine.
7. Size clusters based on workload profile — memory-intensive jobs (joins, aggregations) need more executor memory, while embarrassingly parallel tasks benefit from more cores; start with `spark.executor.memory` at 4-8 GB and `spark.executor.cores` at 4-5 per executor.
8. Tune memory allocation by setting `spark.memory.fraction` (default 0.6) and `spark.memory.storageFraction` (default 0.5) based on whether workloads are compute-heavy or cache-heavy; monitor GC overhead and switch to off-heap memory (`spark.memory.offHeap.enabled`) for large shuffles.
9. Minimize shuffle overhead by pre-partitioning data with `repartition()` on join keys, using `reduceByKey` instead of `groupByKey`, and tuning `spark.sql.shuffle.partitions` (default 200) to match cluster parallelism and data volume.
10. Integrate with Delta Lake for ACID transactions, schema evolution, time travel, and Z-ordering on high-cardinality filter columns — use `OPTIMIZE` and `VACUUM` commands regularly to compact small files and reclaim storage.
11. Monitor job performance through the Spark UI — examine stage DAGs for unnecessary shuffles, check the Storage tab for cached partition distribution, review the SQL tab for physical plan details, and watch executor metrics for GC pressure and task skew.
12. Test Spark applications locally using `SparkSession.builder.master("local[*]")` with small representative datasets, validate schema expectations with assertions, and use `spark-testing-base` or direct DataFrame comparisons for unit testing transformations.
