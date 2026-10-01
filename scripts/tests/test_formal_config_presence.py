"""SPEC-295 A8: a committed settings file the test's reader refuses fails the module by assertion.

The class is "a refusal never escapes a test as an error": every test that reads the committed file
through the test's own reader, and every test that loads it, fails by an assertion when the file is
refused or unreadable, so a red run names the file and never reads as a crash. The population is
generated twice, from the module under test and never listed by hand: every call of the reader in
its source, found by its syntax tree, and every planted fault the module itself generates, each
installed as the committed file and each run against every test of the module."""

import ast
import contextlib
import importlib.util
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

from _support import examined

MODULE = Path(__file__).with_name("test_formal_config.py")
READER = "read"
LOADER = "load"
REFUSAL = "Refused"


def is_named(node, name):
    return isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == name


def reader_calls(tree):
    """Every call of the reader outside the reader itself, as (test, line, guarded, fails,
    committed): whether a handler of the refusal encloses it, whether that handler fails by an
    assertion (`self.fail`), and whether it reads the committed file (its argument is a load or a
    name bound to one in the same test)."""
    found = []
    loaded = {}

    def visit(node, test, handler):
        if isinstance(node, ast.FunctionDef):
            if node.name == READER:
                return
            if node.name.startswith("test"):
                test = node.name
        if isinstance(node, ast.Assign) and is_named(node.value, LOADER):
            loaded.setdefault(test, set()).update(
                t.id for t in node.targets if isinstance(t, ast.Name)
            )
        if is_named(node, READER):
            committed = any(
                is_named(arg, LOADER)
                or (isinstance(arg, ast.Name) and arg.id in loaded.get(test, ()))
                for arg in node.args
            )
            fails = handler is not None and any(
                isinstance(call, ast.Call)
                and isinstance(call.func, ast.Attribute)
                and call.func.attr == "fail"
                for call in ast.walk(handler)
            )
            found.append((test, node.lineno, handler is not None, fails, committed))
        if isinstance(node, ast.Try):
            refusals = [
                h for h in node.handlers if isinstance(h.type, ast.Name) and h.type.id == REFUSAL
            ]
            for child in node.body:
                visit(child, test, refusals[0] if refusals else handler)
            for part in (node.handlers, node.orelse, node.finalbody):
                for child in part:
                    visit(child, test, handler)
            return
        for child in ast.iter_child_nodes(node):
            visit(child, test, handler)

    visit(tree, None, None)
    return found


def load_callers(tree):
    """The tests that call the loader of the committed file directly."""
    return {
        node.name
        for node in ast.walk(tree)
        if isinstance(node, ast.FunctionDef)
        and node.name.startswith("test")
        and any(is_named(call, LOADER) for call in ast.walk(node))
    }


def fresh_module():
    """The module under test, executed again under another name so its settings path can be moved
    without touching the module the runner holds."""
    spec = importlib.util.spec_from_file_location("formal_config_under_plant", MODULE)
    module = importlib.util.module_from_spec(spec)
    with contextlib.redirect_stdout(io.StringIO()):
        spec.loader.exec_module(module)
    return module


# A link at the path: the checker reads a link's own text, so the file behind it is never read.
LINK = "a link"


def unloadable():
    """Every way the committed file fails to load, one per arm of the loader, as (label, bytes,
    None for no file, or LINK): absent; a link, here to a readable copy of the declared file; and
    each way the parser refuses bytes: syntax, encoding, a depth past every recursion limit it
    keeps, and an integer past the interpreter's digit limit."""
    deep = sys.getrecursionlimit() * 100
    return [
        ("the file is absent", None),
        ("the file is a link", LINK),
        ("the file is not JSON", b"{"),
        ("the file is not UTF-8", b"\xff"),
        ("the file nests past the parser's depth", b"[" * deep + b"]" * deep),
        (
            "the file holds an integer past the digit limit",
            b"1" * (sys.get_int_max_str_digits() + 1),
        ),
    ]


def plants(module):
    """Every way the committed file can be refused, as (label, payload, unloadable): each way it
    fails to load, and each fault the module generates from its own field table, written as the
    file."""
    found = [(label, payload, True) for label, payload in unloadable()]
    for name, doc in module.planted_faults():
        found.append((name, (json.dumps(doc, indent=2) + "\n").encode("utf-8"), False))
    return found


def install(directory, payload, module):
    """The plant as a file in a fresh `directory`, and its path."""
    directory.mkdir()
    path = directory / "formal.json"
    if payload is LINK:
        target = directory / "declared.json"
        target.write_text(json.dumps(module.EXPECTED, indent=2) + "\n", encoding="utf-8")
        path.symlink_to(target.name)
    elif payload is not None:
        path.write_bytes(payload)
    return path


def refusal_of(payload):
    """The exception name with which the parser refuses the bytes as the loader reads them, UTF-8
    text then JSON, or None when it reads them."""
    try:
        json.loads(payload.decode("utf-8"))
    except (ValueError, RecursionError) as error:
        return type(error).__name__
    return None


def run_with(module, path):
    """Run every test of the module with the committed file at `path`: (failed, errored) names."""
    names = unittest.TestLoader().getTestCaseNames(module.FormalConfig)
    module.CONFIG = path
    suite = unittest.TestSuite(module.FormalConfig(name) for name in names)
    with contextlib.redirect_stdout(io.StringIO()):
        result = unittest.TextTestRunner(stream=io.StringIO(), verbosity=0).run(suite)
    failed = {test._testMethodName for test, _ in result.failures}
    errored = {
        test._testMethodName: trace.strip().splitlines()[-1] for test, trace in result.errors
    }
    return failed, errored


class PresenceControls(unittest.TestCase):
    def test_every_call_of_the_reader_fails_by_assertion_when_it_refuses(self):
        """A8: each call of the reader in the module is inside a handler of its refusal, and each
        call that reads the committed file is inside one that fails the test by an assertion."""
        tree = ast.parse(MODULE.read_text(encoding="utf-8"))
        calls = examined("calls of the reader", reader_calls(tree))
        unsafe = [
            f"{test} line {line}"
            for test, line, guarded, fails, committed in calls
            if not (fails if committed else guarded)
        ]
        self.assertEqual(unsafe, [], "a call of the reader whose refusal would escape as an error")
        presence = examined(
            "presence controls of the committed file",
            sorted({test for test, _, _, _, committed in calls if committed}),
        )
        self.assertGreaterEqual(len(presence), 2)

    def test_a_refused_committed_file_fails_every_reader_by_assertion_and_errors_none(self):
        """A8: each way the committed file can be refused, each way it fails to load and every
        fault the module generates, is installed as the file, and the module then runs with no
        error at all, a failure by assertion for each test that reads the committed file, and, for
        each file that fails to load, exactly the tests that load it."""
        tree = ast.parse(MODULE.read_text(encoding="utf-8"))
        calls = reader_calls(tree)
        presence = {test for test, _, _, _, committed in calls if committed}
        loaders = load_callers(tree)
        self.assertTrue(presence <= loaders)
        module = fresh_module()
        population = examined("planted refusals of the committed file", plants(module))
        escaped, unread, unloaded = [], [], []
        with tempfile.TemporaryDirectory() as scratch:
            for index, (label, payload, unloads) in enumerate(population):
                path = install(Path(scratch) / str(index), payload, module)
                failed, errored = run_with(module, path)
                if errored:
                    escaped.append(f"{label}: {errored}")
                if not presence <= failed:
                    unread.append(f"{label}: {sorted(presence - failed)} did not fail")
                if unloads:
                    if failed != loaders:
                        unloaded.append(
                            f"{label}: failed {sorted(failed)}, loaders {sorted(loaders)}"
                        )
        print(f"examined {len(loaders)} tests that load the committed file")
        self.assertEqual(escaped, [], "a refusal escaped a test as an error, not a failure")
        self.assertEqual(unread, [], "a presence control passed on a refused file")
        self.assertEqual(unloaded, [], "a file that fails to load did not fail exactly the loaders")

    def test_each_planted_refusal_is_the_kind_its_label_names(self):
        """A8's population is what it says: a plant that fails to load is a file the parser
        refuses, a link or no file, each installed as that kind with the bytes it names, and a
        generated fault is a file the parser reads. A plant of another kind would leave the loader's
        arm it stands for unexercised while the run still read green."""
        module = fresh_module()
        population = examined("planted refusals held to the kind they name", plants(module))
        refusals = set()
        with tempfile.TemporaryDirectory() as scratch:
            for index, (label, payload, unloads) in enumerate(population):
                path = install(Path(scratch) / str(index), payload, module)
                if payload is None:
                    self.assertFalse(os.path.lexists(path), f"{label}: a file is there")
                elif payload is LINK:
                    self.assertTrue(path.is_symlink(), f"{label}: no link is there")
                else:
                    self.assertTrue(os.path.lexists(path), f"{label}: no file is there")
                    self.assertFalse(path.is_symlink(), f"{label}: a link is there")
                    self.assertEqual(path.read_bytes(), payload, f"{label}: other bytes")
                    refusal = refusal_of(payload)
                    self.assertEqual(refusal is not None, unloads, f"{label}: the parser's verdict")
                    if refusal is not None:
                        refusals.add(refusal)
                self.assertTrue(unloads or payload not in (None, LINK), f"{label}: marked loadable")
        self.assertEqual(
            refusals,
            {"JSONDecodeError", "UnicodeDecodeError", "RecursionError", "ValueError"},
            "the parser's four ways to refuse the bytes, each planted: syntax, encoding, depth, digits",
        )


if __name__ == "__main__":
    unittest.main()
