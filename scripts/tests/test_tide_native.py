"""Every native view reads its colours through the Tide extension, no view or property list sets
a colour scheme, and the extension names every Tide colour set the catalog holds (SPEC-593 R11
and A10 to A12, ADR-615 D4, ADR-616 D2).

Three censuses, each a function of the texts it is handed, so every planted control lives in
memory: a test hands its census the live tree, and its planted twin hands it texts that each break
one rule and must be refused by that rule's reason. They read code text only and run no Swift
tool; the Swift compile is read in continuous integration.

- the colours (A10): every file under `ios/App/Sources` that declares a `View` struct, the twelve
  view files of SPEC-593 section 4 among them, read with its comments and string text removed,
  names at least one of the six `Color.tide<Role>` members and no other colour: no `Color(`,
  `UIColor(` or `#colorLiteral(`, no other `Color.` member, no hierarchical or material style
  argument and no named system colour argument;
- the colour scheme (A11): no Swift file under `ios` and no property list under `ios/App` names
  `preferredColorScheme`, `overrideUserInterfaceStyle` or `UIUserInterfaceStyle`, read over the
  whole text, so the app follows the system appearance;
- the colour sets (A12): the `Tide` colour sets of the app's asset catalog, all six, are exactly
  the names `TideColours.swift` passes as the first argument of `Color(`.
"""

import re
import unittest

from _support import REPO, examined
from test_ios_thin_swift import code_of, live_tree
from test_one_static_library import tracked_paths

APP = "ios/App/Sources/"
EXTENSION = APP + "TideColours.swift"
CATALOG = "ios/App/Assets.xcassets/"
PLANTED_VIEW = APP + "PlantedView.swift"
PLANTED_PLIST = "ios/App/Info.plist"
# SPEC-593 section 4: the twelve view files, each of which reads its colours through the extension.
VIEWS = tuple(
    APP + name + ".swift"
    for name in (
        "AccountView",
        "AnswerBar",
        "CardFaceView",
        "CardHealthView",
        "DataView",
        "DeckListView",
        "ReviewChrome",
        "ReviewView",
        "SyncView",
        "UndoControl",
        "UndoDialogView",
        "VoicePickerView",
    )
)
# ADR-615 D4: the six roles, each a colour set and the extension's member of the same name.
ROLES = ("Background", "Ink", "Accent", "Card", "Muted", "Border")
SETS = frozenset("Tide" + role for role in ROLES)
MEMBERS = frozenset("tide" + role for role in ROLES)
# SPEC-593 section 1's form of a view declaration, read one line at a time.
DECLARES_VIEW = re.compile(r"struct [A-Za-z]+ *:[^{\n]*View")
# A colour made in place: SwiftUI's, UIKit's, or a literal.
MADE = re.compile(r"(?:\bColor|\bUIColor|#colorLiteral)\s*\(")
# A member of `Color`, which only the six Tide members may be.
MEMBER = re.compile(r"\bColor\s*\.\s*([A-Za-z_]\w*)")
# An implicit member argument (a leading dot, not a member of an expression) naming a hierarchical
# or material style, or a named system colour; and the style types named in full.
STYLES = (
    "primary",
    "secondary",
    "tertiary",
    "quaternary",
    "quinary",
    "bar",
    "ultraThinMaterial",
    "thinMaterial",
    "regularMaterial",
    "thickMaterial",
    "ultraThickMaterial",
)
SYSTEM_COLOURS = (
    "red",
    "orange",
    "yellow",
    "green",
    "mint",
    "teal",
    "cyan",
    "blue",
    "indigo",
    "purple",
    "pink",
    "brown",
    "gray",
    "black",
    "white",
    "clear",
    "accentColor",
)
IMPLICIT = re.compile(r"(?<![\w)\]?!.])\.(" + "|".join(STYLES + SYSTEM_COLOURS) + r")\b")
STYLE_TYPES = re.compile(r"\b(?:Material|HierarchicalShapeStyle)\b")
# ADR-616 D2: the names that set a colour scheme over the system appearance.
SCHEME = re.compile(r"preferredColorScheme|overrideUserInterfaceStyle|UIUserInterfaceStyle")
# A Tide colour set of the app's catalog, by its path, and a name passed to `Color(`.
SET_PATH = re.compile(r"ios/App/Assets\.xcassets/(Tide[A-Za-z]*)\.colorset/Contents\.json")
NAMED = re.compile(r'\bColor\(\s*"([^"\\]*)"')


def refuse(case, problems):
    """Fail on any problem, every one on the failure's first line, so a red quotes its reason."""
    if problems:
        case.fail(f"{len(problems)} problem(s): " + "; ".join(problems))


def colour_problems(sources):
    """Each view file's colours that do not come through the six Tide members (A10)."""
    problems = []
    for path, text in sorted(sources.items()):
        code = code_of(text)
        named = set()
        for found in MADE.finditer(code):
            problems.append(f"{path}: makes a colour in place: {found.group(0).replace(' ', '')}")
        for found in MEMBER.finditer(code):
            if found.group(1) in MEMBERS:
                named.add(found.group(1))
            else:
                problems.append(f"{path}: names Color.{found.group(1)}, which is no Tide member")
        for found in IMPLICIT.finditer(code):
            problems.append(f"{path}: passes .{found.group(1)}, a system style or colour")
        for found in STYLE_TYPES.finditer(code):
            problems.append(f"{path}: names {found.group(0)}, a system style")
        if not named:
            problems.append(f"{path}: names none of the six Tide members")
    return problems


def scheme_problems(sources):
    """Each Swift file or property list that sets a colour scheme over the system's (A11)."""
    return [
        f"{path}: names {found.group(0)}, which overrides the system appearance"
        for path, text in sorted(sources.items())
        for found in SCHEME.finditer(text)
    ]


def named_sets(text):
    """The names an extension's text passes as the first argument of `Color(`, its whole-line
    comments left out."""
    lines = [line for line in text.splitlines() if not line.lstrip().startswith("//")]
    return {found.group(1) for found in NAMED.finditer("\n".join(lines))}


def set_problems(catalog, extension):
    """The Tide colour sets the catalog holds against the names the extension passes (A12)."""
    named = named_sets(extension)
    problems = []
    unnamed = sorted(set(catalog) - named)
    if unnamed:
        problems.append(f"{EXTENSION}: names no Color( for the catalog set(s) {', '.join(unnamed)}")
    held = sorted(named - set(catalog))
    if held:
        problems.append(
            f"{EXTENSION}: names Color( for {', '.join(held)}, which the catalog does not hold"
        )
    return problems


def read(path):
    return (REPO / path).read_text(encoding="utf-8")


def planted_view(line="", root="            .background(Color.tideBackground)\n"):
    """A view that reads its root colour through the extension, with one planted line."""
    return (
        "import SwiftUI\n\nstruct PlantedView: View {\n    var body: some View {\n"
        '        Text("planted")\n' + root + line + "    }\n}\n"
    )


def planted_plist(entry=""):
    """A property list that names one key of its own, with one planted entry."""
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n<plist version="1.0">\n<dict>\n'
        "\t<key>CFBundleDisplayName</key>\n\t<string>Planted</string>\n" + entry + "</dict>\n"
        "</plist>\n"
    )


def planted_extension(roles=ROLES):
    """An extension whose members each pass their set's name to `Color(`."""
    members = "".join(
        f'    static var tide{role}: Color {{ Color("Tide{role}") }}\n' for role in roles
    )
    return "import SwiftUI\n\nextension Color {\n" + members + "}\n"


# A10's plants: each a line in an otherwise good view, and the one reason it is refused by.
COLOUR_PLANTS = {
    "a colour literal": (
        "            .foregroundStyle(Color(red: 0.2, green: 0.3, blue: 0.4))\n",
        "makes a colour in place: Color(",
    ),
    "a secondary style": (
        "            .foregroundStyle(.secondary)\n",
        "passes .secondary, a system style or colour",
    ),
    "a UIKit colour": (
        "            .tint(UIColor(white: 0.5, alpha: 1))\n",
        "makes a colour in place: UIColor(",
    ),
    "a colour literal macro": (
        "            .tint(#colorLiteral(red: 0.2, green: 0.3, blue: 0.4, alpha: 1))\n",
        "makes a colour in place: #colorLiteral(",
    ),
    "a system colour member": (
        "            .tint(Color.red)\n",
        "names Color.red, which is no Tide member",
    ),
    "a named system colour": (
        "            .tint(.blue)\n",
        "passes .blue, a system style or colour",
    ),
    "the bar style": (
        "            .background(.bar)\n",
        "passes .bar, a system style or colour",
    ),
    "a material": (
        "            .background(.ultraThinMaterial)\n",
        "passes .ultraThinMaterial, a system style or colour",
    ),
    "a material named in full": (
        "            .background(Material.thin)\n",
        "names Material, a system style",
    ),
}
# A11's plants: each a file that sets a colour scheme, and the name it is refused by.
SCHEME_PLANTS = {
    "a view's preferred scheme": (
        PLANTED_VIEW,
        planted_view("            .preferredColorScheme(.light)\n"),
        "preferredColorScheme",
    ),
    "a window's style override": (
        PLANTED_VIEW,
        planted_view("            .onAppear { window.overrideUserInterfaceStyle = .light }\n"),
        "overrideUserInterfaceStyle",
    ),
    "a property list's style": (
        PLANTED_PLIST,
        planted_plist("\t<key>UIUserInterfaceStyle</key>\n\t<string>Light</string>\n"),
        "UIUserInterfaceStyle",
    ),
}
# A12's plants: each a catalog and an extension that disagree, and the one problem they read.
SIX = sorted(SETS)
SET_PLANTS = {
    "a seventh set in the catalog": (
        SIX + ["TideExtra"],
        planted_extension(),
        f"{EXTENSION}: names no Color( for the catalog set(s) TideExtra",
    ),
    "a set the catalog does not hold": (
        SIX,
        planted_extension(ROLES + ("Extra",)),
        f"{EXTENSION}: names Color( for TideExtra, which the catalog does not hold",
    ),
    "a member left out": (
        SIX,
        planted_extension(ROLES[:-1]),
        f"{EXTENSION}: names no Color( for the catalog set(s) TideBorder",
    ),
}


class TheNativeViewsReadTheTideSets(unittest.TestCase):
    def test_every_view_reads_colours_only_through_the_tide_extension(self):
        swift, _ = live_tree()
        views = {
            path: text
            for path, text in swift.items()
            if path.startswith(APP) and DECLARES_VIEW.search(code_of(text))
        }
        print(f"examined {sum(path in views for path in VIEWS)} of {len(VIEWS)} view files")
        problems = [
            f"{path}: declares no View struct, or git does not track it"
            for path in VIEWS
            if path not in views
        ]
        judged = examined("view files judged", sorted(views))
        refuse(self, problems + colour_problems({path: views[path] for path in judged}))

    def test_a_planted_colour_literal_in_a_view_is_refused(self):
        self.assertEqual(colour_problems({PLANTED_VIEW: planted_view()}), [])
        for plant in examined("planted colours", COLOUR_PLANTS):
            line, reason = COLOUR_PLANTS[plant]
            with self.subTest(plant=plant):
                found = colour_problems({PLANTED_VIEW: planted_view(line)})
                self.assertEqual(found, [f"{PLANTED_VIEW}: {reason}"])
        found = colour_problems({PLANTED_VIEW: planted_view(root="")})
        self.assertEqual(found, [f"{PLANTED_VIEW}: names none of the six Tide members"])

    def test_no_view_or_property_list_overrides_the_colour_scheme(self):
        swift, _ = live_tree()
        plists = [
            path
            for path in tracked_paths(REPO)
            if path.startswith("ios/App/") and path.endswith(".plist")
        ]
        sources = {path: swift[path] for path in examined("Swift files under ios", sorted(swift))}
        for path in examined("property lists under ios/App", plists):
            sources[path] = read(path)
        refuse(self, scheme_problems(sources))

    def test_a_planted_scheme_override_is_refused(self):
        clean = {PLANTED_VIEW: planted_view(), PLANTED_PLIST: planted_plist()}
        self.assertEqual(scheme_problems(clean), [])
        for plant in examined("planted scheme overrides", SCHEME_PLANTS):
            path, text, name = SCHEME_PLANTS[plant]
            with self.subTest(plant=plant):
                found = scheme_problems({**clean, path: text})
                self.assertEqual(
                    found, [f"{path}: names {name}, which overrides the system appearance"]
                )

    def test_the_tide_extension_names_every_colour_set_in_the_catalog(self):
        tracked = tracked_paths(REPO)
        catalog = sorted(found.group(1) for path in tracked if (found := SET_PATH.fullmatch(path)))
        print(f"examined {sum(name in catalog for name in SIX)} of {len(SIX)} Tide colour sets")
        problems = [f"{CATALOG}: holds no {name}.colorset" for name in SIX if name not in catalog]
        extension = read(EXTENSION) if EXTENSION in tracked else ""
        sets = examined("Tide colour sets in the catalog", catalog)
        refuse(self, problems + set_problems(sets, extension))

    def test_a_colour_set_the_extension_does_not_name_is_refused(self):
        self.assertEqual(set_problems(SIX, planted_extension()), [])
        for plant in examined("planted colour sets", SET_PLANTS):
            catalog, extension, problem = SET_PLANTS[plant]
            with self.subTest(plant=plant):
                self.assertEqual(set_problems(catalog, extension), [problem])


if __name__ == "__main__":
    unittest.main()
