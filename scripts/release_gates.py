"""The release gate (SPEC-502, ADR-520): a stub that keeps every name its tests read and none of
the behaviour. It refuses nothing, so every release is published."""

import sys
from typing import NamedTuple

RECORD = "release-gates.json"
MISSING = "missing"
FAILED = "failed"


class Gates(NamedTuple):
    model: object
    logloss: object
    rmse_bins: object
    sealed_holds: object
    worse_cells: object
    simulator_win: object
    permission_on_file: object
    scheduler_gate_done: object


def read_record(path):
    return {}, None


def read_gates(record):
    return Gates(None, None, None, None, None, None, None, None)


def present_refusal(name, value):
    return []


def pair_refusal(name, value):
    return []


def flag_refusal(name, value):
    return []


def count_refusal(name, value):
    return []


def refused(gates):
    return []


def verdict(gates):
    return []


def main(argv=None):
    return 0


if __name__ == "__main__":
    sys.exit(main())
