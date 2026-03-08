---
triggers: ["COBOL", "mainframe", "CICS", "JCL", "DB2 COBOL", "COBOL modernization", "batch processing COBOL", "copybook", "VSAM"]
tools_allowed: ["read_file", "write_file", "bash"]
category: cobol
---

# COBOL

When writing or modernizing COBOL code:

1. Structure programs with the four divisions: `IDENTIFICATION DIVISION` (program metadata), `ENVIRONMENT DIVISION` (file assignments), `DATA DIVISION` (variables and records), `PROCEDURE DIVISION` (logic) — every COBOL program follows this structure.
2. Use structured programming: `PERFORM paragraph-name`, `PERFORM ... UNTIL`, `EVALUATE ... WHEN ... END-EVALUATE` (switch/case) — avoid `GO TO`; use `PERFORM ... THRU` for paragraph ranges; keep paragraphs small and single-purpose.
3. Define data with PIC clauses: `01 WS-AMOUNT PIC 9(7)V99` (7 digits, 2 decimal), `01 WS-NAME PIC X(30)` (30 chars), `01 WS-FLAG PIC 9 VALUE 0` — `9` for numeric, `X` for alphanumeric, `A` for alphabetic, `V` for implied decimal.
4. Use COPY for reusable data definitions: `COPY CUSTOMER-REC.` includes a copybook — copybooks define shared record layouts for files, DB2 tables, and CICS communication areas; change once, rebuild all programs.
5. Handle packed decimal (COMP-3) for financial calculations: `01 WS-TOTAL PIC S9(9)V99 COMP-3` — COMP-3 stores two digits per byte; use for money to avoid floating-point rounding; `COMPUTE WS-TOTAL = WS-PRICE * WS-QTY`.
6. For CICS online transactions: `EXEC CICS RECEIVE MAP('SCRN01') MAPSET('MAP01') INTO(WS-INPUT) END-EXEC` — use BMS (Basic Mapping Support) for screens; `EXEC CICS RETURN TRANSID('TRN1')` for pseudo-conversational design.
7. For DB2 database access: `EXEC SQL SELECT NAME, BALANCE INTO :WS-NAME, :WS-BALANCE FROM ACCOUNTS WHERE ACCT_NO = :WS-ACCT END-EXEC` — always check `SQLCODE` after each SQL statement; `0` = success, `100` = not found, negative = error.
8. For batch processing: read input files with `READ input-file INTO ws-record AT END SET ws-eof TO TRUE`, process records in a loop, write output — use `SORT` verb for sorting; `FILE STATUS` for error handling on file operations.
9. Use VSAM for indexed files: `SELECT CUSTOMER-FILE ASSIGN TO 'CUSTFILE' ORGANIZATION IS INDEXED ACCESS MODE IS DYNAMIC RECORD KEY IS CUST-ID` — KSDS for keyed access, RRDS for relative, ESDS for sequential.
10. Debug with `DISPLAY` statements: `DISPLAY 'WS-AMOUNT=' WS-AMOUNT` — use CEDF (CICS debugging) for online; use `READY TRACE` for paragraph-level tracing; IBM Debug Tool for interactive debugging on z/OS.
11. Modernization strategies: wrap COBOL programs as web services via CICS Web Services or IBM z/OS Connect; extract business rules into a rules engine; use GnuCOBOL for Linux migration; refactor incrementally rather than big-bang rewrites.
12. Write test cases with COBOL unit testing frameworks (zUnit, COBOL-Check): test individual paragraphs with mock data; validate record layouts against file definitions; test boundary conditions on PIC fields (max values, negative numbers, decimals).
