#!/usr/bin/env python3
"""DeckStreak's SLO evaluator: the API's burn rates against deploy/slo.json, paged once per burn
episode (SPEC-031 R4; ADR-031).

    slo-evaluate.py DECLARATION [--now SECONDS]

deck-streak-slo.timer runs it every five minutes as deck-streak-slo.service. For each SLO whose SLI
is read from the journal, it reads that unit's response events once, over its alerts' longest
window, and counts, for each alert, the responses and the failed ones in its long window and in its
short one. An alert burns when both windows' error ratios exceed its burn rate times the error
budget: the multiwindow, multi-burn-rate alert of the SRE workbook's "Alerting on SLOs". A window
that holds no response does not burn.

An alert that starts to burn is printed at priority 3 and the script exits 1, so its unit fails and
OnFailure= pages through the alert template unit, which quotes these lines: the page is the SLO's
id, its windows and its counts, never a request's content. An alert still burning is printed at
priority 4 and pages no one; one that stops burning ends its episode at priority 5, and its next burn
pages again. Every alert routes to the one alert path (deploy/slo.json), so a ticket reaches the
owner too, once per episode. The burning set is kept in $STATE_DIRECTORY. A run that cannot measure
(a journal it cannot read, a declaration it cannot parse) is an episode of its own: it pages once,
and again only after a run that measured.

A response event is the trace event SPEC-025's layer writes: one JSON object after its priority
prefix, flattened by the kernel's log format (ADR-020), with "message" "finished processing request"
and an integer "status"; an event nested under "fields" is read the same way. Good is a status below
500, and total is every response event (SPEC-031 R2). --now fixes the clock for a test; the unit runs
without it. Standard library only.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from fractions import Fraction
from pathlib import Path

RESPONSE = "finished processing request"
STATE_FILE = "burning.json"
# The key of the evaluator's own episode: a run that could not measure.
UNMEASURED = "evaluator"
SECONDS = {"m": 60, "h": 3_600, "d": 86_400, "w": 604_800}
EXIT_QUIET, EXIT_PAGE, EXIT_USAGE = 0, 1, 2


class Unmeasured(Exception):
    """Why a run cannot measure, in words that carry no value it read."""


def window_seconds(text: object) -> int:
    """A window such as `30m`, `6h` or `3d`, in seconds."""
    match = re.fullmatch(r"(\d+)([mhdw])", str(text).strip())
    if match is None:
        raise Unmeasured(f"{text!r} is not a window such as 30m, 6h or 3d")
    return int(match.group(1)) * SECONDS[match.group(2)]


def exact(number: object) -> Fraction:
    """A declared number as the exact decimal it is written as, so 5.6 times 1% is 7/125."""
    if isinstance(number, bool) or not isinstance(number, (int, float)):
        raise Unmeasured(f"{number!r} is not a number")
    return Fraction(str(number))


def error_budget(objective: object) -> Fraction:
    """The share of responses that may fail: one less the objective."""
    budget = 1 - exact(objective)
    if not 0 < budget < 1:
        raise Unmeasured(f"the objective {objective!r} is not strictly between 0 and 1")
    return budget


def burning(alert: dict, budget: Fraction, long: tuple[int, int], short: tuple[int, int]) -> bool:
    """Whether both windows, each (failed, total), fail past the alert's burn rate times the budget.
    A window that holds no response does not burn."""
    threshold = exact(alert["burn_rate"]) * budget
    return all(total > 0 and Fraction(failed, total) > threshold for failed, total in (long, short))


def count(responses: list[tuple[float, int]], now: float, seconds: int) -> tuple[int, int]:
    """(failed, total) of the responses in the window (now - seconds, now]."""
    start = now - seconds
    inside = [status for at, status in responses if start < at <= now]
    return sum(status >= 500 for status in inside), len(inside)


def message_text(entry: dict) -> str | None:
    """An entry's MESSAGE: journald exports one it holds as bytes as an array of numbers, and one
    past its size limit as null."""
    message = entry.get("MESSAGE")
    if isinstance(message, str):
        return message
    if isinstance(message, list) and all(
        isinstance(byte, int) and 0 <= byte <= 255 for byte in message
    ):
        return bytes(message).decode("utf-8", errors="replace")
    return None


def response_status(message: str) -> int | None:
    """The status of a response event, or None for any other message."""
    if not message.startswith("{"):
        return None
    try:
        event = json.loads(message)
    except json.JSONDecodeError:
        return None
    if not isinstance(event, dict):
        return None
    fields = event["fields"] if isinstance(event.get("fields"), dict) else event
    status = fields.get("status")
    if fields.get("message") != RESPONSE or isinstance(status, bool):
        return None
    return status if isinstance(status, int) else None


def responses(unit: str, since: int, until: int) -> list[tuple[float, int]]:
    """The response events `unit` wrote to the journal from `since` to `until`, as (seconds,
    status): read in one pass, the export streamed rather than held whole."""
    command = [
        "journalctl",
        f"--unit={unit}",
        f"--since=@{since}",
        f"--until=@{until}",
        "--output=json",
        "--output-fields=MESSAGE",
        "--no-pager",
        "--quiet",
    ]
    found = []
    with tempfile.TemporaryFile() as errors:
        try:
            process = subprocess.Popen(
                command,
                stdout=subprocess.PIPE,
                stderr=errors,
                text=True,
                encoding="utf-8",
                errors="replace",
            )
        except OSError as error:
            raise Unmeasured(f"journalctl could not run: {error.strerror}") from error
        with process:
            for line in process.stdout:
                try:
                    entry = json.loads(line)
                    at = int(entry["__REALTIME_TIMESTAMP"]) / 1_000_000
                except (json.JSONDecodeError, KeyError, TypeError, ValueError):
                    continue
                message = message_text(entry)
                status = None if message is None else response_status(message)
                if status is not None:
                    found.append((at, status))
        if process.returncode != 0:
            errors.seek(0)
            said = errors.read().decode("utf-8", errors="replace").strip().splitlines()
            detail = f": {said[-1]}" if said else ""
            raise Unmeasured(f"journalctl exited {process.returncode}{detail}")
    return found


def evaluate(declaration: object, now: float) -> dict[str, str]:
    """Every alert burning at `now`, by its episode key, with the line that says so."""
    if not isinstance(declaration, dict) or not isinstance(declaration.get("slos"), list):
        raise Unmeasured("the declaration holds no list of SLOs")
    found = {}
    for slo in declaration["slos"]:
        sli = slo.get("sli") if isinstance(slo, dict) else None
        if not isinstance(sli, dict) or sli.get("source") != "journal":
            continue
        budget = error_budget(slo["objective"])
        alerts = slo["alerts"]
        longest = max(window_seconds(alert["long_window"]) for alert in alerts)
        seen = responses(slo["unit"], int(now) - longest, int(now))
        for alert in alerts:
            long_window, short_window = alert["long_window"], alert["short_window"]
            long = count(seen, now, window_seconds(long_window))
            short = count(seen, now, window_seconds(short_window))
            if not burning(alert, budget, long, short):
                continue
            key = f"{slo['id']}:{alert['severity']}:{long_window}/{short_window}"
            found[key] = (
                f"slo {slo['id']} {alert['severity']}: {long[0]} of {long[1]} response(s) failed "
                f"over {long_window} and {short[0]} of {short[1]} over {short_window}, past "
                f"{alert['burn_rate']}x the error budget"
            )
    return found


def remembered(path: Path) -> set[str]:
    """The episode keys the last run left burning; none when it left no readable record."""
    try:
        keys = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return set()
    except (OSError, ValueError):
        print("<4>slo evaluation found its record unreadable, and starts afresh", flush=True)
        return set()
    return {key for key in keys if isinstance(key, str)} if isinstance(keys, list) else set()


def remember(path: Path, keys: set[str]) -> None:
    """Records the burning keys, whole or not at all: written beside the record, then renamed."""
    try:
        with tempfile.NamedTemporaryFile(
            "w", dir=path.parent, prefix=".burning.", delete=False, encoding="utf-8"
        ) as out:
            json.dump(sorted(keys), out)
            written = out.name
        os.replace(written, path)
    except OSError as error:
        # The next run pages again, which says as much; a line here says why.
        print(f"<4>slo evaluation cannot keep its record: {error.strerror}", flush=True)


def reason(error: Exception) -> str:
    """Why a run measured nothing, in words that carry no value it read."""
    if isinstance(error, Unmeasured):
        return str(error)
    if isinstance(error, OSError):
        return f"the declaration cannot be read: {error.strerror}"
    if isinstance(error, ValueError):
        return "the declaration is not JSON"
    return f"the declaration lacks a field it needs ({type(error).__name__})"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("declaration", type=Path, help="deploy/slo.json, as the release holds it")
    parser.add_argument(
        "--now", type=float, help="the clock, in seconds since the epoch, for tests"
    )
    args = parser.parse_args(argv)
    directory = os.environ.get("STATE_DIRECTORY", "").split(":")[0]
    if not directory:
        print("<3>slo evaluation has no $STATE_DIRECTORY to keep its episodes in", flush=True)
        return EXIT_USAGE
    record = Path(directory) / STATE_FILE
    before = remembered(record)
    now = time.time() if args.now is None else args.now
    try:
        current = evaluate(json.loads(args.declaration.read_text(encoding="utf-8")), now)
    except (Unmeasured, OSError, ValueError, KeyError, TypeError) as error:
        current, unmeasured = None, reason(error)
    else:
        unmeasured = None

    lines = []
    paged = False
    if current is None:
        # A run that measured nothing ends no episode: the burning set stands as it was.
        after = before | {UNMEASURED}
        if UNMEASURED in before:
            lines.append(f"<4>slo evaluation failed again: {unmeasured}")
        else:
            lines.append(f"<3>slo evaluation failed: {unmeasured}")
            paged = True
    else:
        after = set(current)
        for key, line in sorted(current.items()):
            if key in before:
                lines.append(f"<4>{line}; still burning, paged when it started")
            else:
                lines.append(f"<3>{line}")
                paged = True
        for key in sorted(before - after - {UNMEASURED}):
            slo, _, rest = key.partition(":")
            severity, _, windows = rest.partition(":")
            lines.append(f"<5>slo {slo} {severity}: {windows} no longer burns; the episode ended")
        if UNMEASURED in before:
            lines.append("<5>slo evaluation measures again")
    remember(record, after)
    for line in lines:
        print(line, flush=True)
    return EXIT_PAGE if paged else EXIT_QUIET


if __name__ == "__main__":
    sys.exit(main())
