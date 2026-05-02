"""Seed a toy preference dataset into <workspace>/.vibecli/workspace.db.

The DPO algorithm reads `rl_preferences` rows where chosen ∈ {a, b}.
This script writes 6 hand-crafted preferences whose ordering is obvious
(polite vs. rude continuations) — DPO accuracy should hit 1.0 quickly.

Usage:
    python seed_preferences.py <workspace_path> <suite_id>
"""

from __future__ import annotations

import sqlite3
import sys
import time
from pathlib import Path
from uuid import uuid4

PREFS = [
    (
        "Q: How do I reset my password?\nA:",
        " You can reset it from the account settings page.",
        " Figure it out yourself.",
        "a",
    ),
    (
        "Q: What is 2 + 2?\nA:",
        " 4.",
        " Probably some number, who cares.",
        "a",
    ),
    (
        "Q: Can you help me with my homework?\nA:",
        " Sure, what subject is it?",
        " No, do it alone.",
        "a",
    ),
    (
        "Q: Is the customer always right?\nA:",
        " Not always, but their concerns deserve careful consideration.",
        " They are usually wrong and annoying.",
        "a",
    ),
    (
        "Q: Recommend a beginner-friendly programming language.\nA:",
        " Python is widely recommended for its readable syntax.",
        " Just pick whatever, doesn't matter.",
        "a",
    ),
    (
        "Q: What should I do if I feel overwhelmed?\nA:",
        " Take a short break and prioritize one task at a time.",
        " Just push through, breaks are weakness.",
        "a",
    ),
]


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    workspace_path = Path(sys.argv[1])
    suite_id = sys.argv[2]

    db_dir = workspace_path / ".vibecli"
    db_dir.mkdir(parents=True, exist_ok=True)
    db_path = db_dir / "workspace.db"

    conn = sqlite3.connect(str(db_path))
    try:
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS rl_preferences (
                pref_id        TEXT PRIMARY KEY,
                suite_id       TEXT,
                prompt         TEXT NOT NULL,
                completion_a   TEXT NOT NULL,
                completion_b   TEXT NOT NULL,
                chosen         TEXT,
                rationale      TEXT,
                reviewer       TEXT,
                created_at     INTEGER NOT NULL,
                judged_at      INTEGER
            )
            """
        )
        conn.execute(
            "DELETE FROM rl_preferences WHERE suite_id = ?",
            (suite_id,),
        )
        now = int(time.time())
        for prompt, a, b, chosen in PREFS:
            conn.execute(
                """
                INSERT INTO rl_preferences
                (pref_id, suite_id, prompt, completion_a, completion_b,
                 chosen, rationale, reviewer, created_at, judged_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    str(uuid4()),
                    suite_id,
                    prompt,
                    a,
                    b,
                    chosen,
                    "toy demo preference",
                    "example-script",
                    now,
                    now,
                ),
            )
        conn.commit()
        count = conn.execute(
            "SELECT COUNT(*) FROM rl_preferences WHERE suite_id = ?",
            (suite_id,),
        ).fetchone()[0]
        print(f"seeded {count} preferences into {db_path} (suite={suite_id})")
    finally:
        conn.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
