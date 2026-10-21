"""Within one tree the sync service never states a minimum its own clients fail (SPEC-374 R3, A7;
ADR-385): `CLIENT_LEVEL`, the level both clients build, and `MINIMUM_CLIENT_LEVEL`, the oldest
level the service accepts, are each defined once in the tree's Rust sources, and
`1 <= MINIMUM_CLIENT_LEVEL <= CLIENT_LEVEL`.

The census is a function of a root and the source paths it is handed: `git grep` lists the `.rs`
files that name either constant, the tracked ones in the live tree and every one in a planted
control, a temporary tree the census must refuse by name, and only those are read."""

import re
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

LEVEL = "CLIENT_LEVEL"
MINIMUM = "MINIMUM_CLIENT_LEVEL"
# Where SPEC-374 R1 and R2 define each.
SITES = {
    LEVEL: ["crates/engine-core/src/handshake.rs"],
    MINIMUM: ["crates/api/src/minimum_client.rs"],
}
# A definition of either constant, at any visibility, with its value.
DEFINITION = re.compile(
    r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?const[ \t]+(CLIENT_LEVEL|MINIMUM_CLIENT_LEVEL)"
    r"[ \t]*:[ \t]*u32[ \t]*=[ \t]*([0-9_]+)[ \t]*;"
)


def tracked_sources(root):
    """Every `.rs` file git tracks at `root`, as git lists it."""
    listed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--", "*.rs"],
        capture_output=True,
        text=True,
        check=True,
    )
    return [path for path in listed.stdout.split("\0") if path]


def naming_sources(root, tracked=True):
    """The `.rs` files under `root` that name either constant, as `git grep` lists them: those git
    tracks in a repository, or every one in a planted tree that is not one (`tracked=False`)."""
    command = ["git", "-C", str(root), "grep", "-l", "-z", "-w", "-e", LEVEL, "-e", MINIMUM]
    if not tracked:
        command.append("--no-index")
    listed = subprocess.run([*command, "--", "*.rs"], capture_output=True, text=True, check=False)
    # git grep exits 1 when nothing matched, which the census then judges; anything else failed.
    if listed.returncode not in (0, 1):
        raise AssertionError(f"git grep failed at {root}: {listed.stderr}")
    return sorted(path for path in listed.stdout.split("\0") if path)


def definitions(root, paths):
    """{name: [(path, value)]} for every definition of either constant in `paths` under `root`."""
    found = {LEVEL: [], MINIMUM: []}
    for path in paths:
        text = (Path(root) / path).read_text(encoding="utf-8")
        for match in DEFINITION.finditer(text):
            found[match.group(1)].append((path, int(match.group(2).replace("_", ""))))
    return found


def refusals(found):
    """Why the definitions `found` break R3, each by name; empty when they hold."""
    problems = [
        f"{name} is defined {len(found[name])} time(s), not once: {found[name]}"
        for name in (LEVEL, MINIMUM)
        if len(found[name]) != 1
    ]
    if problems:
        return problems
    level = found[LEVEL][0][1]
    minimum = found[MINIMUM][0][1]
    if not 1 <= minimum <= level:
        problems.append(f"{MINIMUM} {minimum} is not between 1 and {LEVEL} {level}")
    return problems


def plant(root, files):
    """A temporary tree at `root` holding `files`, {path: text}."""
    for path, text in files.items():
        target = Path(root) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


class ClientLevelTest(unittest.TestCase):
    def test_the_minimum_never_exceeds_the_level_a_tree_builds(self):
        found = definitions(REPO, naming_sources(REPO))
        self.assertEqual(refusals(found), [], "the tree's minimum and level break R3")
        self.assertEqual(
            {name: [path for path, _ in sites] for name, sites in found.items()},
            SITES,
            "each constant is defined where the SPEC names it",
        )
        examined("tracked Rust source(s)", tracked_sources(REPO))

        planted = [
            (
                "a minimum above the level",
                {
                    "crates/core/src/level.rs": "pub const CLIENT_LEVEL: u32 = 1;\n",
                    "crates/service/src/minimum.rs": "pub const MINIMUM_CLIENT_LEVEL: u32 = 2;\n",
                },
                [f"{MINIMUM} 2 is not between 1 and {LEVEL} 1"],
            ),
            (
                "the level defined twice",
                {
                    "crates/core/src/level.rs": "pub const CLIENT_LEVEL: u32 = 2;\n",
                    "crates/web/src/level.rs": "const CLIENT_LEVEL: u32 = 3;\n",
                    "crates/service/src/minimum.rs": "pub const MINIMUM_CLIENT_LEVEL: u32 = 1;\n",
                },
                [
                    f"{LEVEL} is defined 2 time(s), not once: "
                    "[('crates/core/src/level.rs', 2), ('crates/web/src/level.rs', 3)]"
                ],
            ),
        ]
        judged = []
        for case, files, expected in planted:
            with tempfile.TemporaryDirectory() as root:
                plant(root, files)
                named = naming_sources(root, tracked=False)
                judged.append((case, refusals(definitions(root, named))))
            self.assertEqual(judged[-1], (case, expected), "the census refuses a planted tree")
        examined("planted tree(s)", judged)


if __name__ == "__main__":
    unittest.main()
