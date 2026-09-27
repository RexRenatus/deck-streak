"""Dependabot moves a major version only with the house radar (SPEC-033 A13)."""

import re
import unittest

from _support import REPO, examined

CONFIG = REPO / ".github" / "dependabot.yml"
# stack.json pins the majors of these ecosystems; the owner moves them on the radar.
PINNED = ["cargo", "npm"]
MAJOR_IGNORED = re.compile(
    r'(?m)^    ignore:\n      - dependency-name: "\*"\n'
    r'        update-types: \["version-update:semver-major"\]$'
)


def ecosystems(text):
    blocks = re.split(r"(?m)^  - package-ecosystem: *", text)[1:]
    return {block.split("\n", 1)[0].strip(): block for block in blocks}


class DependabotFollowsTheRadar(unittest.TestCase):
    def test_a_major_version_moves_only_with_the_house_radar(self):
        found = ecosystems(CONFIG.read_text(encoding="utf-8"))
        self.assertLessEqual(set(PINNED), set(found))
        for name in examined("pinned ecosystem(s)", PINNED):
            self.assertRegex(found[name], r"(?m)^    target-branch: dev$", name)
            self.assertRegex(found[name], MAJOR_IGNORED, name)


if __name__ == "__main__":
    unittest.main()
