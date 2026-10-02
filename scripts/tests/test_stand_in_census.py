"""The census of stand-ins that run a real program when their plant fails (SPEC-129 A19, #497; A23
to A26, #532).

Every planted text below is built from separate lines at run time, so no single string constant of
this module parses as a stand-in: the census also parses each string constant it meets, and would
otherwise list this module's own plants.
"""

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from _stand_in_census import census, main, reading
from _support import examined

HERE = Path(__file__).parent
EXEC = "os.execv(sys.executable, [sys.executable, *words])"
FAILED = "sys.exit(f'plant_wrapper failed: {failure!r}')"


def stand_in(handler):
    """The text of a stand-in whose plant failure is handled by `handler` (the lines in the except)."""
    lines = [
        "if script.is_file():",
        "    try:",
        "        wrapper = plant_wrapper(script, machine)",
        "    except Exception:",
        *[f"        {line}" for line in handler],
        "    if wrapper is not None:",
        "        sys.exit(wrapper.main())",
        "os.execv(sys.executable, [sys.executable, *words])",
    ]
    return "\n".join(lines) + "\n"


def module(*lines):
    """A module's text from its lines."""
    return "\n".join(lines) + "\n"


def guarded(handler, *after, load="wrapper = load_wrapper(script)", imports=("import os",)):
    """A module that loads its wrapper in a `try`, handles the failure with `handler` (lines in the
    except, which names the failure `failure`), then runs `after` at the module's level."""
    return module(
        *imports,
        "import sys",
        "try:",
        f"    {load}",
        "except Exception as failure:",
        *[f"    {line}" for line in handler],
        *after,
    )


def census_of(files):
    """(files examined, arms) of the census over a scratch directory holding `files`, a mapping of
    each relative path to its text."""
    with tempfile.TemporaryDirectory() as scratch:
        for name, text in files.items():
            path = Path(scratch) / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        return census(scratch)


HANDLER = "except Exception as failure:"


class TheCensusOfStandIns(unittest.TestCase):
    def test_no_stand_in_falls_back_to_a_real_program_on_a_failed_plant(self):
        files, parsed, skipped, found = reading(HERE)
        examined("test files read for stand-ins", range(files))
        print(f"examined {files} files, {parsed} str constants parsed, {skipped} skipped")
        self.assertEqual(found, [])
        self.assertGreater(files, 1)

    def test_a_planted_fallback_is_listed(self):
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "planted.py").write_text(stand_in(["wrapper = None"]), "utf-8")
            files, found = census(scratch)
        self.assertEqual(files, 1)
        self.assertEqual(
            [(name, arm) for name, _, arm in found], [("planted.py", "except Exception:")]
        )
        self.assertEqual(found[0][1], 4)

    def test_a_stand_in_that_exits_on_a_failed_plant_is_not_listed(self):
        closed = ["sys.exit(f'plant_wrapper failed: {failure!r}')"]
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "closed.py").write_text(stand_in(closed), "utf-8")
            files, found = census(scratch)
        self.assertEqual((files, found), (1, []))

    def test_the_census_lists_the_arm_the_base_carried_when_run_over_its_text(self):
        base = stand_in(["wrapper = None"])
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "base.py").write_text(base, "utf-8")
            (Path(scratch) / "other.py").write_text("x = 1\n", "utf-8")
            files, found = census(scratch)
        self.assertEqual((files, len(found)), (2, 1))

    def test_a_system_call_after_the_handler_is_listed(self):
        load = "wrapper = plant_wrapper(script)"
        planted = guarded(["wrapper = None"], "os.system('cargo test')", load=load)
        self.assertEqual(census_of({"planted.py": planted}), (1, [("planted.py", 5, HANDLER)]))
        control = guarded([FAILED], "os.system('cargo test')", load=load)
        self.assertEqual(census_of({"planted.py": control}), (1, []))

    def test_an_exec_far_after_the_handler_is_listed(self):
        load = "wrapper = plant_wrapper(script)"
        far = [f"step_{k} = {k}" for k in range(13)]
        planted = guarded(["wrapper = None"], *far, EXEC, load=load)
        self.assertEqual(census_of({"planted.py": planted}), (1, [("planted.py", 5, HANDLER)]))
        control = guarded([FAILED], *far, EXEC, load=load)
        self.assertEqual(census_of({"planted.py": control}), (1, []))

    def test_a_plant_whose_text_has_no_plant_word_is_listed(self):
        planted = guarded(["wrapper = None"], EXEC)
        self.assertNotIn("plant", planted)
        self.assertEqual(census_of({"planted.py": planted}), (1, [("planted.py", 5, HANDLER)]))
        control = guarded([FAILED], EXEC)
        self.assertEqual(census_of({"planted.py": control}), (1, []))

    def test_a_stand_in_in_a_subdirectory_is_listed(self):
        planted = stand_in(["wrapper = None"])
        files = {"sub/inner.py": planted, "a/b/deep.py": planted}
        self.assertEqual(
            census_of(files),
            (
                2,
                [("a/b/deep.py", 4, "except Exception:"), ("sub/inner.py", 4, "except Exception:")],
            ),
        )
        control = stand_in([FAILED])
        self.assertEqual(census_of({"sub/inner.py": control, "a/b/deep.py": control}), (2, []))

    def test_a_program_run_in_the_handler_itself_is_listed(self):
        planted = guarded([EXEC])
        self.assertEqual(census_of({"planted.py": planted}), (1, [("planted.py", 5, HANDLER)]))

    def test_a_program_run_in_the_finally_block_is_listed(self):
        planted = guarded(["wrapper = None"], "finally:", f"    {EXEC}")
        self.assertEqual(census_of({"planted.py": planted}), (1, [("planted.py", 5, HANDLER)]))

    def test_a_program_run_in_the_else_block_is_not_listed(self):
        planted = guarded(["wrapper = None"], "else:", f"    {EXEC}")
        self.assertEqual(census_of({"planted.py": planted}), (1, []))
        reached = guarded(["wrapper = None"], EXEC)
        self.assertEqual(census_of({"planted.py": reached}), (1, [("planted.py", 5, HANDLER)]))

    def test_a_program_run_in_a_nested_def_is_not_listed(self):
        planted = guarded(["wrapper = None"], "def later():", f"    {EXEC}")
        self.assertEqual(census_of({"planted.py": planted}), (1, []))
        lambdas = guarded(["wrapper = None"], f"later = lambda: {EXEC}")
        self.assertEqual(census_of({"planted.py": lambdas}), (1, []))
        reached = guarded(["wrapper = None"], "def later():", "    pass", EXEC)
        self.assertEqual(census_of({"planted.py": reached}), (1, [("planted.py", 5, HANDLER)]))

    def test_a_return_inside_a_def_in_the_handler_does_not_leave(self):
        handler = ["def fallback():", "    return None", "wrapper = fallback()"]
        planted = guarded(handler, EXEC)
        self.assertEqual(census_of({"planted.py": planted}), (1, [("planted.py", 5, HANDLER)]))
        control = guarded(["return None"], EXEC)
        self.assertEqual(census_of({"planted.py": control}), (1, []))

    def test_a_call_through_an_import_alias_is_listed(self):
        spellings = {
            "import os as o": "o.system('cargo test')",
            "from os import execv as run_it": "run_it(sys.executable, [sys.executable])",
            "from os import posix_spawnp": "posix_spawnp('cargo', ['cargo'], {})",
            "import subprocess as sp": "sp.run(['cargo', 'test'])",
            "from runpy import run_path": "run_path('stand_in.py')",
        }
        for imported, call in examined("import spellings", spellings.items()):
            with self.subTest(imported):
                planted = guarded(["wrapper = None"], call, imports=(imported,))
                self.assertEqual(
                    census_of({"planted.py": planted}), (1, [("planted.py", 5, HANDLER)])
                )
                unbound = guarded(["wrapper = None"], call, imports=("import json",))
                self.assertEqual(census_of({"planted.py": unbound}), (1, []))

    def test_a_call_in_a_nested_block_is_listed(self):
        openers = {
            "for": "for attempt in range(2):",
            "while": "while retrying:",
            "with": "with lock:",
            "if": "if script.is_file():",
            "try": "try:",
        }
        tails = {"try": ("except OSError:", "    raise")}
        for kind, opener in examined("enclosing blocks", openers.items()):
            for where, call in (("its block", "    os.system('cargo test')"), ("after", EXEC)):
                with self.subTest(kind=kind, where=where):
                    planted = module(
                        "import os",
                        "import sys",
                        opener,
                        "    try:",
                        "        wrapper = load_wrapper(script)",
                        "    except Exception:",
                        "        wrapper = None",
                        "    step = 1",
                        *((call,) if where == "its block" else ()),
                        *tails.get(kind, ()),
                        *((call,) if where == "after" else ()),
                    )
                    self.assertEqual(
                        census_of({"planted.py": planted}),
                        (1, [("planted.py", 6, "except Exception:")]),
                    )
        scoped = module(
            "import os",
            "def run():",
            "    try:",
            "        wrapper = load_wrapper(script)",
            "    except Exception:",
            "        wrapper = None",
            EXEC,
        )
        self.assertEqual(census_of({"planted.py": scoped}), (1, []))

    def test_a_stand_in_held_in_a_str_constant_is_listed_at_its_line(self):
        held = module("import os", "", f"SOURCE = {stand_in(['wrapper = None'])!r}")
        self.assertEqual(
            census_of({"held.py": held}),
            (1, [("held.py", 3, "str constant, its line 4: except Exception:")]),
        )
        closed = module("import os", "", f"SOURCE = {stand_in([FAILED])!r}")
        self.assertEqual(census_of({"held.py": closed}), (1, []))

    def test_the_census_counts_the_constants_it_parses_and_skips(self):
        counted = module("PARSES = 'x = 1'", "SKIPPED = 'def ('", "ALSO = f'{x} = 1'")
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "counted.py").write_text(counted, encoding="utf-8")
            self.assertEqual(reading(scratch), (1, 1, 1, []))
            printed = io.StringIO()
            with contextlib.redirect_stdout(printed):
                code = main(scratch)
        self.assertEqual(
            printed.getvalue(), "examined 1 files, 1 str constants parsed, 1 skipped, 0 arms\n"
        )
        self.assertEqual(code, 0)

    def test_a_stand_in_assembled_at_run_time_is_not_listed(self):
        source = stand_in(["wrapper = None"])
        cut = source.index("except")
        assembled = module("import os", f"SOURCE = {source[:cut]!r} + {source[cut:]!r}")
        self.assertEqual(census_of({"assembled.py": assembled}), (1, []))
        whole = module("import os", f"SOURCE = {source!r}")
        self.assertEqual(len(census_of({"whole.py": whole})[1]), 1)

    def test_a_call_through_a_computed_name_is_not_listed(self):
        computed = guarded(["wrapper = None"], "getattr(os, 'system')('cargo test')")
        self.assertEqual(census_of({"planted.py": computed}), (1, []))
        named = guarded(["wrapper = None"], "os.system('cargo test')")
        self.assertEqual(census_of({"planted.py": named}), (1, [("planted.py", 5, HANDLER)]))

    def test_a_program_run_by_a_function_the_handler_calls_is_not_listed(self):
        helper = ("def fallback():", f"    {EXEC}")
        called = guarded(["fallback()"], imports=("import os", *helper))
        self.assertEqual(census_of({"planted.py": called}), (1, []))
        later = guarded(["wrapper = None"], "fallback()", imports=("import os", *helper))
        self.assertEqual(census_of({"planted.py": later}), (1, []))
        inline = guarded(["wrapper = None"], EXEC, imports=("import os", *helper))
        self.assertEqual(census_of({"planted.py": inline}), (1, [("planted.py", 7, HANDLER)]))

    def test_a_handler_that_leaves_on_some_paths_counts_as_leaving(self):
        some = guarded(["if strict:", "    raise", "wrapper = None"], EXEC)
        self.assertEqual(census_of({"planted.py": some}), (1, []))
        never = guarded(["if strict:", "    wrapper = None", "wrapper = None"], EXEC)
        self.assertEqual(census_of({"planted.py": never}), (1, [("planted.py", 5, HANDLER)]))


if __name__ == "__main__":
    unittest.main()
