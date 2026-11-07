#!/usr/bin/env python3
"""The regression record checker (SPEC-618, ADR-640).

    perf-record.py check --dir perf/regressions.d --branch <head ref> --benchmarks FILE

A pull request that keeps a change CodSpeed reads slower carries a regression record, one JSON
file of the closed schema `deckstreak.perf-regression.v1`:

    schema        "deckstreak.perf-regression.v1"
    branch        the pull request's own head ref
    benchmarks    one or more {name, change_pct, base, head, instrument}: a benchmark a bench of
                  the tree defines, its change in percent (above 0), the two commit shas, and
                  the instrument, "simulation"
    cause, why    what adds the instructions, and the behaviour it buys
    spec          the SPEC requirement it serves
    alternatives  what was tried to avoid it, each with its measured change (a list of text)
    remediation   "requested", or "not-needed: <reason>"

It reads every `*.json` file directly in the folder `--dir` names (an absent or empty folder
examines none) and refuses a record by arm: an unknown or a missing key, a blank text, a change of
0 or less, a benchmark no bench defines (the names are the lines of the file `--benchmarks`
names), a branch other than `--branch`, and any other value outside the schema. It prints one
line per refusal, or OK, and `perf-record: examined <n> records, <m> refused`; it exits 1 when it
refused one or cannot read the list. Standard library only.
"""

import argparse
import json
import re
import sys
from pathlib import Path

SCHEMA = "deckstreak.perf-regression.v1"
KEYS = (
    "schema",
    "branch",
    "benchmarks",
    "cause",
    "why",
    "spec",
    "alternatives",
    "remediation",
)
BENCH_KEYS = ("name", "change_pct", "base", "head", "instrument")
TEXTS = ("cause", "why", "spec")
SHA = re.compile(r"[0-9a-f]{7,40}")
REMEDIATION = re.compile(r"requested|not-needed: \S.*")


def has_text(value):
    return isinstance(value, str) and bool(value.strip())


def key_problems(where, value, keys):
    if not isinstance(value, dict):
        return [where + " is no JSON object"]
    found = [where + " holds the unknown key " + key for key in sorted(value) if key not in keys]
    found += [where + " lacks the key " + key for key in keys if key not in value]
    return found


def bench_problems(index, bench, names):
    where = "benchmark %d" % index
    found = key_problems(where, bench, BENCH_KEYS)
    if found:
        return found
    if bench["name"] not in names:
        found.append(where + " names " + repr(bench["name"]) + ", which no bench defines")
    change = bench["change_pct"]
    if isinstance(change, bool) or not isinstance(change, (int, float)) or change <= 0:
        found.append(where + " change_pct " + repr(change) + " is not above 0")
    for key in ("base", "head"):
        if not (isinstance(bench[key], str) and SHA.fullmatch(bench[key])):
            found.append(where + " " + key + " is no commit sha")
    if bench["instrument"] != "simulation":
        found.append(where + " instrument is not simulation")
    return found


def problems(record, branch, names):
    found = key_problems("the record", record, KEYS)
    if found:
        return found
    if record["schema"] != SCHEMA:
        found.append("schema is not " + SCHEMA)
    if record["branch"] != branch:
        found.append("branch " + repr(record["branch"]) + " is not the head ref " + branch)
    benches = record["benchmarks"]
    if not (isinstance(benches, list) and benches):
        found.append("benchmarks holds no benchmark")
    else:
        for index, bench in enumerate(benches):
            found += bench_problems(index, bench, names)
    for key in TEXTS:
        if not has_text(record[key]):
            found.append(key + " is blank")
    alternatives = record["alternatives"]
    if not (isinstance(alternatives, list) and all(has_text(a) for a in alternatives)):
        found.append("alternatives is no list of text")
    remedy = record["remediation"]
    if not (isinstance(remedy, str) and REMEDIATION.fullmatch(remedy)):
        found.append("remediation is neither requested nor not-needed: <reason>")
    return found


def check(args):
    try:
        names = {
            line.strip()
            for line in Path(args.benchmarks).read_text(encoding="utf-8").splitlines()
            if line.strip()
        }
    except (OSError, ValueError):
        print("perf-record: VOID cannot read the benchmark list at " + args.benchmarks)
        return 1
    refused = 0
    files = sorted(Path(args.dir).glob("*.json"))
    for path in files:
        try:
            record = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            found = ["the record is no JSON document"]
        else:
            found = problems(record, args.branch, names)
        for problem in found:
            print("perf-record: " + path.name + ": " + problem)
        if found:
            refused += 1
        else:
            print("perf-record: " + path.name + ": OK")
    print("perf-record: examined %d records, %d refused" % (len(files), refused))
    if refused:
        return 1


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("check",))
    parser.add_argument("--dir", required=True)
    parser.add_argument("--branch", required=True)
    parser.add_argument("--benchmarks", required=True)
    return check(parser.parse_args(argv))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
