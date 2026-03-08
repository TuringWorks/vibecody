---
triggers: ["PL/SQL", "Oracle PL/SQL", "Oracle database", "Oracle stored procedure", "Oracle package", "Oracle trigger", "DBMS_OUTPUT", "Oracle cursor"]
tools_allowed: ["read_file", "write_file", "bash"]
category: sql
---

# PL/SQL (Oracle)

When writing PL/SQL code for Oracle databases:

1. Use packages to organize related procedures, functions, and types: `CREATE OR REPLACE PACKAGE pkg_orders AS PROCEDURE create_order(p_customer_id NUMBER); FUNCTION get_total(p_order_id NUMBER) RETURN NUMBER; END;` — packages provide encapsulation, overloading, and initialization.
2. Always use bind variables and avoid string concatenation in SQL: `EXECUTE IMMEDIATE 'SELECT * FROM users WHERE id = :1' USING p_id;` — prevents SQL injection and enables cursor sharing for better performance.
3. Handle exceptions explicitly: `EXCEPTION WHEN NO_DATA_FOUND THEN ... WHEN TOO_MANY_ROWS THEN ... WHEN OTHERS THEN DBMS_OUTPUT.PUT_LINE(SQLERRM); RAISE;` — always re-raise in `WHEN OTHERS` unless you have specific recovery logic.
4. Use explicit cursors for multi-row processing: `FOR rec IN (SELECT id, name FROM customers WHERE active = 'Y') LOOP process_customer(rec.id, rec.name); END LOOP;` — cursor FOR loops handle open/fetch/close automatically.
5. Use bulk operations for performance: `FORALL i IN 1..l_ids.COUNT INSERT INTO audit_log VALUES (l_ids(i), SYSDATE);` — `BULK COLLECT` for fetching: `SELECT id BULK COLLECT INTO l_ids FROM large_table;` — reduces context switches between SQL and PL/SQL engines.
6. Use `%TYPE` and `%ROWTYPE` for type anchoring: `l_name customers.name%TYPE; l_row customers%ROWTYPE;` — if the table column type changes, the PL/SQL variable adapts automatically at recompilation.
7. Use autonomous transactions for logging: `PRAGMA AUTONOMOUS_TRANSACTION;` in a logging procedure — allows the log INSERT to commit independently of the calling transaction; essential for error logging that survives rollbacks.
8. Create triggers judiciously: `CREATE TRIGGER trg_audit AFTER INSERT OR UPDATE ON orders FOR EACH ROW BEGIN INSERT INTO audit_log(action, order_id) VALUES(:NEW.status, :NEW.id); END;` — avoid complex logic in triggers; prefer application-level processing.
9. Use collections (associative arrays, nested tables, varrays): `TYPE t_ids IS TABLE OF NUMBER INDEX BY PLS_INTEGER;` — associative arrays for lookup tables; nested tables for `BULK COLLECT`; varrays for fixed-size ordered collections.
10. Profile with `DBMS_PROFILER` or `DBMS_HPROF`: identify slow procedures; use `EXPLAIN PLAN` and `V$SQL_PLAN` for SQL within PL/SQL; use `DBMS_XPLAN.DISPLAY_CURSOR` to see actual execution plans.
11. Use `utPLSQL` for unit testing: `CREATE PROCEDURE test_get_total IS BEGIN ut.expect(pkg_orders.get_total(1001)).to_equal(250.00); END;` — run with `exec ut.run('test_get_total')` — supports before/after hooks and suite organization.
12. Use conditional compilation for environment-specific code: `$IF DBMS_DB_VERSION.VER_LE_12 $THEN ... $ELSE ... $END` — useful for maintaining code across Oracle 12c, 19c, and 23ai with version-specific features.
