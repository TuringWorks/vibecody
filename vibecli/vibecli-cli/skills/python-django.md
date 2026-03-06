---
triggers: ["django", "Django REST", "DRF", "Django model", "Django migration", "Django admin", "Django view"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: python
---

# Python Django

When building Django applications:

1. Follow Django conventions: `models.py`, `views.py`, `urls.py`, `serializers.py`, `admin.py`
2. Use class-based views (CBVs) for CRUD; function-based views for custom logic
3. Always create migrations with `makemigrations` — never edit the DB schema directly
4. Use `select_related()` for FK joins and `prefetch_related()` for M2M — avoid N+1 queries
5. Register models in `admin.py` with `list_display`, `search_fields`, `list_filter`
6. Use Django REST Framework serializers for API validation and response formatting
7. Use `@login_required` or `IsAuthenticated` permission class — never skip auth
8. Use `F()` and `Q()` objects for complex queries — avoid raw SQL when possible
9. Use signals sparingly — prefer explicit method calls for business logic
10. Settings: use `django-environ` for env vars; separate `base.py`, `dev.py`, `prod.py`
11. Use `transaction.atomic()` for operations that must succeed or fail together
12. Test with `TestCase` (DB rollback per test) or `SimpleTestCase` (no DB needed)
