#!/usr/bin/env python3
"""Write the engine's schema snapshot from its committed fresh-collection golden (SPEC-498 R16).

The snapshot is the schema a fresh collection of the pinned engine holds, one statement each,
under one pin line, `PRAGMA user_version = N;`, the version the golden records. A migration
plan's source snapshot reads it; the golden stays the record and this file is derived from it:

    python3 tools/parity-oracle/anki/snapshot.py          # write the snapshot
    python3 tools/parity-oracle/anki/snapshot.py --check  # exit 1 when it differs

It reads the golden as data and starts nothing.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
GOLDEN = ROOT / "tools" / "parity-oracle" / "goldens" / "anki.schema.snapshot.json"
SNAPSHOT = HERE / "schema-snapshot.sql"
#: The keys of the fresh collection's one answer.
ANSWER_KEYS = {"col_ver", "schema", "user_version"}


def snapshot_text(golden: dict) -> str:
    """The snapshot of `golden`: two comment lines, the pin line, then each statement."""
    cases = golden.get("cases")
    if not (isinstance(cases, list) and len(cases) == 1):
        raise ValueError("the fresh-collection golden holds one case")
    answer = cases[0].get("output")
    if not (isinstance(answer, dict) and set(answer) == ANSWER_KEYS):
        raise ValueError(f"the fresh collection's answer is not {sorted(ANSWER_KEYS)}")
    version, col_ver, schema = answer["user_version"], answer["col_ver"], answer["schema"]
    for name, value in (("user_version", version), ("col_ver", col_ver)):
        if not isinstance(value, int) or isinstance(value, bool):
            raise ValueError(f"{name} is not an integer")
    if not (isinstance(schema, list) and schema and all(isinstance(s, str) for s in schema)):
        raise ValueError("schema is not a list of statements")
    lines = [
        "-- The schema of a fresh collection of the engine at "
        f"{golden.get('source_commit')}, col.ver {col_ver},",
        "-- derived from tools/parity-oracle/goldens/anki.schema.snapshot.json (SPEC-498).",
        f"PRAGMA user_version = {version};",
        "",
    ]
    for statement in schema:
        lines += [statement.strip().rstrip(";").rstrip() + ";", ""]
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--check", action="store_true", help="exit 1 when the snapshot differs")
    args = parser.parse_args(argv)
    sys.dont_write_bytecode = True
    text = snapshot_text(json.loads(GOLDEN.read_text(encoding="utf-8")))
    if args.check:
        current = SNAPSHOT.read_text(encoding="utf-8") if SNAPSHOT.is_file() else None
        if current != text:
            print(f"{SNAPSHOT.relative_to(ROOT)} differs from its golden", file=sys.stderr)
            return 1
        return 0
    SNAPSHOT.write_text(text, encoding="utf-8")
    print(SNAPSHOT.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    sys.exit(main())
