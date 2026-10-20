"""SPEC-426: no Lean file under `formal/lean` holds an escape hatch in its code, outside the empty
allow-list `ALLOWED` (ADR-440 D4).

The guard reads every `.lean` path `git ls-files` lists under `formal/lean/`, from the work tree, a
link at any component refused. Its matcher lexes each file into code: comments and string text are
removed, each removed character a space and every newline kept, so a hit keeps its line, and an
interpolation's `{` to `}` spans stay code. The code is then a stream of names, guillemet names and
numerals. A hatch word is a token equal to it, and `linter-off` and `heartbeats-zero` are three
consecutive tokens. A module whose code names `warningAsError` is refused as `key-off`, since it
would switch the lakefile's key off for itself (ADR-440 D2). The lexing is held to the pinned
Lean's own reading of `ORACLE_PLANTS`, recorded as `LEAN_WARNS` (SPEC-426 R8).
"""

import collections
import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

from _support import REPO, examined, refuse_link_components

# R4: the population is every tracked `.lean` path under this prefix, and holds the root module.
ROOT_PREFIX = "formal/lean/"
ROOT_MODULE = "formal/lean/Formal.lean"

# R5: the word arms, each refused where a token of the code equals it.
HATCH_WORDS = (
    "sorry",  # a hole: the declaration rests on sorryAx
    "admit",  # the tactic that closes a goal with sorry
    "axiom",  # a new axiom, which no proof discharges
    "native_decide",  # the compiler's evaluation, trusted by an axiom
    "implemented_by",  # runs other code than the definition the kernel checked
    "extern",  # runs foreign code in the definition's place
    "unsafe",  # a declaration the kernel does not check
    "nolint",  # a linter switched off for one declaration
)
# R5: the two arms of three consecutive tokens, each opened by this command.
OPTION_COMMAND = "set_option"
LINTER_PREFIX = "linter."
HEARTBEATS = "maxHeartbeats"
# R9: the lakefile's key, which a module that names it could switch off for itself.
KEY = "warningAsError"
# R6: a numeral's base by its prefix; any other numeral is decimal.
NUMERAL_BASES = {"0x": 16, "0b": 2, "0o": 8}

# R7: the allow-list, each entry (path, arm, count, reason). It is empty, and a new entry is a
# suppression ADR-440 D6 counts.
ALLOWED = ()

# R8: SPEC-426 section 9's planted files, each one whole Lean file the pinned Lean elaborates alone.
ORACLE_PLANTS = {
    "a sorry in code": "theorem planted : True := sorry\n",
    "a sorry in a line comment": "theorem planted : True := trivial -- sorry\n",
    "a sorry in a block comment": "/- sorry -/\ntheorem planted : True := trivial\n",
    "a sorry in a nested block comment": (
        "/- outer /- inner -/ sorry -/\ntheorem planted : True := trivial\n"
    ),
    "a sorry in a doc comment": "/-- sorry -/\ntheorem planted : True := trivial\n",
    "a sorry in a string": 'def planted : String := "sorry"\n',
    "a sorry after an escaped quote in a string": 'def planted : String := "\\" sorry"\n',
    "a sorry after a string that holds a comment marker": (
        'def planted : String := "--" ++ toString (sorry : Nat)\n'
    ),
    "a sorry after a string that holds a block opener": (
        'def planted : String := "/-" ++ toString (sorry : Nat)\n'
    ),
    "a sorry after a quote character literal": (
        "def planted : Char × Nat := ('\"', (sorry : Nat))\n"
    ),
    "a sorry after an escaped character literal": (
        "def planted : Char × Nat := ('\\'', (sorry : Nat))\n"
    ),
    "a sorry in an interpolation": 'def planted : String := s!"{(sorry : Nat)}"\n',
    "a sorry in an interpolated string's text": 'def planted : String := s!"sorry {1}"\n',
    "a sorry in a raw string": 'def planted : String := r#"a " sorry"#\n',
    "a sorry after a primed name": "def planted (x' : Nat) : Nat := x' + (sorry : Nat)\n",
    "a quoted name spelled as the word": "def «sorry» : Nat := 1\ndef planted : Nat := «sorry»\n",
    "a theorem name that holds the word": "theorem no_sorry_admits : True := trivial\n",
}

# R8: whether the pinned Lean printed its `sorry` warning when it elaborated each plant alone, read
# in SPEC-426 section 4's local reading. The pinned Lean spells that warning with backticks,
# "declaration uses `sorry`".
LEAN_WARNS = {
    "a sorry in code": True,
    "a sorry in a line comment": False,
    "a sorry in a block comment": False,
    "a sorry in a nested block comment": False,
    "a sorry in a doc comment": False,
    "a sorry in a string": False,
    "a sorry after an escaped quote in a string": False,
    "a sorry after a string that holds a comment marker": True,
    "a sorry after a string that holds a block opener": True,
    "a sorry after a quote character literal": True,
    "a sorry after an escaped character literal": True,
    "a sorry in an interpolation": True,
    "a sorry in an interpolated string's text": False,
    "a sorry in a raw string": False,
    "a sorry after a primed name": True,
    "a quoted name spelled as the word": False,
    "a theorem name that holds the word": False,
}

# A5: each arm planted in code, the whole text of one line.
ARM_PLANTS = (
    ("sorry", "theorem planted : True := sorry"),
    ("admit", "theorem planted : True := by admit"),
    ("axiom", "axiom planted : False"),
    ("native_decide", "theorem planted : 1 = 1 := by native_decide"),
    ("implemented_by", "@[implemented_by plantedImpl] def planted : Nat := 0"),
    ("extern", '@[extern "planted_c"] opaque planted : Nat'),
    ("unsafe", "unsafe def planted : Nat := 0"),
    ("linter-off", "set_option linter.missingDocs false"),
    ("heartbeats-zero", "set_option maxHeartbeats 0"),
    ("nolint", "@[nolint unusedArguments] def planted (n : Nat) : Nat := 0"),
)
# A5: code no arm refuses, each the whole text of one line.
ADMITTED = (
    "theorem planted : 1 = 1 := by decide",
    "set_option linter.unusedVariables true",
    "set_option maxHeartbeats 400000",
    "def planted : Nat := unsafeCast 0",
    "theorem rposition_some_admits : True := trivial",
)
# A5: a zero heartbeat limit in each base, and under a dotted heartbeat name.
ZERO_PLANTS = (
    "set_option maxHeartbeats 0x0",
    "set_option maxHeartbeats 0b0",
    "set_option maxHeartbeats 0o0",
    "set_option synthInstance.maxHeartbeats 0",
)
# A6: each construct left open, at line 2 of its file.
UNTERMINATED = (
    ("unterminated-comment", "/- a comment that never closes\ntheorem planted : True := trivial\n"),
    ("unterminated-string", 'def planted : String := "a string that never closes\n'),
    ("unterminated-raw-string", 'def planted : String := r#"a raw string that never closes"\n'),
    ("unterminated-interpolation", 'def planted : String := s!"{(1 : Nat)\n'),
)
# A6: a terminated file whose comment, string and interpolation each span two lines.
MULTILINE = (
    "/- a comment\n   over two lines -/\n"
    'def planted : String := "a string\nover two lines" ++ s!"{(1 :\n Nat)}"\n'
)
# The path a planted file is judged under; no file is written there.
PLANTED_PATH = "formal/lean/Formal/Planted.lean"
# The line a planted module puts its planted text at.
PLANTED_HEAD = "-- a planted module\n\n"
PLANTED_LINE = 3


def lean_paths(listed):
    """The listed paths under `ROOT_PREFIX` that end in `.lean`, in the order listed."""
    return [path for path in listed if path.startswith(ROOT_PREFIX) and path.endswith(".lean")]


def read_sources(root, paths):
    """Each path's text, read from `root` as UTF-8, a link at any component refused by assertion."""
    sources = {}
    for path in paths:
        refuse_link_components(Path(root) / path, root)
        sources[path] = (Path(root) / path).read_text(encoding="utf-8")
    return sources


def live_sources(root):
    """Every Lean file git tracks under `formal/lean`, read from the work tree at `root`; a file git
    does not track is never judged."""
    listing = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--", "formal/lean"],
        capture_output=True,
        check=False,
    )
    if listing.returncode != 0:
        raise AssertionError(f"git ls-files exited {listing.returncode}: {listing.stderr!r}")
    listed = [path for path in listing.stdout.decode("utf-8").split("\0") if path]
    return read_sources(root, lean_paths(listed))


def is_identifier_character(character):
    """A character a Lean name holds after its first: a letter, a digit, `_`, `'`, `!` or `?`."""
    return character.isalnum() or character in "_'!?"


def blank(out, start, end):
    """Remove `out[start:end]` from the code: each character a space, each newline kept."""
    for index in range(start, end):
        if out[index] != "\n":
            out[index] = " "


def refuse(path, text, start, out, problems, arm):
    """Refuse the construct opened at `start` by its arm, at its line, and read nothing past it."""
    problems.append((path, text.count("\n", 0, start) + 1, arm))
    blank(out, start, len(text))
    return None


def lean_code(path, text):
    """The code of one Lean file and the lexer's refusals, as (code, problems): comments and string
    text removed, each removed character a space and every newline kept (ADR-440 D4)."""
    out = list(text)
    problems = []
    lex(path, text, 0, out, problems, None)
    return "".join(out), problems


def lex(path, text, index, out, problems, opened):
    """Lex code from `index`. `opened` is the index of the `{` that opened an interpolation, whose
    code ends after its matching `}`, or None for the file's own code. Answers the index after the
    code, or None once a construct is refused as unterminated."""
    braces = 0
    while index is not None and index < len(text):
        character = text[index]
        if text.startswith("/-", index):
            index = block_comment(path, text, index, out, problems)
        elif text.startswith("--", index):
            index = line_comment(text, index, out)
        elif character == '"':
            index = string(path, text, index, out, problems)
        elif (hashes := raw_hashes(text, index)) is not None:
            index = raw_string(path, text, index, hashes, out, problems)
        elif (end := char_literal_end(text, index)) is not None:
            blank(out, index, end)
            index = end
        elif opened is not None and character == "{":
            braces += 1
            index += 1
        elif opened is not None and character == "}" and braces > 0:
            braces -= 1
            index += 1
        elif opened is not None and character == "}":
            return index + 1
        else:
            index += 1
    if index is not None and opened is not None:
        return refuse(path, text, opened, out, problems, "unterminated-interpolation")
    return index


def block_comment(path, text, start, out, problems):
    """`/-` to its matching `-/`, nesting counted; a doc or module comment opens the same way."""
    nesting = 1
    index = start + 2
    while index < len(text):
        if text.startswith("/-", index):
            nesting += 1
            index += 2
        elif text.startswith("-/", index):
            nesting -= 1
            index += 2
            if nesting == 0:
                blank(out, start, index)
                return index
        else:
            index += 1
    return refuse(path, text, start, out, problems, "unterminated-comment")


def line_comment(text, start, out):
    """`--` to the end of its line."""
    end = text.find("\n", start)
    end = len(text) if end < 0 else end
    blank(out, start, end)
    return end


def string(path, text, start, out, problems):
    """`"` to the next unescaped `"`. After `!` (`s!`, `m!`, `f!`) the string is interpolated, and
    each `{` to its matching `}` is code, lexed by the same rules."""
    interpolated = start > 0 and text[start - 1] == "!"
    piece = start
    index = start + 1
    while index < len(text):
        character = text[index]
        if character == "\\":
            index += 2  # a backslash takes the next character with it
        elif character == '"':
            blank(out, piece, index + 1)
            return index + 1
        elif interpolated and character == "{":
            blank(out, piece, index)
            index = lex(path, text, index + 1, out, problems, index)
            if index is None:
                return None
            piece = index
        else:
            index += 1
    return refuse(path, text, start, out, problems, "unterminated-string")


def raw_hashes(text, start):
    """How many `#` a raw string opened at `start` holds, or None: `r`, any number of `#`, then
    `"`, where the `r` follows no identifier character."""
    if text[start] != "r" or (start > 0 and is_identifier_character(text[start - 1])):
        return None
    index = start + 1
    while index < len(text) and text[index] == "#":
        index += 1
    if index < len(text) and text[index] == '"':
        return index - start - 1
    return None


def raw_string(path, text, start, hashes, out, problems):
    """A raw string: no escape, and it ends at a `"` followed by as many `#` as opened it."""
    close = '"' + "#" * hashes
    end = text.find(close, start + hashes + 2)
    if end < 0:
        return refuse(path, text, start, out, problems, "unterminated-raw-string")
    blank(out, start, end + len(close))
    return end + len(close)


def char_literal_end(text, start):
    """The index after a character literal opened at `start`, or None: a `'` that follows no
    identifier character, holding one character or one backslash escape and closed by `'`. Any
    other `'` is code, a name's prime."""
    if text[start] != "'" or (start > 0 and is_identifier_character(text[start - 1])):
        return None
    close = start + (3 if text.startswith("\\", start + 1) else 2)
    if close < len(text) and text[close] == "'" and "\n" not in text[start + 1 : close]:
        return close + 1
    return None


def name_part_end(code, start):
    """The index after one part of a name opened at `start`: a guillemet part `«…»`, which ends at
    its line's end when it does not close, or an identifier."""
    if code[start] == "«":
        index = start + 1
        while index < len(code) and code[index] not in "»\n":
            index += 1
        return index + 1 if index < len(code) and code[index] == "»" else index
    index = start + 1
    while index < len(code) and is_identifier_character(code[index]):
        index += 1
    return index


def opens_name(character):
    """Whether a name, or one dotted part of it, opens at this character."""
    return character.isalpha() or character in "_«"


def tokens(code):
    """The code as (token, line) pairs, punctuation dropped: names (a letter or `_`, then letters,
    digits, `_`, `'`, `!` and `?`, dotted parts joined by `.`), a guillemet name as one token, and
    numerals."""
    found = []
    line = 1
    index = 0
    while index < len(code):
        character = code[index]
        if character == "\n":
            line += 1
            index += 1
        elif opens_name(character):
            end = name_part_end(code, index)
            while end + 1 < len(code) and code[end] == "." and opens_name(code[end + 1]):
                end = name_part_end(code, end + 1)
            found.append((code[index:end], line))
            index = end
        elif character.isdigit():
            end = index + 1
            while end < len(code) and (code[end].isalnum() or code[end] == "_"):
                end += 1
            found.append((code[index:end], line))
            index = end
        else:
            index += 1
    return found


def numeral_value(token):
    """A numeral's value, read in the base its prefix names, or None for a token that is not one."""
    base = NUMERAL_BASES.get(token[:2].lower(), 10)
    digits = token if base == 10 else token[2:]
    try:
        return int(digits, base)
    except ValueError:
        return None


def hatch_hits(path, text):
    """Every escape hatch in one Lean file's code, as (path, line, arm), the lexer's refusals
    first (R5, R6)."""
    code, problems = lean_code(path, text)
    found = tokens(code)
    hits = list(problems)
    for index, (token, line) in enumerate(found):
        if token in HATCH_WORDS:
            hits.append((path, line, token))
        if token != OPTION_COMMAND or index + 2 >= len(found):
            continue
        name, value = found[index + 1][0], found[index + 2][0]
        if name.startswith(LINTER_PREFIX) and value == "false":
            hits.append((path, line, "linter-off"))
        heartbeats = name == HEARTBEATS or name.endswith("." + HEARTBEATS)
        if heartbeats and numeral_value(value) == 0:
            hits.append((path, line, "heartbeats-zero"))
    return hits


def key_off_hits(path, text):
    """Every place one Lean file's code names `KEY`, as (path, line, `key-off`) (R9)."""
    code, _ = lean_code(path, text)
    hits = []
    for token, line in tokens(code):
        if token == KEY:
            hits.append((path, line, "key-off"))
    return hits


def reconcile(hits, allowed):
    """Split hits by the allow-list, as (left, stale): `left` holds every hit no entry admits, and
    `stale` every entry that admits no hit or names another count than its file's (R7)."""
    counts = collections.Counter((path, arm) for path, _, arm in hits)
    admitted = set()
    stale = []
    for entry in allowed:
        path, arm, count, _reason = entry
        found = counts[(path, arm)]
        if found and found == count:
            admitted.add((path, arm))
        else:
            stale.append(entry)
    left = [hit for hit in hits if (hit[0], hit[2]) not in admitted]
    return left, stale


def census(sources, allowed):
    """Every hit over a set of sources, each file's hatches and its key, reconciled with the
    allow-list, as (left, stale)."""
    hits = []
    for path in sorted(sources):
        hits += hatch_hits(path, sources[path])
        hits += key_off_hits(path, sources[path])
    return reconcile(hits, allowed)


def planted_module(line_text):
    """A planted module that holds `line_text` as its line `PLANTED_LINE`."""
    return PLANTED_HEAD + line_text + "\n"


def prose_forms(line_text):
    """`line_text` as prose: in a line comment, a block comment, a doc comment and a string."""
    quoted = line_text.replace("\\", "\\\\").replace('"', '\\"')
    return (
        ("a line comment", f"-- {line_text}\n"),
        ("a block comment", f"/- {line_text} -/\n"),
        ("a doc comment", f"/-- {line_text} -/\ndef documented : Nat := 0\n"),
        ("a string", f'def prose : String := "{quoted}"\n'),
    )


class EscapeHatches(unittest.TestCase):
    def test_no_lean_file_under_formal_lean_holds_an_escape_hatch(self):
        """A4: the live tree holds no hit outside `ALLOWED`, its root module among the examined; a
        copy of it with one hatch planted at a known line of one file is refused naming exactly
        that file, line and arm; and a link at a tracked `.lean` path is refused by assertion."""
        started = time.monotonic()
        sources = live_sources(REPO)
        paths = examined("Lean files under formal/lean", sorted(sources))
        lines = sum(text.count("\n") for text in sources.values())
        print(f"read {lines} lines in {time.monotonic() - started:.2f} s")
        print(f"allowed {len(ALLOWED)} entries")
        self.assertIn(ROOT_MODULE, paths)
        left, stale = census(sources, ALLOWED)
        self.assertEqual(left, [], "a hit no ALLOWED entry admits")
        self.assertEqual(stale, [], "an ALLOWED entry that admits no hit, or names another count")
        planted = dict(sources)
        body = planted[ROOT_MODULE]
        body = body if body.endswith("\n") else body + "\n"
        line = body.count("\n") + 1
        planted[ROOT_MODULE] = body + "theorem plantedHole : True := sorry\n"
        left, stale = census(planted, ALLOWED)
        self.assertEqual(left, [(ROOT_MODULE, line, "sorry")], "the planted hatch")
        self.assertEqual(stale, [])
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "formal" / "lean").mkdir(parents=True)
            held = root / "held.lean"
            held.write_text("theorem planted : True := trivial\n", encoding="utf-8")
            linked = "formal/lean/Linked.lean"
            os.symlink(held, root / linked)
            with self.assertRaises(AssertionError) as caught:
                read_sources(root, [linked])
            self.assertIn(f"{linked} is a link", str(caught.exception))

    def test_each_hatch_in_code_is_refused_by_its_name_and_none_in_prose(self):
        """A5: each of the ten arms, planted in code, is refused by its own arm at its line, and the
        same text in a line comment, a block comment, a doc comment and a string is admitted;
        `decide`, a linter set true, a nonzero heartbeat limit, `unsafeCast` and a name that holds
        a word are admitted; a zero in each base, and a dotted heartbeat name, read as zero."""
        plants = examined("arms planted in code", ARM_PLANTS)
        self.assertEqual(len({arm for arm, _ in plants}), 10)
        wrong = []
        for arm, line_text in plants:
            hits = hatch_hits(PLANTED_PATH, planted_module(line_text))
            if hits != [(PLANTED_PATH, PLANTED_LINE, arm)]:
                wrong.append(f"{arm} in code: {hits}")
            for form, text in prose_forms(line_text):
                if hatch_hits(PLANTED_PATH, text):
                    wrong.append(f"{arm} in {form}: {hatch_hits(PLANTED_PATH, text)}")
        for line_text in examined("admitted lines of code", ADMITTED):
            hits = hatch_hits(PLANTED_PATH, planted_module(line_text))
            if hits:
                wrong.append(f"{line_text!r} admitted: {hits}")
        for line_text in examined("zero heartbeat limits", ZERO_PLANTS):
            hits = hatch_hits(PLANTED_PATH, planted_module(line_text))
            if hits != [(PLANTED_PATH, PLANTED_LINE, "heartbeats-zero")]:
                wrong.append(f"{line_text!r} as zero: {hits}")
        self.assertEqual(wrong, [], "a planted line the guard read otherwise than its arm")

    def test_an_unterminated_comment_or_string_is_refused_by_name(self):
        """A6: an unterminated block comment, string, raw string and interpolation are each refused
        by their own name at the line they open, and the matcher's code keeps every line of a
        terminated file."""
        wrong = []
        for arm, line_text in examined("unterminated constructs", UNTERMINATED):
            hits = hatch_hits(PLANTED_PATH, "-- planted\n" + line_text)
            if hits != [(PLANTED_PATH, 2, arm)]:
                wrong.append(f"{arm}: {hits}")
        self.assertEqual(wrong, [], "an unterminated construct not refused by its name")
        terminated = examined("terminated files", [*ORACLE_PLANTS.values(), MULTILINE])
        for text in terminated:
            code, problems = lean_code(PLANTED_PATH, text)
            self.assertEqual(problems, [], text)
            self.assertEqual(len(code), len(text), text)
            newlines = [index for index, character in enumerate(text) if character == "\n"]
            kept = [index for index, character in enumerate(code) if character == "\n"]
            self.assertEqual(kept, newlines, text)

    def test_the_matcher_agrees_with_leans_own_verdict_on_each_planted_file(self):
        """A7: for every plant of `ORACLE_PLANTS`, the `sorry` arm refuses it exactly when
        `LEAN_WARNS` records that Lean warned; the two tables name the same plants, and each
        verdict occurs at least once."""
        plants = examined("planted files Lean elaborated", list(ORACLE_PLANTS))
        self.assertEqual(list(LEAN_WARNS), plants)
        self.assertTrue(all(isinstance(verdict, bool) for verdict in LEAN_WARNS.values()))
        self.assertEqual(set(LEAN_WARNS.values()), {True, False})
        wrong = []
        for label in plants:
            hits = hatch_hits(PLANTED_PATH, ORACLE_PLANTS[label])
            refused = any(arm == "sorry" for _, _, arm in hits)
            if refused != LEAN_WARNS[label]:
                wrong.append(
                    f"{label}: the guard refused {refused}, Lean warned {LEAN_WARNS[label]}"
                )
        self.assertEqual(wrong, [], "a plant the matcher reads otherwise than Lean")

    def test_no_lean_module_switches_warnings_as_errors_off(self):
        """A8: the live tree names `warningAsError` in no module's code; a planted module that sets
        it false is refused as `key-off`, and the same name in a comment or a string is admitted."""
        sources = live_sources(REPO)
        paths = examined("Lean files read for the key", sorted(sources))
        named = [hit for path in paths for hit in key_off_hits(path, sources[path])]
        self.assertEqual(named, [], "a module whose code names the key")
        module = "set_option warningAsError false\ntheorem planted : True := trivial\n"
        self.assertEqual(key_off_hits(PLANTED_PATH, module), [(PLANTED_PATH, 1, "key-off")])
        prose = examined(
            "mentions of the key in prose",
            [
                ("a comment", "-- warningAsError\n"),
                ("a string", 'def s : String := "warningAsError"\n'),
            ],
        )
        for form, text in prose:
            self.assertEqual(key_off_hits(PLANTED_PATH, text), [], form)

    def test_the_allow_list_is_empty_and_an_unused_entry_is_refused(self):
        """A9: `ALLOWED` holds no entry; a planted entry that names a planted hit's path, arm and
        count admits it, and one naming another count, or one that admits no hit, is stale."""
        self.assertEqual(ALLOWED, ())
        hits = examined(
            "planted hits",
            [
                ("formal/lean/Formal/Planted.lean", 3, "sorry"),
                ("formal/lean/Formal/Planted.lean", 9, "sorry"),
                ("formal/lean/Formal/Other.lean", 1, "axiom"),
            ],
        )
        admits = ("formal/lean/Formal/Planted.lean", "sorry", 2, "a planted entry")
        self.assertEqual(reconcile(hits, (admits,)), ([hits[2]], []), "the entry admits its hits")
        miscounted = ("formal/lean/Formal/Planted.lean", "sorry", 1, "a planted entry")
        self.assertEqual(reconcile(hits, (miscounted,)), (hits, [miscounted]), "another count")
        unused = ("formal/lean/Formal/Planted.lean", "admit", 1, "a planted entry")
        self.assertEqual(reconcile(hits, (unused,)), (hits, [unused]), "an entry with no hit")


if __name__ == "__main__":
    unittest.main()
