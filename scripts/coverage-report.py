#!/usr/bin/env python3
"""The advisory coverage report (SPEC-592, ADR-613).

Five subcommands read what the coverage workflow produces and print integer counts only:

    plan      which lanes a pull request or a push runs, and why
    provider  the coverage provider is locked at the vitest version
    args      the Rust run flags that config/coverage.json names
    controls  the planted cases each lane reads before it measures
    report    one lane lcov, its not executed changed lines, its step summary

Standard library only. Every measured report exits 0 whatever its figures read; an instrument
that is broken (no examined line, an lcov that cannot be read, a coverage provider locked at
another version than vitest) is VOID and exits 1.
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEFAULT_CONFIG = str(REPO / "config" / "coverage.json")
DEFAULT_FIXTURES = str(REPO / "scripts" / "tests" / "fixtures" / "coverage")
LANES = ("rust", "web", "python")
SUMMARY_LIMIT = 100
RETENTION_PUSH = "90"
RETENTION_PULL = "30"

VOID_PLAN = "coverage-plan: VOID %s"
VOID_PROVIDER = "coverage-provider: VOID %s"
VOID_ARGS = "coverage-args: VOID %s"
VOID_CONTROLS = "coverage-controls: VOID %s"
VOID_REPORT = "coverage-report: VOID lane %s: %s"
LINE_PLAN_LANE = "coverage-plan: lane %s runs: %s"
LINE_PLAN_SKIP = (
    "coverage-plan: lane %s does not run: no changed path the lane measures or configures"
)
LINE_PLAN_WHOLE = "coverage-plan: lane %s runs over the whole tree"
LINE_PLAN_SCOPE = "coverage-plan: scope %s, retention %s days, examined %d changed paths"
LINE_PROVIDER = "coverage-provider: @vitest/coverage-v8 %s equals vitest %s"
LINE_CONTROL_OK = "coverage-controls: case %s read as planted"
LINE_CONTROLS = "coverage-controls: %d of %d planted cases read as planted"
LINE_REPORT = (
    "coverage-report: lane %s examined %d lines in %d files; outside the lane %d files; "
    "exclusions %d rows (config/coverage.json), %d files dropped"
)
LINE_CHANGED = "changed executable lines %d, executed %d, not executed: %s"
LINE_LACKS = "changed files the lcov lacks: %d"
LINE_PUSH = "the changed-line figure is read on pull requests only"
LINE_RUNNER = "runner exit %s"
LINE_SKIPPED = "changed paths with a quoted name, counted and skipped: %d"
NONE_WORD = "none"
MORE_LINE = "and %d more in report.txt"
GLOB_LITERAL = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._/-"

HUNK = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")
INTEGER = re.compile(r"^[0-9]+$")
VITEST_LOCK = re.compile(r"^  vitest@([0-9][0-9A-Za-z.+-]*):$", re.MULTILINE)
PROVIDER_LOCK = re.compile(r"^  '@vitest/coverage-v8@([0-9][0-9A-Za-z.+-]*)':$", re.MULTILINE)


class Void(Exception):
    """The instrument is broken: the report reads nothing it can judge."""


def glob_regex(glob):
    """`**`, `*`, `{a,b}` and the literals [A-Za-z0-9._/-]; any other glob is unreadable."""
    out = []
    i = 0
    while i < len(glob):
        ch = glob[i]
        if glob.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif glob.startswith("**", i):
            out.append(".*")
            i += 2
        elif ch == "*":
            out.append("[^/]*")
            i += 1
        elif ch == "{":
            end = glob.find("}", i)
            if end < 0:
                raise Void("unreadable glob " + glob)
            alts = glob[i + 1 : end].split(",")
            for alt in alts:
                if not alt or any(c not in GLOB_LITERAL for c in alt):
                    raise Void("unreadable glob " + glob)
            out.append("(?:" + "|".join(re.escape(a) for a in alts) + ")")
            i = end + 1
        elif ch in GLOB_LITERAL:
            out.append(re.escape(ch))
            i += 1
        else:
            raise Void("unreadable glob " + glob)
    return re.compile("^" + "".join(out) + "$")


def load_config(path):
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        raise Void("config unreadable: " + str(err)) from err
    lanes = {}
    for lane in LANES:
        spec = data.get("lanes", {}).get(lane)
        if not isinstance(spec, dict):
            raise Void("config holds no lane " + lane)
        lanes[lane] = {
            "measures": [glob_regex(g) for g in spec["measures"]],
            "configures": [glob_regex(g) for g in spec["configures"]],
            "sf_bases": list(spec["sf_bases"]),
            "group_depth": int(spec["group_depth"]),
            "exclusions": [],
        }
    shared = [glob_regex(g) for g in data.get("shared", [])]
    rows = data.get("exclusions", [])
    for row in rows:
        if row["lane"] not in lanes:
            raise Void("exclusion row names no lane: " + row["glob"])
        lanes[row["lane"]]["exclusions"].append(glob_regex(row["glob"]))
    return {"lanes": lanes, "shared": shared, "rows": rows}


def any_match(patterns, path):
    return any(p.match(path) for p in patterns)


def read_lines(path):
    if not path:
        return []
    try:
        text = Path(path).read_text(encoding="utf-8")
    except OSError as err:
        raise Void("cannot read " + path + ": " + str(err)) from err
    return [line.strip() for line in text.splitlines() if line.strip()]


def write_output(path, pairs):
    if not path:
        return
    with Path(path).open("a", encoding="utf-8") as fh:
        for key, value in pairs:
            fh.write(key + "=" + value + "\n")


def cmd_plan(args):
    cfg = load_config(args.config)
    changed = read_lines(args.changed)
    event = args.event
    if event == "push":
        scope, retention, whole = "whole", RETENTION_PUSH, True
    elif event == "pull_request":
        retention = RETENTION_PULL
        release = (
            args.base_ref == "main"
            and args.head_ref == "dev"
            and bool(args.repo)
            and args.head_repo == args.repo
        )
        scope = "whole" if release else "diff"
        whole = release
    else:
        raise Void("an event the plan does not read: " + event)
    results = {}
    for lane in LANES:
        spec = cfg["lanes"][lane]
        first = None
        if not whole:
            for path in changed:
                if any_match(spec["measures"] + spec["configures"] + cfg["shared"], path):
                    first = path
                    break
        results[lane] = whole or first is not None
        if whole:
            print(LINE_PLAN_WHOLE % lane)
        elif first is not None:
            print(LINE_PLAN_LANE % (lane, first))
        else:
            print(LINE_PLAN_SKIP % lane)
    print(LINE_PLAN_SCOPE % (scope, retention, len(changed)))
    pairs = [("scope", scope)]
    for lane in LANES:
        pairs.append((lane, "true" if results[lane] else "false"))
    pairs.append(("retention", retention))
    write_output(args.github_output, pairs)
    return 0


def provider_versions(text):
    vitest = sorted(set(VITEST_LOCK.findall(text)))
    provider = sorted(set(PROVIDER_LOCK.findall(text)))
    if not vitest:
        raise Void("the lockfile locks no vitest")
    if not provider:
        raise Void("the lockfile locks no coverage provider")
    if len(vitest) > 1 or len(provider) > 1:
        raise Void("the lockfile locks two versions of vitest or of the provider")
    if vitest[0] != provider[0]:
        raise Void("the provider is locked at " + provider[0] + " and vitest at " + vitest[0])
    return vitest[0]


def cmd_provider(args):
    try:
        text = Path(args.lockfile).read_text(encoding="utf-8")
    except OSError as err:
        raise Void("cannot read " + args.lockfile + ": " + str(err)) from err
    version = provider_versions(text)
    print(LINE_PROVIDER % (version, version))
    return 0


def cmd_args(args):
    cfg = load_config(args.config)
    if args.lane != "rust":
        return 0
    binaries = []
    for row in cfg["rows"]:
        if row["lane"] != "rust":
            continue
        if "package" in row:
            print("--exclude")
            print(row["package"])
        if "binary" in row:
            binaries.append(row["binary"])
    if binaries:
        print("-E")
        print("not (" + " | ".join("binary_id(=" + b + ")" for b in binaries) + ")")
    return 0


def new_file():
    return {"da": {}, "fnf": 0, "fnh": 0, "brf": 0, "brh": 0}


def parse_lcov(text):
    """The records of an lcov text as {SF: {da, fnf, fnh, brf, brh}}; anything unreadable is VOID."""
    if not text.strip():
        raise Void("the lcov is empty")
    files = {}
    current = None
    for number, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line:
            continue
        tag, _, value = line.partition(":")
        if current is None:
            if tag in ("TN", "VER"):
                continue
            if tag != "SF" or not value:
                raise Void("lcov line %d lies outside a record" % number)
            current = {
                "sf": value,
                "file": new_file(),
                "fn": 0,
                "fnda": 0,
                "br": 0,
                "brda": 0,
                "sums": set(),
            }
            continue
        if tag == "end_of_record":
            data = current["file"]
            if "FNF" not in current["sums"]:
                data["fnf"] = current["fn"]
                data["fnh"] = current["fnda"]
            if "BRF" not in current["sums"]:
                data["brf"] = current["br"]
                data["brh"] = current["brda"]
            merged = files.setdefault(current["sf"], new_file())
            for ln, hits in data["da"].items():
                merged["da"][ln] = max(merged["da"].get(ln, 0), hits)
            for key in ("fnf", "fnh", "brf", "brh"):
                merged[key] += data[key]
            current = None
        elif tag == "SF":
            raise Void("lcov line %d opens a record inside a record" % number)
        elif tag == "DA":
            parts = value.split(",")
            if len(parts) < 2 or not INTEGER.match(parts[0]) or not INTEGER.match(parts[1]):
                raise Void("lcov line %d holds a DA record that is not integers" % number)
            current["file"]["da"][int(parts[0])] = int(parts[1])
        elif tag in ("FNF", "FNH", "BRF", "BRH"):
            if not INTEGER.match(value):
                raise Void("lcov line %d holds a count that is not an integer" % number)
            current["sums"].add(tag)
            key = tag.lower()
            current["file"][key] = int(value)
        elif tag == "FN":
            current["fn"] += 1
        elif tag == "FNDA":
            parts = value.split(",")
            if not INTEGER.match(parts[0]):
                raise Void("lcov line %d holds a FNDA record that is not an integer" % number)
            if int(parts[0]) > 0:
                current["fnda"] += 1
        elif tag == "BRDA":
            parts = value.split(",")
            if len(parts) != 4:
                raise Void("lcov line %d holds a BRDA record that is not four fields" % number)
            current["br"] += 1
            if INTEGER.match(parts[3]) and int(parts[3]) > 0:
                current["brda"] += 1
    if current is not None:
        raise Void("the lcov ends inside a record, " + current["sf"])
    return files


def resolve_path(sf, root, lane_spec):
    """The repository path of an SF record, or None when it lies outside the lane."""
    if sf.startswith("/"):
        prefix = root.rstrip("/") + "/"
        if not sf.startswith(prefix):
            return None
        rel = os.path.normpath(sf[len(prefix) :])
        return rel if any_match(lane_spec["measures"], rel) else None
    for base in lane_spec["sf_bases"]:
        rel = os.path.normpath(os.path.join(base, sf))
        if any_match(lane_spec["measures"], rel):
            return rel
    return None


def parse_diff(text):
    """{path: {added line numbers}} and the count of quoted paths skipped."""
    changed = {}
    skipped = 0
    path = None
    for raw in text.splitlines():
        if raw.startswith("+++ "):
            target = raw[4:]
            if target.startswith('"'):
                skipped += 1
                path = None
            elif target.startswith("b/"):
                path = target[2:]
                changed.setdefault(path, set())
            else:
                path = None
        elif raw.startswith("@@") and path is not None:
            m = HUNK.match(raw)
            if m:
                start = int(m.group(1))
                length = 1 if m.group(2) is None else int(m.group(2))
                for ln in range(start, start + length):
                    changed[path].add(ln)
    return changed, skipped


def group_of(path, depth):
    parts = path.split("/")[:-1]
    return "/".join(parts[:depth]) if parts else "."


def read_llvm_regions(path, root, spec):
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        entries = data["data"][0]["files"]
    except (OSError, ValueError, KeyError, IndexError, TypeError) as err:
        raise Void("the llvm summary is unreadable: " + str(err)) from err
    regions = {}
    for entry in entries:
        rel = resolve_path(entry["filename"], root, spec)
        if rel is None:
            continue
        reg = entry["summary"]["regions"]
        regions[rel] = (int(reg["count"]), int(reg["covered"]))
    return regions


def measure(lane, cfg, lcov_path, root, diff_text=None, llvm_path=None):
    spec = cfg["lanes"][lane]
    try:
        text = Path(lcov_path).read_text(encoding="utf-8")
    except OSError as err:
        raise Void("the lcov is missing: " + str(err)) from err
    records = parse_lcov(text)
    examined = {}
    outside = 0
    dropped = 0
    for sf, data in records.items():
        rel = resolve_path(sf, root, spec)
        if rel is None:
            outside += 1
        elif any_match(spec["exclusions"], rel):
            dropped += 1
        else:
            merged = examined.setdefault(rel, new_file())
            for ln, hits in data["da"].items():
                merged["da"][ln] = max(merged["da"].get(ln, 0), hits)
            for key in ("fnf", "fnh", "brf", "brh"):
                merged[key] += data[key]
    lines = sum(len(d["da"]) for d in examined.values())
    if lines == 0:
        raise Void(
            "examined 0 lines in %d records, %d outside the lane, %d dropped"
            % (len(records), outside, dropped)
        )
    rows = len([r for r in cfg["rows"] if r["lane"] == lane])
    result = {
        "lane": lane,
        "lines": lines,
        "files": len(examined),
        "outside": outside,
        "rows": rows,
        "dropped": dropped,
        "examined": examined,
        "changed": None,
    }
    if diff_text is not None:
        changed, skipped = parse_diff(diff_text)
        executable = 0
        executed = 0
        not_executed = []
        lacks = []
        for path in sorted(changed):
            if not any_match(spec["measures"], path) or any_match(spec["exclusions"], path):
                continue
            data = examined.get(path)
            if data is None:
                lacks.append(path)
                continue
            for ln in sorted(changed[path]):
                if ln in data["da"]:
                    executable += 1
                    if data["da"][ln] > 0:
                        executed += 1
                    else:
                        not_executed.append("%s:%d" % (path, ln))
        result["changed"] = {
            "executable": executable,
            "executed": executed,
            "not_executed": not_executed,
            "lacks": lacks,
            "skipped": skipped,
        }
    if llvm_path:
        result["regions"] = read_llvm_regions(llvm_path, root, spec)
    return result


def group_rows(result, cfg):
    depth = cfg["lanes"][result["lane"]]["group_depth"]
    groups = {}
    for path, data in result["examined"].items():
        g = groups.setdefault(group_of(path, depth), [0, 0, 0, 0, 0, 0, 0])
        g[0] += len(data["da"])
        g[1] += len([h for h in data["da"].values() if h > 0])
        g[3] += data["fnf"]
        g[4] += data["fnh"]
        g[5] += data["brf"]
        g[6] += data["brh"]
    rows = []
    for name in sorted(groups):
        g = groups[name]
        rows.append(
            {
                "group": name,
                "lines": g[0],
                "executed": g[1],
                "not_executed": g[0] - g[1],
                "functions": g[3],
                "functions_executed": g[4],
                "branches": g[5],
                "branches_taken": g[6],
            }
        )
    if "regions" in result:
        by_group = {}
        for path, (count, covered) in result["regions"].items():
            if path in result["examined"]:
                key = group_of(path, depth)
                cur = by_group.setdefault(key, [0, 0])
                cur[0] += count
                cur[1] += covered
        for row in rows:
            row["regions"], row["regions_executed"] = by_group.get(row["group"], [0, 0])
    return rows


def table(rows, with_regions):
    heads = [
        "group",
        "lines",
        "lines executed",
        "lines not executed",
        "functions",
        "functions executed",
        "branches",
        "branches taken",
    ]
    keys = [
        "group",
        "lines",
        "executed",
        "not_executed",
        "functions",
        "functions_executed",
        "branches",
        "branches_taken",
    ]
    if with_regions:
        heads += ["regions", "regions executed"]
        keys += ["regions", "regions_executed"]
    out = ["| " + " | ".join(heads) + " |", "|" + "|".join(["---"] * len(heads)) + "|"]
    for row in rows:
        out.append("| " + " | ".join(str(row[k]) for k in keys) + " |")
    return out


def runner_exit(path):
    if not path:
        return None
    try:
        text = Path(path).read_text(encoding="utf-8").strip()
    except OSError:
        return "unknown"
    return text if INTEGER.match(text) else "unknown"


def cmd_report(args):
    cfg = load_config(args.config)
    lane = args.lane
    diff_text = None
    if args.diff and args.event != "push":
        try:
            diff_text = Path(args.diff).read_text(encoding="utf-8")
        except OSError as err:
            raise Void("the diff is unreadable: " + str(err)) from err
    result = measure(lane, cfg, args.lcov, args.root, diff_text, args.llvm_summary)
    out = [
        LINE_REPORT
        % (
            lane,
            result["lines"],
            result["files"],
            result["outside"],
            result["rows"],
            result["dropped"],
        )
    ]
    changed = result["changed"]
    entries = []
    if changed is not None:
        entries = changed["not_executed"]
        shown = " ".join(entries) if entries else NONE_WORD
        out.append(LINE_CHANGED % (changed["executable"], changed["executed"], shown))
        lacks = " ".join(changed["lacks"])
        out.append(LINE_LACKS % len(changed["lacks"]) + (" " + lacks if lacks else ""))
        if changed["skipped"]:
            out.append(LINE_SKIPPED % changed["skipped"])
    elif args.event == "push":
        out.append(LINE_PUSH)
    code = runner_exit(args.runner_exit_file)
    if code is not None:
        out.append(LINE_RUNNER % code)
    for line in out:
        print(line)
    rows = group_rows(result, cfg)
    with_regions = "regions" in result
    if args.summary:
        text = ["### coverage: " + lane, ""] + table(rows, with_regions) + [""]
        if changed is not None:
            text += [out[1], ""]
            for entry in entries[:SUMMARY_LIMIT]:
                text.append("- " + entry)
            if len(entries) > SUMMARY_LIMIT:
                text.append(MORE_LINE % (len(entries) - SUMMARY_LIMIT))
            text.append("")
        with Path(args.summary).open("a", encoding="utf-8") as fh:
            fh.write("\n".join(text) + "\n")
    if args.out_dir:
        os.makedirs(args.out_dir, exist_ok=True)
        body = list(out) + [""] + table(rows, with_regions) + [""] + entries
        Path(args.out_dir, "report.txt").write_text("\n".join(body) + "\n", encoding="utf-8")
        figures = {
            "lane": lane,
            "examined_lines": result["lines"],
            "examined_files": result["files"],
            "outside_files": result["outside"],
            "exclusion_rows": result["rows"],
            "dropped_files": result["dropped"],
            "groups": rows,
        }
        if changed is not None:
            figures["changed_executable"] = changed["executable"]
            figures["changed_executed"] = changed["executed"]
            figures["not_executed"] = entries
            figures["lacking_files"] = changed["lacks"]
        if code is not None:
            figures["runner_exit"] = code
        Path(args.out_dir, "report.json").write_text(
            json.dumps(figures, indent=2) + "\n", encoding="utf-8"
        )
    return 0


def cmd_controls(args):
    cfg = load_config(args.config)
    fixtures = Path(args.fixtures)
    cases = []

    def read(name):
        try:
            return fixtures / name, (fixtures / name).read_text(encoding="utf-8")
        except OSError as err:
            raise Void("cannot read the fixture " + name + ": " + str(err)) from err

    def reads_void(name):
        path, _ = read(name)
        try:
            measure("python", cfg, str(path), "/planted")
        except Void:
            return True
        return False

    def prints_uncovered():
        lcov, _ = read("uncovered.lcov")
        _, diff = read("uncovered.diff")
        got = measure("python", cfg, str(lcov), "/planted", diff)
        return got["changed"]["not_executed"] == ["scripts/planted_control.py:3"]

    def provider_void():
        _, text = read("provider-mismatch.txt")
        try:
            provider_versions(text)
        except Void:
            return True
        return False

    cases.append(("examined-none.lcov", lambda: reads_void("examined-none.lcov")))
    cases.append(("unclosed.lcov", lambda: reads_void("unclosed.lcov")))
    cases.append(("uncovered.lcov", prints_uncovered))
    if args.lane == "web":
        cases.append(("provider-mismatch.txt", provider_void))
    for name, check in cases:
        if not check():
            print(VOID_CONTROLS % ("the planted case " + name + " did not read as planted"))
            return 1
        print(LINE_CONTROL_OK % name)
    print(LINE_CONTROLS % (len(cases), len(cases)))
    return 0


def build_parser():
    parser = argparse.ArgumentParser(description="the advisory coverage report")
    sub = parser.add_subparsers(dest="command", required=True)

    plan = sub.add_parser("plan")
    plan.add_argument("--event", required=True)
    plan.add_argument("--base-ref", default="")
    plan.add_argument("--head-ref", default="")
    plan.add_argument("--head-repo", default="")
    plan.add_argument("--repo", default="")
    plan.add_argument("--changed", default="")
    plan.add_argument("--config", default=DEFAULT_CONFIG)
    plan.add_argument("--github-output", default="")

    provider = sub.add_parser("provider")
    provider.add_argument("--lockfile", required=True)

    flags = sub.add_parser("args")
    flags.add_argument("--lane", required=True, choices=LANES)
    flags.add_argument("--config", default=DEFAULT_CONFIG)

    controls = sub.add_parser("controls")
    controls.add_argument("--lane", required=True, choices=LANES)
    controls.add_argument("--fixtures", default=DEFAULT_FIXTURES)
    controls.add_argument("--config", default=DEFAULT_CONFIG)

    report = sub.add_parser("report")
    report.add_argument("--lane", required=True, choices=LANES)
    report.add_argument("--lcov", required=True)
    report.add_argument("--root", required=True)
    report.add_argument("--diff", default="")
    report.add_argument("--event", default="")
    report.add_argument("--llvm-summary", default="")
    report.add_argument("--runner-exit-file", default="")
    report.add_argument("--summary", default="")
    report.add_argument("--out-dir", default="")
    report.add_argument("--config", default=DEFAULT_CONFIG)
    return parser


def main(argv):
    args = build_parser().parse_args(argv)
    handlers = {
        "plan": (cmd_plan, VOID_PLAN),
        "provider": (cmd_provider, VOID_PROVIDER),
        "args": (cmd_args, VOID_ARGS),
        "controls": (cmd_controls, VOID_CONTROLS),
        "report": (cmd_report, None),
    }
    handler, template = handlers[args.command]
    try:
        return handler(args)
    except Void as err:
        if template is None:
            print(VOID_REPORT % (args.lane, err))
        else:
            print(template % err)
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
