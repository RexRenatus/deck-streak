#!/usr/bin/env python3
"""Evaluate deploy/slo.json against the journal, and page on a burn (observability pack).

Copy to deploy/slo-evaluate.py and run it from a oneshot unit on a timer, every five minutes
(slo-evaluate.template.service and .timer). Standard library only.

For each SLO whose sli.source is "journal", it counts the service's response events -- the
TraceLayer's "finished processing request" events, JSON on stdout -- over each alert's long and
short windows. An alert fires when BOTH windows' error ratio reach burn_rate x (1 - objective):
the multiwindow, multi-burn-rate alert of the SRE workbook's "Alerting on SLOs".

It pages through the ONE alert path. A page that starts is printed with the <3> prefix and the
script exits 1, so its unit fails and OnFailure= starts the alert template unit, whose script
quotes this unit's error lines to Telegram. A ticket is printed with <4> and pages no one. An
alert pages once when it starts firing, and again only after it has stopped: the firing set is
kept in $STATE_DIRECTORY (StateDirectory= in the unit).
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

HOURS = {"m": 1 / 60, "h": 1.0, "d": 24.0, "w": 168.0}
RESPONSE = "finished processing request"


def hours(text: str) -> float:
    found = re.fullmatch(r"(\d+(?:\.\d+)?)([mhdw])", text.strip())
    if not found:
        raise ValueError(f"not a duration: {text!r}")
    return float(found.group(1)) * HOURS[found.group(2)]


def response_events(unit: str, window_hours: float) -> list[dict]:
    """The response events a unit wrote to the journal within the window."""
    seconds = max(1, int(window_hours * 3600))
    listing = subprocess.run(
        [
            "journalctl",
            "--unit",
            unit,
            "--since",
            f"-{seconds}s",
            "--output",
            "json",
            "--no-pager",
        ],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    found = []
    for line in listing.splitlines():
        try:
            message = json.loads(json.loads(line).get("MESSAGE", ""))
        except (json.JSONDecodeError, TypeError, AttributeError):
            continue
        fields = message.get("fields", {}) if isinstance(message, dict) else {}
        if fields.get("message") == RESPONSE:
            found.append(fields)
    return found


def milliseconds(latency: object) -> float | None:
    """tower-http writes latency as `<n> <unit>`, milliseconds by default."""
    found = re.fullmatch(r"\s*(\d+(?:\.\d+)?)\s*(ms|s|μs|us|ns)\s*", str(latency))
    if not found:
        return None
    scale = {"s": 1000.0, "ms": 1.0, "μs": 0.001, "us": 0.001, "ns": 0.000001}
    return float(found.group(1)) * scale[found.group(2)]


def is_bad(fields: dict, sli: dict) -> bool:
    if sli["kind"] == "availability":
        return int(fields.get("status", 0)) >= 500
    latency = milliseconds(fields.get("latency"))
    return latency is None or latency > float(sli["threshold_ms"])


def error_ratio(unit: str, sli: dict, window_hours: float) -> tuple[float, int]:
    events = response_events(unit, window_hours)
    bad = sum(is_bad(fields, sli) for fields in events)
    return (bad / len(events) if events else 0.0), len(events)


def main(argv: list[str]) -> int:
    declaration = json.loads(Path(argv[1]).read_text(encoding="utf-8"))
    state_dir = Path(os.environ.get("STATE_DIRECTORY", "."))
    state_file = state_dir / "firing.json"
    try:
        was_firing = set(json.loads(state_file.read_text(encoding="utf-8")))
    except (OSError, json.JSONDecodeError):
        was_firing = set()
    firing: set[str] = set()
    paged = False
    for slo in declaration["slos"]:
        if slo["sli"]["source"] != "journal":
            continue
        budget = 1 - float(slo["objective"])
        for alert in slo["alerts"]:
            key = f"{slo['id']}:{alert['long_window']}/{alert['short_window']}"
            threshold = float(alert["burn_rate"]) * budget
            long_ratio, total = error_ratio(
                slo["unit"], slo["sli"], hours(alert["long_window"])
            )
            short_ratio, _ = error_ratio(
                slo["unit"], slo["sli"], hours(alert["short_window"])
            )
            if total == 0 or long_ratio < threshold or short_ratio < threshold:
                continue
            firing.add(key)
            line = (
                f"slo {slo['id']} {alert['severity']}: error ratio {long_ratio:.4f} over "
                f"{alert['long_window']} and {short_ratio:.4f} over {alert['short_window']}, "
                f"at or above {threshold:.4f} ({alert['burn_rate']}x the budget)"
            )
            if alert["severity"] == "page" and key not in was_firing:
                print("<3>" + line, flush=True)
                paged = True
            else:
                print("<4>" + line, flush=True)
    state_file.write_text(json.dumps(sorted(firing)), encoding="utf-8")
    return 1 if paged else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
