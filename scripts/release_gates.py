"""The release gate: a release tag is published only when the scheduler's release gates are on file
(SPEC-502, ADR-520).

The release workflow runs this script directly before its publish step. It reads one record,
`release-gates.json` at the root of the tag's checkout, and decides whether the release may be
published. The record is a public aggregate: `model`, the sha256 of the scheduler's weights file;
the full development set's log loss and RMSE(bins), each as a [the scheduler, FSRS-7] pair; and
the outcomes of the sealed set, the scorecard, the offline simulator, the weights' written
permission and the scheduler's own final release check.

A record that is not on file refuses as `record`. A gate the record lacks, or holds in another
type than its schema's, refuses as missing, and a gate on file that does not hold refuses as
failed; every refusing gate is named, one line each, in the record's order. Exit 0 publishes, 1
refuses, 2 is a usage error. The script reads that one file, writes nothing and starts no program.
"""

import argparse
import json
import math
import re
import sys
from pathlib import Path
from typing import NamedTuple

RECORD = "release-gates.json"
MISSING = "missing"
FAILED = "failed"
DIGEST = re.compile(r"[0-9a-f]{64}")
DATE = re.compile(r"[0-9]{4}-[0-9]{2}-[0-9]{2}")


class Gates(NamedTuple):
    """Each gate's value as the record holds it, or None when the record lacks it or holds it in
    another type than its schema's."""

    model: object
    logloss: object
    rmse_bins: object
    sealed_holds: object
    worse_cells: object
    simulator_win: object
    permission_on_file: object
    scheduler_gate_done: object


class TwiceError(ValueError):
    """A JSON object that holds one key twice, which a reader would take as its last value."""


def once(pairs):
    """An object hook that refuses a key held twice in one object."""
    keys = [key for key, _ in pairs]
    if len(set(keys)) != len(keys):
        raise TwiceError("a key twice")
    return dict(pairs)


def read_record(path):
    """The record at `path` as a JSON object, or None with why it is not on file."""
    if not path.is_file():
        return None, f"{RECORD} is not on file"
    try:
        document = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=once)
    except TwiceError:
        return None, f"{RECORD} holds a key twice in one object"
    except (UnicodeDecodeError, ValueError):
        return None, f"{RECORD} is not UTF-8 JSON"
    except OSError:
        return None, f"{RECORD} cannot be read"
    if not isinstance(document, dict):
        return None, f"{RECORD} is not a JSON object"
    return document, None


def number(value):
    """A finite JSON number, or None: a boolean, a string or a non-finite number is none."""
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    if isinstance(value, float) and not math.isfinite(value):
        return None
    return value


def pair(value):
    """A [the scheduler, FSRS-7] pair of finite numbers, or None."""
    if not isinstance(value, list) or len(value) != 2:
        return None
    first, second = number(value[0]), number(value[1])
    if first is None or second is None:
        return None
    return (first, second)


def flag(value):
    """A boolean, or None."""
    return value if isinstance(value, bool) else None


def count(value):
    """An integer that is not a boolean, or None."""
    return value if isinstance(value, int) and not isinstance(value, bool) else None


def permission(value):
    """The permission's `value`, when it is an object holding a boolean `value` and a `date` that
    is null or `YYYY-MM-DD`; else None."""
    if not isinstance(value, dict) or "date" not in value:
        return None
    date = value["date"]
    if date is not None and not (isinstance(date, str) and DATE.fullmatch(date)):
        return None
    return flag(value.get("value"))


def read_gates(record):
    """Each gate's value from a record, a JSON object."""
    dev = record.get("dev")
    if not isinstance(dev, dict):
        dev = {}
    model = record.get("model")
    return Gates(
        model=model if isinstance(model, str) and DIGEST.fullmatch(model) else None,
        logloss=pair(dev.get("logloss")),
        rmse_bins=pair(dev.get("rmse_bins")),
        sealed_holds=flag(record.get("sealed_holds")),
        worse_cells=count(record.get("worse_cells")),
        simulator_win=flag(record.get("simulator_win")),
        permission_on_file=permission(record.get("permission_on_file")),
        scheduler_gate_done=flag(record.get("scheduler_gate_done")),
    )


def present_refusal(name, value):
    """A gate that holds whenever it is on file."""
    if value is None:
        return [(name, MISSING)]
    return []


def pair_refusal(name, value):
    """A [the scheduler, FSRS-7] pair, which holds when the scheduler's figure is below FSRS-7's."""
    if value is None:
        return [(name, MISSING)]
    if value[0] < value[1]:
        return []
    return [(name, FAILED)]


def flag_refusal(name, value):
    """A boolean gate, which holds when it is true."""
    if value is None:
        return [(name, MISSING)]
    if value:
        return []
    return [(name, FAILED)]


def count_refusal(name, value):
    """A count of worse cells, which holds at zero."""
    if value is None:
        return [(name, MISSING)]
    if value == 0:
        return []
    return [(name, FAILED)]


def refused(gates):
    """Every refusing gate of a record, in the record's order, each with why."""
    return (
        present_refusal("model", gates.model)
        + pair_refusal("dev.logloss", gates.logloss)
        + pair_refusal("dev.rmse_bins", gates.rmse_bins)
        + flag_refusal("sealed_holds", gates.sealed_holds)
        + count_refusal("worse_cells", gates.worse_cells)
        + flag_refusal("simulator_win", gates.simulator_win)
        + flag_refusal("permission_on_file", gates.permission_on_file)
        + flag_refusal("scheduler_gate_done", gates.scheduler_gate_done)
    )


def verdict(gates):
    """The release's refusals: the record itself when none is on file (`gates` is None), else each
    refusing gate. An empty list publishes."""
    if gates is None:
        return [("record", MISSING)]
    return refused(gates)


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Refuse the release's publish step unless the release gates hold (SPEC-502)."
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="the checkout whose release-gates.json is read (default: the one holding this script)",
    )
    args = parser.parse_args(argv)
    document, cause = read_record(args.root / RECORD)
    gates = None if document is None else read_gates(document)
    refusals = verdict(gates)
    if not refusals:
        print(f"release-gates: PASS: every gate holds for the scheduler's weights {gates.model}")
        return 0
    for name, why in refusals:
        line = f"release-gates: REFUSED: {name} ({why})"
        print(f"{line}: {cause}" if name == "record" else line)
    print(f"release-gates: the release is not published: {len(refusals)} gate(s) refused")
    return 1


if __name__ == "__main__":
    sys.exit(main())
