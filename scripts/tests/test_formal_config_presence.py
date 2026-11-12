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

from _support import examined, refuse_link_components

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
# A link at the path's directory: the checker reads the path in HEAD's tree, where the directory is a
# link's own text, so no file is at the path and the one behind the link is never read.
LINK_DIR = "a link at the directory"


def unloadable():
    """Every way the committed file fails to load, one per arm of the loader, as (label, bytes,
    None for no file, LINK or LINK_DIR): absent; a link, here to a readable copy of the declared
    file; the file's directory a link, here to a directory holding the declared file; and each way
    the parser refuses bytes: syntax, encoding, a depth past every recursion limit it keeps, and an
    integer past the interpreter's digit limit."""
    deep = sys.getrecursionlimit() * 100
    return [
        ("the file is absent", None),
        ("the file is a link", LINK),
        ("the file's directory is a link", LINK_DIR),
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
    """The plant as a file in a fresh `directory`, laid out as the repository lays the settings
    file, `directory/config/formal.json`, and its path. Every plant is a kind the harness knows: a
    payload of none of them is refused here, by assertion."""
    directory.mkdir()
    if payload is LINK_DIR:
        (directory / "declared").mkdir()
        (directory / "declared" / "formal.json").write_text(
            json.dumps(module.EXPECTED, indent=2) + "\n", encoding="utf-8"
        )
        (directory / "config").symlink_to("declared")
        return directory / "config" / "formal.json"
    (directory / "config").mkdir()
    path = directory / "config" / "formal.json"
    if payload is LINK:
        target = path.with_name("declared.json")
        target.write_text(json.dumps(module.EXPECTED, indent=2) + "\n", encoding="utf-8")
        path.symlink_to(target.name)
    elif isinstance(payload, bytes):
        path.write_bytes(payload)
    elif payload is not None:
        raise AssertionError(f"a plant of a kind the harness does not know: {payload!r}")
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
    module.ROOT = path.parents[1]
    suite = unittest.TestSuite(module.FormalConfig(name) for name in names)
    with contextlib.redirect_stdout(io.StringIO()):
        result = unittest.TextTestRunner(stream=io.StringIO(), verbosity=0).run(suite)
    failed = {test._testMethodName for test, _ in result.failures}
    errored = {
        test._testMethodName: trace.strip().splitlines()[-1] for test, trace in result.errors
    }
    return failed, errored


# A link can stand at any component of a path, in any of these kinds. The checker reads the path in
# HEAD's tree, so a link at any component holds no file there, whatever it names.
RELATIVE = "a relative link"
ABSOLUTE = "an absolute link"
CHAIN = "a chain of two links"
TO_DIRECTORY = "a link to a directory"
DANGLING = "a dangling link"
LOOP = "a link to itself"
LINK_KINDS = (RELATIVE, ABSOLUTE, CHAIN, TO_DIRECTORY, DANGLING, LOOP)


def lay(root, module, settings):
    """A tree under `root` holding every path the module reads, each as a real file: the settings
    file and a pin file beside it."""
    (root / "config").mkdir()
    (root / settings).write_text(json.dumps(module.EXPECTED, indent=2) + "\n", encoding="utf-8")
    (root / module.PIN_FILE).write_text("{}\n", encoding="utf-8")


def link_at(root, relative, kind):
    """Replace the real entry at `relative` under `root` by a link of `kind`; the real entry moves
    beside it, so a link that is followed finds the file the tree would hold."""
    entry = root / relative
    real = entry.with_name(entry.name + ".real")
    entry.rename(real)
    if kind == RELATIVE:
        entry.symlink_to(real.name)
    elif kind == ABSOLUTE:
        entry.symlink_to(real)
    elif kind == CHAIN:
        hop = entry.with_name(entry.name + ".hop")
        hop.symlink_to(real.name)
        entry.symlink_to(hop.name)
    elif kind == TO_DIRECTORY:
        elsewhere = root / "elsewhere"
        elsewhere.mkdir(exist_ok=True)
        entry.symlink_to(os.path.relpath(elsewhere, entry.parent))
    elif kind == DANGLING:
        entry.symlink_to("nowhere")
    elif kind == LOOP:
        entry.symlink_to(entry.name)
    else:
        raise AssertionError(f"a link of a kind the harness does not know: {kind!r}")
    return entry


def is_kind(entry, kind):
    """Whether the link at `entry` is the kind it is labelled."""
    if not entry.is_symlink():
        return False
    target = os.readlink(entry)
    if kind == RELATIVE:
        return not os.path.isabs(target) and entry.parent.joinpath(target).exists()
    if kind == ABSOLUTE:
        return os.path.isabs(target) and Path(target).exists()
    if kind == CHAIN:
        return entry.parent.joinpath(target).is_symlink() and entry.exists()
    if kind == TO_DIRECTORY:
        return entry.parent.joinpath(target).is_dir() and entry.is_dir()
    if kind == DANGLING:
        return not os.path.lexists(entry.parent / target)
    return kind == LOOP and target == entry.name


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
                    self.assertFalse(path.parent.is_symlink(), f"{label}: a link at the directory")
                elif payload is LINK_DIR:
                    self.assertTrue(path.parent.is_symlink(), f"{label}: no link at the directory")
                    self.assertFalse(path.is_symlink(), f"{label}: a link at the path")
                    self.assertTrue(path.is_file(), f"{label}: no file behind the link")
                else:
                    self.assertFalse(path.parent.is_symlink(), f"{label}: a link at the directory")
                    self.assertTrue(os.path.lexists(path), f"{label}: no file is there")
                    self.assertFalse(path.is_symlink(), f"{label}: a link is there")
                    self.assertEqual(path.read_bytes(), payload, f"{label}: other bytes")
                    refusal = refusal_of(payload)
                    self.assertEqual(refusal is not None, unloads, f"{label}: the parser's verdict")
                    if refusal is not None:
                        refusals.add(refusal)
                self.assertTrue(
                    unloads or payload not in (None, LINK, LINK_DIR), f"{label}: marked loadable"
                )
        self.assertEqual(
            refusals,
            {"JSONDecodeError", "UnicodeDecodeError", "RecursionError", "ValueError"},
            "the parser's four ways to refuse the bytes, each planted: syntax, encoding, depth, digits",
        )

    def test_a_link_at_any_component_of_a_path_the_module_reads_is_refused_by_assertion(self):
        """A8: the module reads the settings file and the pin path as the tree stores them, and a
        link at any component of either, from the directory the file sits in to the file itself, is
        a path the checker finds no file at. Every component of every path the module reads is
        replaced by a link of every kind in a fresh tree, and the reader of the path then refuses
        it by assertion naming that component; a link at the pin file itself is the pin file, a
        second source, and so is listed."""
        module = fresh_module()
        settings = module.CONFIG.relative_to(module.REPO)
        pin = module.PIN_FILE
        paths = [("settings", settings), ("pin", pin)]
        spots = examined(
            "components of the paths the module reads",
            [
                (name, path, depth)
                for name, path in paths
                for depth in range(1, len(path.parts) + 1)
            ],
        )
        cases = [(spot, kind) for spot in spots for kind in LINK_KINDS]
        wrong = []
        with tempfile.TemporaryDirectory() as scratch:
            for index, ((name, path, depth), kind) in enumerate(
                examined(f"link(s) at {len(spots)} component(s)", cases)
            ):
                label = f"{kind} at {Path(*path.parts[:depth]).as_posix()} of the {name} path"
                root = Path(scratch) / str(index)
                root.mkdir()
                lay(root, module, settings)
                component = Path(*path.parts[:depth])
                entry = link_at(root, component, kind)
                if not is_kind(entry, kind):
                    wrong.append(f"{label}: not planted as that kind")
                    continue
                module.CONFIG = root / settings
                module.ROOT = root
                last = depth == len(path.parts)
                try:
                    if name == "settings":
                        module.load()
                    else:
                        found = module.toolchain_sources(module.EXPECTED, root)
                except AssertionError as refusal:
                    if last and name == "pin":
                        wrong.append(f"{label}: refused, not listed as the pin file")
                    elif not str(refusal).startswith(f"{component.as_posix()} is a link"):
                        wrong.append(f"{label}: refused without naming the component: {refusal}")
                except Exception as error:
                    wrong.append(f"{label}: an error, not a refusal: {error!r}")
                else:
                    if last and name == "pin":
                        want = ["toolchain.identity", pin.as_posix()]
                        if found != want:
                            wrong.append(f"{label}: listed {found}, not {want}")
                    else:
                        wrong.append(f"{label}: not refused")
        self.assertEqual(
            wrong, [], "a link at a component of a path the module reads was followed or misread"
        )
        self.assertEqual(
            len(cases), len(spots) * len(LINK_KINDS), "a case of the product was not planted"
        )
        self.assertIn(("pin", pin, len(pin.parts)), spots, "the pin file itself is not planted")
        self.assertIn(("settings", settings, 1), spots, "the first component is not planted")

    def test_the_walk_admits_a_tree_with_no_link_and_refuses_a_path_outside_its_root(self):
        """The walk's own bounds, held by a control on each side: a real tree is admitted at every
        component, a tree whose root is itself a link is admitted too, the root being where the
        checkout sits and no component of a path in it, and a path outside the root is refused by
        assertion, never admitted unread."""
        module = fresh_module()
        settings = module.CONFIG.relative_to(module.REPO)
        with tempfile.TemporaryDirectory() as scratch:
            real = Path(scratch) / "real"
            real.mkdir()
            lay(real, module, settings)
            alias = Path(scratch) / "alias"
            alias.symlink_to(real.name)
            outside = Path(scratch) / "outside.json"
            outside.write_text("{}\n", encoding="utf-8")
            for root in examined("roots a real tree is admitted in", [real, alias]):
                for relative in (settings, module.PIN_FILE):
                    refuse_link_components(root / relative, root)
            with self.assertRaises(AssertionError, msg="a path outside the root"):
                refuse_link_components(outside, real)


if __name__ == "__main__":
    unittest.main()
