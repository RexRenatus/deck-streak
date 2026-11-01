#!/usr/bin/env python3
"""The pull request's Mini App mutation sweep in three shards (SPEC-419, ADR-433).

`deal` runs in each shard. It asks StrykerJS's own mutation server how many mutants each file the
plan names holds, deals the files whole by that count, largest first onto the shard with the fewest
mutants so far, and writes the shard's record and the two outputs its StrykerJS step reads. A shard
dealt no mutant writes its record all the same, marked empty, so it is read by name, never absent.

`union` runs in the job of the leg's own name. It reads every shard's record and StrykerJS report by
name and writes their files as the one report the unchanged judge reads, or reads VOID by name, exit
3, and writes nothing. Success falls through to exit 0; only a refusal returns a code.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

SHARDS = 3
PREFIX = "mutation-web-shard-"
RECORD = "shard.json"
REPORT = "mutation.json"
EXITED = "stryker.exit"
SCHEMA = "deckstreak-web-shard/1"
STRYKER_BIN = "node_modules/@stryker-mutator/core/bin/stryker.js"
EXIT_VOID = 3


class Refused(Exception):
    """A count or a record this script cannot read."""


def deal(counts, shards):
    """Whole files dealt by mutant count, largest first, each onto the shard holding the fewest
    mutants so far: equal counts go by path and equally light shards by the lowest index."""
    dealt = [[] for _ in range(shards)]
    for path in sorted(counts, key=lambda name: (-counts[name], name)):
        loads = [sum(counts[held] for held in hand) for hand in dealt]
        dealt[min(range(shards), key=lambda index: (loads[index], index))].append(path)
    return dealt


def stryker_command(node, files):
    """StrykerJS's mutation server over exactly `files`, run from the app so it reads the run's own
    configuration; the binary is named relative to the app, the server's working directory."""
    return [node, STRYKER_BIN, "serve", "stdio", "--", "--mutate", ",".join(files)]


def receive(stream):
    """One message in the server's framing: header lines, a blank line, then Content-Length bytes."""
    headers = {}
    line = stream.readline()
    while line.strip():
        name, _, value = line.decode().partition(":")
        headers[name.strip().lower()] = value.strip()
        line = stream.readline()
    if "content-length" not in headers:
        raise Refused("the mutation server answered no Content-Length")
    return json.loads(stream.read(int(headers["content-length"])))


def ask(server, ident, method):
    """One request and its one reply, refused unless the reply answers this request's id."""
    body = json.dumps({"jsonrpc": "2.0", "id": ident, "method": method, "params": {}}).encode()
    server.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
    server.stdin.flush()
    message = receive(server.stdout)
    if message.get("id") != ident or "error" in message:
        raise Refused(f"the mutation server refused {method}: {message}")
    return message.get("result")


def discover(command, cwd):
    """The server's `configure` then its `discover` result. The server never exits when its input
    closes, so the client ends it."""
    try:
        with subprocess.Popen(
            command, cwd=cwd, stdin=subprocess.PIPE, stdout=subprocess.PIPE
        ) as server:
            try:
                ask(server, 1, "configure")
                return ask(server, 2, "discover")
            finally:
                server.kill()
    except (OSError, ValueError) as error:
        raise Refused(f"the mutation server could not be read: {error}") from error


def count_mutants(files, result):
    """Each planned file's mutants as discover names them; a file it names none for counts 0, and a
    file it names that the plan does not is refused."""
    found = result.get("files") if isinstance(result, dict) else None
    if not isinstance(found, dict):
        raise Refused(f"discover answered no files: {result}")
    stray = sorted(set(found) - set(files))
    if stray:
        raise Refused(f"discover answered {stray[0]}, which the plan does not name")
    return {path: len(found.get(path, {}).get("mutants", [])) for path in files}


def shard_record(shard, counts):
    """What one shard writes: the whole deal of every count, and its own share of it."""
    dealt = deal(counts, SHARDS)
    files = dealt[shard]
    mutants = sum(counts[path] for path in files)
    return {
        "schema": SCHEMA,
        "shard": shard,
        "shards": SHARDS,
        "counts": counts,
        "deal": dealt,
        "files": files,
        "mutants": mutants,
        "empty": mutants == 0,
    }


def read_json(path):
    """The JSON a file holds, or None when it is absent or does not parse."""
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def planned_files(path):
    """The plan's `stryker_mutate` list, or None when the plan holds none."""
    plan = read_json(path)
    planned = plan.get("stryker_mutate") if isinstance(plan, dict) else None
    return planned if isinstance(planned, list) else None


def run_deal(args):
    """Counts the plan's files, deals them, and writes this shard's record and outputs."""
    planned = planned_files(args.plan)
    if planned is None:
        print(f"web shards: VOID {args.plan} holds no plan with a stryker_mutate list")
        return EXIT_VOID
    counts = {}
    if planned:
        try:
            counts = count_mutants(planned, discover(stryker_command(args.node, planned), args.app))
        except Refused as refused:
            print(f"web shards: VOID the count: {refused}")
            return EXIT_VOID
    record = shard_record(args.shard, counts)
    where = args.out / f"{PREFIX}{args.shard}"
    where.mkdir(parents=True, exist_ok=True)
    (where / RECORD).write_text(json.dumps(record), encoding="utf-8")
    print(
        f"web shards: shard {args.shard} of {SHARDS} holds {len(record['files'])} file(s), "
        f"{record['mutants']} mutant(s)"
    )
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as sink:
            sink.write(
                f"run={'true' if record['mutants'] else 'false'}\n"
                f"mutate={','.join(record['files'])}\n"
            )


def read_shard(where, shard, planned):
    """One shard's dealt files, its report's files and its findings, read by its name."""
    name = f"{PREFIX}{shard}"
    record = read_json(where / name / RECORD)
    if not isinstance(record, dict):
        return [], {}, [f"MISSING: {name} holds no readable {RECORD}"]
    if (record.get("schema"), record.get("shard"), record.get("shards")) != (SCHEMA, shard, SHARDS):
        return [], {}, [f"PARTIAL: {name} is not shard {shard} of {SHARDS}"]
    counts = record.get("counts")
    if not isinstance(counts, dict) or sorted(counts) != sorted(planned):
        return [], {}, [f"PARTIAL: {name} counted other files than the plan names"]
    files = deal(counts, SHARDS)[shard]
    if record.get("files") != files:
        return [], {}, [f"MISDEALT: {name} holds {record.get('files')}, not its share {files}"]
    if sum(counts[path] for path in files) == 0:
        return files, {}, []
    if not (where / name / EXITED).is_file():
        return [], {}, [f"CUT: {name} holds no {EXITED}, so its StrykerJS step never finished"]
    report = read_json(where / name / REPORT)
    found = report.get("files") if isinstance(report, dict) else None
    if not isinstance(found, dict):
        return [], {}, [f"MISSING: {name} holds no {REPORT} with a files map"]
    problems = [
        f"FOREIGN: {name} reports {path}, which is not dealt to it"
        for path in sorted(found)
        if path not in files
    ]
    for path in files:
        held = len(found.get(path, {}).get("mutants", []))
        if held != counts[path]:
            problems.append(f"PARTIAL: {name} reports {held} of {counts[path]} mutant(s) of {path}")
    return files, found, problems


def union(planned, reports, result):
    """Every shard read by name: the merged report, the findings, and how many shards were whole."""
    findings = [] if result == "success" else [f"CUT: the shard jobs read {result}, not success"]
    merged, held, whole = {}, [], 0
    for shard in range(SHARDS):
        files, found, problems = read_shard(reports, shard, planned)
        findings += problems
        held += files
        merged.update(found)
        if not problems:
            whole += 1
    if sorted(held) != sorted(planned):
        findings.append(
            f"UNDEALT: the shards hold {len(held)} file(s) and the plan names {len(planned)}"
        )
    return {"files": merged}, findings, whole


def run_union(args):
    """Reads every shard by name and writes the union, or reads VOID and writes nothing."""
    planned = planned_files(args.plan)
    if planned is None:
        print(f"web shards: VOID {args.plan} holds no plan with a stryker_mutate list")
        return EXIT_VOID
    report, findings, whole = union(planned, args.reports, args.result)
    for finding in findings:
        print(f"web shards: {finding}")
    print(f"web shards: counted {whole} of {SHARDS} reports whole")
    print(f"examined {SHARDS} report(s)")
    if findings:
        print(f"web shards: verdict: VOID: {len(findings)} finding(s), so no report is written")
        return EXIT_VOID
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report), encoding="utf-8")
    print(f"web shards: verdict: ok, the union of {len(report['files'])} file(s) is {args.out}")


def main(argv=None):
    parser = argparse.ArgumentParser()
    verbs = parser.add_subparsers(required=True)
    dealing = verbs.add_parser("deal")
    for flag in ("--plan", "--out", "--app"):
        dealing.add_argument(flag, type=Path, required=True)
    dealing.add_argument("--node", required=True)
    dealing.add_argument("--shard", type=int, choices=range(SHARDS), required=True)
    dealing.set_defaults(run=run_deal)
    uniting = verbs.add_parser("union")
    for flag in ("--plan", "--reports", "--out"):
        uniting.add_argument(flag, type=Path, required=True)
    uniting.add_argument("--result", required=True)
    uniting.set_defaults(run=run_union)
    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
