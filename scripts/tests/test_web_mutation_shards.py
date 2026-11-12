"""The pull request's Mini App mutation sweep is dealt into three shards and read back by name
(SPEC-419, ADR-433).

A6 to A8 hold the deal over fixed tables of counts. A9 to A13 run the script as CI runs it, in
scratch directories. Its count is asked of a stand-in for StrykerJS's mutation server: a Python
program written at the path of StrykerJS's own binary in a scratch app and started as the `--node`
program, which logs how it was started and each request it reads, and answers one reply per request
in the server's framing. Every record and report a union reads here is literal JSON the test
writes, never the script it judges.
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

sys.path.insert(0, str(REPO / "scripts"))
import web_mutation_shards  # noqa: E402

SCRIPT = REPO / "scripts" / "web_mutation_shards.py"
BIN = "node_modules/@stryker-mutator/core/bin/stryker.js"
SCHEMA = "deckstreak-web-shard/1"
COUNTS = {
    "src/a.ts": 40,
    "src/b.ts": 30,
    "src/c.svelte": 20,
    "src/d.ts": 20,
    "src/e.ts": 10,
    "src/f.ts": 5,
    "src/g.ts": 0,
}
# COUNTS dealt largest first onto the lightest shard, by hand: loads 45, 40 and 40.
DEAL = [
    ["src/a.ts", "src/f.ts"],
    ["src/b.ts", "src/e.ts", "src/g.ts"],
    ["src/c.svelte", "src/d.ts"],
]
# The plan names its files in its own order, which is not the deal's.
PLANNED = ["src/g.ts", "src/d.ts", "src/a.ts", "src/f.ts", "src/c.svelte", "src/e.ts", "src/b.ts"]
TWO = {"src/a.ts": 4, "src/b.ts": 3}
TWO_DEAL = [["src/a.ts"], ["src/b.ts"], []]
REFUSED = {"code": -32601, "message": "refused"}
# The stand-in mutation server. It reads `fake.json` in its working directory: the counts it names
# a mutant list for, and a mode that makes it refuse a method, answer under another id, answer no
# `files`, answer what is not JSON, or read the first request and leave. After its second reply it
# lingers, as the real server does, so only the client can end it.
SERVER = r"""import json
import os
import sys
import time

os.close(2)
here = os.getcwd()
with open("fake.json", encoding="utf-8") as held:
    fake = json.load(held)
mode = fake["mode"]
log = open("asked.jsonl", "a", encoding="utf-8")


def note(value):
    log.write(json.dumps(value) + "\n")
    log.flush()


def read():
    length = None
    line = sys.stdin.buffer.readline()
    while line.strip():
        name, _, value = line.decode().partition(":")
        if name.strip().lower() == "content-length":
            length = int(value)
        line = sys.stdin.buffer.readline()
    if length is None:
        sys.exit(0)
    return json.loads(sys.stdin.buffer.read(length))


def send(body):
    sys.stdout.buffer.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
    sys.stdout.buffer.flush()


files = {
    path: {"mutants": [{"id": path + "#" + str(n)} for n in range(count)]}
    for path, count in fake["counts"].items()
}
results = {
    "configure": {"version": "stand-in"},
    "discover": {"other": 1} if mode == "no-files" else {"files": files},
}
note({"argv": sys.argv, "cwd": here})
for _ in range(2):
    asked = read()
    note(asked)
    if mode == "silent":
        sys.exit(0)
    method = asked.get("method")
    reply = {"jsonrpc": "2.0", "id": asked.get("id")}
    if mode == "error-" + str(method) or method not in results:
        reply["error"] = {"code": -32601, "message": "refused"}
    else:
        reply["result"] = results[method]
    if mode == "wrong-id":
        reply["id"] = "other"
    send(b"not json" if mode == "garbage" else json.dumps(reply).encode())
time.sleep(60)
"""


def run(*arguments, output=None):
    """The script as CI runs it; GITHUB_OUTPUT is set only when `output` names a file."""
    env = {name: value for name, value in os.environ.items() if name != "GITHUB_OUTPUT"}
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    if output is not None:
        env["GITHUB_OUTPUT"] = str(output)
    return subprocess.run(
        [sys.executable, str(SCRIPT), *(str(argument) for argument in arguments)],
        capture_output=True,
        text=True,
        env=env,
        timeout=30,
        check=False,
    )


def write(path, value):
    """`value` at `path`, as text when it is text and as JSON otherwise."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value if isinstance(value, str) else json.dumps(value), encoding="utf-8")
    return path


def load(path):
    """The JSON a file holds."""
    return json.loads(path.read_text(encoding="utf-8"))


def plan(root, files):
    """A plan as `mutation-verdict.py plan` writes it, cut to what the shards read."""
    web = {"applies": bool(files), "files": [f"web/app/{path}" for path in files]}
    return write(root / "plan.json", {"classes": {"web": web}, "stryker_mutate": files})


def app(root, counts, mode="answer"):
    """A scratch app whose StrykerJS binary is the stand-in server."""
    where = root / "app"
    write(where / BIN, SERVER)
    write(where / "fake.json", {"counts": counts, "mode": mode})
    return where


def asked(where):
    """What the stand-in logged: how it was started, then each request it read; None if it never
    started."""
    log = where / "asked.jsonl"
    if not log.is_file():
        return None
    return [json.loads(line) for line in log.read_text(encoding="utf-8").splitlines()]


def dealing(plan_path, shard, out, where, node=sys.executable):
    """The deal's command line, as a shard runs it."""
    return [
        "deal",
        "--plan",
        plan_path,
        "--shard",
        shard,
        "--out",
        out,
        "--app",
        where,
        "--node",
        node,
    ]


def uniting(plan_path, reports, out, result="success"):
    """The union's command line, as the verdict job runs it."""
    return ["union", "--plan", plan_path, "--reports", reports, "--result", result, "--out", out]


def record(shard, counts, dealt):
    """The record a shard writes, from a deal written out by hand."""
    files = dealt[shard]
    mutants = sum(counts[path] for path in files)
    return {
        "schema": SCHEMA,
        "shard": shard,
        "shards": 3,
        "counts": counts,
        "deal": dealt,
        "files": files,
        "mutants": mutants,
        "empty": mutants == 0,
    }


def report(counts, files):
    """A StrykerJS report over `files`, each with its counted mutants."""
    return {
        "files": {
            path: {
                "language": "typescript",
                "source": f"export const at = '{path}';\n",
                "mutants": [{"id": f"{path}#{n}", "status": "Killed"} for n in range(counts[path])],
            }
            for path in files
        }
    }


def shard_tree(root, counts, dealt, changes=None):
    """The three shards' directories as the verdict job downloads them, each written as its shard
    writes it, after `changes` (a relative path to a value, or to None for no file)."""
    files = {}
    for shard in range(3):
        name = f"mutation-web-shard-{shard}"
        files[f"{name}/shard.json"] = record(shard, counts, dealt)
        if sum(counts[path] for path in dealt[shard]):
            files[f"{name}/stryker.exit"] = "0\n"
            files[f"{name}/mutation.json"] = report(counts, dealt[shard])
    files.update(changes or {})
    where = root / "shards"
    where.mkdir(parents=True)
    for relative, value in files.items():
        if value is not None:
            write(where / relative, value)
    return where


def merged(counts, dealt):
    """Every shard's report files in one map."""
    found = {}
    for files in dealt:
        found.update(report(counts, files)["files"])
    return found


def dealt_line(shard, files, mutants):
    """What the deal prints for one shard."""
    return f"web shards: shard {shard} of 3 holds {files} file(s), {mutants} mutant(s)\n"


def void_lines(findings, whole):
    """What the union prints when it reads VOID."""
    lines = [f"web shards: {finding}" for finding in findings]
    lines += [
        f"web shards: counted {whole} of 3 reports whole",
        "examined 3 report(s)",
        f"web shards: verdict: VOID: {len(findings)} finding(s), so no report is written",
    ]
    return "".join(f"{line}\n" for line in lines)


def undealt(held, planned):
    """The union's finding when the shards do not hold every planned file exactly once."""
    return f"UNDEALT: the shards hold {held} file(s) and the plan names {planned}"


def ok_lines(out, files):
    """What the union prints when every shard is whole."""
    return (
        "web shards: counted 3 of 3 reports whole\n"
        "examined 3 report(s)\n"
        f"web shards: verdict: ok, the union of {files} file(s) is {out}\n"
    )


def without(arguments, flag):
    """A command line with one flag and its value taken out."""
    at = arguments.index(flag)
    return arguments[:at] + arguments[at + 2 :]


class TheDealPutsTheLargestFileOntoTheLightestShard(unittest.TestCase):
    """A6 to A8 (R2): whole files, by mutant count, largest first onto the lightest shard."""

    def test_the_deal_puts_the_largest_file_first_onto_the_lightest_shard(self):
        self.assertEqual(web_mutation_shards.deal(COUNTS, 3), DEAL)
        self.assertEqual([sum(COUNTS[path] for path in files) for files in DEAL], [45, 40, 40])

    def test_equal_counts_go_by_path_and_equal_shards_by_the_lowest_index(self):
        tied = {"src/z.ts": 7, "src/y.ts": 7, "src/x.ts": 7, "src/w.ts": 7}
        self.assertEqual(
            web_mutation_shards.deal(tied, 3),
            [["src/w.ts", "src/z.ts"], ["src/x.ts"], ["src/y.ts"]],
        )
        self.assertEqual(
            web_mutation_shards.deal({"src/b.ts": 0, "src/a.ts": 0}, 3),
            [["src/a.ts", "src/b.ts"], [], []],
        )

    def test_every_file_is_dealt_to_exactly_one_shard_and_the_total_is_kept(self):
        counts = {f"src/f{index:02}.ts": (index * 37) % 101 for index in range(40)}
        for shards in examined("shard counts", range(1, 5)):
            with self.subTest(shards=shards):
                dealt = web_mutation_shards.deal(counts, shards)
                self.assertEqual(len(dealt), shards)
                held = [path for files in dealt for path in files]
                self.assertEqual(sorted(held), sorted(counts))
                loads = [sum(counts[path] for path in files) for files in dealt]
                self.assertEqual(sum(loads), sum(counts.values()))
                self.assertLessEqual(max(loads) - min(loads), max(counts.values()))
        held = []
        for shard in examined("shards dealt by the command line", range(3)):
            with self.subTest(shard=shard), tempfile.TemporaryDirectory() as scratch:
                root = Path(scratch)
                out = root / "shards"
                done = run(*dealing(plan(root, PLANNED), shard, out, app(root, COUNTS)))
                mutants = sum(COUNTS[path] for path in DEAL[shard])
                self.assertEqual(
                    (done.returncode, done.stdout),
                    (0, dealt_line(shard, len(DEAL[shard]), mutants)),
                    done.stderr,
                )
                written = load(out / f"mutation-web-shard-{shard}" / "shard.json")
                self.assertEqual(written, record(shard, COUNTS, DEAL))
                held += written["files"]
        self.assertEqual(sorted(held), sorted(PLANNED))


class EachShardRecordsItsDeal(unittest.TestCase):
    """A9 and A10 (R2, R4, R5): the count is StrykerJS's own, and every shard writes its record."""

    def test_a_shard_dealt_nothing_writes_an_explicit_empty_record(self):
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        out = root / "shards"
        # A re-run finds its own directory already there.
        (out / "mutation-web-shard-2").mkdir(parents=True)
        output = write(root / "github-output", "")
        where = app(root / "two", TWO)
        done = run(*dealing(plan(root / "two", list(TWO)), 2, out, where), output=output)
        self.assertEqual((done.returncode, done.stdout), (0, dealt_line(2, 0, 0)), done.stderr)
        written = load(out / "mutation-web-shard-2" / "shard.json")
        self.assertEqual(written, record(2, TWO, TWO_DEAL))
        self.assertIs(written["empty"], True)
        self.assertEqual(output.read_text(encoding="utf-8"), "run=false\nmutate=\n")

        # A shard dealt only a file with no mutant runs no StrykerJS either.
        output = write(root / "github-output", "")
        zero = {"src/g.ts": 0}
        where = app(root / "zero", zero)
        done = run(*dealing(plan(root / "zero", ["src/g.ts"]), 0, out, where), output=output)
        self.assertEqual((done.returncode, done.stdout), (0, dealt_line(0, 1, 0)), done.stderr)
        written = load(out / "mutation-web-shard-0" / "shard.json")
        self.assertEqual(written, record(0, zero, [["src/g.ts"], [], []]))
        self.assertEqual(output.read_text(encoding="utf-8"), "run=false\nmutate=src/g.ts\n")

        # A plan that names no web file: every shard writes its empty record and starts nothing.
        none = root / "none"
        for shard in examined("shards of a plan with no web file", range(3)):
            with self.subTest(shard=shard):
                output = write(none / f"github-output-{shard}", "")
                arguments = dealing(plan(none, []), shard, none / "shards", none / "app")
                arguments[-1] = none / "no-node"
                done = run(*arguments, output=output)
                self.assertEqual(
                    (done.returncode, done.stdout), (0, dealt_line(shard, 0, 0)), done.stderr
                )
                written = load(none / "shards" / f"mutation-web-shard-{shard}" / "shard.json")
                self.assertEqual(written, record(shard, {}, [[], [], []]))
                self.assertEqual(output.read_text(encoding="utf-8"), "run=false\nmutate=\n")
        self.assertIsNone(asked(none / "app"))

        # The union reads an empty record by name, as whole as any other.
        for name, counts, dealt in (("none", {}, [[], [], []]), ("two", TWO, TWO_DEAL)):
            with self.subTest(union=name):
                base = root / "union" / name
                reports = shard_tree(base, counts, dealt)
                union_out = base / "mutation.json"
                done = run(*uniting(plan(base, list(counts)), reports, union_out))
                self.assertEqual(
                    (done.returncode, done.stdout),
                    (0, ok_lines(union_out, len(counts))),
                    done.stderr,
                )
                self.assertEqual(load(union_out), {"files": merged(counts, dealt)})

    def test_the_count_is_strykers_own_discover_over_the_planned_files(self):
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        # The server names no mutant for src/g.ts, which then counts 0.
        named = {path: count for path, count in COUNTS.items() if path != "src/g.ts"}
        where = app(root, named)
        out = root / "runner" / "shards"
        output = write(root / "github-output", "earlier=kept\n")
        done = run(*dealing(plan(root, PLANNED), 0, out, where), output=output)
        self.assertEqual((done.returncode, done.stdout), (0, dealt_line(0, 2, 45)), done.stderr)
        self.assertEqual(
            asked(where),
            [
                {
                    "argv": [BIN, "serve", "stdio", "--", "--mutate", ",".join(PLANNED)],
                    "cwd": str(where.resolve()),
                },
                {"jsonrpc": "2.0", "id": 1, "method": "configure", "params": {}},
                {"jsonrpc": "2.0", "id": 2, "method": "discover", "params": {}},
            ],
        )
        self.assertEqual(load(out / "mutation-web-shard-0" / "shard.json"), record(0, COUNTS, DEAL))
        self.assertEqual(
            output.read_text(encoding="utf-8"), "earlier=kept\nrun=true\nmutate=src/a.ts,src/f.ts\n"
        )

        refused = "the mutation server refused"
        unread = "the mutation server could not be read"
        cases = [
            (
                "configure refused",
                named,
                "error-configure",
                f"{refused} configure: " + str({"jsonrpc": "2.0", "id": 1, "error": REFUSED}),
            ),
            (
                "discover refused",
                named,
                "error-discover",
                f"{refused} discover: " + str({"jsonrpc": "2.0", "id": 2, "error": REFUSED}),
            ),
            (
                "an answer under another id",
                named,
                "wrong-id",
                f"{refused} configure: "
                + str({"jsonrpc": "2.0", "id": "other", "result": {"version": "stand-in"}}),
            ),
            ("a silent server", named, "silent", "the mutation server answered no Content-Length"),
            (
                "an answer that is not JSON",
                named,
                "garbage",
                f"{unread}: Expecting value: line 1 column 1 (char 0)",
            ),
            ("no files", named, "no-files", "discover answered no files: {'other': 1}"),
            (
                "a stray file",
                {**named, "src/z.ts": 3},
                "answer",
                "discover answered src/z.ts, which the plan does not name",
            ),
            ("no binary", named, "answer", None),
        ]
        for label, counts, mode, reason in examined("refused counts", cases):
            with self.subTest(case=label), tempfile.TemporaryDirectory() as scratch:
                base = Path(scratch)
                node = base / "no-node" if reason is None else sys.executable
                if reason is None:
                    reason = f"{unread}: [Errno 2] No such file or directory: '{node}'"
                output = write(base / "github-output", "earlier=kept\n")
                arguments = dealing(
                    plan(base, PLANNED), 0, base / "shards", app(base, counts, mode)
                )
                arguments[-1] = node
                done = run(*arguments, output=output)
                self.assertEqual(
                    (done.returncode, done.stdout),
                    (3, f"web shards: VOID the count: {reason}\n"),
                    done.stderr,
                )
                self.assertFalse((base / "shards").exists())
                self.assertEqual(output.read_text(encoding="utf-8"), "earlier=kept\n")


class TheUnionReadsEveryShardByName(unittest.TestCase):
    """A11 and A12 (R5, R6, R7): one report from three shards read by name, or VOID by name."""

    def test_the_union_reads_every_shard_by_name_and_merges_their_files(self):
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        reports = shard_tree(root, COUNTS, DEAL)
        out = root / "runner" / "mutation" / "stryker" / "mutation.json"
        done = run(*uniting(plan(root, PLANNED), reports, out))
        self.assertEqual((done.returncode, done.stdout), (0, ok_lines(out, 7)), done.stderr)
        self.assertEqual(load(out), {"files": merged(COUNTS, DEAL)})

    def test_a_missing_cut_or_partial_shard_reads_void_and_writes_nothing(self):
        shard = "mutation-web-shard-"
        first = record(0, COUNTS, DEAL)
        second = record(1, COUNTS, DEAL)
        split = {**COUNTS, "src/f.ts": 25}
        split_deal = [
            ["src/a.ts", "src/e.ts"],
            ["src/b.ts", "src/d.ts"],
            ["src/f.ts", "src/c.svelte", "src/g.ts"],
        ]
        gone = {
            f"{shard}1/{name}": None for name in ("shard.json", "stryker.exit", "mutation.json")
        }
        cases = [
            (
                "a shard's artifact is missing",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                gone,
                [f"MISSING: {shard}1 holds no readable shard.json", undealt(4, 7)],
                2,
            ),
            (
                "the shards' job was cut",
                COUNTS,
                DEAL,
                PLANNED,
                "cancelled",
                {},
                ["CUT: the shard jobs read cancelled, not success"],
                3,
            ),
            (
                "a shard's StrykerJS never finished",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}2/stryker.exit": None},
                [
                    f"CUT: {shard}2 holds no stryker.exit, so its StrykerJS step never finished",
                    undealt(5, 7),
                ],
                2,
            ),
            (
                "a shard kept no report",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/mutation.json": None},
                [f"MISSING: {shard}0 holds no mutation.json with a files map", undealt(5, 7)],
                2,
            ),
            (
                "a report with no files map",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/mutation.json": {"files": []}},
                [f"MISSING: {shard}0 holds no mutation.json with a files map", undealt(5, 7)],
                2,
            ),
            (
                "a report one mutant short",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/mutation.json": report({**COUNTS, "src/a.ts": 39}, DEAL[0])},
                [f"PARTIAL: {shard}0 reports 39 of 40 mutant(s) of src/a.ts"],
                2,
            ),
            (
                "a report of a file not dealt to it",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/mutation.json": report(COUNTS, [*DEAL[0], "src/b.ts"])},
                [f"FOREIGN: {shard}0 reports src/b.ts, which is not dealt to it"],
                2,
            ),
            (
                "a shard that ran other files than its share",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/shard.json": {**first, "files": ["src/a.ts"]}},
                [
                    f"MISDEALT: {shard}0 holds ['src/a.ts'], not its share "
                    "['src/a.ts', 'src/f.ts']",
                    undealt(5, 7),
                ],
                2,
            ),
            (
                "a file split across two shards",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {
                    f"{shard}2/shard.json": record(2, split, split_deal),
                    f"{shard}2/mutation.json": report(split, split_deal[2]),
                },
                [undealt(8, 7)],
                3,
            ),
            (
                "a plan that names a file no shard counted",
                COUNTS,
                DEAL,
                [*PLANNED, "src/h.ts"],
                "success",
                {},
                [
                    f"PARTIAL: {shard}0 counted other files than the plan names",
                    f"PARTIAL: {shard}1 counted other files than the plan names",
                    f"PARTIAL: {shard}2 counted other files than the plan names",
                    undealt(0, 8),
                ],
                0,
            ),
            (
                "counts that are not a map",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/shard.json": {**first, "counts": PLANNED}},
                [f"PARTIAL: {shard}0 counted other files than the plan names", undealt(5, 7)],
                2,
            ),
            (
                "a record of another shard",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}1/shard.json": {**second, "shard": 0}},
                [f"PARTIAL: {shard}1 is not shard 1 of 3", undealt(4, 7)],
                2,
            ),
            (
                "a record of another schema",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/shard.json": {**first, "schema": "deckstreak-web-shard/0"}},
                [f"PARTIAL: {shard}0 is not shard 0 of 3", undealt(5, 7)],
                2,
            ),
            (
                "a record of another shard count",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/shard.json": {**first, "shards": 4}},
                [f"PARTIAL: {shard}0 is not shard 0 of 3", undealt(5, 7)],
                2,
            ),
            (
                "a record that does not parse",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/shard.json": "{"},
                [f"MISSING: {shard}0 holds no readable shard.json", undealt(5, 7)],
                2,
            ),
            (
                "a record that is not a map",
                COUNTS,
                DEAL,
                PLANNED,
                "success",
                {f"{shard}0/shard.json": []},
                [f"MISSING: {shard}0 holds no readable shard.json", undealt(5, 7)],
                2,
            ),
            (
                "an empty shard's record left absent",
                TWO,
                TWO_DEAL,
                list(TWO),
                "success",
                {f"{shard}2/shard.json": None},
                [f"MISSING: {shard}2 holds no readable shard.json"],
                2,
            ),
        ]
        for label, counts, dealt, planned, result, changes, findings, whole in examined(
            "planted shards", cases
        ):
            with self.subTest(case=label), tempfile.TemporaryDirectory() as scratch:
                root = Path(scratch)
                reports = shard_tree(root, counts, dealt, changes)
                out = root / "mutation.json"
                done = run(*uniting(plan(root, planned), reports, out, result))
                self.assertEqual(
                    (done.returncode, done.stdout), (3, void_lines(findings, whole)), done.stderr
                )
                self.assertFalse(out.exists())


class TheCommandLine(unittest.TestCase):
    """A13 (R2, R4): what the command line refuses, and the deal's outputs."""

    def test_the_command_line_refuses_what_it_cannot_read(self):
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        empty = plan(root, [])
        out = root / "shards"
        deal = dealing(empty, 0, out, root / "app", root / "no-node")
        union = uniting(empty, root / "reports", root / "mutation.json")
        lines = [
            ([], "the following arguments are required"),
            (["merge"], "invalid choice: 'merge'"),
            *(
                (without(deal, flag), f"the following arguments are required: {flag}")
                for flag in ("--plan", "--shard", "--out", "--app", "--node")
            ),
            *(
                (without(union, flag), f"the following arguments are required: {flag}")
                for flag in ("--plan", "--reports", "--result", "--out")
            ),
            ([*deal[:4], 3, *deal[5:]], "argument --shard: invalid choice: '?3'? "),
            ([*deal[:4], "one", *deal[5:]], "argument --shard: invalid int value: 'one'"),
        ]
        for arguments, said in examined("refused command lines", lines):
            with self.subTest(arguments=[str(argument) for argument in arguments]):
                done = run(*arguments)
                self.assertEqual((done.returncode, done.stdout), (2, ""), done.stderr)
                self.assertRegex(done.stderr, said)
                self.assertFalse(out.exists())

        plans = {
            "absent": None,
            "not JSON": "{",
            "not a map": [],
            "no stryker_mutate": {},
            "a stryker_mutate that is not a list": {"stryker_mutate": "src/a.ts"},
        }
        for label, text in examined("unreadable plans", plans.items()):
            for verb in ("deal", "union"):
                with self.subTest(plan=label, verb=verb), tempfile.TemporaryDirectory() as scratch:
                    base = Path(scratch)
                    path = base / "plan.json"
                    if text is not None:
                        write(path, text)
                    output = write(base / "github-output", "earlier=kept\n")
                    if verb == "deal":
                        arguments = dealing(path, 1, base / "shards", base / "app", sys.executable)
                    else:
                        arguments = uniting(path, base / "shards", base / "mutation.json")
                    done = run(*arguments, output=output)
                    self.assertEqual(
                        (done.returncode, done.stdout),
                        (3, f"web shards: VOID {path} holds no plan with a stryker_mutate list\n"),
                        done.stderr,
                    )
                    self.assertEqual(
                        sorted(child.name for child in base.iterdir()),
                        ["github-output"] if text is None else ["github-output", "plan.json"],
                    )
                    self.assertEqual(output.read_text(encoding="utf-8"), "earlier=kept\n")

        # The deal appends its two outputs to what the job wrote before, and with no
        # GITHUB_OUTPUT it still writes its record.
        output = write(root / "github-output", "earlier=kept\n")
        done = run(*deal, output=output)
        self.assertEqual((done.returncode, done.stdout), (0, dealt_line(0, 0, 0)), done.stderr)
        self.assertEqual(output.read_text(encoding="utf-8"), "earlier=kept\nrun=false\nmutate=\n")
        again = root / "again"
        done = run(*dealing(empty, 1, again, root / "app", root / "no-node"))
        self.assertEqual((done.returncode, done.stdout), (0, dealt_line(1, 0, 0)), done.stderr)
        self.assertEqual(
            load(again / "mutation-web-shard-1" / "shard.json"), record(1, {}, [[], [], []])
        )


if __name__ == "__main__":
    unittest.main()
