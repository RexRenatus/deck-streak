"""SPEC-358 A3 (R2): the flag and bury rules are defined once in the workspace, in the core.

Every Rust file git tracks is read as data, with its comments and its string and character
literals removed, and each of `RED`, `toggled_red`, `BURY_USER`, `BuryOf` and `bury_of` must be
defined exactly once, in `crates/engine-core/src/review.rs`, which both adapters reach. The census
prints counts and the paths of the definitions it finds, never a file's text. The parked Rust
files are dropped by name before any open and counted (#158).
"""

import re
import unittest

from _support import REPO, examined
from test_one_static_library import tracked_paths

HOME = "crates/engine-core/src/review.rs"
PARKED = "crates/vault/src/readings_tree.rs"
RULES = ("RED", "toggled_red", "BURY_USER", "BuryOf", "bury_of")
# A definition of a name: an item that introduces it, never a use, an import or a call.
ITEM = r"\b(?:const|static|fn|struct|enum|union|type|trait|mod)\s+"
DEFINES = {name: re.compile(rf"{ITEM}{name}\b|\bmacro_rules!\s*{name}\b") for name in RULES}


RAW = re.compile(r'b?r(#*)"')
CHAR = re.compile(r"'(?:\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f]{1,6}\}|.)|[^\\'])'")


def code_of(source):
    """`source` with its comments, string literals and character literals removed: a name in a
    comment or a string is prose, not a definition. Nested block comments and raw strings of any
    hash count are read as the compiler reads them; a lifetime is kept as code."""
    out = []
    index, end = 0, len(source)
    while index < end:
        char = source[index]
        before = source[index - 1] if index else " "
        raw = RAW.match(source, index) if char in "br" else None
        if source.startswith("//", index):
            newline = source.find("\n", index)
            index = end if newline < 0 else newline
        elif source.startswith("/*", index):
            depth, index = 1, index + 2
            while index < end and depth:
                if source.startswith("/*", index):
                    depth, index = depth + 1, index + 2
                elif source.startswith("*/", index):
                    depth, index = depth - 1, index + 2
                else:
                    index += 1
            out.append(" ")
        elif raw is not None and not (before.isalnum() or before == "_"):
            close = '"' + raw.group(1)
            stop = source.find(close, raw.end())
            index = end if stop < 0 else stop + len(close)
            out.append('""')
        elif char == '"':
            index += 1
            while index < end and source[index] != '"':
                index += 2 if source[index] == "\\" else 1
            index += 1
            out.append('""')
        elif char == "'" and (literal := CHAR.match(source, index)) is not None:
            index = literal.end()
            out.append("' '")
        else:
            out.append(char)
            index += 1
    return "".join(out)


def definitions(files):
    """For each rule, the paths of `files` (path to text) that define it, one entry per
    definition, in path order."""
    found = {name: [] for name in RULES}
    for path in sorted(files):
        code = code_of(files[path])
        for name, pattern in DEFINES.items():
            found[name] += [path] * len(pattern.findall(code))
    return found


def problems(files):
    """Each rule not defined exactly once, in `HOME`, named with the paths that define it."""
    return [
        f"{name}: defined {len(paths)} time(s), at {', '.join(paths) or 'no path'}, "
        f"not once in {HOME}"
        for name, paths in definitions(files).items()
        if paths != [HOME]
    ]


def parked(path):
    """Whether `path` names the parked drill surface, read from the name alone."""
    name = path.lower()
    return "drill" in name or "readings_tree" in name


def listing(paths):
    """`paths` split into `(kept, skipped)` by `parked`, in input order; no file is opened."""
    kept = [path for path in paths if not parked(path)]
    skipped = [path for path in paths if parked(path)]
    return kept, skipped


def tracked_rust():
    """Every unparked Rust file git tracks, read as data and never printed, and the count of
    parked ones dropped by name before any open."""
    kept, skipped = listing([path for path in tracked_paths(REPO) if path.endswith(".rs")])
    files = {
        path: (REPO / path).read_text(encoding="utf-8", errors="replace")
        for path in kept
        if (REPO / path).is_file()
    }
    return files, len(skipped)


class TheFlagAndBuryRulesLiveOnceInTheCore(unittest.TestCase):
    def test_the_flag_and_bury_rules_live_once_in_the_core(self):
        """R2: each of the five is defined once in the workspace, in the core's review rule; a
        planted second copy is refused by its name, and a name in a comment or a string is not a
        definition."""
        files, skipped = tracked_rust()
        print(f"parked-skip: {skipped} Rust file(s) dropped by name before any open")
        self.assertGreater(
            skipped, 0, "parked-skip is 0: no parked file was dropped, so the census is red"
        )
        found = definitions(files)
        for name, paths in found.items():
            print(f"{name}: {len(paths)} definition(s): {', '.join(paths) or 'none'}")
        self.assertEqual(problems(files), [])

        # The controls: the rule's own file is accepted, and each plant is refused by name.
        home = (
            "pub const RED: u32 = 1;\n"
            "pub fn toggled_red(flag: u32) -> u32 { if flag == RED { 0 } else { RED } }\n"
            "pub const BURY_USER: i32 = 2;\n"
            "pub struct BuryOf { pub mode: i32 }\n"
            "pub fn bury_of(card: i64) -> BuryOf { let _ = card; BuryOf { mode: BURY_USER } }\n"
        )
        web = "crates/web-engine/src/study.rs"
        prose = (
            "// pub fn toggled_red(flag: u32) -> u32\n"
            "/* pub const RED: u32 = 1; /* struct BuryOf */ fn bury_of */\n"
            'const A: &str = "pub const BURY_USER: i32 = 2; \\" fn bury_of";\n'
            'const B: &str = r#"pub struct BuryOf "fn toggled_red" "#;\n'
            "fn quoted<'a>(x: &'a str) -> char { let _ = x; '\"' }\n"
            "fn after() { let _ = RED; let _ = toggled_red(0); }\n"
        )
        plants = {
            "the rule's own file": ({HOME: home}, []),
            "a second copy of the flag's rule": (
                {HOME: home, web: "pub fn toggled_red(flag: u32) -> u32 { flag }\n"},
                [f"toggled_red: defined 2 time(s), at {HOME}, {web}, not once in {HOME}"],
            ),
            "a second copy of the bury's mode": (
                {HOME: home, web: "pub(crate) const BURY_USER: i32 = 2;\n"},
                [f"BURY_USER: defined 2 time(s), at {HOME}, {web}, not once in {HOME}"],
            ),
            "the names in comments, strings and uses": ({HOME: home, web: prose}, []),
            "the rules in the web engine alone": (
                {web: home},
                [f"{name}: defined 1 time(s), at {web}, not once in {HOME}" for name in RULES],
            ),
        }
        for plant, (planted, wanted) in plants.items():
            with self.subTest(plant=plant):
                self.assertEqual(problems(planted), wanted)
        with self.subTest(plant="the listing drops the parked names, in order"):
            planted = [HOME, "crates/x/src/drill_notes.rs", "crates/x/tests/Drill_cards.rs", PARKED]
            planted.append("crates/x/src/a.rs")
            self.assertEqual(
                listing(planted),
                (
                    [HOME, "crates/x/src/a.rs"],
                    ["crates/x/src/drill_notes.rs", "crates/x/tests/Drill_cards.rs", PARKED],
                ),
            )
        with self.subTest(plant="the rule's own file is not parked"):
            self.assertFalse(parked(HOME))
        with self.subTest(plant="the parked file is parked"):
            self.assertTrue(parked(PARKED))
        examined("Rust files", files)


if __name__ == "__main__":
    unittest.main()
