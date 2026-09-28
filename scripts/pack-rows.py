#!/usr/bin/env python3
"""pack-rows: run every tree row of DeckStreak's vendored packs against a checkout (ADR-004).

    python3 scripts/pack-rows.py [--root DIR] [--pack NAME ...] [--json]

The vendored packs live in `.packs/skills/packs/<pack>/`, each with its own `checks.json`, and
their standard-library probes in `.packs/scripts/` (the methodology probes in `scripts/`, where
their rows name them). This runner reads each pack's rows and runs them exactly as the pack
declares them, substituting `{skills}` and `{root}`. The packs stay box-only, so this runner is how
the vendored rows run in CI and in the local gate, while the runner-built packs run on the
maintainer's box through `scripts/box-packs.sh`; ADR-004 and ADR-056 record the choice.

`.packs/wiring.json` gives each pack a state:

* `enforced`: every row runs, and a blocking row that is RED, VOID or in ERROR fails the gate.
* `pending`: every row runs; a blocking VOID row reads `pending` (its subject is not built yet,
  and `enforced_by` names the issue that builds it), but RED and ERROR still fail the gate: a pack
  is pending only for the subject it has not got, never for a defect in what exists. A pending pack
  whose every blocking row ran and passed has outgrown its state, and is refused as STALE: its
  subject exists and is green, so its state must say `enforced` (SPEC-030 R6).
* `deferred`: no row runs until `enforced_by` lands the subject. Used only where the pack reads
  an absent subject as a finding rather than as VOID.
* `phxd`: the pack's rows are built into phxd and run on the maintainer's box (ADR-004); this
  runner lists them and runs none.

A pack may also name `excluded_rows` (never run here, each with the reason it is not DeckStreak's
subject) and `deferred_rows` (not judged until the named issue). Every excluded row is still
counted and printed, so nothing leaves the report silently. Every deferred row runs in a separate
pass after the others: one that passes is refused as STALE, because its deferral has outlived its
reason, and one that is red, VOID or in error stays deferred and fails nothing (SPEC-030 R7). The
summary line reports that pass's count and time.

A row's exit is 0 green, 1 a finding, 2 a usage error, 3 VOID (nothing examined, never a pass).
An advisory row never fails the gate. The runner exits 0 when no row fails, 1 when one does, 2
when the wiring is malformed (an unknown key or state, a pack it names that is not vendored, or a
vendored pack it does not name), and 3 when it examined nothing.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
SKILLS = REPO / ".packs" / "skills"
WIRING = REPO / ".packs" / "wiring.json"
STATES = ("enforced", "pending", "deferred", "phxd")
PACK_KEYS = {"state", "enforced_by", "excluded_rows", "deferred_rows", "note"}
FAILING = ("RED", "VOID", "ERROR", "STALE")
# The verdicts of a row the main pass ran; a deferred row's pass is counted on its own.
RAN = ("ok", "advisory", "pending", "RED", "VOID", "ERROR")


class WiringError(Exception):
    """The wiring manifest cannot be trusted; the message names why."""


@dataclass
class Row:
    pack: str
    ident: str
    severity: str
    command: list[str]
    timeout: int


def load_wiring() -> dict[str, dict]:
    data = json.loads(WIRING.read_text(encoding="utf-8"))
    if data.get("schema") != "deckstreak.pack-wiring.v1":
        raise WiringError(f"{WIRING}: unknown schema {data.get('schema')!r}")
    packs = data.get("packs", {})
    for name, entry in packs.items():
        unknown = set(entry) - PACK_KEYS
        if unknown:
            raise WiringError(f"{name}: unknown key(s) {sorted(unknown)}")
        state = entry.get("state")
        if state not in STATES:
            raise WiringError(f"{name}: state {state!r} is not one of {STATES}")
        if state in ("pending", "deferred") and not entry.get("enforced_by"):
            raise WiringError(f"{name}: a {state} pack names the issue that enforces it")
        for key in ("excluded_rows", "deferred_rows"):
            for row, why in entry.get(key, {}).items():
                if not str(why).strip():
                    raise WiringError(f"{name}:{row}: {key} needs a reason or an issue")
    return packs


def rows_of(pack: str, root: Path) -> list[Row]:
    checks = json.loads((SKILLS / "packs" / pack / "checks.json").read_text(encoding="utf-8"))
    rows = []
    for check in checks.get("checks", []):
        if check.get("scope", "tree") != "tree":
            continue
        probe = check.get("probe")
        if not isinstance(probe, dict) or not probe.get("command"):
            raise WiringError(f"{pack}:{check.get('id')}: not a command row; is the pack phxd?")
        command = [
            part.replace("{skills}", str(SKILLS)).replace("{root}", str(root))
            for part in probe["command"]
        ]
        severity = "advisory" if check.get("severity") == "advisory" else "block"
        timeout = int(probe.get("timeout_seconds", 120))
        rows.append(Row(pack, check["id"], severity, command, timeout))
    return rows


def run(row: Row, root: Path) -> tuple[int | None, float, str]:
    started = time.monotonic()
    try:
        done = subprocess.run(
            row.command,
            cwd=root,
            capture_output=True,
            text=True,
            timeout=row.timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return None, time.monotonic() - started, "timeout"
    lines = (done.stdout + done.stderr).strip().splitlines()
    return done.returncode, time.monotonic() - started, lines[-1] if lines else ""


def judge(code: int | None, severity: str, state: str) -> str:
    if code == 0:
        return "ok"
    if severity == "advisory":
        return "advisory"
    if code == 1:
        return "RED"
    if code == 3:
        return "VOID" if state == "enforced" else "pending"
    return "ERROR"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", default=str(REPO))
    parser.add_argument("--pack", action="append", default=[])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    try:
        wiring = load_wiring()
    except WiringError as error:
        print(f"pack-rows: wiring refused: {error}")
        return 2
    vendored = sorted(p.name for p in (SKILLS / "packs").iterdir() if (p / "checks.json").is_file())
    unnamed = sorted(set(vendored) - set(wiring))
    missing = sorted(set(wiring) - set(vendored))
    if unnamed or missing:
        print(f"pack-rows: wiring refused: no state for {unnamed}; not vendored: {missing}")
        return 2
    chosen = args.pack or vendored
    report: list[dict] = []
    waiting: list[tuple[dict, Row, str]] = []
    for pack in chosen:
        entry = wiring[pack]
        state = entry["state"]
        if state == "phxd":
            report.append({"pack": pack, "row": "*", "verdict": "phxd", "why": "run on the box"})
            continue
        if state == "deferred":
            why = f"deferred to {entry['enforced_by']}"
            report.append({"pack": pack, "row": "*", "verdict": "deferred", "why": why})
            continue
        try:
            rows = rows_of(pack, root)
        except WiringError as error:
            print(f"pack-rows: wiring refused: {error}")
            return 2
        excluded = entry.get("excluded_rows", {})
        deferred = entry.get("deferred_rows", {})
        for name in sorted((set(excluded) | set(deferred)) - {r.ident for r in rows}):
            print(f"pack-rows: wiring refused: {pack} names row {name}, which the pack lacks")
            return 2
        ran_here = []
        for row in rows:
            if row.ident in excluded:
                verdict, why, code, seconds, tail = "excluded", excluded[row.ident], None, 0.0, ""
            elif row.ident in deferred:
                verdict, code, seconds, tail = "deferred", None, 0.0, ""
                why = f"deferred to {deferred[row.ident]}"
            else:
                code, seconds, tail = run(row, root)
                verdict, why = judge(code, row.severity, state), ""
            item = {
                "pack": pack,
                "row": row.ident,
                "severity": row.severity,
                "state": state,
                "exit": code,
                "verdict": verdict,
                "seconds": round(seconds, 2),
                "tail": tail[:200],
                "why": why,
            }
            report.append(item)
            if verdict == "deferred":
                waiting.append((item, row, deferred[row.ident]))
            elif verdict != "excluded":
                ran_here.append(item)
        blocking = [item for item in ran_here if item["severity"] == "block"]
        if state == "pending" and blocking and all(item["exit"] == 0 for item in blocking):
            why = (
                f"pending on {entry['enforced_by']}, but every blocking row ran and passed "
                f"({len(blocking)} row(s)): its state must say enforced"
            )
            report.append({"pack": pack, "row": "*", "verdict": "STALE", "why": why})
    started = time.monotonic()
    for item, row, issue in waiting:
        code, seconds, tail = run(row, root)
        item.update(exit=code, seconds=round(seconds, 2), tail=tail[:200])
        if code == 0:
            item["verdict"] = "STALE"
            item["why"] = f"deferred to {issue}, but the row passes: remove its deferral"
        else:
            still = "a timeout" if code is None else f"exit {code}"
            item["why"] = f"deferred to {issue}; still {still}: {tail[:80]}"
    deferred_pass = {
        "ran": len(waiting),
        "seconds": round(time.monotonic() - started, 2),
        "stale": sum(1 for item, _, _ in waiting if item["verdict"] == "STALE"),
    }
    ran = [item for item in report if item["verdict"] in RAN]
    failures = [item for item in report if item["verdict"] in FAILING]
    if args.json:
        payload = {
            "examined": len(ran),
            "failures": len(failures),
            "deferred_pass": deferred_pass,
            "rows": report,
        }
        print(json.dumps(payload, indent=2))
    else:
        for item in report:
            mark = "FAIL" if item["verdict"] in FAILING else "    "
            detail = item["why"] or f"exit {item.get('exit')}: {item.get('tail', '')[:110]}"
            print(f"{mark} {item['verdict']:9} {item['pack']}:{item['row']} {detail}")
        counts: dict[str, int] = {}
        for item in report:
            counts[item["verdict"]] = counts.get(item["verdict"], 0) + 1
        summary = ", ".join(f"{key} {value}" for key, value in sorted(counts.items()))
        second = (
            f"deferred pass: ran {deferred_pass['ran']} row(s) in {deferred_pass['seconds']}s, "
            f"stale {deferred_pass['stale']}"
        )
        print(
            f"PACK ROWS: examined {len(ran)} row(s) of {len(chosen)} pack(s): {summary}; {second}"
        )
    if not ran:
        print("PACK ROWS VOID: no row was examined")
        return 3
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
