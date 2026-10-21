"""Size the per-shard budget of a Rust leg's unmutated baseline (SPEC-480 section 9, ADR-509): the
most seconds any of run 39033655190's whole shards spent in its unmutated baseline (Build + Test
phases, read from each shard artifact's mutants.out/outcomes.json), times the margin
TIMEOUT_MARGIN 1.5 (SPEC-362 R7), rounded up, as the owner's ruling that bounds a shard's
unmutated baseline by a time budget sized it. A whole shard: the baseline summary reads Success
and every listed mutant has an outcome.
usage: python3 -I -B tools/rust-baseline-budget/size_budget.py <dir of the downloaded artifacts>
download, from a checkout of the repository:
    gh run download 39033655190 -p 'mutation-rust-shard-*' -D <dir>"""

import json
import math
import os
import statistics
import sys

MARGIN = 1.5
D = sys.argv[1]
whole, void, other = [], [], []
for name in sorted(os.listdir(D)):
    p = os.path.join(D, name, name, "mutants.out")
    o = json.load(open(os.path.join(p, "outcomes.json"), encoding="utf-8"))
    listed = len(json.load(open(os.path.join(p, "mutants.json"), encoding="utf-8")))
    base = [x for x in o["outcomes"] if x.get("scenario") == "Baseline"]
    if len(base) != 1:
        other.append(name)
        continue
    ph = {}
    for r in base[0]["phase_results"]:
        d = r["duration"]
        ph[r["phase"]] = d["secs"] + d["nanos"] / 1e9 if isinstance(d, dict) else float(d)
    done = sum(1 for x in o["outcomes"] if x.get("scenario") != "Baseline")
    row = (name, ph.get("Build", 0.0), ph.get("Test", 0.0))
    if base[0]["summary"] == "Success" and done == listed:
        whole.append(row)
    elif base[0]["summary"] == "Timeout":
        void.append(row)
    else:
        other.append(name)
n = len(whole) + len(void) + len(other)
if not whole:
    sys.exit("examined %d shards, 0 whole: VOID, no budget" % n)
tot = [b + t for _, b, t in whole]
top = max(whole, key=lambda r: r[1] + r[2])
budget = math.ceil(max(tot) * MARGIN)
print("examined %d shards: whole %d, baseline Timeout %d, other %d" % (n, len(whole), len(void), len(other)))
print("whole baselines (Build + Test) s: max %.1f (%s: build %.1f, test %.1f), median %.1f, min %.1f" % (max(tot), top[0], top[1], top[2], statistics.median(tot), min(tot)))
print("whole Test phase s: max %.1f, median %.1f; whole Build phase s: max %.1f" % (max(t for _, _, t in whole), statistics.median(t for _, _, t in whole), max(b for _, b, _ in whole)))
print("Timeout baselines: Test phase min %.1f max %.1f (the 2300 s --timeout cut them), Build max %.1f" % (min(t for _, _, t in void), max(t for _, _, t in void), max(b for _, b, _ in void)) if void else "Timeout baselines: 0")
print("BUDGET %d s = ceil(%.3f x %.1f)" % (budget, max(tot), MARGIN))
