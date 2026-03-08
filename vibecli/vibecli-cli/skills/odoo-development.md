---
triggers: ["Odoo", "odoo", "odoo module", "odoo model", "odoo view", "odoo ORM", "odoo controller", "odoo.sh", "OWL odoo"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: odoo
---

# Odoo ERP Development

When working with Odoo ERP development:

1. Structure modules with the standard layout (`__init__.py`, `__manifest__.py`, `models/`, `views/`, `security/`, `data/`, `controllers/`, `static/`, `wizard/`, `reports/`) and declare all dependencies explicitly in `__manifest__.py` under `depends`.
2. Define models by inheriting `models.Model` for database-backed records, `models.TransientModel` for wizards, and `models.AbstractModel` for mixins; use `_inherit` for extension, `_inherits` for delegation inheritance, and `_name` only when creating new models.
3. Use the ORM API methods (`create`, `write`, `search`, `browse`, `unlink`) instead of raw SQL; leverage `search_count` for existence checks, `read_group` for aggregations, and `with_context`/`sudo()` sparingly with clear justification.
4. Define views in XML with `<record model="ir.ui.view">` using proper `inherit_id` and XPath expressions (`position="before|after|replace|inside|attributes"`) to extend existing views without duplicating the entire template.
5. Build controllers by extending `http.Controller` with `@http.route` decorators; use `type='json'` for AJAX/RPC calls and `type='http'` for page rendering; always set `auth='user'` unless public access is intentional, and validate all input parameters.
6. Create wizards as `TransientModel` classes with an action method that processes `active_ids` from context; define the wizard form view and an `ir.actions.act_window` with `target='new'` to open it as a modal dialog.
7. Define security rules in `security/ir.model.access.csv` with columns `id,name,model_id:id,group_id:id,perm_read,perm_write,perm_create,perm_unlink`; add record rules in `security/rules.xml` using domain filters for row-level access control.
8. Build QWeb reports by defining a `<template>` with `t-foreach`, `t-if`, `t-esc`, and `t-raw` directives; register the report action with `<record model="ir.actions.report">` specifying `report_type="qweb-pdf"` and the correct `model` binding.
9. Schedule automated actions using `ir.cron` XML records with `interval_number`, `interval_type`, and `code` or `model_id`/`method` references; set `doall=False` to skip missed executions and handle exceptions inside the cron method to prevent silent failures.
10. Deploy on Odoo.sh by organizing branches (production, staging, development), using the `requirements.txt` for pip dependencies, running migrations with `--update` on the target module, and monitoring logs in the Odoo.sh dashboard for post-deploy errors.
11. Build frontend components using OWL (Odoo Web Library) with class-based components, `useState`/`useRef` hooks, XML templates with `t-on-click` event handlers, and register them via `registry.category('actions')` or patch existing components for customization.
12. Write `@api.constrains` methods for model-level validation, use `@api.depends` for computed fields with proper dependency declaration, and implement `@api.onchange` for UI-only field updates; always call `super()` when overriding CRUD methods to preserve the inheritance chain.
