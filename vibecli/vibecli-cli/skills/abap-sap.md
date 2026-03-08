---
triggers: ["ABAP", "SAP ABAP", "SAP development", "SAP HANA ABAP", "ABAP OO", "ABAP CDS", "SAP Fiori", "ALV report", "BAPI", "RFC"]
tools_allowed: ["read_file", "write_file", "bash"]
category: erp
---

# ABAP (SAP)

When writing ABAP code for SAP systems:

1. Use ABAP Objects (OO) over procedural ABAP: `CLASS zcl_order_processor DEFINITION. PUBLIC SECTION. METHODS: process IMPORTING iv_order_id TYPE vbeln. ENDCLASS.` — modern SAP development uses clean ABAP principles with OO design.
2. Use ABAP CDS views for data modeling: `@AbapCatalog.sqlViewName: 'ZSALES_V' define view ZI_SalesOrder as select from vbak { key vbeln, erdat, netwr }` — CDS views push processing to HANA database layer; use associations for join-on-demand.
3. Use the new ABAP syntax (7.40+): inline declarations `DATA(lv_name) = 'value'`, string templates `|Order { lv_id } created|`, constructor expressions `VALUE #( ( 1 ) ( 2 ) ( 3 ) )`, table expressions `lt_orders[ vbeln = lv_id ]`.
4. For ALV reports: use `CL_SALV_TABLE` instead of legacy function modules — `cl_salv_table=>factory( IMPORTING r_salv_table = lo_alv CHANGING t_table = lt_data ). lo_alv->display( ).` — supports sorting, filtering, and export built-in.
5. Use RAP (RESTful ABAP Programming) for Fiori apps: define business objects with CDS views, behavior definitions, and behavior implementations — RAP supports managed (framework-handled CRUD) and unmanaged scenarios.
6. Handle database access: use `SELECT FROM ztable FIELDS col1, col2 WHERE key = @lv_key INTO TABLE @DATA(lt_result).` — always use `@` for host variables; use `UP TO n ROWS` to limit results; avoid `SELECT *`.
7. Use internal tables efficiently: `LOOP AT lt_orders ASSIGNING FIELD-SYMBOL(<ls_order>) WHERE status = 'OPEN'.` — field symbols avoid data copying; use `READ TABLE ... BINARY SEARCH` on sorted tables; use hashed tables for key lookups.
8. Error handling: use `TRY. ... CATCH cx_root INTO DATA(lx_error). MESSAGE lx_error->get_text( ) TYPE 'E'. ENDTRY.` — define custom exceptions inheriting from `CX_STATIC_CHECK` or `CX_DYNAMIC_CHECK`.
9. For integrations: use BAPIs (`BAPI_SALESORDER_CREATEFROMDAT2`) for standard business operations; RFC for system-to-system calls; OData services for Fiori/external consumers; IDocs for EDI/batch.
10. Use ABAP Unit for testing: `CLASS ltcl_test DEFINITION FOR TESTING RISK LEVEL HARMLESS DURATION SHORT. METHODS test_calc FOR TESTING. ENDCLASS.` — use `cl_abap_unit_assert=>assert_equals( exp = 4 act = lo_calc->add( 2, 2 ) )`.
11. Performance: use `FOR ALL ENTRIES IN` instead of nested selects; avoid `SELECT` in loops; use buffered tables; check runtime with transaction `SAT` (ABAP runtime analysis) or SQL trace `ST05`.
12. Follow clean ABAP principles: single responsibility, no magic numbers, meaningful names, short methods (<20 statements), no obsolete statements (`MOVE` → `=`, `FORM/ENDFORM` → methods, `WRITE` → ALV/Fiori).
