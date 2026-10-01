"""The census of stand-ins that run a real program when their plant fails (SPEC-129 A19, #497)."""

import tempfile
import unittest
from pathlib import Path

from _stand_in_census import census
from _support import examined

HERE = Path(__file__).parent


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


class TheCensusOfStandIns(unittest.TestCase):
    def test_no_stand_in_falls_back_to_a_real_program_on_a_failed_plant(self):
        files, found = census(HERE)
        examined("test files read for stand-ins", range(files))
        self.assertGreater(files, 1)
        self.assertEqual(found, [])

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


if __name__ == "__main__":
    unittest.main()
