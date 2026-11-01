"""The write-back script writes each known key as its issue number, and refuses an unknown key by
name before it writes anything (SPEC-437 R4 and R5, A6 to A8).

Each test loads the script by its path and points its manifest at one in a temporary directory,
so it reads and writes only there. Each compares whole printed lines, an exit code and every
file's bytes.
"""

import contextlib
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO

SCRIPT = REPO / "scripts" / "write-back-issues.py"
MAP = REPO / "scripts" / "mutation-python.json"
ISSUES = {"issues": {"DS-W1-07": {"number": 812}, "EPIC-W3": {"number": 5}}}


def write_back_module():
    """The write-back script, imported as a module, so a test points its manifest elsewhere."""
    spec = importlib.util.spec_from_file_location("write_back_issues", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class TheIssuesAreWrittenBack(unittest.TestCase):
    def setUp(self):
        holder = tempfile.TemporaryDirectory()
        self.addCleanup(holder.cleanup)
        self.home = Path(holder.name)
        self.manifest = self.home / "issues-manifest.json"
        self.manifest.write_text(json.dumps(ISSUES), encoding="utf-8")

    def planted(self, texts):
        """Each text written to its own file under the scratch directory, in order."""
        paths = []
        for index, text in enumerate(texts):
            path = self.home / f"doc-{index}.md"
            path.write_bytes(text.encode("utf-8"))
            paths.append(path)
        return paths

    def write_back(self, paths):
        module = write_back_module()
        sink = io.StringIO()
        with mock.patch.object(module, "MANIFEST", self.manifest):
            with contextlib.redirect_stdout(sink):
                code = module.main([str(path) for path in paths])
        return code, sink.getvalue()

    def test_a_known_key_is_written_as_its_issue_number(self):
        """A6: both keys the manifest holds are written as `#<number>`, a file with no token is
        written as it was, and the count of files written is printed."""
        paths = self.planted(["see {{issue:DS-W1-07}} and {{issue:EPIC-W3}}\n", "no token here\n"])
        code, said = self.write_back(paths)
        self.assertEqual((code, said), (0, "write-back-issues: 2 file(s) written\n"))
        self.assertEqual(
            [path.read_bytes() for path in paths], [b"see #812 and #5\n", b"no token here\n"]
        )

    def test_an_unknown_key_is_named_and_nothing_is_written(self):
        """A7: two keys the manifest lacks are named in sorted order, the exit is 1, and no file
        is written, the one holding only a known key included."""
        texts = [
            "see {{issue:DS-W1-07}}\n",
            "{{issue:OWN-G4}} and {{issue:EPIC-W3}} and {{issue:DS-W9-01}}\n",
        ]
        paths = self.planted(texts)
        code, said = self.write_back(paths)
        self.assertEqual(
            (code, said),
            (1, "write-back-issues: no issue for ['DS-W9-01', 'OWN-G4']; nothing written\n"),
        )
        self.assertEqual(
            [path.read_bytes() for path in paths], [text.encode("utf-8") for text in texts]
        )

    def test_the_map_runs_this_module_for_the_write_back_script(self):
        """A8: the mutation map names this module for the write-back script, so its mutants are
        run against these tests."""
        entry = json.loads(MAP.read_text(encoding="utf-8"))["scripts/write-back-issues.py"]
        self.assertEqual(entry, {"dir": "scripts/tests", "modules": ["test_write_back_issues"]})


if __name__ == "__main__":
    unittest.main()
