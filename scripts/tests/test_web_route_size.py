"""The route-size check gives each web route the JavaScript it needs beyond the shell, compares it
with the route's entry in the budget file, and reports, at warn, a route over its entry, a route
with no entry, an entry for a route that is gone, and a route more than the margin under its entry;
it reads VOID, never a pass, when the build or the table does not read or holds no route (SPEC-652
A1 to A8, ADR-674).

Every case but two runs the check as a command over a planted app: a client build in the form the
pinned SvelteKit and Vite leave it (a manifest of entries, each naming its file, its static and
dynamic imports and its CSS, and a generated node for each page and layout) and the pages under
`src/routes`. The expected figures are measured here by gzip itself and summed by the rule written
here, never read from the check. Two cases load the check: one reads the exit a finding earns at the
fail level, which the repository's warn level cannot show, and one the bytecode setting its load
leaves."""

import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined
from test_mutation_python_judge_kills import load

CHECK = REPO / "scripts" / "web-route-size.py"
# The ratchet's margin in bytes gzip -9, written here apart from the check.
MARGIN = 1024
# Where the client build leaves its generated nodes and its output, under the app.
NODES = ".svelte-kit/generated/build/client-optimized/nodes"
CLIENT = Path(".svelte-kit") / "output" / "client"
MANIFEST = CLIENT / ".vite" / "manifest.json"
FAR = "_app/immutable/chunks/far.js"

web_route_size = load(CHECK, "web_route_size")


def node(number):
    """The manifest key, and source, of the generated node `number`."""
    return f"{NODES}/{number}.js"


# A client build: the start and app entries, the root layout (node 0), the error page (node 1,
# whose node names no route file), the root page (node 2), a page at /a (node 3), a layout at /b
# (node 4) with a page at /b/c under it (node 5), and a page in a group's folder (node 6). /a
# imports a chunk that imports another, which imports the first back, and a chunk the root layout
# imports too, which is the shell's; it imports one chunk dynamically and carries a stylesheet,
# neither of which is its JavaScript. The app entry imports every node dynamically, as the client
# app does.
BUILD = {
    "entry/start.js": {
        "file": "_app/immutable/entry/start.js",
        "isEntry": True,
        "imports": ["_runtime.js"],
    },
    "entry/app.js": {
        "file": "_app/immutable/entry/app.js",
        "isEntry": True,
        "imports": ["_runtime.js"],
        "dynamicImports": [node(n) for n in range(7)],
    },
    "_runtime.js": {"file": "_app/immutable/chunks/runtime.js"},
    node(0): {
        "file": "_app/immutable/nodes/0.js",
        "src": node(0),
        "isEntry": True,
        "imports": ["_shared.js"],
    },
    node(1): {"file": "_app/immutable/nodes/1.js", "src": node(1), "isEntry": True},
    node(2): {"file": "_app/immutable/nodes/2.js", "src": node(2), "isEntry": True},
    node(3): {
        "file": "_app/immutable/nodes/3.js",
        "src": node(3),
        "isEntry": True,
        "imports": ["_near.js", "_shared.js"],
        "dynamicImports": ["_lazy.js"],
        "css": ["_app/immutable/assets/3.css"],
    },
    node(4): {
        "file": "_app/immutable/nodes/4.js",
        "src": node(4),
        "isEntry": True,
        "imports": ["_section.js"],
    },
    node(5): {"file": "_app/immutable/nodes/5.js", "src": node(5), "isEntry": True},
    node(6): {"file": "_app/immutable/nodes/6.js", "src": node(6), "isEntry": True},
    "_near.js": {"file": "_app/immutable/chunks/near.js", "imports": ["_far.js"]},
    "_far.js": {"file": FAR, "imports": ["_near.js"]},
    "_shared.js": {"file": "_app/immutable/chunks/shared.js"},
    "_section.js": {"file": "_app/immutable/chunks/section.js"},
    "_lazy.js": {"file": "_app/immutable/chunks/lazy.js", "isDynamicEntry": True},
}
# The route file each node's source re-exports; the error page's names none.
ROUTES = {
    0: "+layout.svelte",
    1: None,
    2: "+page.svelte",
    3: "a/+page.svelte",
    4: "b/+layout.svelte",
    5: "b/c/+page.svelte",
    6: "(app)/g/+page.svelte",
}
# Each figure by the rule: the keys whose files it sums.
RULE = {
    "(shell)": ["entry/start.js", "entry/app.js", "_runtime.js", node(0), "_shared.js", node(1)],
    "/": [node(2)],
    "/a": [node(3), "_near.js", "_far.js"],
    "/b/c": [node(5), node(4), "_section.js"],
    "/(app)/g": [node(6)],
}


def gzip_size(path):
    """The `gzip -9 -n` size of a file, measured by gzip itself."""
    out = subprocess.run(["gzip", "-9", "-n", "-c", str(path)], capture_output=True, check=True)
    return len(out.stdout)


def plant(root, chunks, routes, files=None):
    """A built app under `root`: the manifest `chunks`; a generated node for each of `routes`,
    naming its route file or none, and that route file; and each chunk's file and stylesheet, of
    bytes of its own unless `files` gives them. Returns the app's folder."""
    app = Path(root) / "app"
    (app / MANIFEST).parent.mkdir(parents=True)
    (app / NODES).mkdir(parents=True)
    (app / "src" / "routes").mkdir(parents=True)
    (app / MANIFEST).write_text(json.dumps(chunks), encoding="utf-8")
    for number, route_file in routes.items():
        named = "../../../kit/error.svelte"
        if route_file is not None:
            named = f"../../../../../src/routes/{route_file}"
            page = app / "src" / "routes" / route_file
            page.parent.mkdir(parents=True, exist_ok=True)
            page.write_text("<p>planted</p>\n", encoding="utf-8")
        text = f'export {{ default as component }} from "{named}";\n'
        (app / NODES / f"{number}.js").write_text(text, encoding="utf-8")
    for index, chunk in enumerate(chunks.values()):
        if not isinstance(chunk, dict):
            continue
        for name in [chunk.get("file"), *chunk.get("css", [])]:
            if name is None:
                continue
            path = app / CLIENT / name
            path.parent.mkdir(parents=True, exist_ok=True)
            given = (files or {}).get(name)
            path.write_bytes(os.urandom(400 + 97 * index) if given is None else given)
    return app


def figures(app):
    """Each figure of BUILD by the rule above, every file measured by gzip itself."""
    size = {key: gzip_size(app / CLIENT / chunk["file"]) for key, chunk in BUILD.items()}
    return {route: sum(size[key] for key in keys) for route, keys in RULE.items()}


def table(root, entries):
    """A budget file under `root` holding `entries`, {route: bytes}, each measured by job 7."""
    path = Path(root) / "budgets.json"
    routes = {route: {"bytes": figure, "job": 7} for route, figure in entries.items()}
    path.write_text(json.dumps({"_": "planted", "routes": routes}), encoding="utf-8")
    return path


def run_check(app, budgets, path=None):
    """Run the check over a planted app and budget file, with `path` as its PATH when given.
    Returns its exit code and its output."""
    env = dict(os.environ) if path is None else dict(os.environ, PATH=path)
    done = subprocess.run(
        [sys.executable, str(CHECK), "--app", str(app), "--budgets", str(budgets)],
        capture_output=True,
        text=True,
        env=env,
        check=False,
        timeout=60,
    )
    return done.returncode, done.stdout + done.stderr


def build_cases(root):
    """Each VOID case of the build as (why, app, budget file, PATH or None, the reason the check
    names), each planted in a folder of its own under `root`."""
    cases = []

    def fresh(name, chunks=None, routes=None, files=None):
        folder = Path(root) / name
        folder.mkdir()
        app = plant(folder, BUILD if chunks is None else chunks, routes or ROUTES, files)
        return folder, app

    folder, app = fresh("empty", {}, {0: "+layout.svelte"})
    cases.append(("no page", app, table(folder, {}), None, "examined 0 route(s)"))
    folder = Path(root) / "unbuilt"
    (folder / "app" / "src" / "routes").mkdir(parents=True)
    unbuilt = folder / "app"
    reason = f"the manifest {unbuilt / MANIFEST} cannot be read: "
    cases.append(("no manifest", unbuilt, table(folder, {}), None, reason))
    for name, text, why in (
        ("text", "{", "cannot be read: "),
        ("list", "[]", "is not a JSON object"),
    ):
        folder, app = fresh(f"manifest-{name}")
        (app / MANIFEST).write_text(text, encoding="utf-8")
        reason = f"the manifest {app / MANIFEST} {why}"
        cases.append((f"a manifest of {name}", app, table(folder, {}), None, reason))
    folder, app = fresh("number", {**BUILD, "_odd.js": 5})
    reason = "the manifest's entry _odd.js is not an object"
    cases.append(("an entry that is a number", app, table(folder, {}), None, reason))
    folder, app = fresh("unrouted")
    page = app / "src" / "routes" / "d" / "+page.svelte"
    page.parent.mkdir(parents=True)
    page.write_text("<p>planted</p>\n", encoding="utf-8")
    reason = "the page src/routes/d/+page.svelte has no node in the manifest"
    cases.append(("a page with no node", app, table(folder, {}), None, reason))
    extra = {node(9): {"file": "_app/immutable/nodes/9.js", "src": node(9), "isEntry": True}}
    folder, app = fresh("sourceless", {**BUILD, **extra})
    reason = f"the node {node(9)} cannot be read: "
    cases.append(("a node with no source", app, table(folder, {}), None, reason))
    folder, app = fresh("twice", {**BUILD, **extra}, {**ROUTES, 9: "a/+page.svelte"})
    reason = "two nodes serve the route /a"
    cases.append(("two nodes for one route", app, table(folder, {}), None, reason))
    for name, far in (
        ("an import the manifest does not hold", {"file": FAR, "imports": ["_gone.js"]}),
        ("a chunk with no file", {"imports": ["_near.js"]}),
    ):
        folder, app = fresh(name.replace(" ", "-"), {**BUILD, "_far.js": far})
        gone = "_gone.js" if "file" in far else "_far.js"
        reason = f"the manifest names {gone} and holds no file for it"
        cases.append((name, app, table(folder, {}), None, reason))
    folder, app = fresh("missing")
    (app / CLIENT / FAR).unlink()
    reason = f"the file {app / CLIENT / FAR} is missing"
    cases.append(("a missing file", app, table(folder, {}), None, reason))
    folder, app = fresh("empty-file", files={FAR: b""})
    reason = f"the file {app / CLIENT / FAR} is empty"
    cases.append(("an empty file", app, table(folder, {}), None, reason))
    for name, outside in (("up", "../far.js"), ("absolute", str(Path(root) / "absolute.js"))):
        moved = {"_far.js": {"file": outside, "imports": ["_near.js"]}}
        folder, app = fresh(f"outside-{name}", {**BUILD, **moved})
        reason = f"the manifest names {outside}, outside the client folder"
        cases.append((f"a file named {name}", app, table(folder, {}), None, reason))
    folder, app = fresh("no-gzip")
    (folder / "bare").mkdir()
    reason = "gzip is not on PATH, so no figure is measured"
    cases.append(("no gzip", app, table(folder, {}), str(folder / "bare"), reason))
    for name, script, code in (
        ("fails after printing", "#!/bin/sh\nprintf x\nexit 3\n", 3),
        ("prints nothing", "#!/bin/sh\nexit 0\n", 0),
    ):
        folder, app = fresh(f"gzip-{code}")
        (folder / "tools").mkdir()
        (folder / "tools" / "gzip").write_text(script, encoding="utf-8")
        (folder / "tools" / "gzip").chmod(0o700)
        reason = f"gzip exited {code} on "
        cases.append((f"a gzip that {name}", app, table(folder, {}), str(folder / "tools"), reason))
    return cases


def table_cases(root):
    """Each VOID case of the budget file, as `build_cases` gives them, over one planted build."""
    app = plant(Path(root), BUILD, ROUTES)
    entry = "the entry for /a needs whole bytes and the job that measured them"
    shape = "the entry for /a is not an object of bytes and job"
    cases = []
    for index, (name, body, reason) in enumerate(
        (
            ("a list", [], "the budget file {path} is not a JSON object"),
            ("no routes", {"routes": []}, "the budget file {path} holds no routes object"),
            ("a key", {"routes": {"a": {"bytes": 5, "job": 7}}}, "the budget file's entry 'a'"),
            ("a number", {"routes": {"/a": 5}}, shape),
            ("bytes alone", {"routes": {"/a": {"bytes": 5}}}, shape),
            ("bytes as text", {"routes": {"/a": {"bytes": "5", "job": 7}}}, entry),
            ("zero bytes", {"routes": {"/a": {"bytes": 0, "job": 7}}}, entry),
            ("a job as text", {"routes": {"/a": {"bytes": 5, "job": "7"}}}, entry),
            ("job zero", {"routes": {"/a": {"bytes": 5, "job": 0}}}, entry),
        )
    ):
        path = Path(root) / f"budgets-{index}.json"
        path.write_text(json.dumps(body), encoding="utf-8")
        cases.append((f"a budget file of {name}", app, path, None, reason.format(path=path)))
    return cases


class EachRouteIsHeldToItsEntry(unittest.TestCase):
    def test_a_route_over_its_entry_is_reported_and_one_at_its_entry_is_not(self):
        with tempfile.TemporaryDirectory() as root:
            app = plant(root, BUILD, ROUTES)
            want = figures(app)
            a = want["/a"]
            code, out = run_check(app, table(root, {**want, "/a": a - 1}))
            self.assertIn(f"web-route-size: WARN OVER BUDGET /a: gzip-9 {a} > {a - 1} by 1", out)
            self.assertIn("web-route-size: WARN: 1 finding(s), reported at warn", out)
            self.assertEqual(code, 0, out)
            code, out = run_check(app, table(root, want))
            self.assertIn("web-route-size: PASS: every route within its budget", out)
            self.assertNotIn("OVER BUDGET", out)
            self.assertEqual(code, 0, out)

    def test_a_route_with_no_entry_is_reported(self):
        with tempfile.TemporaryDirectory() as root:
            app = plant(root, BUILD, ROUTES)
            want = figures(app)
            bc = want["/b/c"]
            entries = {route: figure for route, figure in want.items() if route != "/b/c"}
            code, out = run_check(app, table(root, entries))
            self.assertIn(f"web-route-size: route /b/c: 3 file(s), gzip-9 {bc}, budget none", out)
            self.assertIn(f"web-route-size: WARN NO BUDGET /b/c: gzip-9 {bc}; add its entry", out)
            self.assertIn("web-route-size: WARN: 1 finding(s), reported at warn", out)
            self.assertEqual(code, 0, out)

    def test_an_entry_for_a_route_that_is_gone_is_reported(self):
        with tempfile.TemporaryDirectory() as root:
            app = plant(root, BUILD, ROUTES)
            want = figures(app)
            code, out = run_check(app, table(root, {**want, "/gone": 5}))
            self.assertIn("web-route-size: WARN GONE /gone: an entry for no route; remove it", out)
            self.assertIn("web-route-size: WARN: 1 finding(s), reported at warn", out)
            self.assertEqual(code, 0, out)

    def test_a_route_past_the_margin_under_its_entry_asks_the_entry_lowered(self):
        with tempfile.TemporaryDirectory() as root:
            app = plant(root, BUILD, ROUTES)
            want = figures(app)
            a = want["/a"]
            code, out = run_check(app, table(root, {**want, "/a": a + MARGIN + 1}))
            self.assertIn(
                f"web-route-size: WARN RATCHET /a: gzip-9 {a}, budget {a + MARGIN + 1}, "
                f"more than {MARGIN} under it; lower its entry to {a}",
                out,
            )
            self.assertEqual(code, 0, out)
            code, out = run_check(app, table(root, {**want, "/a": a + MARGIN}))
            self.assertIn("web-route-size: PASS: every route within its budget", out)
            self.assertNotIn("RATCHET", out)
            self.assertEqual(code, 0, out)

    def test_a_routes_figure_is_its_static_closure_beyond_the_shell(self):
        with tempfile.TemporaryDirectory() as root:
            app = plant(root, BUILD, ROUTES)
            want = figures(app)
            code, out = run_check(app, table(root, want))
            for route in examined("route figure(s)", RULE):
                self.assertIn(
                    f"web-route-size: route {route}: {len(RULE[route])} file(s), "
                    f"gzip-9 {want[route]}, budget {want[route]}",
                    out,
                )
            self.assertIn("web-route-size: examined 4 route(s) and the shell", out)
            self.assertEqual(code, 0, out)

    def test_a_build_or_table_that_does_not_read_or_holds_no_route_is_void(self):
        with tempfile.TemporaryDirectory() as root:
            (Path(root) / "build").mkdir()
            (Path(root) / "table").mkdir()
            cases = build_cases(Path(root) / "build") + table_cases(Path(root) / "table")
            for why, app, budgets, path, reason in examined("VOID case(s)", cases):
                code, out = run_check(app, budgets, path)
                self.assertIn(f"web-route-size: VOID: {reason}", out, why)
                self.assertEqual(code, 2, f"{why}: {out}")
                self.assertNotIn("PASS", out, why)


class TheLevelDecidesWhetherAFindingFails(unittest.TestCase):
    def test_a_finding_fails_at_the_fail_level_and_passes_at_warn(self):
        self.assertEqual(web_route_size.LEVEL, "warn")
        figures_ = {"(shell)": 900, "/a": 300}
        counts = {"(shell)": 4, "/a": 2}
        over = {"(shell)": 900, "/a": 299}
        cases = [
            ("fail", over, 1, ["FAIL OVER BUDGET /a: gzip-9 300 > 299 by 1", "FAIL: 1 finding(s)"]),
            ("warn", over, 0, ["WARN OVER BUDGET /a: gzip-9 300 > 299 by 1", "WARN: 1 finding(s)"]),
            ("fail", figures_, 0, ["PASS: every route within its budget"]),
        ]
        for level, entries, expected, lines in examined("level case(s)", cases):
            shown = io.StringIO()
            with contextlib.redirect_stdout(shown):
                code = web_route_size.report(figures_, counts, entries, level)
            for line in lines:
                self.assertIn(f"web-route-size: {line}", shown.getvalue(), level)
            self.assertEqual(code, expected, f"{level}: {shown.getvalue()}")


class TheCommandLineStatesItself(unittest.TestCase):
    def test_the_help_names_the_check_and_both_options_are_required(self):
        helped = subprocess.run(
            [sys.executable, str(CHECK), "--help"],
            capture_output=True,
            text=True,
            check=False,
            env={**os.environ, "COLUMNS": "200"},
            timeout=60,
        )
        words = " ".join(helped.stdout.split())
        for line in examined(
            "help line(s)",
            [
                "usage: web-route-size [-h] --app APP --budgets BUDGETS",
                "each web route's JavaScript against its recorded budget",
                "--app APP the web app's folder, built",
                "--budgets BUDGETS the budget file, one entry per route",
            ],
        ):
            self.assertIn(line, words)
        self.assertEqual(helped.returncode, 0, helped.stderr)
        for given in examined("missing option case(s)", [[], ["--app", "a"], ["--budgets", "b"]]):
            done = subprocess.run(
                [sys.executable, str(CHECK), *given],
                capture_output=True,
                text=True,
                check=False,
                timeout=60,
            )
            self.assertIn("the following arguments are required", done.stderr, given)
            self.assertNotIn("Traceback", done.stderr, given)
            self.assertEqual(done.returncode, 2, given)

    def test_loading_the_check_turns_bytecode_writing_off(self):
        # A copy in a scratch folder, so the bytecode its own load caches lands there, never beside
        # the tree's script.
        with tempfile.TemporaryDirectory() as scratch:
            copy = Path(scratch) / CHECK.name
            copy.write_bytes(CHECK.read_bytes())
            kept = sys.dont_write_bytecode
            sys.dont_write_bytecode = False
            try:
                load(copy, "web_route_size_copy")
                written = sys.dont_write_bytecode
            finally:
                sys.dont_write_bytecode = kept
        self.assertIs(written, True)


if __name__ == "__main__":
    unittest.main()
