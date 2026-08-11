---
name: "Python Best Practices"
description: "Python Best Practices: Practical rules — e.g. Use type hints everywhere: def greet(name: str) -> str. Use when the task involves python, pip, pytest, django, flask."
category: python
triggers: ["python", "pip", "pytest", "django", "flask", "fastapi"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
---

# Python Best Practices

1. Use type hints everywhere: `def greet(name: str) -> str:`
2. Use `pathlib.Path` instead of `os.path` for file operations
3. Use f-strings for formatting: `f"Hello {name}"`
4. Use dataclasses or Pydantic for data containers
5. Use `ruff` for linting and formatting (replaces black, isort, flake8)
6. Use virtual environments: `python3 -m venv .venv`
7. Use `pyproject.toml` for project config (PEP 621)
8. Handle exceptions specifically: `except ValueError` not `except Exception`
9. Use `logging` module, not `print()` for production code
10. Use async/await with `asyncio` for I/O-bound operations
