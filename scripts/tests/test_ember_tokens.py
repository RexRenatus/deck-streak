"""The Ember palette's six role values and the app's colour sets that copy them (SPEC-578 A4 to A8;
ADR-597 D2, D3).

The token file holds the six values once, as `modes.ember`: a background, an ink and an accent for a
dark and a light mode, each an alias of a raw colour in `ref.ember`. The iPhone and iPad app copies
them as three colour sets, `EmberBackground`, `EmberInk` and `EmberAccent`, each with a universal
entry that carries the light value and an entry with the `luminosity` appearance `dark` that carries
the dark value. `findings(root)` reads both and returns one `"<reason>: <path>: <detail>"` line for
each role missing from the token file, colour set or entry missing, value that differs from its
token, and raw colour literal outside the token route (the compile, the page's stylesheet and every
Swift file of the app's sources). The tests run it over this repository, and over fixture trees in a
temporary directory: a planted fault for each reason, and a complete planted set as the control.
The standard library only; nothing is imported from the tree.
"""

import json
import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

TOKENS = Path("web/app/src/lib/design/tokens.json")
CATALOG = Path("ios/App/Assets.xcassets")
COMPILE = Path("web/app/scripts/build-tokens.ts")
STYLESHEET = Path("web/app/src/routes/layout.css")
SWIFT_SOURCES = Path("ios/App/Sources")
MODES = ("dark", "light")
ROLES = ("background", "ink", "accent")
SETS = {"EmberBackground": "background", "EmberInk": "ink", "EmberAccent": "accent"}
ALIAS = re.compile(r"^\{([\w.-]+)\}$")
HEX = re.compile(r"^#[0-9a-f]{6}$")
COMPONENT = re.compile(r"^0x[0-9A-F]{2}$")
DARK_APPEARANCE = [{"appearance": "luminosity", "value": "dark"}]
# A raw colour: a hex literal, a colour function, or a Swift initialiser that takes components.
LITERALS = re.compile(
    r"#[0-9a-fA-F]{3,8}\b"
    r"|\b(?:rgb|hsl)a?\("
    r"|\boklch\("
    r"|\b(?:Color|UIColor)\(\s*(?:red:|hue:|\.sRGB)"
)


def dig(document, dotted):
    """The value at a dotted path of a parsed JSON document, or None when any step is missing."""
    at = document
    for key in dotted.split("."):
        if not isinstance(at, dict) or key not in at:
            return None
        at = at[key]
    return at


def hex_of(document, value, seen=()):
    """The hex of a colour token's `$value`: its own `hex`, or that of the token an alias names."""
    if isinstance(value, str):
        path = ALIAS.match(value)
        if not path or path.group(1) in seen:
            return None
        target = dig(document, path.group(1))
        return hex_of(
            document,
            target.get("$value") if isinstance(target, dict) else None,
            seen + (path.group(1),),
        )
    found = value.get("hex") if isinstance(value, dict) else None
    return found if isinstance(found, str) and HEX.match(found) else None


def token_values(root):
    """The six role values of the token file, as ({(mode, role): hex}, findings)."""
    problems = []
    try:
        document = json.loads((Path(root) / TOKENS).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        document = None
    values = {}
    for mode in MODES:
        for role in ROLES:
            where = f"modes.ember.{mode}.{role}"
            token = dig(document, where)
            found = hex_of(document, token.get("$value")) if isinstance(token, dict) else None
            if found is None:
                problems.append(f"role-missing: {TOKENS.as_posix()}: {where} holds no colour")
            else:
                values[(mode, role)] = found
    return values, problems


def entry_hex(entry):
    """The 8-bit hex an asset catalog colour entry holds, or None when it is not whole sRGB with
    full alpha."""
    colour = entry.get("color") if isinstance(entry, dict) else None
    parts = colour.get("components") if isinstance(colour, dict) else None
    if (
        not isinstance(colour, dict)
        or colour.get("color-space") != "srgb"
        or not isinstance(parts, dict)
    ):
        return None
    if parts.get("alpha") != "1.000":
        return None
    channels = [parts.get(name) for name in ("red", "green", "blue")]
    if not all(isinstance(c, str) and COMPONENT.match(c) for c in channels):
        return None
    return "#" + "".join(c[2:] for c in channels).lower()


def set_entries(root):
    """The colour sets of the catalog, as ({(set, mode): hex}, missing, findings): `missing` names
    each set, and each (set, mode) entry, the catalog lacks."""
    entries, problems = {}, []
    for name in SETS:
        path = CATALOG / f"{name}.colorset" / "Contents.json"
        try:
            document = json.loads((Path(root) / path).read_text(encoding="utf-8"))
        except (OSError, ValueError):
            problems.append(f"set-missing: {path.as_posix()}: the colour set {name} is not there")
            continue
        colours = document.get("colors") if isinstance(document, dict) else None
        colours = colours if isinstance(colours, list) else []
        light = [c for c in colours if isinstance(c, dict) and "appearances" not in c]
        dark = [
            c for c in colours if isinstance(c, dict) and c.get("appearances") == DARK_APPEARANCE
        ]
        for mode, found in (("light", light), ("dark", dark)):
            if len(found) != 1:
                problems.append(
                    f"entry-missing: {path.as_posix()}: {name} holds {len(found)} {mode} entries, not 1"
                )
                continue
            value = entry_hex(found[0])
            if value is None:
                problems.append(
                    f"value-differs: {path.as_posix()}: {name} {mode} entry is not whole sRGB with alpha 1.000"
                )
                continue
            entries[(name, mode)] = value
    return entries, problems


def literal_files(root):
    """The files the literal scan reads: the compile, the page's stylesheet and every Swift file of
    the app's sources, those that exist."""
    root = Path(root)
    files = [root / COMPILE, root / STYLESHEET]
    files += (
        sorted((root / SWIFT_SOURCES).rglob("*.swift")) if (root / SWIFT_SOURCES).is_dir() else []
    )
    return [f for f in files if f.is_file()]


def literal_findings(root):
    """A `raw-colour` finding for each scanned file that holds a colour literal."""
    problems = []
    for path in literal_files(root):
        text = path.read_text(encoding="utf-8", errors="replace")
        for number, line in enumerate(text.splitlines(), 1):
            found = LITERALS.search(line)
            if found:
                relative = path.relative_to(Path(root)).as_posix()
                problems.append(f"raw-colour: {relative}: line {number} holds {found.group(0)}")
    return problems


def findings(root):
    """Every fault of the Ember copies under `root`, as `"<reason>: <path>: <detail>"` strings."""
    values, problems = token_values(root)
    entries, set_problems = set_entries(root)
    problems += set_problems
    for (name, mode), got in sorted(entries.items()):
        want = values.get((mode, SETS[name]))
        if want is not None and got != want:
            path = (CATALOG / f"{name}.colorset" / "Contents.json").as_posix()
            problems.append(
                f"value-differs: {path}: {name} {mode} entry holds {got}, its token {want}"
            )
    return problems + literal_findings(root)


# The complete planted set, spelt here and never copied from the tree: four raw colours, the six
# roles that alias them, and the three colour sets that copy the roles.
NEUTRAL_50, NEUTRAL_950 = (0xFA, 0xF6, 0xF2), (0x15, 0x12, 0x10)
ACCENT_400, ACCENT_700 = (0xFF, 0x9A, 0x3C), (0xA5, 0x4D, 0x06)


def raw_colour(rgb):
    return {
        "$value": {
            "colorSpace": "srgb",
            "components": [round(c / 255, 4) for c in rgb],
            "hex": "#" + "".join(f"{c:02x}" for c in rgb),
        }
    }


def planted_tokens():
    return {
        "ref": {
            "ember": {
                "neutral": {"50": raw_colour(NEUTRAL_50), "950": raw_colour(NEUTRAL_950)},
                "accent": {"400": raw_colour(ACCENT_400), "700": raw_colour(ACCENT_700)},
            }
        },
        "modes": {
            "$type": "color",
            "ember": {
                "dark": {
                    "background": {"$value": "{ref.ember.neutral.950}"},
                    "ink": {"$value": "{ref.ember.neutral.50}"},
                    "accent": {"$value": "{ref.ember.accent.400}"},
                },
                "light": {
                    "background": {"$value": "{ref.ember.neutral.50}"},
                    "ink": {"$value": "{ref.ember.neutral.950}"},
                    "accent": {"$value": "{ref.ember.accent.700}"},
                },
            },
        },
    }


def colour_entry(rgb, dark):
    entry = {
        "color": {
            "color-space": "srgb",
            "components": {
                "alpha": "1.000",
                "blue": f"0x{rgb[2]:02X}",
                "green": f"0x{rgb[1]:02X}",
                "red": f"0x{rgb[0]:02X}",
            },
        },
        "idiom": "universal",
    }
    if dark:
        entry["appearances"] = list(DARK_APPEARANCE)
    return entry


def planted_sets():
    pairs = {
        "EmberBackground": (NEUTRAL_50, NEUTRAL_950),
        "EmberInk": (NEUTRAL_950, NEUTRAL_50),
        "EmberAccent": (ACCENT_700, ACCENT_400),
    }
    return {
        name: {
            "colors": [colour_entry(light, dark=False), colour_entry(dark, dark=True)],
            "info": {"author": "xcode", "version": 1},
        }
        for name, (light, dark) in pairs.items()
    }


def write(root, relative, text):
    path = Path(root) / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def plant(root, tokens=None, sets=None):
    """Write the complete planted set under `root`, with `tokens` or `sets` replacing a part."""
    write(root, TOKENS, json.dumps(planted_tokens() if tokens is None else tokens, indent=2))
    for name, document in (planted_sets() if sets is None else sets).items():
        write(root, CATALOG / f"{name}.colorset" / "Contents.json", json.dumps(document, indent=2))
    write(
        root, COMPILE, "export function compileTokens(input: unknown): string {\n  return '';\n}\n"
    )
    write(root, STYLESHEET, "@import './tokens.css';\nbody {\n  color: var(--foreground);\n}\n")
    write(
        root,
        SWIFT_SOURCES / "Plain.swift",
        'import SwiftUI\n\nstruct Plain: View {\n    var body: some View { Text("a") }\n}\n',
    )


class TheAppCopiesTheTokens(unittest.TestCase):
    def test_each_colour_set_holds_its_tokens_values(self):
        """A4: the repository's three colour sets hold their tokens' light and dark values."""
        values, _ = token_values(REPO)
        examined("Ember roles", values)
        self.assertEqual(len(values), 6, "the token file holds six role values")
        entries, _ = set_entries(REPO)
        examined("colour-set entries", entries)
        self.assertEqual(
            len({name for name, _ in entries}), 3, "three colour sets, each with two entries"
        )
        self.assertEqual(len(entries), 6)
        examined("files the literal scan reads", literal_files(REPO))
        self.assertEqual(findings(REPO), [])

    def test_a_colour_set_that_differs_from_its_token_is_refused(self):
        """A5: a value one byte off, and an entry that is not there, are each refused."""
        with tempfile.TemporaryDirectory() as tmp:
            plant(tmp)
            self.assertEqual(findings(tmp), [], "the control: the complete set reads green")

            off = planted_sets()
            off["EmberInk"]["colors"][1]["color"]["components"]["red"] = "0xFB"
            plant(tmp, sets=off)
            found = findings(tmp)
            self.assertEqual(len(found), 1, found)
            self.assertTrue(found[0].startswith("value-differs: "), found)
            self.assertIn("EmberInk", found[0])
            self.assertIn("dark", found[0])

            no_dark = planted_sets()
            del no_dark["EmberInk"]["colors"][1]
            plant(tmp, sets=no_dark)
            found = findings(tmp)
            self.assertEqual(len(found), 1, found)
            self.assertTrue(found[0].startswith("entry-missing: "), found)
            self.assertIn("EmberInk", found[0])
            self.assertIn("dark", found[0])

            no_light = planted_sets()
            del no_light["EmberAccent"]["colors"][0]
            plant(tmp, sets=no_light)
            found = findings(tmp)
            self.assertEqual(len(found), 1, found)
            self.assertTrue(found[0].startswith("entry-missing: "), found)
            self.assertIn("EmberAccent", found[0])
            self.assertIn("light", found[0])

    def test_a_role_missing_from_either_mode_is_refused(self):
        """A6: a token file without one role of one mode is refused, naming the mode and the role."""
        with tempfile.TemporaryDirectory() as tmp:
            for mode, role in (("light", "accent"), ("dark", "ink")):
                tokens = planted_tokens()
                del tokens["modes"]["ember"][mode][role]
                plant(tmp, tokens=tokens)
                found = findings(tmp)
                self.assertEqual(len(found), 1, found)
                self.assertTrue(found[0].startswith("role-missing: "), found)
                self.assertIn(mode, found[0])
                self.assertIn(role, found[0])

    def test_a_raw_colour_literal_outside_the_token_route_is_refused(self):
        """A7: a colour literal in a Swift file or the stylesheet is refused; the repository holds none."""
        with tempfile.TemporaryDirectory() as tmp:
            plant(tmp)
            write(
                tmp, SWIFT_SOURCES / "Planted.swift", "let red = Color(red: 1, green: 0, blue: 0)\n"
            )
            found = findings(tmp)
            self.assertEqual(len(found), 1, found)
            self.assertTrue(found[0].startswith("raw-colour: "), found)
            self.assertIn("Planted.swift", found[0])

            plant(tmp)
            (Path(tmp) / SWIFT_SOURCES / "Planted.swift").unlink()
            write(tmp, STYLESHEET, "body {\n  color: #ff0000;\n}\n")
            found = findings(tmp)
            self.assertEqual(len(found), 1, found)
            self.assertTrue(found[0].startswith("raw-colour: "), found)
            self.assertIn("layout.css", found[0])

            for literal in (
                "rgb(0, 0, 0)",
                "hsl(0 0% 0%)",
                "oklch(0.5 0.1 20)",
                "UIColor(red: 0, green: 0, blue: 0, alpha: 1)",
            ):
                plant(tmp)
                write(tmp, COMPILE, f"const c = '{literal}';\n")
                self.assertEqual(len(findings(tmp)), 1, literal)

        examined("files the literal scan reads", literal_files(REPO))
        self.assertEqual(literal_findings(REPO), [])

    def test_a_complete_planted_set_reads_green(self):
        """A8: the complete planted set reads no finding, the control for A5 to A7."""
        with tempfile.TemporaryDirectory() as tmp:
            plant(tmp)
            values, _ = token_values(tmp)
            examined("Ember roles", values)
            entries, _ = set_entries(tmp)
            examined("colour-set entries", entries)
            examined("files the literal scan reads", literal_files(tmp))
            self.assertEqual(findings(tmp), [])


if __name__ == "__main__":
    unittest.main()
