---
triggers: ["Transact-SQL", "T-SQL", "SQL Server", "SSMS", "SQL Server stored procedure", "SQL Server performance", "SSIS", "SSRS", "Azure SQL"]
tools_allowed: ["read_file", "write_file", "bash"]
category: sql
---

# Transact-SQL (SQL Server)

When writing T-SQL for Microsoft SQL Server:

1. Use `SET NOCOUNT ON` at the top of stored procedures — prevents "N rows affected" messages that add network overhead; use `BEGIN TRY...BEGIN CATCH` for structured error handling with `ERROR_MESSAGE()`, `ERROR_NUMBER()`, `ERROR_LINE()`.
2. Use parameterized queries exclusively: `EXEC sp_executesql N'SELECT * FROM Users WHERE Id = @id', N'@id INT', @id = @userId;` — prevents SQL injection; enables plan reuse; never concatenate user input into dynamic SQL.
3. Use CTEs and window functions: `WITH ranked AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) AS rn FROM employees) SELECT * FROM ranked WHERE rn <= 3;` — get top N per group without correlated subqueries.
4. Index strategy: clustered index on the primary key (one per table); nonclustered indexes on frequently filtered/joined columns; use `INCLUDE` for covering indexes; filtered indexes for selective queries — check `sys.dm_db_index_usage_stats` for unused indexes.
5. Use `MERGE` for upsert operations: `MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN UPDATE ... WHEN NOT MATCHED THEN INSERT ...;` — atomic insert/update/delete in a single statement.
6. Temp tables vs table variables: use `#temp` tables for large datasets (statistics, indexes, parallel plans); use `@table` variables for small sets (<100 rows) or when you need transaction isolation — temp tables get actual row counts for better plans.
7. Use `JSON` functions (SQL Server 2016+): `SELECT JSON_VALUE(data, '$.name'), JSON_QUERY(data, '$.items') FROM documents;` — `OPENJSON()` to shred JSON into rows; `FOR JSON PATH` to produce JSON output.
8. Monitor with DMVs: `sys.dm_exec_query_stats` for query performance; `sys.dm_exec_requests` for active queries; `sys.dm_os_wait_stats` for bottleneck analysis — use `SET STATISTICS IO ON; SET STATISTICS TIME ON;` for per-query analysis.
9. Use stored procedures for business logic: `CREATE PROCEDURE usp_CreateOrder @CustomerId INT, @Total DECIMAL(10,2) AS BEGIN ... END;` — prefix with `usp_` (user stored procedure); use output parameters and `RETURN` codes for status.
10. Implement proper transaction handling: `BEGIN TRANSACTION; BEGIN TRY ... COMMIT; END TRY BEGIN CATCH ROLLBACK; THROW; END CATCH;` — use `XACT_ABORT ON` to auto-rollback on errors; avoid long-running transactions (they hold locks).
11. For ETL: use SSIS (SQL Server Integration Services) for complex data flows; use `BULK INSERT` or `bcp` for fast flat-file loading; use PolyBase for querying external data (Hadoop, Azure Blob, S3) directly from T-SQL.
12. Use SQL Server 2022+ features: `GENERATE_SERIES()` for number sequences, `GREATEST()`/`LEAST()` for min/max across columns, `STRING_AGG()` for concatenation, `APPROX_COUNT_DISTINCT()` for approximate cardinality on large tables.
