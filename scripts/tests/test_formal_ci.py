"""The formal workflow's checks: covers fresh or listed, the Lean axiom audit, the workflow (SPEC-614).

A1, A2, A3, A5 and A6 build their own trees in a temporary folder and read no file of the live
tree. A4 and A7 read the live tree, each counting its population from the entry files and refusing
zero; A4 reaches a covered file only through the checker. A8 and A9 read the workflow as text, each
beside a planted copy it refuses.
"""

import contextlib
import hashlib
import io
import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

sys.path.insert(0, str(REPO / "scripts"))
import formal_ci  # noqa: E402

SCRIPT = REPO / "scripts" / "formal_ci.py"
WORKFLOW = REPO / ".github" / "workflows" / "formal.yml"
ENTRY_GLOBS = ("formal/lean/Formal/*.lean", "formal/tla/*/*.tla", "crates/*/src/kani_proofs.rs")
ALLOWED = ["propext", "Classical.choice", "Quot.sound"]

LEXER = (
    "fn lexer() {\n"
    '    let a = "}";\n'
    '    let b = r#"}"#;\n'
    '    let c = b"}";\n'
    '    let d = br"}";\n'
    '    let e = cr"}";\n'
    '    let n = c"}";\n'
    "    let f = '}';\n"
    "    let g = '\\}';\n"
    '    let h = "\\"}";\n'
    '    let i = br##"}"#}"##;\n'
    '    let j = xr"\\"}";\n'
    "    /* } /* } */ } */\n"
    "    // } CAFE_MARK\n"
    '    let k: &\'static str = "{";\n'
    '    let m = "\n'
    '}";\n'
    "    /*\n"
    "}\n"
    "*/\n"
    "}\n"
)
RUST_ITEMS = {
    "fetch": "pub(crate) async fn fetch() -> u8 {\n    7\n}\n",
    "raw_read": "unsafe fn raw_read() {}\n",
    "callback": 'extern "C" fn callback() {}\n',
    "limit": "pub const fn limit() -> u8 {\n    3\n}\n",
    "chosen": "default fn chosen() {}\n",
    "Point": "struct Point {\n    x: u8,\n}\n",
    "Kind": "enum Kind {\n    One,\n}\n",
    "Speak": "trait Speak {\n    fn speak(&self);\n}\n",
    "inner": "mod inner {\n    pub fn deep() {}\n}\n",
    "Bits": "union Bits {\n    word: u32,\n}\n",
    "Alias": "type Alias = u8;\n",
    "LIMIT": "const LIMIT: u8 = 1;\n",
    "NAME": 'static NAME: &str = "}";\n',
    "COUNT": "static mut COUNT: u8 = 0;\n",
    "shout": "macro_rules! shout {\n    () => {};\n}\n",
    "lexer": LEXER,
}
RUST_HOSTILE = "// fn fetch() {}\nfn fetchd() {}\nfn fetch_more() {}\n"
PY_ITEMS = {
    "Holder": "class Holder:\n    def method(self):\n        return 1\n",
    "anchor_py": (
        "def anchor_py(\n"
        "    first,\n"
        "    second,\n"
        ") -> None:\n"
        '    """A docstring\n'
        'with a column-0 line."""\n'
        '    text = "(#"\n'
        "    value = [\n"
        "1, 2]\n"
        "\n"
        "# a comment at column 0\n"
        "    # an indented comment\n"
        "    return value \\\n"
        "+ 1\n"
    ),
    "later": "async def later():\n    return '''(\n'''\n",
    "Child": "class Child(Holder):\n    pass\n",
}
SH_ITEMS = {
    "tell": 'tell() {\n  if true; then\n    echo "}"\n  fi\n}\n',
    "second": "function second {\n  echo two\n}\n",
    "one_line": "one_line() { echo one; }\n",
    "after": "after () {\n  :\n}\n",
}


def sha(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def cover(marker, path, anchor, digest):
    return f"{marker} @phx covers {path} anchor={anchor} digest=sha256:{digest}\n"


def plant(test, files, stale=None):
    """A temporary tree holding `files` (text, or bytes), the allowed axioms and a stale list."""
    folder = tempfile.TemporaryDirectory()
    test.addCleanup(folder.cleanup)
    root = Path(folder.name)
    listing = {"schema": "formal-stale.v1", "covers": stale or []}
    files = {
        "config/formal.json": json.dumps({"axioms": ALLOWED}),
        "config/formal-stale.json": json.dumps(listing),
        **files,
    }
    for name, body in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(body, bytes):
            path.write_bytes(body)
        else:
            path.write_text(body, encoding="utf-8")
    return root


def run(*argv):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = formal_ci.main([str(arg) for arg in argv])
        except SystemExit as stop:
            code = ("exit", stop.code)
    return code, out.getvalue().splitlines()


def found(lines, *kinds):
    return [line for line in lines if line.split(" ", 1)[0] in kinds]


def three_entries(py, sh, rs, digests):
    return {
        "tool/s.py": py,
        "tool/s.sh": sh,
        "src/lib.rs": rs,
        "formal/lean/Formal/E.lean": cover("--", "tool/s.py", "step", digests[0]),
        "formal/tla/M/M.tla": cover("\\*", "tool/s.sh", "tell", digests[1]),
        "crates/k/src/kani_proofs.rs": cover("//", "src/lib.rs", "fetch", digests[2]),
    }


PY_STEP = "def step():\n    return 1\n"
SH_TELL = "tell() {\n  echo one\n}\n"
RS_FETCH = "fn fetch() -> u8 {\n    7\n}\n"


class CoversAreFreshOrListed(unittest.TestCase):
    def test_a_moved_span_reads_stale_by_entry_path_and_anchor(self):
        old = [sha(PY_STEP), sha(SH_TELL), sha(RS_FETCH)]
        fresh = plant(self, three_entries(PY_STEP, SH_TELL, RS_FETCH, old))
        code, out = run("covers", "--root", fresh)
        self.assertEqual(out[:1], ["examined 3 covers"])
        self.assertEqual((code, found(out, "STALE", "UNRESOLVED", "LISTED")), (0, []))
        self.assertEqual(out[-1:], ["0 findings, 0 listed"])
        child = subprocess.run(
            [sys.executable, "-B", str(SCRIPT), "covers", "--root", str(fresh)],
            capture_output=True,
            text=True,
            timeout=60,
        )
        self.assertEqual(
            (child.returncode, child.stdout.splitlines()[:1]), (0, ["examined 3 covers"])
        )
        moved = [
            PY_STEP.replace("1", "2"),
            SH_TELL.replace("one", "two"),
            RS_FETCH.replace("7", "8"),
        ]
        tree = plant(self, three_entries(*moved, old))
        code, out = run("covers", "--root", tree)
        self.assertEqual(code, 1)
        self.assertEqual(
            found(out, "STALE", "UNRESOLVED", "LISTED"),
            [
                f"STALE lean/E tool/s.py::step is sha256 {sha(moved[0])}, the entry recorded {old[0]}",
                f"STALE tla/M tool/s.sh::tell is sha256 {sha(moved[1])}, the entry recorded {old[1]}",
                f"STALE kani/k src/lib.rs::fetch is sha256 {sha(moved[2])}, the entry recorded {old[2]}",
            ],
        )
        self.assertEqual(out[-1:], ["3 findings, 0 listed"])
        code, out = run("covers", "--root", plant(self, {}))
        self.assertEqual((code, out), (2, ["examined 0 covers"]))
        for argv in ([], ["covers"], ["covers", "--root"]):
            self.assertEqual(run(*argv)[0], ("exit", 2), argv)

    def test_an_anchor_absent_or_defined_twice_is_unresolved(self):
        rust = (
            "fn twice() {}\nfn twice() {}\n"
            'pub const unsafe extern "C" fn quirk() {}\n'
            "fn open() {\n    let x = 1;\n"
        )
        entry = "".join(
            cover("--", path, anchor, "0" * 64)
            for path, anchor in (
                ("src/a.rs", "missing"),
                ("src/a.rs", "twice"),
                ("src/a.rs", "quirk"),
                ("src/a.rs", "open"),
                ("src/a.sh", "loose"),
                ("src/gone.py", "x"),
                ("docs/a.md", "x"),
            )
        )
        entry += "-- @phx covers src/a.rs anchor=x digest=sha256:abc\n"
        files = {
            "src/a.rs": rust,
            "src/a.sh": "loose() {\n  echo x\n  }\n",
            "docs/a.md": "x\n",
            "formal/lean/Formal/E.lean": entry,
        }
        code, out = run("covers", "--root", plant(self, files))
        self.assertEqual(out[:1], ["examined 8 covers"])
        self.assertEqual(code, 1)
        self.assertEqual(
            found(out, "STALE", "UNRESOLVED", "LISTED"),
            [
                "UNRESOLVED lean/E src/a.rs::missing is defined 0 times, not once",
                "UNRESOLVED lean/E src/a.rs::twice is defined 2 times, not once",
                "UNRESOLVED lean/E src/a.rs::quirk is defined 0 times, not once",
                "UNRESOLVED lean/E src/a.rs::open never ends",
                "UNRESOLVED lean/E src/a.sh::loose never ends",
                "UNRESOLVED lean/E src/gone.py::x the file is absent",
                "UNRESOLVED lean/E docs/a.md::x is not a .rs, .py or .sh file",
                "UNRESOLVED lean/E formal/lean/Formal/E.lean:8 is not covers <path> anchor=<item> "
                "digest=sha256:<64 hex>",
            ],
        )
        self.assertEqual(out[-1:], ["8 findings, 0 listed"])

    def test_each_reader_reproduces_a_digest_written_by_the_rule(self):
        rust = RUST_HOSTILE + "\n".join(RUST_ITEMS.values()) + "\nfn after() {}\n"
        rust_bytes = rust.encode("utf-8").replace(b"CAFE_MARK", b"caf\xff")
        python = "\n\n".join(PY_ITEMS.values()) + "\n\ndef tail():\n    pass\n"
        shell = "\n".join(SH_ITEMS.values()) + "\nend() {\n  :\n}\n"
        expected = {
            ("src/lib.rs", a): s.replace("CAFE_MARK", "caf\ufffd") for a, s in RUST_ITEMS.items()
        }
        expected |= {("tool/run.py", a): s for a, s in PY_ITEMS.items()}
        expected |= {("tool/run.sh", a): s for a, s in SH_ITEMS.items()}
        entry = "".join(cover("--", p, a, sha(s)) for (p, a), s in expected.items())
        files = {"src/lib.rs": rust_bytes, "tool/run.py": python, "tool/run.sh": shell}
        root = plant(self, {**files, "formal/lean/Formal/Readers.lean": entry})
        for (path, anchor), span in expected.items():
            text = (root / path).read_bytes().decode("utf-8", "replace")
            self.assertEqual(formal_ci.span(path, text, anchor), (span, None), anchor)
        code, out = run("covers", "--root", root)
        self.assertEqual(out[:1], [f"examined {len(expected)} covers"])
        self.assertEqual((code, found(out, "STALE", "UNRESOLVED", "LISTED")), (0, []))

    def test_every_cover_at_head_is_fresh_or_listed_exactly(self):
        lines = [
            line
            for pattern in ENTRY_GLOBS
            for path in sorted(REPO.glob(pattern))
            for line in path.read_text(encoding="utf-8").splitlines()
            if "@phx covers" in line
        ]
        examined("@phx covers lines in the entry files", lines)
        code, out = run("covers", "--root", REPO)
        self.assertEqual(out[:1], [f"examined {len(lines)} covers"])
        rows = json.loads((REPO / "config/formal-stale.json").read_text(encoding="utf-8"))["covers"]
        listed = [
            f"STALE {r['entry']} {r['path']}::{r['anchor']} is sha256 {r['now']}, the entry recorded "
            f"{r['recorded']} (listed)"
            for r in rows
        ]
        self.assertEqual(sorted(found(out, "STALE", "UNRESOLVED", "LISTED")), sorted(listed))
        self.assertEqual(code, 0, out)

    def test_a_listed_row_fresh_undeclared_or_moved_again_is_a_finding(self):
        old, now, other = sha("def step():\n    return 0\n"), sha(PY_STEP), "1" * 64
        files = {
            "tool/s.py": PY_STEP,
            "formal/lean/Formal/E.lean": cover("--", "tool/s.py", "step", old),
        }
        stale = f"STALE lean/E tool/s.py::step is sha256 {now}, the entry recorded {old}"

        def row(recorded, current, anchor="step"):
            return {
                "entry": "lean/E",
                "path": "tool/s.py",
                "anchor": anchor,
                "recorded": recorded,
                "now": current,
                "cure": "re-stamp",
            }

        code, out = run("covers", "--root", plant(self, files, [row(old, now)]))
        self.assertEqual((code, found(out, "STALE", "LISTED")), (0, [stale + " (listed)"]))
        self.assertEqual(out[-1:], ["0 findings, 1 listed"])
        for planted, finding in (
            (
                row(old, other),
                f"LISTED lean/E tool/s.py::step reads {now}, the list says {other} over {old}",
            ),
            (
                row(other, now),
                f"LISTED lean/E tool/s.py::step reads {now}, the list says {now} over {other}",
            ),
            (row(old, now, "ghost"), "LISTED lean/E tool/s.py::ghost no entry declares it"),
        ):
            code, out = run("covers", "--root", plant(self, files, [row(old, now), planted]))
            self.assertEqual((code, found(out, "LISTED")), (1, [finding]), planted)
        fresh = {**files, "formal/lean/Formal/E.lean": cover("--", "tool/s.py", "step", now)}
        code, out = run("covers", "--root", plant(self, fresh, [row(now, now)]))
        self.assertEqual(
            (code, found(out, "STALE", "LISTED")),
            (1, ["LISTED lean/E tool/s.py::step is fresh, so its row must go"]),
        )
        tree = plant(self, files, [row(old, now)])
        (tree / "config/formal-stale.json").write_text(
            json.dumps({"schema": "formal-stale.v0", "covers": []})
        )
        code, out = run("covers", "--root", tree)
        self.assertEqual(code, 1)
        self.assertIn(
            "LISTED config/formal-stale.json is schema formal-stale.v0, not formal-stale.v1", out
        )
        self.assertIn(stale, out)


DECLARING = {
    "formal/lean/Formal/E.lean": "-- @phx theorem t ramp=report\n-- @phx witness w kills=t\n",
    "formal/lean/Formal/F.lean": "-- @phx theorem u ramp=report\n",
    "formal/lean/Formal/G.lean": "-- no declaration here\n",
}
PRINTED = (
    "'Formal.E.t' depends on axioms: [propext,\n Classical.choice,\n Quot.sound]\n"
    "'Formal.E.w' does not depend on any axioms\n"
    "'Formal.F.u' depends on axioms: [propext]\n"
)


class TheAxiomsAreAudited(unittest.TestCase):
    def audit(self, files, output):
        root = plant(self, files)
        (root / "lean-output.txt").write_text(output, encoding="utf-8")
        return run("audit", "--root", root, "--output", root / "lean-output.txt")

    def test_an_axiom_outside_the_allowed_set_or_sorry_is_refused(self):
        code, out = self.audit(DECLARING, PRINTED)
        self.assertEqual((code, out), (0, ["examined 3 declarations", "0 findings"]))
        u = "'Formal.F.u' depends on axioms: [propext]"
        w = "'Formal.E.w' does not depend on any axioms\n"
        for output, finding in (
            (PRINTED.replace(u, u.replace("]", ", sorryAx]")), "AXIOM Formal.F.u uses sorryAx"),
            (
                PRINTED.replace("[propext]", "[Formal.F.u._native.ax_1_1]"),
                "AXIOM Formal.F.u uses Formal.F.u._native.ax_1_1",
            ),
            (PRINTED.replace(w, ""), "MISSING Formal.E.w is printed 0 times, not once"),
            (PRINTED + w, "MISSING Formal.E.w is printed 2 times, not once"),
            (
                PRINTED + "audit.lean:4:0: error: unknown constant\n",
                "UNPLACED audit.lean:4:0: error: unknown constant",
            ),
            (
                PRINTED + "'Formal.G.z' does not depend on any axioms\n",
                "UNPLACED 'Formal.G.z' does not depend on any axioms",
            ),
        ):
            code, out = self.audit(DECLARING, output)
            self.assertEqual(
                (code, out), (1, ["examined 3 declarations", finding, "1 findings"]), finding
            )
        code, out = self.audit({}, "")
        self.assertEqual((code, out), (2, ["examined 0 declarations"]))
        self.assertEqual(run("audit", "--root", ".")[0], ("exit", 2))

    def test_the_audit_source_names_every_declared_theorem_and_witness_once(self):
        root = plant(self, DECLARING)
        code, out = run("audit-source", "--root", root, "--out", root / "audit.lean")
        self.assertEqual((code, out), (0, ["examined 3 declarations"]))
        self.assertTrue((root / "audit.lean").is_file(), "audit-source wrote no file")
        self.assertEqual(
            (root / "audit.lean").read_text(encoding="utf-8").splitlines(),
            ["import Formal.E", "import Formal.F"]
            + [f"#print axioms Formal.{n}" for n in ("E.t", "E.w", "F.u")],
        )
        empty = plant(self, {"formal/lean/Formal/G.lean": "-- none\n"})
        self.assertEqual(
            run("audit-source", "--root", empty, "--out", empty / "a.lean"),
            (2, ["examined 0 declarations"]),
        )
        self.assertEqual(run("audit-source", "--root", empty)[0], ("exit", 2))
        declared = {}
        for path in sorted(REPO.glob("formal/lean/Formal/*.lean")):
            for line in path.read_text(encoding="utf-8").splitlines():
                match = re.match(r"-- @phx (?:theorem|witness) (\S+)", line)
                if match:
                    declared.setdefault(path.stem, []).append(
                        f"#print axioms Formal.{path.stem}.{match[1]}"
                    )
        names = examined(
            "declared theorems and witnesses", [n for group in declared.values() for n in group]
        )
        examined("declaring modules", declared)
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        written = Path(folder.name) / "audit.lean"
        code, out = run("audit-source", "--root", REPO, "--out", written)
        self.assertEqual((code, out), (0, [f"examined {len(names)} declarations"]))
        self.assertTrue(written.is_file(), "audit-source wrote no file")
        lines = written.read_text(encoding="utf-8").splitlines()
        self.assertEqual(lines, [f"import Formal.{m}" for m in declared] + names)


def steps(text):
    """The workflow's steps, each as its text, in order."""
    return re.split(r"(?m)^      - ", text.split("\n    steps:\n", 1)[-1])[1:]


STEP_ORDER = (
    ("uses: actions/checkout@", "persist-credentials: false"),
    (
        "name: the covers of every formal entry are fresh or listed",
        "python3 scripts/formal_ci.py covers --root .",
    ),
    ("name: Lean 4.34.0, checksum verified", "sha256sum -c -"),
    ("name: every Lean entry builds", "lake build"),
    ("name: every declared theorem and witness uses only the allowed axioms", "lake env lean"),
)


def workflow_problems(text):
    problems = []
    for block in (
        "on:\n  pull_request:\n    branches: [dev, main]\n    types: [opened, synchronize, reopened]\n"
        "  push:\n    branches: [dev, main]\n",
        "\npermissions:\n  contents: read\n",
        "\njobs:\n  formal:\n",
        "    runs-on: ubuntu-24.04\n",
        "\nconcurrency:\n  group: formal-${{ github.event_name == 'pull_request' && github.ref || github.run_id }}\n",
    ):
        if block not in text:
            problems.append(f"missing {block!r}")
    if "timeout-minutes:" not in text or "secrets." in text or "actions/cache" in text:
        problems.append("a timeout is missing, or a secret or a cache is read")
    found_steps = steps(text)
    if len(found_steps) != len(STEP_ORDER):
        problems.append(f"{len(found_steps)} steps, not {len(STEP_ORDER)}")
    for step, (head, command) in zip(found_steps, STEP_ORDER, strict=False):
        if not step.startswith(head) or command not in step:
            problems.append(f"step {step.splitlines()[0]!r} is not {head!r} running {command!r}")
    build, audit = (found_steps + ["", "", "", "", ""])[3:5]
    if (
        "working-directory: formal/lean" not in build
        or "audit-source" not in audit
        or "formal_ci.py audit " not in audit
    ):
        problems.append("the build or the audit runs outside formal/lean or skips a verb")
    return problems


def install_problems(text, toolchain):
    version = toolchain.strip().removeprefix("leanprover/lean4:v")
    urls = re.findall(r"releases/download/v(\S+)/lean-(\S+)-linux\.tar\.zst", text)
    digests = re.findall(r"LEAN_SHA256: ([0-9a-f]{64})\n", text)
    problems = []
    if urls != [(version, version)]:
        problems.append(f"the archive {urls} is not lean {version}")
    if len(digests) != 1 or 'echo "$LEAN_SHA256  ' not in text or "| sha256sum -c -" not in text:
        problems.append("the archive is not checked against one LEAN_SHA256")
    if f"leanprover/lean4:v{version}" not in text:
        problems.append("the step does not refuse another toolchain pin")
    return problems


class TheWorkflowRunsTheChecks(unittest.TestCase):
    def test_the_formal_workflow_runs_its_checks_in_order_on_every_pull_request_and_push(self):
        self.assertTrue(WORKFLOW.is_file(), "no .github/workflows/formal.yml")
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertEqual(workflow_problems(text), [])
        self.assertIn("name: formal\n", text)
        build = "      - name: every Lean entry builds\n"
        at = text.index(build)
        planted = text[:at] + text[text.index("      - name: every declared", at) :]
        self.assertIn("4 steps, not 5", workflow_problems(planted))
        self.assertNotEqual(
            workflow_problems(text.replace("contents: read", "contents: write")), []
        )

    def test_the_lean_install_is_the_toolchain_pin_checksum_verified(self):
        self.assertTrue(WORKFLOW.is_file(), "no .github/workflows/formal.yml")
        text = WORKFLOW.read_text(encoding="utf-8")
        toolchain = (REPO / "formal/lean/lean-toolchain").read_text(encoding="utf-8")
        self.assertEqual(install_problems(text, toolchain), [])
        version = toolchain.strip().removeprefix("leanprover/lean4:v")
        self.assertRegex(version, r"^\d+\.\d+\.\d+$")
        other = text.replace(f"download/v{version}/", "download/v4.33.0/")
        self.assertNotEqual(install_problems(other, toolchain), [])
        self.assertNotEqual(install_problems(text, "leanprover/lean4:v4.35.0\n"), [])


if __name__ == "__main__":
    unittest.main()
