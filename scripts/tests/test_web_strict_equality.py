"""The web client holds strict equality and one spelling for an absent value (SPEC-656, ADR-678).

A text census over the web client's sources, every `.ts` file and every `.svelte` component under
`web/app/src`, read as the compiler reads them: a comment, a string, a regular expression literal
and a template literal's text are never code, and a template literal's `${ }` is. Two rules:

- `loose-equality`: each `==` and `!=` that is not `===` or `!==`, in a `.ts` file, in a
  component's `<script>` blocks and in its markup's `{ }` expressions.
- `absent-spelling`: the client spells an absent value `null` at its boundaries, so a boundary
  site that spells it `undefined` is reported: an optional member or parameter (`name?:`) or a
  `| undefined` in an exported interface or type alias of the API module or of a module it imports
  a type from; a `$state` whose type names `Answer<` and starts with no value, with `undefined`,
  or holds `| undefined`; and a `$props()` annotation naming `Answer<` with `?:` or `| undefined`.

A site's key is its file and an ordinal or a name, never a line. At `warn` an unlisted site prints
`WARN <rule> <key> line <n>` and passes; at `fail` it fails. A listed entry no site matches fails
as `STALE <rule> <key>` at both levels, so a fix removes its entry. Moving `LEVEL` to `fail` is its
own delivery, under ADR-450 D6. An empty population is VOID, never a pass.
"""

import posixpath
import re
import unittest
from pathlib import Path

from _support import REPO, examined

SOURCE = "web/app/src"
API = "web/app/src/lib/api.ts"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "web-strict-equality"
RULES = ("loose-equality", "absent-spelling")
LEVEL = "warn"

ANSWER_STATE = "an Answer not yet answered: the page shows its loading state while it is undefined"
LISTED = {
    "loose-equality": {},
    "absent-spelling": {
        "web/app/src/lib/api.ts ApiOptions optional #1": "fetch: the caller's transport, the page's own fetch when omitted",
        "web/app/src/lib/api.ts ApiOptions optional #2": "askSignIn: the caller's sign-in hook, none inside Telegram",
        "web/app/src/lib/api.ts Api optional-parameter #1": "wallet's before: the first page when omitted",
        "web/app/src/lib/streak/streak.ts StreakTrack optional #1": "freezes: served on the language track only",
        "web/app/src/lib/streak/streak.ts StreakTrack optional #2": "freezeCap: served on the language track only",
        "web/app/src/lib/streak/streak.ts StreakView optional #1": "calendar: present when the server serves it",
        "web/app/src/lib/engine/protocol.ts Request optional #1": "the engine worker's open request: languages when given",
        "web/app/src/lib/engine/protocol.ts UndoOffer optional #1": "the engine worker's undo offer: its kind when given",
        "web/app/src/lib/economy/WalletHeader.svelte props union-undefined #1": "wallet: a balance not yet answered",
        "web/app/src/lib/settings/SignInMethods.svelte listed state": ANSWER_STATE,
        "web/app/src/routes/+layout.svelte wallet state": ANSWER_STATE,
        "web/app/src/routes/+page.svelte today state": ANSWER_STATE,
        "web/app/src/routes/badges/+page.svelte badges state": ANSWER_STATE,
        "web/app/src/routes/insights/+page.svelte listed state": ANSWER_STATE,
        "web/app/src/routes/law/+page.svelte read state": ANSWER_STATE,
        "web/app/src/routes/level/+page.svelte level state": ANSWER_STATE,
        "web/app/src/routes/level/+page.svelte exchange state": ANSWER_STATE,
        "web/app/src/routes/progress/+page.svelte progress state": ANSWER_STATE,
        "web/app/src/routes/records/+page.svelte records state": ANSWER_STATE,
        "web/app/src/routes/records/+page.svelte board state": ANSWER_STATE,
        "web/app/src/routes/score/+page.svelte today state": ANSWER_STATE,
        "web/app/src/routes/streak/+page.svelte streak state": ANSWER_STATE,
        "web/app/src/routes/wallet/+page.svelte wallet state": ANSWER_STATE,
    },
}

# A regular expression may start after one of these, or after one of the keywords.
REGEX_AFTER = set("(,=:[!&|?{};+-*%<>~^")
REGEX_KEYWORDS = {"return", "typeof", "case", "do", "else", "in", "of", "new", "delete", "void"}
REGEX_KEYWORDS |= {"throw", "instanceof", "yield", "await"}
WORD = re.compile(r"[A-Za-z0-9_$]")
LOOSE = re.compile(r"(?<![=!<>])(==|!=)(?!=)")
SCRIPT = re.compile(r"<script\b[^>]*>(.*?)</script>", re.S)
STYLE = re.compile(r"<style\b[^>]*>.*?</style>", re.S)
HTML_COMMENT = re.compile(r"<!--.*?-->", re.S)
HIDDEN = (SCRIPT, STYLE, HTML_COMMENT)
IMPORT = re.compile(r"""\bimport\s+(type\s+)?\{([^}]*)\}\s*from\s*['"](\.[^'"]*)['"]""")
DECLARATION = re.compile(r"\bexport\s+(interface|type)\s+([A-Za-z_$][\w$]*)")
OPTIONAL = re.compile(r"(?<=[\w$\]])\?:")
UNDEFINED_UNION = re.compile(r"(?<!\|)\|\s*undefined\b|\bundefined\s*\|(?!\|)")
STATE = re.compile(r"\b(?:let|const)\s+([A-Za-z_$][\w$]*)\s*=\s*\$state(?:\.raw)?\s*<")
PROPS = re.compile(r"\$props\(\)")
LET = re.compile(r"\blet\b")


def mask(text):
    """The text with comments, strings, regular expressions and template text blanked; line
    breaks are kept, so a match's line is its line in the file, and `${ }` stays code."""
    out = list(text)
    size = len(text)
    index = 0
    stack = [["code", 0]]
    last, word = "", ""

    def blank(start, end):
        for position in range(start, min(end, size)):
            if out[position] != "\n":
                out[position] = " "

    while index < size:
        char = text[index]
        pair = text[index : index + 2]
        if stack[-1][0] == "template":
            if char == "\\":
                blank(index, index + 2)
                index += 2
            elif char == "`":
                stack.pop()
                blank(index, index + 1)
                index, last, word = index + 1, ")", ""
            elif pair == "${":
                blank(index, index + 2)
                stack.append(["code", 0])
                index, last, word = index + 2, "{", ""
            else:
                blank(index, index + 1)
                index += 1
            continue
        if pair == "//":
            end = text.find("\n", index)
            end = size if end < 0 else end
            blank(index, end)
            index = end
            continue
        if pair == "/*":
            end = text.find("*/", index + 2)
            end = size if end < 0 else end + 2
            blank(index, end)
            index = end
            continue
        if char in "'\"":
            end = index + 1
            while end < size and text[end] not in (char, "\n"):
                end += 2 if text[end] == "\\" else 1
            blank(index, end + 1)
            index, last, word = end + 1, ")", ""
            continue
        if char == "`":
            blank(index, index + 1)
            stack.append(["template", 0])
            index += 1
            continue
        if char == "/" and (last in REGEX_AFTER or last == "" or word in REGEX_KEYWORDS):
            end, in_class = index + 1, False
            while end < size and text[end] != "\n":
                if text[end] == "\\":
                    end += 2
                    continue
                if text[end] == "/" and not in_class:
                    break
                in_class = (in_class or text[end] == "[") and text[end] != "]"
                end += 1
            end += 1
            while end < size and WORD.match(text[end]):
                end += 1
            blank(index, end)
            index, last, word = end, ")", ""
            continue
        if char == "{":
            stack[-1][1] += 1
        elif char == "}" and stack[-1][1] == 0 and len(stack) > 1:
            stack.pop()
            blank(index, index + 1)
            index += 1
            continue
        elif char == "}":
            stack[-1][1] -= 1
        if WORD.match(char):
            end = index
            while end < size and WORD.match(text[end]):
                end += 1
            last, word, index = "a", text[index:end], end
            continue
        if not char.isspace():
            last, word = char, ""
        index += 1
    return "".join(out)


def blanked(text):
    return ["\n" if char == "\n" else " " for char in text]


def component_code(text):
    """A component's code at its own offsets: each `<script>` block, and each `{ }` expression of
    its markup once its scripts, styles and comments are set aside; everything else blanked."""
    out = blanked(text)
    for match in SCRIPT.finditer(text):
        out[match.start(1) : match.end(1)] = mask(match.group(1))
    markup = text
    for pattern in HIDDEN:
        markup = pattern.sub(lambda found: "".join(blanked(found.group(0))), markup)
    index, size = 0, len(markup)
    while index < size:
        if markup[index] != "{":
            index += 1
            continue
        end, depth, quote = index, 0, ""
        while end < size:
            char = markup[end]
            if quote:
                end += 2 if char == "\\" else 1
                quote = "" if char == quote else quote
                continue
            if char in "'\"`":
                quote = char
            depth += {"{": 1, "}": -1}.get(char, 0)
            if depth == 0:
                break
            end += 1
        out[index + 1 : end] = mask(markup[index + 1 : end])
        index = end + 1
    return "".join(out)


def code_of(path, text):
    return component_code(text) if path.endswith(".svelte") else mask(text)


def line_of(text, offset):
    return text.count("\n", 0, offset) + 1


def loose_sites(files):
    """Each loose comparison, as (key, line); a key is the file and the site's ordinal in it."""
    found = []
    for path, text in files:
        code = code_of(path, text)
        for number, match in enumerate(LOOSE.finditer(code), 1):
            found.append((f"{path} #{number}", line_of(code, match.start())))
    return found


def boundary_modules(files):
    """The API module and each module it imports a type from, as their paths."""
    texts = dict(files)
    if API not in texts:
        return []
    code, found = mask(texts[API]), [API]
    for match in IMPORT.finditer(texts[API]):
        names = [name.strip() for name in match.group(2).split(",")]
        typed = match.group(1) or any(name.startswith("type ") for name in names)
        if code[match.start()] != "i" or not typed:
            continue
        target = posixpath.normpath(posixpath.join(posixpath.dirname(API), match.group(3)))
        target = target.removesuffix(".js") + ".ts"
        if target in texts and target not in found:
            found.append(target)
    return found


def declaration_end(code, start, interface):
    """Where the declaration whose name ends at `start` ends: an interface's closing brace, or a
    type alias's `;` outside any bracket."""
    depth, index = 0, code.find("{" if interface else "=", start)
    while 0 <= index < len(code):
        char = code[index]
        depth += 1 if char in "({[" else -1 if char in ")}]" else 0
        if depth == 0 and char == ("}" if interface else ";"):
            return index + 1
        index += 1
    return len(code)


def declaration_sites(path, text):
    """Each `undefined` spelling inside the module's exported interfaces and type aliases."""
    code, found = mask(text), []
    for declared in DECLARATION.finditer(code):
        start, name = declared.end(), declared.group(2)
        body = code[start : declaration_end(code, start, declared.group(1) == "interface")]
        counts = {}
        spelled = [("optional", m) for m in OPTIONAL.finditer(body)]
        spelled += [("union-undefined", m) for m in UNDEFINED_UNION.finditer(body)]
        for form, match in sorted(spelled, key=lambda each: each[1].start()):
            opened = body.count("(", 0, match.start()) - body.count(")", 0, match.start())
            if form == "optional" and opened > 0:
                form = "optional-parameter"
            counts[form] = counts.get(form, 0) + 1
            line = line_of(code, start + match.start())
            found.append((f"{path} {name} {form} #{counts[form]}", line))
    return found


def state_sites(path, code):
    """Each `$state` holding an Answer that starts with no value or `undefined`, or holds it."""
    found = []
    for match in STATE.finditer(code):
        depth, end = 1, match.end()
        while end < len(code) and depth:
            if code[end - 1 : end + 1] != "=>":
                depth += {"<": 1, ">": -1}.get(code[end], 0)
            end += 1
        argument, rest = code[match.end() : end - 1], code[end:].lstrip()
        if "Answer<" not in argument or not rest.startswith("("):
            continue
        start = rest[1:].lstrip()
        empty = start.startswith(")") or re.match(r"undefined\b", start)
        if empty or UNDEFINED_UNION.search(argument):
            found.append((f"{path} {match.group(1)} state", line_of(code, match.start())))
    return found


def props_sites(path, code):
    """Each `?:` and `| undefined` in a `$props()` declaration that names an Answer."""
    found = []
    for match in PROPS.finditer(code):
        lets = [each.start() for each in LET.finditer(code, 0, match.start())]
        start = lets[-1] if lets else match.start()
        declaration = code[start : match.start()]
        if "Answer<" not in declaration:
            continue
        counts = {}
        for form, pattern in (("optional", OPTIONAL), ("union-undefined", UNDEFINED_UNION)):
            for each in pattern.finditer(declaration):
                counts[form] = counts.get(form, 0) + 1
                line = line_of(code, start + each.start())
                found.append((f"{path} props {form} #{counts[form]}", line))
    return found


def absent_sites(files, boundary):
    """Each boundary site that spells an absent value `undefined`, as (key, line)."""
    found = []
    for path, text in files:
        if path in boundary:
            found += declaration_sites(path, text)
        if ".test." in path or not path.endswith((".svelte", ".svelte.ts")):
            continue
        code = code_of(path, text)
        found += state_sites(path, code)
        if path.endswith(".svelte"):
            found += props_sites(path, code)
    return sorted(found, key=lambda site: (site[0].split(" ")[0], site[1], site[0]))


def census(files):
    """The sites of both rules over (path, text) pairs; an empty population is VOID."""
    files = examined("web source file(s)", files)
    boundary = examined("boundary module(s)", boundary_modules(files))
    return {"loose-equality": loose_sites(files), "absent-spelling": absent_sites(files, boundary)}


def verdict(found, listed, level):
    """(printed lines, failures): an unlisted site prints WARN at `warn` and fails at `fail`; a
    listed entry no site matches fails as STALE at both."""
    printed, failures = [], []
    for rule in RULES:
        keys = {key for key, _line in found[rule]}
        for key, line in found[rule]:
            if key in listed[rule]:
                continue
            if level == "fail":
                failures.append(f"FAIL {rule} {key} line {line}")
            else:
                printed.append(f"WARN {rule} {key} line {line}")
        for key in listed[rule]:
            if key not in keys:
                failures.append(f"STALE {rule} {key}")
    return printed, failures


def sources(root=REPO):
    """Every `.ts` file and `.svelte` component under the web client's source root."""
    base = Path(root) / SOURCE
    return [
        (path.relative_to(root).as_posix(), path.read_text(encoding="utf-8"))
        for path in sorted(base.rglob("*"))
        if path.is_file() and path.name.endswith((".ts", ".svelte"))
    ]


def planted(name):
    """A planted tree's sources: each `.fixture` file, read by the name before its suffix."""
    root = FIXTURES / name
    files = []
    for path in sorted(root.rglob("*.fixture")):
        relative = path.relative_to(root).as_posix().removesuffix(".fixture")
        if relative.endswith((".ts", ".svelte")):
            files.append((relative, path.read_text(encoding="utf-8")))
    return files


def marked(files, marker):
    """(path, line) of each planted line whose comment says `<marker>`."""
    return [
        (path, number)
        for path, text in files
        for number, line in enumerate(text.splitlines(), 1)
        if f" {marker}" in line
    ]


class TheWebClientHoldsStrictEqualityAndOneAbsentSpelling(unittest.TestCase):
    def test_the_repository_holds_exactly_todays_listing(self):
        files = sources()
        found = census(files)
        printed, failures = verdict(found, LISTED, LEVEL)
        for line in printed:
            print(line)
        self.assertEqual(LEVEL, "warn")
        self.assertEqual(failures, [])
        self.assertEqual([path for path, _text in files if path == API], [API])
        for rule in RULES:
            for key, reason in LISTED[rule].items():
                self.assertTrue(reason.strip(), f"{key} is listed with no reason")

    def test_a_planted_loose_comparison_is_found_and_its_lookalikes_are_not(self):
        files = planted("plant")
        found = census(files)["loose-equality"]
        sites = sorted((key.split(" ")[0], line) for key, line in found)
        self.assertEqual(sites, sorted(marked(files, "planted: loose-equality")))
        self.assertEqual(
            sorted(key for key, _line in found),
            [
                "web/app/src/lib/api.ts #1",
                "web/app/src/lib/views.ts #1",
                "web/app/src/routes/+page.svelte #1",
                "web/app/src/routes/+page.svelte #2",
            ],
        )
        self.assertGreater(len(marked(files, "accepted:")), 0)

    def test_a_planted_boundary_with_the_other_spelling_is_found(self):
        files = planted("plant")
        found = census(files)["absent-spelling"]
        sites = sorted({(key.split(" ")[0], line) for key, line in found})
        self.assertEqual(sites, sorted(set(marked(files, "planted: absent-spelling"))))
        self.assertEqual(
            sorted(key for key, _line in found),
            [
                "web/app/src/lib/Header.svelte props optional #1",
                "web/app/src/lib/Header.svelte props union-undefined #1",
                "web/app/src/lib/api.ts Options optional #1",
                "web/app/src/lib/api.ts Options optional-parameter #1",
                "web/app/src/lib/views.ts View optional #1",
                "web/app/src/lib/views.ts View union-undefined #1",
                "web/app/src/routes/+page.svelte view state",
            ],
        )
        self.assertEqual(boundary_modules(files), [API, "web/app/src/lib/views.ts"])

    def test_the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails(self):
        found = {"loose-equality": [("web/app/src/a.ts #1", 3)], "absent-spelling": []}
        listed = {"loose-equality": {"web/app/src/b.ts #1": "planted"}, "absent-spelling": {}}
        unlisted = "loose-equality web/app/src/a.ts #1 line 3"
        stale = "STALE loose-equality web/app/src/b.ts #1"
        self.assertEqual(verdict(found, listed, "warn"), ([f"WARN {unlisted}"], [stale]))
        self.assertEqual(verdict(found, listed, "fail"), ([], [f"FAIL {unlisted}", stale]))

    def test_an_empty_population_is_void(self):
        self.assertEqual(planted("empty"), [])
        with self.assertRaisesRegex(AssertionError, "examined 0 web source file"):
            census(planted("empty"))
        no_api = [(path, text) for path, text in planted("plant") if path != API]
        with self.assertRaisesRegex(AssertionError, "examined 0 boundary module"):
            census(no_api)


if __name__ == "__main__":
    unittest.main()
