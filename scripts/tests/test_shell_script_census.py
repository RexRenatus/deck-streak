"""Every tracked shell script is checked by the pinned ShellCheck, which starts at warn (SPEC-655,
ADR-677).

The hygiene job's ShellCheck step downloads the pinned release against its digest, lists every
tracked shell script by R1's rule, runs the tool over that list with `--norc --format=json1
--severity=style`, and saves the list, the tool's version, its exit status and its output under
`$CHECK_LOG_DIR/shellcheck`, before the python stage runs this module. This census never runs
ShellCheck. It lists the same population from the tree and refuses an empty one (R2), refuses a
saved output that is missing, off the pin or over another list (R3, R4), and judges each finding
against `LISTED` (R5, R6): at warn an unlisted finding prints a `WARN` line and passes, at fail it
fails, and a listed finding no longer found fails as `STALE` at both levels. The planted files
under FIXTURES end in `.fixture`, so R1's rule counts none of them in the tree.
"""

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "shell-scripts"
# R1: a tracked file is a shell script by its extension or, when its name holds no dot, by a first
# line that names a shell ShellCheck reads, directly or through `env`. A link is no script.
EXTENSIONS = (".sh", ".bash", ".dash", ".ksh")
SHELLS = ("sh", "bash", "dash", "ksh")
ENV = r"(?:env\s+(?:-S\s+)?)?"
SHEBANG = re.compile(rf"#!\s*/(?:\S*/)?{ENV}({'|'.join(SHELLS)})(?:\s|$)")
# R3: the release the step pins, and the files it saves for this census.
VERSION = "0.11.0"
SAVED = ("files.txt", "version.txt", "status.txt", "output.json")
# R5: `warn` or `fail`. Moving it to `fail` is its own delivery (ADR-677 D4).
LEVEL = "warn"
# R6: today's findings, keyed by path and check code, each with the count found and the reason it
# stands. An unlisted finding prints at warn.
LISTED = {}


def tracked(root=REPO):
    """The paths git tracks below `root`."""
    listing = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"], capture_output=True, check=True
    ).stdout.decode("utf-8")
    return [path for path in listing.split("\0") if path]


def shell_of(path):
    """R1: the shell a file's first line names, or None."""
    with path.open("rb") as script:
        first = script.readline(512).decode("utf-8", "replace")
    named = SHEBANG.match(first)
    return named.group(1) if named else None


def population(root, paths):
    """R1: the shell scripts among `paths` below `root`, sorted."""
    return []


def saved(folder):
    """R3, R4: the step's saved list and findings, or VOID: a file missing, a tool off its pin, an
    exit status that says a script was not judged whole, or a status its findings contradict."""
    return [], []


def verdict(scripts, files, comments, listed, level):
    """R4 to R6: the WARN lines to print and the problems that fail, for one saved output."""
    return [], []


def judge(root, paths, folder, listed=LISTED, level=LEVEL):
    """The census over one tree: R2's count first, then the saved output, then the verdict."""
    files, comments = saved(folder)
    return verdict(population(root, paths), files, comments, listed, level)


def planted(root, *names):
    """Plant fixtures in `root` under their names without `.fixture`, and return the names."""
    for name in names:
        shutil.copyfile(FIXTURES / f"{name}.fixture", Path(root) / name)
    return list(names)


def save(folder, files, output, status, version="version.txt"):
    """Write a saved output the way the step writes one, from the planted fixtures."""
    folder = Path(folder)
    folder.mkdir()
    (folder / "files.txt").write_text("".join(f"{name}\n" for name in files), encoding="utf-8")
    shutil.copyfile(FIXTURES / f"{version}.fixture", folder / "version.txt")
    (folder / "status.txt").write_text(f"{status}\n", encoding="utf-8")
    shutil.copyfile(FIXTURES / f"{output}.fixture", folder / "output.json")
    return folder


class PlantedTreesAreJudged(unittest.TestCase):
    def test_a_script_is_counted_by_its_extension_or_its_shell_shebang_and_nothing_else(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmp:
            names = planted(tmp, "clean.sh", "direct-tool", "fake-tool", "not-shell")
            (Path(tmp) / "notes.txt").write_text("#!/bin/sh\n", encoding="utf-8")
            os.symlink("clean.sh", Path(tmp) / "linked.sh")
            self.assertEqual(
                population(tmp, [*names, "notes.txt", "linked.sh"]),
                ["clean.sh", "direct-tool", "fake-tool"],
            )

    def test_an_unlisted_finding_prints_a_warn_line_at_warn_and_fails_at_fail(self):
        self.assertEqual(LEVEL, "warn")
        said = "WARN finding.sh SC2086 found 1, listed 0, at line(s) 3"
        with tempfile.TemporaryDirectory() as tmp:
            names = planted(tmp, "clean.sh", "finding.sh")
            folder = save(Path(tmp) / "out", names, "output-finding.json", "1")
            self.assertEqual(judge(tmp, names, folder, {}, "warn"), ([said], []))
            self.assertEqual(judge(tmp, names, folder, {}, "fail"), ([], [said]))

    def test_a_listed_finding_passes_and_one_no_longer_found_is_stale_at_both_levels(
        self,
    ):
        listed = {("finding.sh", "SC2086"): (1, "a planted reason")}
        with tempfile.TemporaryDirectory() as tmp:
            names = planted(tmp, "clean.sh", "finding.sh")
            found = save(Path(tmp) / "found", names, "output-finding.json", "1")
            clean = save(Path(tmp) / "clean", names, "output-clean.json", "0")
            for level in ("warn", "fail"):
                self.assertEqual(judge(tmp, names, found, listed, level), ([], []))
                self.assertEqual(
                    judge(tmp, names, clean, listed, level),
                    ([], ["STALE finding.sh SC2086 found 0, listed 1"]),
                )
                bare = {("finding.sh", "SC2086"): (1, " ")}
                self.assertEqual(
                    judge(tmp, names, found, bare, level),
                    ([], ["UNREASONED finding.sh SC2086 listed 1, with no reason"]),
                )

    def test_an_empty_population_and_a_saved_output_off_its_pin_or_its_list_are_refused(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmp:
            empty = save(Path(tmp) / "empty", [], "output-clean.json", "0")
            for paths in ([], planted(tmp, "not-shell")):
                with self.assertRaisesRegex(AssertionError, "examined 0 shell script"):
                    judge(tmp, paths, empty)
            names = planted(tmp, "clean.sh", "finding.sh")
            missing = save(Path(tmp) / "missing", names, "output-clean.json", "0")
            (missing / "output.json").unlink()
            off = save(
                Path(tmp) / "off",
                names,
                "output-clean.json",
                "0",
                "version-off-pin.txt",
            )
            refused = {
                "VOID: no saved output.json": missing,
                f"is not ShellCheck {VERSION}'s": off,
                "VOID: ShellCheck exited 2,": save(
                    Path(tmp) / "two", names, "output-clean.json", "2"
                ),
                "exited 0 with 1 finding": save(
                    Path(tmp) / "zero", names, "output-finding.json", "0"
                ),
                "exited 1 with 0 finding": save(Path(tmp) / "one", names, "output-clean.json", "1"),
            }
            for said, folder in refused.items():
                with (
                    self.subTest(said),
                    self.assertRaisesRegex(AssertionError, re.escape(said)),
                ):
                    judge(tmp, names, folder)
            drift = save(Path(tmp) / "drift", ["clean.sh"], "output-clean.json", "0")
            self.assertEqual(
                judge(tmp, names, drift),
                ([], ["DRIFT: the step checked 1 script(s), not the census's 2"]),
            )


class TheRepositorysScriptsAreChecked(unittest.TestCase):
    def test_every_tracked_shell_script_is_judged_from_the_pinned_shellchecks_saved_output(
        self,
    ):
        paths = tracked()
        self.assertIn("scripts/check.sh", population(REPO, paths))
        log_dir = os.environ.get("CHECK_LOG_DIR")
        if not log_dir:
            self.fail("VOID: no $CHECK_LOG_DIR, so no saved ShellCheck output to judge")
        warned, problems = judge(REPO, paths, Path(log_dir) / "shellcheck")
        for said in warned:
            print(said)
        print(f"{len(warned)} unlisted finding key(s) at {LEVEL}")
        self.assertEqual(problems, [])


if __name__ == "__main__":
    unittest.main()
