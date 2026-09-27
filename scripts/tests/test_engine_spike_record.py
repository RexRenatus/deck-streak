"""ADR-009 records the engine spike's numbers against ADR-022's budgets, and a final status
(SPEC-022 A1, R4).

ADR-022 fixed the budgets before anything was measured. ADR-009's Confirmation carries one row per
budget: the budget as ADR-022 states it, what `engine-measure.yml` measured, and a verdict that
follows from the number. It names the run and the engine tag it measured, and ADR-009's status
follows from the verdicts: `accepted` when every budget holds, `superseded` when any fails. A
status of `proposed` is not final, so it is refused.
"""

import re
import unittest

from _support import REPO, examined

DECISIONS = REPO / "docs" / "decisions"
ADR_009 = DECISIONS / "ADR-009-ingest-from-the-anki-sync-server.md"
ADR_022 = DECISIONS / "ADR-022-the-ingest-spike-protocol-and-budgets.md"
#: The engine's pin in the workspace manifest (ADR-022).
ENGINE_TAG = re.compile(
    r'(?m)^anki = \{ git = "https://github\.com/ankitects/anki\.git", tag = "([^"]+)"'
)
BUDGETS = ["measure", "how", "budget"]
MEASURED = ["measure", "budget", "measured", "verdict"]
#: A budget or a measurement: "at most 20 minutes", "12.4 minutes", "96.5 MiB".
QUANTITY = re.compile(r"^(at most )?(\d+(?:\.\d+)?) (minutes|MiB|seconds)$")
STATUS = re.compile(r"(?m)^status: (\S+)$")
RUN = re.compile(r"`engine-measure\.yml` run \d+")
#: The six measures SPEC-022 R3 names, in ADR-022's order.
MEASURES = [
    "cold build",
    "binary size",
    "open and queue",
    "full download",
    "incremental sync",
    "licences",
]


def cells(line):
    """A markdown table row's cells, or None for a line that is not a row."""
    if not line.startswith("|"):
        return None
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def table(text, header):
    """The rows under the markdown table whose header row is `header`, or None."""
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if cells(line) == header:
            rows = []
            for row in lines[index + 2 :]:
                if cells(row) is None:
                    break
                rows.append(cells(row))
            return rows
    return None


def section(text, heading):
    """The body of the `### <heading>` section, up to the next heading."""
    match = re.search(rf"(?ms)^### {re.escape(heading)}\n(.*?)(?=^#|\Z)", text)
    return match.group(1) if match else ""


def budget(text):
    """A budget cell as a comparable value: ("pass",), or (number, unit), or None."""
    if text.startswith("pass"):
        return ("pass",)
    match = QUANTITY.match(text)
    if match is None or match.group(1) is None:
        return None
    return (float(match.group(2)), match.group(3))


def held(measured, expected):
    """Whether a measurement holds its budget, or why it cannot be read as one."""
    if expected == ("pass",):
        if measured not in ("pass", "fail"):
            return None, f"measured {measured!r} is not pass or fail"
        return measured == "pass", None
    match = QUANTITY.match(measured)
    if match is None or match.group(1) is not None or match.group(3) != expected[1]:
        return None, f"measured {measured!r} is not a number of {expected[1]}"
    return float(match.group(2)) <= expected[0], None


def findings(adr_022, adr_009, tag):
    """Why ADR-009 does not record ADR-022's measurement and a final status; empty when it does."""
    expected = {row[0]: budget(row[2]) for row in table(adr_022, BUDGETS) or []}
    confirmation = section(adr_009, "Confirmation")
    rows = table(confirmation, MEASURED)
    if rows is None:
        return [
            "ADR-009's Confirmation holds no table of measure, budget, measured, verdict"
        ]
    found = []
    if RUN.search(confirmation) is None:
        found.append("ADR-009's Confirmation names no `engine-measure.yml` run")
    if f"`{tag}`" not in confirmation:
        found.append(f"ADR-009's Confirmation does not name the engine tag `{tag}`")
    recorded = [row[0] for row in rows]
    for measure in expected:
        if recorded.count(measure) != 1:
            found.append(
                f"{measure}: recorded {recorded.count(measure)} time(s), not once"
            )
    verdicts = []
    for row in rows:
        if len(row) != len(MEASURED) or row[0] not in expected:
            found.append(f"{row[0]}: is not one of ADR-022's budgets")
            continue
        measure, stated, measured, verdict = row
        if budget(stated) != expected[measure]:
            found.append(f"{measure}: the budget {stated!r} is not ADR-022's")
            continue
        holds, why = held(measured, expected[measure])
        if why is not None:
            found.append(f"{measure}: {why}")
            continue
        if verdict != ("pass" if holds else "fail"):
            found.append(
                f"{measure}: the verdict {verdict!r} does not follow from {measured!r} "
                f"against {stated!r}"
            )
        verdicts.append(verdict)
    status = STATUS.search(adr_009)
    status = status.group(1) if status else None
    if status not in ("accepted", "superseded"):
        found.append(
            f"ADR-009's status is {status!r}, not final (accepted or superseded)"
        )
    elif status == "accepted" and "fail" in verdicts:
        found.append("ADR-009 is accepted although a budget failed")
    elif status == "superseded" and "fail" not in verdicts:
        found.append("ADR-009 is superseded although every budget held")
    return found


#: A planted record in which every budget holds.
HOLDING = [
    ("cold build", "at most 20 minutes", "12.5 minutes", "pass"),
    ("binary size", "at most 100 MiB", "40.0 MiB", "pass"),
    ("open and queue", "at most 256 MiB", "90.0 MiB", "pass"),
    ("full download", "at most 256 MiB", "200.0 MiB", "pass"),
    ("incremental sync", "at most 60 seconds", "2.0 seconds", "pass"),
    ("licences", "pass", "pass", "pass"),
]


def planted(status="accepted", rows=HOLDING, run="`engine-measure.yml` run 1"):
    """A planted ADR-009 at tag `0.0` whose Confirmation records `rows`."""
    body = "\n".join(f"| {' | '.join(row)} |" for row in rows)
    return (
        f"---\nstatus: {status}\n---\n\n# A planted ADR\n\n### Confirmation\n\n"
        f"Measured at tag `0.0` in {run}:\n\n"
        f"| measure | budget | measured | verdict |\n|---|---|---|---|\n{body}\n\n"
        "## What would make this wrong\n"
    )


def replaced(measure, stated, measured, verdict):
    """HOLDING with `measure`'s row stating another budget, measurement or verdict."""
    return [
        (measure, stated, measured, verdict) if row[0] == measure else row
        for row in HOLDING
    ]


def dropped(measure):
    """HOLDING without `measure`'s row."""
    return [row for row in HOLDING if row[0] != measure]


class TheSpikeIsRecorded(unittest.TestCase):
    def test_adr_009_records_the_measured_numbers_and_a_final_status(self):
        adr_022 = ADR_022.read_text(encoding="utf-8")
        budgets = examined("budget(s) ADR-022 fixes", table(adr_022, BUDGETS) or [])
        self.assertEqual([row[0] for row in budgets], MEASURES)
        # Each planted defect is refused by name before the committed record is judged.
        self.assertEqual(findings(adr_022, planted(), "0.0"), [])
        refusals = {
            "a proposed status": (
                planted(status="proposed"),
                ["ADR-009's status is 'proposed', not final (accepted or superseded)"],
            ),
            "an accepted status over a failed budget": (
                planted(
                    rows=replaced(
                        "full download",
                        "at most 256 MiB",
                        "300.0 MiB",
                        "fail",
                    )
                ),
                ["ADR-009 is accepted although a budget failed"],
            ),
            "a verdict the number contradicts": (
                planted(
                    rows=replaced(
                        "cold build",
                        "at most 20 minutes",
                        "21.0 minutes",
                        "pass",
                    )
                ),
                [
                    "cold build: the verdict 'pass' does not follow from '21.0 minutes' "
                    "against 'at most 20 minutes'"
                ],
            ),
            "a missing budget": (
                planted(rows=dropped("incremental sync")),
                ["incremental sync: recorded 0 time(s), not once"],
            ),
            "a budget other than ADR-022's": (
                planted(
                    rows=replaced(
                        "binary size",
                        "at most 200 MiB",
                        "40.0 MiB",
                        "pass",
                    )
                ),
                ["binary size: the budget 'at most 200 MiB' is not ADR-022's"],
            ),
            "a measurement in another unit": (
                planted(
                    rows=replaced(
                        "open and queue",
                        "at most 256 MiB",
                        "90.0 seconds",
                        "pass",
                    )
                ),
                ["open and queue: measured '90.0 seconds' is not a number of MiB"],
            ),
            "no run named": (
                planted(run="a run"),
                ["ADR-009's Confirmation names no `engine-measure.yml` run"],
            ),
        }
        for name, (text, refusal) in examined("planted defect(s)", refusals.items()):
            with self.subTest(name):
                self.assertEqual(findings(adr_022, text, "0.0"), refusal)
        tag = ENGINE_TAG.search((REPO / "Cargo.toml").read_text(encoding="utf-8"))
        self.assertIsNotNone(tag, "Cargo.toml pins no engine tag (ADR-022)")
        self.assertEqual(
            findings(adr_022, ADR_009.read_text(encoding="utf-8"), tag.group(1)), []
        )


if __name__ == "__main__":
    unittest.main()
