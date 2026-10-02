"""Every `Setting` implementation's `SHAPE` is pinned by its literal (SPEC-192).

A refusal names its setting and its shape (`the setting X is malformed: it must be <shape>`). A
test that compares against the constant (`Hour::SHAPE`) reads the code under test back to itself,
so a mutant that rewrites the words passes. The literal is the pin: a test that spells the words,
or a mutation row whose `find` is the constant's line, fails when they change.

The guard enumerates the implementations by walking `crates/*/src` (a git pathspec of that shape
matches nothing), reads each `const SHAPE` literal, and refuses one that is spelled in no test of
its crate (its `tests/`, or the `#[cfg(test)]` module of the implementation's own file) and in no
mutation row that targets the implementation's own file. It reads Rust source with rustc's lexer,
the Reference's token grammar: comments of both forms (`//` and nested `/* */`) do not count, and
neither does an implementation inside one; a `//` inside a literal of any prefix and hash count
is not a comment; whitespace and comments may stand between an attribute's `#`, `!` and `[`; and
a source it cannot tokenize is refused. Only a test module of the implementation's own file
counts, not a line after it, and only when a crate root reaches that file through modules kept
under `--cfg test`. A module is a test module when its attributes keep it under `--cfg test` and
remove it without, read in three-valued logic in which every option but `test` is unknown and
`true` and `false` hold their values (R8). An out-of-line one (`mod tests;`) is read only from
the one file rustc could read for it, below inline modules too (`mod a { mod tests; }` reads
`a/tests.rs`, or the plain `#[path]` it names, from the module directory), and the inner
attributes that open that file are the module's own (`#![cfg(test)]`); an ambiguous or
attribute-made choice is refused. So is a file that any other declaration the guard can see
compiles without `test` (`#[cfg(not(test))] mod tests;`, a `#[path]` naming it). There the file's
own inner attributes count too; a declaration whose one attribute naming a path is
`cfg_attr(P, path = "...")` reaches the file it names only under P and its default file only
without P; and a declaration whose file the guard cannot name (a path literal it cannot read, a
module below an inline module whose attributes name `path`, or attributes it cannot read whole)
may name any file, so unless its attributes remove it without `test` it refuses every one. A
`macro_rules!` body that declares a `cfg(test)` module, by an outer attribute, by one before a
`$( ... )` repetition or by an inner attribute in its braces, is refused by its file, as the guard
does not expand a macro (#433, #441, #458). A literal that two implementations of one crate share
is pinned only by a row on each implementation's file.

An implementation is read from rustc's tokens (#436): an `impl` whose trait path ends in `Setting`
or in a name a `use ... as` binds to it, with or without `r#`, whose `SHAPE` is the first constant
inside its own braces. A pin counts only in an item rustc compiles under test (#449), read by one
evaluator, `rustc_keeps`, that decides `cfg` over `test`, `any`, `all`, `not`, `true` and `false`
and leaves every other option and every `cfg_attr` unknown, so an item it cannot decide is not
read. What the guard does not expand is refused by its file (#535): a module that a macro body or
invocation declares under an attribute it passes in, a `path`, or a `cfg(test)` that
`rustc_keeps` does not prove harmless, an implementation a macro writes, an out-of-line module
declared in a block, and `include!`. A reached file's module directory is known from how a crate
root reaches it, and a `cfg_attr` path below an inline module is read from the file its predicate
chooses under test (#536).
"""

import functools
import itertools
import json
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SPACE = "\t\n\x0b\x0c\r \x85\u200e\u200f\u2028\u2029"
WHITE = re.compile(f"[{SPACE}]+")
KLEENE = {
    "all": lambda values: False if False in values else None if None in values else True,
    "any": lambda values: True if True in values else None if None in values else False,
    "not": lambda values: None if len(values) != 1 or values[0] is None else not values[0],
}
IDENT = re.compile(r"[^\W\d]\w*")
NUMBER = re.compile(r"\d\w*(?:\.\d\w*)?")
QUOTED = re.compile(r'"(?:[^"\\]|\\.)*"', re.DOTALL)
RAW = re.compile(r"(#*)\"")
CHAR = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'")
PUNCT = "!#$%&*+,-./:;<=>?@^|~"
PAIRS = {")": "(", "]": "[", "}": "{"}
SHAPE = re.compile(r'const\s+SHAPE\s*:\s*&\'static\s+str\s*=\s*("(?:[^"\\]|\\.)*")\s*;')
BANDS = REPO / "scripts" / "mutation-rows.d"
REFUSALS = {
    "invocation": "macro invocation passes a cfg(test) module",
    "meta": "macro declares a module under an attribute it passes in",
    "path": "macro declares a module whose path it names",
    "impl": "macro writes an implementation of Setting",
    "block": "block declares an out-of-line module",
    "include": "include! compiles another file's text",
}
KEYWORDS = frozenset(
    "as async await break const continue crate dyn else enum extern false fn for if impl in let "
    "loop match mod move mut pub ref return self Self static struct super trait true type unsafe "
    "use where while abstract become box do final gen macro override priv try typeof unsized "
    "virtual yield".split()
)


def crate_files(root):
    """Every crate source file the guard reads: `crates/*/src/**/*.rs`."""
    return sorted((root / "crates").glob("*/src/**/*.rs"))


def implementations(root):
    """Every implementation of `Setting` under `crates/*/src`, read from rustc's tokens (#436):
    (crate, file, name, quoted literal, span).

    An `impl` whose trait path ends in a name `bound` reads as `Setting` is one (`trait_impl`). The
    literal is the first `const SHAPE = "..."` inside the implementation's own braces, read from
    comment-free text, and `span` is where that constant sits in the file, so it can be excluded.
    One inside a macro's token tree is not read here: `macro_test_modules` refuses its file.
    """
    names = bound(root)
    found = []
    for path in crate_files(root):
        text = lexed(path.read_text(encoding="utf-8"))
        crate, *inside = path.relative_to(root / "crates").parts
        tokens, pairs, _ = scanned(text)
        hidden = unexpanded(tokens, pairs)
        for at in range(len(tokens)):
            read = None if at in hidden else trait_impl(tokens, pairs, at)
            if read is None or read[0] not in names:
                continue
            _, name, start, end = read
            constant = SHAPE.search(text, start, end)
            found.append(
                (
                    crate,
                    Path(*inside),
                    name,
                    constant.group(1) if constant else None,
                    constant.span(1) if constant else None,
                )
            )
    return found


def bound(root):
    """`Setting` and every name a `use ... as` binds to one of them in any crate file, followed
    through a chain (`use self::A as B;`) until no name is added, each without `r#` (#436). A name
    bound in one module is read in every file, so an implementation of another trait it shadows is
    read as one of `Setting` with no shape: a disclosed false refusal, never a miss."""
    return aliases(tuple(lexed(path.read_text(encoding="utf-8")) for path in crate_files(root)))


@functools.cache
def aliases(texts):
    """`bound` over the crate files' comment-free texts."""
    renames = []
    for text in texts:
        found = scanned(text)[0]
        for at, token in enumerate(found):
            if token[:2] != ("word", "use"):
                continue
            end = next((i for i in range(at, len(found)) if found[i][:2] == ("punct", ";")), at)
            renames += [
                (unraw(found[i - 1][1]), unraw(found[i + 1][1]))
                for i in range(at + 2, end - 1)
                if found[i][:2] == ("word", "as") and found[i - 1][0] == found[i + 1][0] == "word"
            ]
    names = {"Setting"}
    while True:
        more = {alias for name, alias in renames if name in names and alias != "_"} - names
        if not more:
            return names
        names |= more


def trait_impl(found, pairs, at):
    """The `impl` at `found[at]` read from its tokens (#436): (its trait path's last word, the
    type's last word, the span of its braces), each word without `r#` and `$`-prefixed when it is a
    metavariable; a type with no word of its own (a tuple) is named by its tokens. None when
    `found[at]` is no `impl` of a trait (`impl X { }`, `-> impl Trait`)."""
    if found[at][:2] != ("word", "impl"):
        return None
    at += 1
    if at < len(found) and found[at][:2] == ("punct", "<"):
        at = angled(found, pairs, at)
    trait, at = last_word(found, pairs, at, ("for",))
    if trait is None or at >= len(found) or found[at][:2] != ("word", "for"):
        return None
    name, brace = last_word(found, pairs, at + 1, ("where",))
    if name is None:
        name = " ".join(token[1] for token in found[at + 1 : brace])
    while brace < len(found) and found[brace][:2] != ("open", "{"):
        if found[brace][0] == "close" or found[brace][:2] == ("punct", ";"):
            return None
        brace = pairs[brace] + 1 if found[brace][0] == "open" else brace + 1
    if brace >= len(found):
        return None
    return trait, name, found[brace][2], found[pairs[brace]][3]


def angled(found, pairs, at):
    """The index after the `< >` group that opens at `found[at]`; an arrow's `>` closes nothing."""
    depth = 0
    while at < len(found):
        kind, word = found[at][:2]
        if (kind, word) == ("punct", "<"):
            depth += 1
        elif (kind, word) == ("punct", ">") and found[at - 1][:2] != ("punct", "-"):
            depth -= 1
            if depth == 0:
                return at + 1
        elif kind == "close":
            return at
        at = pairs[at] + 1 if kind == "open" else at + 1
    return at


def last_word(found, pairs, at, stops):
    """The last word of a path or a type read from `found[at]` outside its `< >`, without `r#` and
    `$`-prefixed when a metavariable, and the index of the token that ends it: a word of `stops`,
    a `{` or a `;` outside `< >`, or the delimiter that closes the group holding it."""
    word, depth = None, 0
    while at < len(found):
        kind, text = found[at][:2]
        if (
            kind == "close"
            or depth == 0
            and (
                kind == "word" and text in stops or (kind, text) in (("open", "{"), ("punct", ";"))
            )
        ):
            break
        if (kind, text) == ("punct", "<"):
            depth += 1
        elif (kind, text) == ("punct", ">") and found[at - 1][:2] != ("punct", "-"):
            depth -= 1
        elif kind == "word" and depth == 0:
            word = ("$" if found[at - 1][:2] == ("punct", "$") else "") + unraw(text)
        at = pairs[at] + 1 if kind == "open" else at + 1
    return word, at


def made_impl(found, pairs, at, names):
    """True when the `impl` or the `for` at `found[at]`, in a tree the guard does not expand, may
    make an implementation of a trait `names` holds (#436). An `impl` whose trait path, read up to
    the `for` after its generic parameters, ends in such a name or in a metavariable, or holds a
    repetition (`impl $($p)::+ for $t`), is one; so is one whose head the tree leaves open with a
    metavariable or such a name in it, or no trait (`make!(impl)`), and one with no `for` whose
    head holds a repetition, or a metavariable beside another word (`impl Setting $f Wide`). A
    `for` whose head holds a metavariable and no `impl` is read by the same rule, since the `impl`
    is passed in (`$k Setting for Wide`); a loop's `for` and a bound's `for<'a>` make none."""
    if found[at][:2] == ("word", "impl"):
        begin = at + 1
        if begin < len(found) and found[begin][:2] == ("punct", "<"):
            begin = angled(found, pairs, begin)
    elif found[at][:2] == ("word", "for") and not looping(found, pairs, at):
        begin = start_of(found, pairs, at)
    else:
        return False
    trait, stop = last_word(found, pairs, begin, ("for",))
    head = passed_in(found, pairs, begin, stop)
    if found[at][1] == "for" and (stop != at or "impl" in head or not {"$", "$("} & set(head)):
        return False
    if stop < len(found) and found[stop][:2] == ("word", "for"):
        return trait is None or trait in names or trait.startswith("$") or "$(" in head
    if stop < len(found) and found[stop][:2] == ("open", "{"):
        return "$(" in head or "$" in head and len(head) > 1
    return trait is None or trait in names or bool({"$", "$("} & set(head))


def passed_in(found, pairs, begin, stop):
    """The head `found[begin:stop]` outside `< >` and outside groups (#436): each word without
    `r#`, "$" for a metavariable and "$(" for a repetition; `$crate` is no metavariable, it names
    the crate."""
    head, depth, at = [], 0, begin
    while at < stop:
        kind, text = found[at][:2]
        if (kind, text) == ("punct", "<"):
            depth += 1
        elif (kind, text) == ("punct", ">") and found[at - 1][:2] != ("punct", "-"):
            depth -= 1
        elif depth == 0 and is_dollar(found[at]) and at + 1 < stop:
            if found[at + 1][0] == "open":
                head.append("$(")
            elif unraw(found[at + 1][1]) != "crate":
                head.append("$")
                at += 1
        elif depth == 0 and kind == "word":
            head.append(unraw(text))
        at = pairs[at] + 1 if kind == "open" else at + 1
    return head


def looping(found, pairs, at):
    """True when the `for` at `found[at]` opens a loop (`for x in ...`) or a bound (`for<'a>`),
    neither of which makes an implementation (#436)."""
    after = at + 1
    if (
        after + 1 < len(found)
        and found[after][:2] == ("punct", "<")
        and (found[after + 1][0] == "lifetime" or found[after + 1][:2] == ("punct", ">"))
    ):
        return True
    while after < len(found):
        kind, word = found[after][:2]
        if kind == "close" or (kind, word) in (("open", "{"), ("punct", ";")):
            return False
        if (kind, word) == ("word", "in"):
            return True
        after = pairs[after] + 1 if kind == "open" else after + 1
    return False


def groups(found, pairs):
    """The token trees the guard does not expand, outermost only (#535): each `macro_rules!` body
    and each macro invocation's (`name!(...)`, `name![...]`, `name!{...}`; a keyword before `!`
    names no macro), as ("rules" or "invocation", the index of its open delimiter)."""
    out, at = [], 0
    while at + 2 < len(found):
        word = found[at][1]
        if found[at][0] == "word" and word not in KEYWORDS and found[at + 1][:2] == ("punct", "!"):
            rules = unraw(word) == "macro_rules" and found[at + 2][0] == "word"
            opening = at + 2 + rules
            if opening < len(found) and found[opening][0] == "open":
                out.append(("rules" if rules else "invocation", opening))
                at = pairs[opening] + 1
                continue
        at += 1
    return out


def unexpanded(found, pairs):
    """The index of every token inside a tree `groups` names."""
    return {at for _, opening in groups(found, pairs) for at in range(opening, pairs[opening] + 1)}


def comment(text, start):
    """The end of the comment at `start` (a `//` line, or a `/* */` block that nests), or None
    when no comment starts there. ValueError when a block is never closed."""
    if text.startswith("//", start):
        end = text.find("\n", start)
        return len(text) if end == -1 else end
    if not text.startswith("/*", start):
        return None
    depth, end = 1, start + 2
    while depth:
        if end >= len(text):
            raise ValueError("a block comment is never closed")
        step = text[end : end + 2]
        depth += {"/*": 1, "*/": -1}.get(step, 0)
        end += 2 if step in ("/*", "*/") else 1
    return end


def documents(text, start):
    """ "outer" or "inner" for the doc comment at `start` (`///`, `/** */`, `//!`, `/*! */`), and
    None for a plain one (`////`, `/**/` and `/***` open plain comments)."""
    opening, after = text[start : start + 3], text[start + 3 : start + 4]
    if opening in ("//!", "/*!"):
        return "inner"
    if opening == "///" and after != "/" or opening == "/**" and after not in ("*", "/"):
        return "outer"
    return None


def suffix(text, end):
    """The end of a literal's suffix (`"s"x`, `1u8`), which is part of the literal's token."""
    word = IDENT.match(text, end)
    return word.end() if word else end


@functools.cache
def scanned(text):
    """`text` read with rustc's lexer (the Reference's Lexical structure): its tokens as (kind,
    word, start, end), each delimiter's partner index, and the spans a comment covers.

    A kind is "word" (an identifier or keyword; a raw one keeps its `r#`), "literal" (a char,
    byte, string, raw, byte string, C string or number, of any prefix, hash count and suffix),
    "lifetime", "punct" (one character), "open", "close", or "outer" or "inner" for a doc
    comment, which rustc reads as a `doc` attribute. A plain comment, whitespace, a leading byte
    order mark and a shebang are no tokens. Whatever rustc's lexer refuses (a comment or literal
    never closed, a reserved prefix, a character no token starts with, a delimiter unpaired)
    raises ValueError, so a source the guard cannot tokenize is never read as a test.
    """
    found, pairs, stack, comments = [], {}, [], []
    at = 1 if text.startswith("\ufeff") else 0
    if text.startswith("#!", at) and not following(text, at + 2).startswith("["):
        end = text.find("\n", at)
        comments.append((at, len(text) if end == -1 else end))
        at = comments[-1][1]
    while at < len(text):
        white = WHITE.match(text, at)
        if white:
            at = white.end()
            continue
        end = comment(text, at)
        if end is not None:
            comments.append((at, end))
            kind = documents(text, at)
            if kind:
                found.append((kind, text[at:end], at, end))
            at = end
            continue
        kind, end = token(text, at)
        if kind == "open":
            stack.append(len(found))
        elif kind == "close":
            if not stack or found[stack[-1]][1] != PAIRS[text[at]]:
                raise ValueError(f"an unpaired {text[at]!r}")
            pairs[stack[-1]], pairs[len(found)] = len(found), stack[-1]
            stack.pop()
        found.append((kind, text[at:end], at, end))
        at = end
    if stack:
        raise ValueError("a delimiter is never closed")
    return found, pairs, comments


def following(text, at):
    """`text` from the first character at or after `at` that is no whitespace and no plain
    comment: the test that tells a shebang (`#!/usr/bin/env`) from an inner attribute (`#! [`)."""
    while at < len(text):
        end = comment(text, at)
        if text[at] in SPACE:
            at += 1
        elif end is not None and documents(text, at) is None:
            at = end
        else:
            break
    return text[at:]


def token(text, at):
    """The kind and end of the one token that starts at `at`."""
    char = text[at]
    number = NUMBER.match(text, at)
    if number:
        return "literal", number.end()
    word = IDENT.match(text, at)
    if word:
        name, end = word.group(), word.end()
        after = text[end : end + 1]
        raw = RAW.match(text, end) if name in ("r", "br", "cr") else None
        if raw:
            close = text.find('"' + raw.group(1), raw.end())
            if close == -1:
                raise ValueError(f"a raw literal is never closed at {at}")
            return "literal", suffix(text, close + 1 + len(raw.group(1)))
        if name == "r" and after == "#":
            ident = IDENT.match(text, end + 1)
            if ident is None or ident.group() in (
                "_",
                "crate",
                "self",
                "Self",
                "super",
            ):
                raise ValueError(f"no raw identifier at {at}")
            return "word", ident.end()
        quoted = {"b": (QUOTED, CHAR), "c": (QUOTED,)}.get(name, ())
        literal = next((m for m in (p.match(text, end) for p in quoted) if m), None)
        if literal:
            return "literal", suffix(text, literal.end())
        if after in ("#", '"', "'"):
            raise ValueError(f"a reserved prefix {name}{after} at {at}")
        return "word", end
    if char == "'":
        literal = CHAR.match(text, at)
        if literal:
            return "literal", suffix(text, literal.end())
        raw = text.startswith("r#", at + 1)
        name = IDENT.match(text, at + 1 + 2 * raw)
        if name is None or text[name.end() : name.end() + 1] in ("'", "#"):
            raise ValueError(f"no character or lifetime at {at}")
        return "lifetime", name.end()
    if char == '"':
        literal = QUOTED.match(text, at)
        if literal is None:
            raise ValueError(f"a string is never closed at {at}")
        return "literal", suffix(text, literal.end())
    if char == "#" and text[at + 1 : at + 2] in ("#", '"'):
        raise ValueError(f"a reserved guarded literal at {at}")
    if char in "([{":
        return "open", at + 1
    if char in ")]}":
        return "close", at + 1
    if char in PUNCT:
        return "punct", at + 1
    raise ValueError(f"no token starts with {char!r} at {at}")


@functools.cache
def lexed(text):
    """`text` with its comments blanked, keeping every offset and newline. A `//` or `/*` inside a
    literal is no comment, and a quote inside a comment opens no literal, because both are read
    from `scanned`'s tokens."""
    bare = list(text)
    for start, end in scanned(text)[2]:
        bare[start:end] = [c if c == "\n" else " " for c in text[start:end]]
    return "".join(bare)


def predicate(words, at, test):
    """The configuration predicate at `words[at]` and the index after it. `test` is the one option
    known here, and the literals `true` and `false` hold their values; any other option or
    key-value is None (unknown), and so is a raw identifier, and `all`, `any` and `not` combine
    values in three-valued logic, so a True or False holds whatever else is configured."""
    word = words[at]
    if word in KLEENE and words[at + 1] == "(":
        values, at = [], at + 2
        while words[at] != ")":
            value, at = predicate(words, at, test)
            values.append(value)
            if words[at] not in (",", ")"):
                raise ValueError(words[at])
            at += words[at] == ","
        return KLEENE[word](values), at + 1
    if word in ("true", "false") and words[at + 1] in (",", ")"):
        return word == "true", at + 1
    if not word.removeprefix("r#").isidentifier():
        raise ValueError(word)
    if words[at + 1] == "=":
        return None, at + 3
    return (test if word == "test" else None), at + 1


def applied(words):
    """The attributes a `cfg_attr` applies, split at its top-level commas."""
    attributes, depth = [[]], 0
    for word in words:
        depth += (word in ("(", "[", "{")) - (word in (")", "]", "}"))
        if word == "," and depth == 0:
            attributes.append([])
        else:
            attributes[-1].append(word)
    return [attribute for attribute in attributes if attribute]


def condition(words, test):
    """Whether one attribute keeps its item: a `cfg` when its predicate holds, a `cfg_attr` when its
    predicate fails or every attribute it applies keeps the item, and any other attribute always,
    unless it names `cfg` or `cfg_attr` in some other spelling (a path, a raw identifier, inside
    `unsafe(...)`), which the guard does not read."""
    if words[0] not in ("cfg", "cfg_attr"):
        if any(word.removeprefix("r#") in ("cfg", "cfg_attr") for word in words):
            raise ValueError(words)
        return True
    if words[1] != "(" or words[-1] != ")":
        raise ValueError(words)
    value, at = predicate(words, 2, test)
    if words[0] == "cfg":
        if at != len(words) - 1:
            raise ValueError(words)
        return value
    if words[at] != ",":
        raise ValueError(words)
    kept = [condition(attribute, test) for attribute in applied(words[at + 1 : -1])]
    return KLEENE["any"]([KLEENE["not"]([value]), KLEENE["all"](kept)])


def test_only(attributes):
    """True when `attributes` keep a module under `--cfg test` and remove it without, whatever
    else is configured (R8). A malformed attribute, or a run that could not be read whole (None),
    decides nothing, so the module is not read."""
    if attributes is None:
        return False
    try:
        held = [
            KLEENE["all"]([condition(each, test) for each in attributes]) for test in (True, False)
        ]
    except (IndexError, ValueError):
        return False
    return held == [True, False]


def rustc_keeps(run, test):
    """Whether rustc keeps an item whose attributes are `run`, in three-valued logic: the one cfg
    evaluator (#449, #536). It reads `cfg` over `test`, `any`, `all`, `not`, `true` and `false`
    exactly and any other option as unknown; every `cfg_attr` is unknown because it may make a
    `cfg`, and so is an attribute it cannot read or a run that could not be read whole (None). Any
    other attribute keeps the item."""
    if run is None:
        return None
    try:
        values = [None if each[0] == "cfg_attr" else condition(each, test) for each in run]
    except (IndexError, ValueError):
        return None
    return KLEENE["all"](values)


def proven(attributes):
    """True when `rustc_keeps` proves a module no test-only module (#536): removed under `test`,
    or compiled without it."""
    return rustc_keeps(attributes, True) is False or rustc_keeps(attributes, False) is True


def leading(found, pairs, at, end):
    """The inner attributes that open a body (`found[at:end]`), each as its words (a doc comment is
    `doc`), and the index after them: `#`, `!` and `[` are three tokens, so whitespace and
    comments between them change nothing."""
    run = []
    while at < end:
        if found[at][0] == "inner":
            run.append(["doc"])
            at += 1
        elif [t[:2] for t in found[at : at + 3]] == [
            ("punct", "#"),
            ("punct", "!"),
            ("open", "["),
        ]:
            run.append([word for _, word, _, _ in found[at + 3 : pairs[at + 2]]])
            at = pairs[at + 2] + 1
        else:
            break
    return run, at


def outer(found, pairs, first, at):
    """The outer attributes of the item whose keyword is `found[at]`, read back over its visibility
    (`pub`, `pub(...)`), or None when what stands before them is no item's end (`;`, `}`, or the
    file's inner attributes), so the run may not be whole."""
    at -= 1
    if at >= first and found[at][:2] == ("close", ")"):
        at = pairs[at] - 1
        if found[at][:2] != ("word", "pub"):
            return None
    at -= at >= first and found[at][:2] == ("word", "pub")
    run = []
    while at >= first:
        if found[at][0] == "outer":
            run.append(["doc"])
            at -= 1
        elif (
            found[at][:2] == ("close", "]") and pairs[at] > first and is_pound(found[pairs[at] - 1])
        ):
            run.append([word for _, word, _, _ in found[pairs[at] + 1 : at]])
            at = pairs[at] - 2
        else:
            break
    if at >= first and found[at][:2] not in (("punct", ";"), ("close", "}")):
        return None
    return run[::-1]


def is_pound(token):
    """True for a `#` punctuation token."""
    return token[:2] == ("punct", "#")


def modules(text):
    """The file's own inner attributes, and each module it declares at its top level (a module
    declared inside an inline module or a block is read by `sites`, not here): (outer attributes
    or None, index of the `mod` keyword); the name is the token after it (#536)."""
    found, pairs, _ = scanned(text)
    own, first = leading(found, pairs, 0, len(found))
    declared, at = [], first
    while at < len(found):
        if found[at][:2] == ("word", "mod") and [t[0] for t in found[at + 1 : at + 3]] in (
            ["word", "punct"],
            ["word", "open"],
        ):
            declared.append((outer(found, pairs, first, at), at))
        at = pairs[at] + 1 if found[at][0] == "open" else at + 1
    return own, declared


def cfg_test_spans(text):
    """The spans of the items rustc compiles under `test` (`compiled_items`) in the file's
    top-level inline test modules (`mod name { ... }` that only `--cfg test` compiles, with the
    file's own inner attributes), read from `scanned`'s tokens. One declared inside another module
    is not read: only a module's file is followed there."""
    found, pairs, _ = scanned(text)
    own, declared = modules(text)
    spans = []
    for run, at in declared:
        if found[at + 2][1] != "{":
            continue
        inner, body = leading(found, pairs, at + 3, pairs[at + 2])
        if test_only(None if run is None else own + run + inner):
            spans += compiled_items(found, pairs, body, pairs[at + 2])
    return spans


def compiled_spans(text, root):
    """The spans of the items rustc compiles under `test` in a file that opens a test module or a
    test crate (#449): an out-of-line test module's file, whose own inner attributes are judged
    with its declaration (R8), or a file under `tests/` (`root`), a crate root, whose inner
    attributes `rustc_keeps` must read as true."""
    found, pairs, _ = scanned(text)
    own, first = leading(found, pairs, 0, len(found))
    if root and rustc_keeps(own, True) is not True:
        return []
    return compiled_items(found, pairs, first, len(found))


def compiled_items(found, pairs, begin, end):
    """The spans of the items in `found[begin:end]` that rustc compiles under `test` (#449). An
    item is read when `rustc_keeps` reads its outer attributes as true under `test`. A kept inline
    module is walked again with its own inner attributes; any other kept item that holds an
    attribute `rustc_keeps` does not read as true (on a statement, a field, an associated item) is
    not read, a false refusal at worst."""
    spans, at = [], begin
    while at < end:
        first, run = at, []
        while at < end and (
            found[at][0] == "outer"
            or is_pound(found[at])
            and at + 1 < end
            and found[at + 1][:2] == ("open", "[")
        ):
            if found[at][0] == "outer":
                run.append(["doc"])
                at += 1
            else:
                run.append([word for _, word, _, _ in found[at + 2 : pairs[at + 1]]])
                at = pairs[at + 1] + 1
        stop = item_end(found, pairs, at, end)
        head = at + (at < stop and found[at][:2] == ("word", "pub"))
        if head < stop and found[head][:2] == ("open", "("):
            head = pairs[head] + 1
        inline = head + 2 < stop and [t[:2] for t in found[head : head + 3 : 2]] == [
            ("word", "mod"),
            ("open", "{"),
        ]
        if rustc_keeps(run, True) is not True:
            pass
        elif inline:
            inner, body = leading(found, pairs, head + 3, pairs[head + 2])
            if rustc_keeps(inner, True) is True:
                spans += compiled_items(found, pairs, body, pairs[head + 2])
        elif rustc_keeps(attributes_inside(found, pairs, at, stop), True) is True:
            spans.append((found[first][2], found[stop - 1][3]))
        at = stop
    return spans


def item_end(found, pairs, at, end):
    """The index after the item that starts at `found[at]` (#449): after its first `;` at depth 0,
    or after a `{ }` group at depth 0 that the next token does not continue (an open delimiter, a
    punctuation other than `#` and `$`, and the words `else`, `as` and `where` continue it)."""
    while at < end:
        kind, word = found[at][:2]
        if (kind, word) == ("punct", ";"):
            return at + 1
        if kind != "open":
            at += 1
            continue
        at = pairs[at] + 1
        after = found[at][:2] if at < end else ("", "")
        if word == "{" and not (
            after[0] == "open"
            or after[0] == "punct"
            and after[1] not in ("#", "$")
            or after in (("word", "else"), ("word", "as"), ("word", "where"))
        ):
            return at
    return end


def attributes_inside(found, pairs, begin, end):
    """Every attribute inside `found[begin:end]`, outer or inner, each as its words (a doc comment
    is `doc`), so an item that holds one `rustc_keeps` does not read as true is not read (#449)."""
    run = []
    for at in range(begin, end):
        if found[at][0] in ("outer", "inner"):
            run.append(["doc"])
        elif is_pound(found[at]) and at + 1 < end:
            bracket = at + 1 + (found[at + 1][:2] == ("punct", "!"))
            if bracket < end and found[bracket][:2] == ("open", "["):
                run.append([word for _, word, _, _ in found[bracket + 1 : pairs[bracket]]])
    return run


def start_of(found, pairs, at):
    """The index where the item whose keyword is `found[at]` begins, its outer attributes included
    (#535, #536): after the previous `;` or `}`, or after the delimiter that opens the group
    holding it, a `$(` that opens a repetition included, so an attribute inside the repetition
    counts and one before it does not, as each module it writes carries the first, or 0."""
    back = at - 1
    while back >= 0:
        kind, word = found[back][:2]
        if kind == "close" and word != "}":
            back = pairs[back] - 1
        elif kind == "open" or (kind, word) in (("close", "}"), ("punct", ";")):
            return back + 1
        else:
            back -= 1
    return 0


def is_dollar(token):
    """True for a `$` punctuation token, which opens a metavariable or a repetition."""
    return token[:2] == ("punct", "$")


PLAIN = re.compile(r'"([^"\\]*)"')


def unraw(word):
    """A module's name or a path word without its `r#` prefix."""
    return word.removeprefix("r#")


def sites(text):
    """The file's own inner attributes, and each module declaration (`mod name;` or `mod name { }`)
    it makes at any depth of inline modules (one inside a function body, a macro or another block is
    not followed): (name, inline module names or None, the declaration's outer attributes, the
    attributes that enclose it, or None when any run could not be read whole, index of the `mod`
    keyword). The names are None below an inline module whose attributes name `path`, which moves
    its directory in a way the guard does not read (#433). The enclosing attributes are the file's
    inner ones, then each enclosing inline module's outer and inner ones."""
    found, pairs, _ = scanned(text)
    own, first = leading(found, pairs, 0, len(found))
    out = []

    def walk(begin, end, names, chain):
        at = begin
        while at < end:
            if found[at][:2] == ("word", "mod") and [t[0] for t in found[at + 1 : at + 3]] in (
                ["word", "punct"],
                ["word", "open"],
            ):
                name = unraw(found[at + 1][1])
                run = outer(found, pairs, begin, at)
                out.append((name, names, run, chain, at))
                if found[at + 2][1] == "{":
                    inner, body = leading(found, pairs, at + 3, pairs[at + 2])
                    moved = run is None or named_paths(run) != ([], False)
                    walk(
                        body,
                        pairs[at + 2],
                        None if names is None or moved else names + (name,),
                        None if chain is None or run is None else chain + run + inner,
                    )
            if found[at][0] == "open":
                at = pairs[at]
            at += 1

    walk(first, len(found), (), own)
    return own, out


def named_paths(run):
    """The `path = "..."` literals that the attributes spell (a `cfg_attr` one included), each
    without its quotes, and whether `path` is spelled any other way (a raw identifier, an escaped
    or raw literal, a bare word), which the guard cannot read."""
    literals, other = [], False
    for each in run:
        for index, word in enumerate(each):
            if unraw(word) != "path":
                continue
            literal = (
                PLAIN.fullmatch(each[index + 2]) if each[index + 1 : index + 2] == ["="] else None
            )
            if literal:
                literals.append(literal.group(1))
            else:
                other = True
    return literals, other


def below(own, names, name, run, homes=None):
    """The files rustc could read for `mod name;` declared in `own` below the inline modules
    `names`: the file a plain `#[path]` names, relative to the module directory the inline names
    make, or else `name.rs` or `name/mod.rs` in that directory, which is below `own`'s own folder or
    below the folder beside `own` (a crate root, a `mod.rs` and a file an attribute loaded all read
    beside themselves). When the walk from the crate roots knows `own`'s module directory (`homes`,
    #536), only that folder counts. Every file a `cfg_attr` path could name counts too, so a rival
    is never missed. Only files that exist are returned, each once."""
    literals, _ = named_paths(run)
    plain = any(unraw(each[0]) == "path" for each in run)
    folders = [own.with_suffix(""), own.parent]
    folders = [folder for folder in folders if homes is None or folder in homes]
    paths = [folder.joinpath(*names) / lit for folder in folders for lit in literals]
    if not plain:
        paths += [folder.joinpath(*names) / f"{name}.rs" for folder in folders]
        paths += [folder.joinpath(*names) / name / "mod.rs" for folder in folders]
    unique = {path.resolve() for path in paths}
    return sorted(path for path in unique if path.is_file())


def beside(own, name, run):
    """The files a top-level `mod name;` of `own` could name, hedged like `below`: a `#[path]` is
    relative to `own`'s directory."""
    literals, _ = named_paths(run)
    plain = any(unraw(each[0]) == "path" for each in run)
    paths = [own.parent / lit for lit in literals]
    if not plain:
        for place in (own.parent, own.with_suffix("")):
            paths += [place / f"{name}.rs", place / name / "mod.rs"]
    unique = {path.resolve() for path in paths}
    return sorted(path for path in unique if path.is_file())


def declared(own, homes=None):
    """(file, attributes) for each out-of-line module (`mod name;`) at any depth of inline modules
    in `own` whose file rustc's choice leaves in no doubt (R8): of every file rustc could read for
    it (`name.rs` or `name/mod.rs`, in `own`'s module directory or beside `own`, since a crate
    root, a `src/bin` file, a `mod.rs` and a file an attribute loaded all read their modules beside
    themselves, and below an inline module's name), exactly one exists and rustc's lexer reads it.
    Where the walk from the crate roots reaches `own`, only its module directory there counts
    (`homes`, from `reached`; #536), so a decoy in the other folder is no candidate. At the top
    level no attribute of the declaration may carry `path` in any spelling (`#[path]`, a raw
    identifier, `cfg_attr` under any predicate); below an inline module one plain
    `#[path = "..."]` names the file from the module directory (#433), and one
    `cfg_attr(P, path = "...")` names the file `P` chooses under test (`cfg_attr_choice`). The
    attributes are `own`'s inner ones, each enclosing inline module's, the declaration's, and that
    file's inner ones. A declaration inside a function body or a macro is not followed."""
    text = own.read_text(encoding="utf-8")
    tokens = scanned(text)[0]
    if homes is None:
        src = next((parent for parent in own.parents if parent.name == "src"), None)
        homes = None if src is None else reached(src).get(own)
    files = []
    for name, names, run, inner, at in sites(text)[1]:
        if tokens[at + 2][1] != ";":
            continue
        if names is None or inner is None:
            continue
        if names:
            literals, other = named_paths(run or [])
            if run is None or other or len(literals) > 1:
                continue
            if any(unraw(each[0]) != "path" and named_paths([each]) != ([], False) for each in run):
                found = cfg_attr_choice(own, names, name, run, homes)
            else:
                found = below(own, names, name, run, homes)
        else:
            if run is None or any(w.removeprefix("r#") == "path" for each in run for w in each):
                continue
            found = [
                path
                for folder in (own.with_suffix(""), own.parent)
                for path in (folder / f"{name}.rs", folder / name / "mod.rs")
                if path.is_file() and (homes is None or folder in homes)
            ]
        if len(found) == 1:
            try:
                body, pairs, _ = scanned(found[0].read_text(encoding="utf-8"))
            except ValueError:
                continue
            files.append((found[0], inner + run + leading(body, pairs, 0, len(body))[0]))
    return files


def cfg_attr_choice(own, names, name, run, homes):
    """The file a `mod name;` below the inline modules `names` reads under `test` when its one
    attribute naming a path is `cfg_attr(P, path = "...")` (#536): the named file, relative to the
    module directory, when `rustc_keeps` reads `cfg(P)` as true under test, the default `name.rs`
    or `name/mod.rs` when false, and none when it does not decide. Any other run names none."""
    gates = gated(own, names, name, run)
    if not gates:
        return []
    value = rustc_keeps([["cfg", "(", *gates[0][0], ")"]], True)
    folders = [
        folder.joinpath(*names)
        for folder in (own.parent, own.with_suffix(""))
        if homes is None or folder in homes
    ]
    if value is True:
        literals = named_paths(run)[0]
        paths = [folder / literal for folder in folders for literal in literals]
    elif value is False:
        paths = [folder / leaf for folder in folders for leaf in (f"{name}.rs", f"{name}/mod.rs")]
    else:
        return []
    unique = {path.resolve() for path in paths}
    return sorted(path for path in unique if path.is_file())


def module_home(path, attributes):
    """The folder where the modules of a reached file `path` live (#536): beside it for a
    `mod.rs` or a file a `path` attribute loaded (rustc reads both like a crate root), and below
    its own name otherwise (`a.rs` reads its modules from `a/`)."""
    if path.name == "mod.rs" or any(unraw(word) == "path" for each in attributes for word in each):
        return {path.parent}
    return {path.with_suffix("")}


def visible(src):
    """(target file or None, attributes or None) for every out-of-line module that any file
    reachable from the crate's roots declares, whatever its attributes and however deep, and for
    every file each could name (`beside`, `below`). The attributes are the whole run from the root
    that keeps the declaration, then the condition its `cfg_attr` path sets on that file (`gated`),
    then that file's own inner attributes (a module's file opens with the module's own
    attributes), or None when unreadable. A declaration whose file the guard cannot name, from a
    path literal it cannot read, from below an inline module whose attributes name `path`, or from
    attributes it cannot read whole, is listed with the target None too: it may name any file
    (#433, #458). So is an out-of-line module a macro declares, unless `rustc_keeps` proves it
    removed without `test`: the guard cannot name its file, and it may compile one without `test`
    (#536 reopens no rival)."""
    roots = [src / "lib.rs", src / "main.rs", *src.glob("bin/*.rs"), *src.glob("bin/*/main.rs")]
    todo = [(root, []) for root in roots if root.is_file()]
    seen, out = set(), []
    while todo:
        file, chain = todo.pop()
        if (file, repr(chain)) in seen:
            continue
        seen.add((file, repr(chain)))
        try:
            text = file.read_text(encoding="utf-8")
            tokens, pairs, _ = scanned(text)
            found = sites(text)[1]
        except ValueError:
            continue
        for _, opening in groups(tokens, pairs):
            for mod in range(opening + 1, pairs[opening]):
                after = mod + 2 + is_dollar(tokens[mod + 1])
                if tokens[mod][:2] != ("word", "mod") or after >= pairs[opening]:
                    continue
                run = outer(tokens, pairs, start_of(tokens, pairs, mod), mod)
                if tokens[after][:2] == ("punct", ";") and rustc_keeps(run, False) is not False:
                    out.append((None, None))
        for name, names, run, inner, at in found:
            if tokens[at + 2][1] == "{":
                continue
            held = None if chain is None or inner is None or run is None else chain + inner + run
            if names is None or run is None or named_paths(run)[1]:
                out.append((None, held))
                if names is None:
                    continue
            gates = gated(file, names, name, run or [])
            for target in (
                below(file, names, name, run or []) if names else beside(file, name, run or [])
            ):
                reach = held
                for words, named, defaults in gates if held is not None else []:
                    if target in named and target not in defaults:
                        reach = reach + [["cfg", "(", *words, ")"]]
                    elif target in defaults and target not in named:
                        reach = reach + [["cfg", "(", "not", "(", *words, ")", ")"]]
                try:
                    body, pairs, _ = scanned(target.read_text(encoding="utf-8"))
                except ValueError:
                    out.append((target, reach))
                    continue
                tail = leading(body, pairs, 0, len(body))[0]
                kept = None if reach is None else reach + tail
                out.append((target, kept))
                todo.append((target, kept))
    return out


def gated(file, names, name, run):
    """The condition under which each file a declaration could name is reached, when the one
    attribute of `run` that names `path` is a `cfg_attr(P, path = "...")` that names it directly:
    `cfg(P)` for a file only the attribute names, `cfg(not(P))` for a default file it moves away
    from, and nothing for a file reached either way (#458). Any other run gates nothing, so each
    file it could name counts as reached: a plain `#[path]` beside it, two attributes that name a
    path (rustc reads the first that applies), a path nested in another `cfg_attr`, or a predicate
    the guard cannot read. A literal it cannot read names no file here (`visible` lists that
    declaration with no target), though its default file is still gated."""
    carrying = [each for each in run if any(unraw(word) == "path" for word in each)]
    if len(carrying) != 1 or carrying[0][:2] != ["cfg_attr", "("]:
        return []
    each = carrying[0]
    try:
        end = predicate(each, 2, True)[1]
    except (IndexError, ValueError):
        return []
    applies = applied(each[end + 1 : -1])
    if any(unraw(one[0]) != "path" and named_paths([one]) != ([], False) for one in applies):
        return []
    literals = named_paths(applies)[0]
    if names:
        places = [folder.joinpath(*names) for folder in (file.with_suffix(""), file.parent)]
        homes = places
    else:
        places, homes = [file.parent], [file.parent, file.with_suffix("")]
    defaults = {
        path.resolve() for home in homes for path in (home / f"{name}.rs", home / name / "mod.rs")
    }
    named = {(place / literal).resolve() for place in places for literal in literals}
    return [(each[2:end], named, defaults)]


def compiled_without_test(attributes):
    """False only when the attributes remove a module without `--cfg test` whatever else is
    configured; an unreadable run, an unknown option and an error all count as compiled (#458)."""
    if attributes is None:
        return True
    try:
        return KLEENE["all"]([condition(each, False) for each in attributes]) is not False
    except (IndexError, ValueError):
        return True


def test_files(own):
    """The file of each out-of-line test module that `own` declares (`declared`), read only when
    its attributes make it a test module. Any other declaration is not read, so a shape only it
    spells is refused."""
    return [file for file, attributes in declared(own) if test_only(attributes)]


def out_of_line(own):
    """`test_files(own)` less each file that another declaration the guard can see compiles
    without `test` (a `#[cfg(not(test))] mod tests;`, a `#[path]` naming it, a `cfg_attr` path
    that applies without `test`): a shape only it spells is production code, so it is refused
    (#458). A declaration whose file the guard cannot name (`visible`'s None, a macro's out-of-line
    module among them, #536) and that is compiled without `test` refuses every file. A test-only
    second declaration is no rival, and a rival on another file refuses nothing."""
    files = test_files(own)
    src = next((parent for parent in own.parents if parent.name == "src"), None)
    if not files or src is None:
        return files
    others = visible(src)
    return [
        file
        for file in files
        if not any(
            target in (None, file.resolve()) and compiled_without_test(attributes)
            for target, attributes in others
        )
    ]


def reached(src):
    """Every file of the crate at `src` that rustc compiles under `--cfg test` whatever else is
    configured: its roots (`lib.rs`, `main.rs`, `bin/*.rs`, `bin/*/main.rs`), and each file a
    reached file declares (`declared`, below inline modules too) whose attributes keep it under
    test. A file that only an undecided attribute, a top-level `#[path]` or a source rustc refuses
    reaches is not in it, so its test modules are not read. Each reached file maps to its module
    directories (`module_home`): a root's is its own folder, and a file's is the one its reaching
    declaration gives it, so a file is read again when another declaration gives it a new one
    (#536)."""
    roots = [src / "lib.rs", src / "main.rs", *src.glob("bin/*.rs"), *src.glob("bin/*/main.rs")]
    homes = {root: {root.parent} for root in roots if root.is_file()}
    todo, seen = list(homes), {}
    while todo:
        file = todo.pop()
        if seen.get(file) == homes[file]:
            continue
        seen[file] = set(homes[file])
        try:
            files = declared(file, homes[file])
        except ValueError:
            continue
        for path, attributes in files:
            try:
                kept = KLEENE["all"]([condition(each, True) for each in attributes])
            except (IndexError, ValueError):
                kept = None
            home = module_home(path, attributes) if kept is True else set()
            homes[path] = homes.get(path, set()) | home
            if kept is True:
                todo.append(path)
    return {file: homes[file] for file in seen}


def test_crate_files(tests):
    """The files under a crate's `tests/` that rustc compiles under `--cfg test`, as cargo builds
    them (#449): each crate root of cargo's test-target discovery (`tests/<name>.rs`,
    `tests/<dir>/main.rs`), whose module directory is its own folder, and each file a compiled file
    declares (`declared`, below inline modules too) whose attributes `rustc_keeps` reads as true
    under test, read from the module directory its declaration gives it (`module_home`). A file no
    compiled declaration reaches is never compiled, so no pin is read from it."""
    roots = [*tests.glob("*.rs"), *tests.glob("*/main.rs")]
    homes = {root: {root.parent} for root in roots if root.is_file()}
    todo, seen = sorted(homes), {}
    while todo:
        file = todo.pop()
        if seen.get(file) == homes[file]:
            continue
        seen[file] = set(homes[file])
        try:
            files = declared(file, homes[file])
        except ValueError:
            continue
        for path, attributes in files:
            if rustc_keeps(attributes, True) is True:
                homes[path] = homes.get(path, set()) | module_home(path, attributes)
                todo.append(path)
    return sorted(seen)


def spelled_elsewhere(root, crate, file, literal, own_span):
    """True when `literal` is spelled, quoted, in an item rustc compiles under `test` (#449): in
    the crate's tests (`compiled_spans`, a file whose inner attributes remove it under test reads
    nothing), inside a test module of the implementation's own source file (`cfg_test_spans`) or
    in a test module's file, outside a comment, and never at any `SHAPE` constant."""
    base = root / "crates" / crate
    constants = {
        (path, span)
        for holder, path, _, _, span in implementations(root)
        if holder == crate and span
    }
    candidates = test_crate_files(base / "tests")
    if base / file in reached(base / "src"):
        candidates += [base / file] + out_of_line(base / file)
    for path in candidates:
        text = path.read_text(encoding="utf-8")
        bare = lexed(text)
        spans = (
            cfg_test_spans(text)
            if path == base / file
            else compiled_spans(text, base / "tests" in path.parents)
        )
        at = bare.find(literal)
        while at != -1:
            inside = any(start <= at and at + len(literal) <= end for start, end in spans)
            if inside and (path.relative_to(base), (at, at + len(literal))) not in constants:
                return True
            at = bare.find(literal, at + 1)
    return False


def rows_pinning(root, crate, file, literal):
    """Ids of the mutation rows that target this implementation's file and whose `find` holds the
    literal."""
    ids = []
    for band in sorted((root / "scripts" / "mutation-rows.d").glob("*.json")):
        table = json.loads(band.read_text(encoding="utf-8")).get("tables", {})
        for row in table.get("MUTATIONS", []):
            if row[1] == crate and row[2] == file.as_posix() and literal in row[3]:
                ids.append(row[0])
    return ids


def macro_test_modules(root):
    """The crate files that hold a source the guard cannot expand, as refusal lines naming the
    crate, the file and the reason (#441, #535). The guard reads tokens and does not expand a
    macro, so a module or an implementation a macro writes would be one it never sees: it refuses
    the file instead.

    In each token tree `groups` names, a `mod` is judged by the attributes just before it, back to
    the previous item's end or the tree's open delimiter (past a `$(` that opens the module's own
    repetition, so `#[cfg(test)] $(mod $n {})*` counts), and by the inner attributes that open its
    braces (`mod x { #![cfg(test)] }`). In order, the first that applies refuses the file: an
    attribute holding a `$` (`meta`: it is passed in), any spelling of `path` (`path`), then `cfg`
    or `cfg_attr` with `test` in any combination unless `proven` reads the module as no test-only
    one (`cfg(not(test))`), in a `macro_rules!` body (#441) or in an invocation (`invocation`). An
    implementation of a trait `bound` names, or of a metavariable, in a tree is refused (`impl`).
    Outside them, `mod name;` in a block (a function body, a `const` block) that `rustc_keeps`
    does not prove removed without `test` is refused (`block`), and so is `include!`
    (`include`)."""
    names = bound(root)
    refused = []
    for path in crate_files(root):
        text = path.read_text(encoding="utf-8")
        found, pairs, _ = scanned(text)
        hit, reasons = False, []
        for source, opening in groups(found, pairs):
            for mod in range(opening + 1, pairs[opening]):
                if made_impl(found, pairs, mod, names):
                    reasons.append("impl")
                if found[mod][:2] != ("word", "mod"):
                    continue
                words, depth, back = set(), 0, mod - 1
                while back > opening:
                    kind, word = found[back][:2]
                    if kind == "close":
                        if depth == 0 and word == "}":
                            break
                        depth += 1
                    elif kind == "open":
                        if depth == 0:
                            if word == "(" and found[back - 1][:2] == ("punct", "$"):
                                back -= 2
                                continue
                            break
                        depth -= 1
                    elif kind == "punct" and word == ";" and depth == 0:
                        break
                    elif kind == "word":
                        words.add(word)
                    back -= 1
                brace = mod + 2 + (found[mod + 1][:2] == ("punct", "$"))
                inner = []
                if brace < len(found) and found[brace][:2] == ("open", "{"):
                    inner = leading(found, pairs, brace + 1, pairs[brace])[0]
                    for each in inner:
                        words.update(each)
                run = outer(found, pairs, back + 1, mod)
                passed = attributes_inside(found, pairs, back + 1, mod) + inner
                if any("$" in each for each in passed):
                    reasons.append("meta")
                elif "path" in {unraw(word) for word in words}:
                    reasons.append("path")
                elif (
                    words & {"cfg", "cfg_attr"}
                    and "test" in words
                    and not proven(None if run is None else run + inner)
                ):
                    if source == "rules":
                        hit = True
                    else:
                        reasons.append("invocation")
        followed = unexpanded(found, pairs) | {at for *_, at in sites(text)[1]}
        for at in range(len(found) - 2):
            if (
                unraw(found[at][1]) == "include"
                and found[at + 1][:2] == ("punct", "!")
                or imports_include(found, at)
                or passed_macro(found, pairs, at)
            ):
                reasons.append("include")
            if (
                found[at][:2] == ("word", "mod")
                and found[at + 1][0] == "word"
                and found[at + 2][:2] == ("punct", ";")
                and at not in followed
                and rustc_keeps(outer(found, pairs, start_of(found, pairs, at), at), False)
                is not False
            ):
                reasons.append("block")
        if hit:
            crate, *inside = path.relative_to(root / "crates").parts
            refused.append(
                f"{crate} ({Path(*inside).as_posix()}) macro_rules! body declares a cfg(test) module"
            )
        for reason in dict.fromkeys(reasons):
            crate, *inside = path.relative_to(root / "crates").parts
            refused.append(f"{crate} ({Path(*inside).as_posix()}) {REFUSALS[reason]}")
    return refused


def imports_include(found, at):
    """True when `found[at]` opens a `use` declaration that may import `include` (#535): one that
    names it, under any alias and in any group (`use core::include as pull;`), or whose path a
    macro passes in (`use core::$m as pull;`; `$crate` names the crate). `use<'a>` captures
    lifetimes and imports nothing."""
    if found[at][:2] != ("word", "use") or found[at + 1][:2] == ("punct", "<"):
        return False
    depth = 0
    for each in range(at + 1, len(found)):
        kind, word = found[each][:2]
        if kind == "close" and depth == 0 or (kind, word) == ("punct", ";") and depth == 0:
            return False
        depth += {"open": 1, "close": -1}.get(kind, 0)
        if kind == "word" and unraw(word) == "include":
            return True
        if is_dollar(found[each]) and unraw(found[each + 1][1]) != "crate":
            return True
    return False


def passed_macro(found, pairs, at):
    """True when `found[at]` opens a macro name a macro passes in, before `!` and a delimiter
    (#535): a metavariable (`$m!(...)`) or a repetition (`$($m)*!(...)`, past its separator and
    operator). Either may name `include`."""
    if not is_dollar(found[at]):
        return False
    after = at + 2
    if found[at + 1][0] == "open":
        after = pairs[at + 1] + 1
        while after < len(found) and found[after][:2] in REPEATS:
            after += 1
    return (
        after + 1 < len(found)
        and found[after][:2] == ("punct", "!")
        and found[after + 1][0] == "open"
    )


REPEATS = (("punct", ","), ("punct", ";"), ("punct", "*"), ("punct", "+"), ("punct", "?"))


def unpinned(root):
    """The implementations whose literal no test spells and no row of their file finds, then each
    crate file that holds a source the guard cannot expand (`macro_test_modules`). A literal that
    two implementations of one crate share is pinned only by a row of each one's file."""
    found = implementations(root)
    holders = {}
    for crate, _, _, literal, _ in found:
        holders[(crate, literal)] = holders.get((crate, literal), 0) + 1
    return [
        f"{crate}::{name} ({file.as_posix()}) {literal}"
        for crate, file, name, literal, span in found
        if literal is None
        or not (
            (holders[(crate, literal)] == 1 and spelled_elsewhere(root, crate, file, literal, span))
            or rows_pinning(root, crate, file, literal)
        )
    ] + macro_test_modules(root)


class EverySettingShapeIsPinnedByItsLiteral(unittest.TestCase):
    def test_every_setting_impl_has_a_shape_literal_that_a_test_or_a_row_pins(self):
        impls = examined("Setting impl(s)", implementations(REPO))
        self.assertGreater(len(impls), 0)
        files = examined("crate file(s) read for a macro_rules! body", crate_files(REPO))
        self.assertEqual(macro_test_modules(REPO), [], f"{len(files)} file(s) examined")
        self.assertEqual(unpinned(REPO), [], f"{len(impls)} impl(s) examined")


IMPL_TEXT = 'impl Setting for Depth {\n    const SHAPE: &\'static str = "a whole depth";\n}\n'


class TheGuardJudgesAPlantedTree(unittest.TestCase):
    """The guard's own arms, on a tree built at run time: one crate, one implementation."""

    def tree(self, test_text=None, rows=()):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        (root / "crates" / "demo" / "src").mkdir(parents=True)
        (root / "crates" / "demo" / "tests").mkdir()
        (root / "scripts" / "mutation-rows.d").mkdir(parents=True)
        (root / "crates" / "demo" / "src" / "depth.rs").write_text(IMPL_TEXT, encoding="utf-8")
        (root / "crates" / "demo" / "src" / "lib.rs").write_text("mod depth;\n", encoding="utf-8")
        if test_text is not None:
            (root / "crates" / "demo" / "tests" / "depth.rs").write_text(
                test_text, encoding="utf-8"
            )
        band = {"tables": {"MUTATIONS": list(rows)}}
        (root / "scripts" / "mutation-rows.d" / "S00000-S00099.json").write_text(
            json.dumps(band), encoding="utf-8"
        )
        return root

    def test_a_shape_only_its_own_constant_spells_is_refused_by_crate_impl_and_literal(self):
        found = unpinned(self.tree())
        self.assertEqual(found, ['demo::Depth (src/depth.rs) "a whole depth"'])
        double = self.tree()
        (double / "crates" / "demo" / "src" / "depth.rs").write_text(
            "#[cfg(test)]\nmod tests {\n" + IMPL_TEXT + "}\n", encoding="utf-8"
        )
        self.assertEqual(unpinned(double), ['demo::Depth (src/depth.rs) "a whole depth"'])
        twin = self.tree()
        module = "#[cfg(test)]\nmod t {"
        pad = IMPL_TEXT.index('"a whole depth"') - len(module) - len("const Y: &str = ")
        module += " " * pad + 'const Y: &str = "a whole depth";\n}\n'
        (twin / "crates" / "twin" / "src").mkdir(parents=True)
        (twin / "crates" / "twin" / "src" / "lib.rs").write_text("mod depth;\n", encoding="utf-8")
        (twin / "crates" / "twin" / "src" / "depth.rs").write_text(
            module + IMPL_TEXT.replace("Depth", "Twin"), encoding="utf-8"
        )
        self.assertEqual(unpinned(twin), ['demo::Depth (src/depth.rs) "a whole depth"'])

    def test_a_shape_a_test_of_the_crate_spells_is_pinned(self):
        root = self.tree('const X: &str = "a whole depth";')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        deeper = self.tree()
        (deeper / "crates" / "demo" / "tests" / "sub").mkdir()
        (deeper / "crates" / "demo" / "tests" / "sub" / "depth.rs").write_text(
            'const X: &str = "a whole depth";', encoding="utf-8"
        )
        self.assertEqual(unpinned(deeper), ['demo::Depth (src/depth.rs) "a whole depth"'])
        (deeper / "crates" / "demo" / "tests" / "sub" / "depth.rs").rename(
            deeper / "crates" / "demo" / "tests" / "sub" / "main.rs"
        )
        self.assertEqual(unpinned(deeper), [])
        quoted = self.tree('const X: &str = "a \\"q\\" shape";')
        (quoted / "crates" / "demo" / "src" / "depth.rs").write_text(
            IMPL_TEXT.replace('"a whole depth"', '"a \\"q\\" shape"'), encoding="utf-8"
        )
        self.assertEqual(unpinned(quoted), [])

    def test_a_shape_the_test_only_reads_back_from_the_constant_is_refused(self):
        self.assertEqual(len(unpinned(self.tree("let shape = Depth::SHAPE;"))), 1)

    def test_a_shape_a_row_of_the_implementations_own_file_finds_is_pinned(self):
        row = ["S00001-DEPTH", "demo", "src/depth.rs", 'str = "a whole depth";', "", "t::k", "d"]
        root = self.tree(rows=[row])
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])

    def test_a_row_of_another_file_does_not_pin_it(self):
        row = ["S00001-DEPTH", "demo", "src/other.rs", 'str = "a whole depth";', "", "t::k", "d"]
        self.assertEqual(len(unpinned(self.tree(rows=[row]))), 1)
        row = ["S00001-DEPTH", "demo", "src/depth.rs", 'str = "a whole width";', "", "t::k", "d"]
        self.assertEqual(len(unpinned(self.tree(rows=[row]))), 1)
        row = ["S00001-DEPTH", "else", "src/depth.rs", 'str = "a whole depth";', "", "t::k", "d"]
        self.assertEqual(len(unpinned(self.tree(rows=[row]))), 1)

    def src(self, root, text):
        (root / "crates" / "demo" / "src" / "more.rs").write_text(text, encoding="utf-8")
        return root

    def test_a_generic_implementation_is_examined(self):
        root = self.src(
            self.tree('const X: &str = "a whole depth";'),
            'impl<T> Setting for Wide<T> {\n    const SHAPE: &\'static str = "a whole width";\n}\n',
        )
        self.assertEqual(len(implementations(root)), 2)
        self.assertEqual(unpinned(root), ['demo::Wide (src/more.rs) "a whole width"'])

    def test_a_shape_only_a_production_line_of_the_crate_spells_is_refused(self):
        code = 'pub fn depth() -> &\'static str {\n    "a whole depth"\n}\n'
        root = self.src(self.tree(), '/// Reads "a whole depth".\n' + code)
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), ['demo::Depth (src/depth.rs) "a whole depth"'])

    def test_a_shape_two_implementations_of_one_crate_share_needs_a_row_of_each_file(self):
        root = self.src(
            self.tree('const X: &str = "a whole depth";'),
            'impl Setting for Deep {\n    const SHAPE: &\'static str = "a whole depth";\n}\n',
        )
        self.assertEqual(len(implementations(root)), 2)
        self.assertEqual(len(unpinned(root)), 2)
        named = self.src(
            self.tree('const X: &str = "a whole depth";\nconst Y: &str = "a whole width";'),
            "impl Setting for Named {\n    const SHAPE: &'static str = WIDTH;\n}\n"
            'impl Setting for Wide {\n    const SHAPE: &\'static str = "a whole width";\n}\n',
        )
        try:
            found = unpinned(named)
        except TypeError as error:
            self.fail(f"a shape with no literal must be refused, not crash the guard: {error}")
        self.assertEqual(found, ["demo::Named (src/more.rs) None"])

    def test_a_shape_only_a_comment_spells_is_refused(self):
        root = self.tree('// "a whole depth"\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(len(unpinned(root)), 1)
        quote = self.tree('let q = \'"\'; // "a whole depth"\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(unpinned(quote)), 1)
        escaped = self.tree('let q = \'\\"\'; // "a whole depth"\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(unpinned(escaped)), 1)


class TheGuardReadsRustSource(unittest.TestCase):
    """Macro impls, comments, strings and test modules as the compiler reads them (A12)."""

    tree = TheGuardJudgesAPlantedTree.tree
    src = TheGuardJudgesAPlantedTree.src

    def own(self, root, text):
        (root / "crates" / "demo" / "src" / "depth.rs").write_text(text, encoding="utf-8")
        return root

    def macro(self, trait):
        body = 'impl TRAIT for $t {\n    const SHAPE: &\'static str = "a whole width";\n}\n'
        return self.src(self.tree('const X: &str = "a whole depth";'), body.replace("TRAIT", trait))

    def test_a_macro_implementation_is_examined(self):
        root = self.macro("Setting")
        self.assertEqual(len(implementations(root)), 2)
        self.assertEqual(unpinned(root), ['demo::$t (src/more.rs) "a whole width"'])

    def test_a_macro_implementation_through_dollar_crate_is_examined(self):
        root = self.macro("$crate::settings::Setting")
        self.assertEqual(len(implementations(root)), 2)
        self.assertEqual(unpinned(root), ['demo::$t (src/more.rs) "a whole width"'])

    def test_a_shape_the_own_files_test_module_spells_is_pinned(self):
        module = '#[cfg(test)]\nmod tests {\n    const X: &str = "a whole depth";\n}\n'
        root = self.own(self.tree(), IMPL_TEXT + module)
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        for header in (
            "mod tests {\n    fn t() {}\n",
            "#[allow(unused)]\nmod tests {\n",
            "pub mod tests {\n",
        ):
            module = "#[cfg(test)]\n" + header + '    const X: &str = "a whole depth";\n}\n'
            self.assertEqual(unpinned(self.own(self.tree(), IMPL_TEXT + module)), [], header)

    def test_a_shape_a_test_spells_after_a_url_on_its_line_is_pinned(self):
        root = self.tree('let (url, shape) = ("http://host/", "a whole depth");')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        raw = self.tree('let (r, shape) = (r#"http://host/"#, "a whole depth");')
        self.assertEqual(unpinned(raw), [])
        escaped = self.tree('let (e, shape) = ("a \\" //", "a whole depth");')
        self.assertEqual(unpinned(escaped), [])
        hashed = self.tree('let (h, shape) = (r#"a" // "#, "a whole depth");')
        self.assertEqual(unpinned(hashed), [])
        byte = self.tree('let (b, shape) = (br##"a"# // " // "##, "a whole depth");')
        self.assertEqual(unpinned(byte), [])
        for literal in ('c"a \\" // /*"', 'cr"a // /*"', 'cr#"a" // /* "#', 'cr##"a"# // /* "##'):
            c = self.tree(f'let (c, shape) = ({literal}, "a whole depth");')
            self.assertEqual(unpinned(c), [], literal)

    def test_a_shape_only_a_block_comment_spells_is_refused(self):
        root = self.tree('/* "a whole depth" */\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(len(unpinned(root)), 1)
        nested = self.tree('/* a /* b */ "a whole depth" */\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(unpinned(nested)), 1)
        slashed = self.tree('/*/ "a whole depth" */\nlet shape = Depth::SHAPE;')
        self.assertEqual(len(unpinned(slashed)), 1)

    def test_a_production_line_after_the_own_files_test_module_is_refused(self):
        module = '#[cfg(test)]\nmod tests {}\n\npub const X: &str = "a whole depth";\n'
        root = self.own(self.tree(), IMPL_TEXT + module)
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(len(unpinned(root)), 1)
        braces = "#[cfg(test)]\nmod tests {\n    /* { */\n    // {\n"
        braces += "    const C: char = '{';\n    const S: &str = \"{\";\n}\n"
        root = self.own(self.tree(), IMPL_TEXT + braces + 'pub const X: &str = "a whole depth";\n')
        self.assertEqual(len(unpinned(root)), 1)

    def test_a_test_attribute_on_a_use_opens_no_test_module(self):
        text = (
            "#[cfg(test)]\nuse std::fmt;\n" + IMPL_TEXT + 'pub const X: &str = "a whole depth";\n'
        )
        root = self.own(self.tree(), text)
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(len(unpinned(root)), 1)


class TheGuardReadsOutOfLineTestModules(unittest.TestCase):
    """A `#[cfg(test)] mod name;` of the implementation's own file, read where the compiler looks
    for it (A13)."""

    tree = TheGuardJudgesAPlantedTree.tree
    SPELLING = 'const X: &str = "a whole depth";\n'

    def declared(self, declaration, *files, own="depth.rs", text=IMPL_TEXT):
        """A tree whose `src/<own>` declares a module, and the module files written at `files`."""
        root = self.tree()
        src = root / "crates" / "demo" / "src"
        if own != "depth.rs":
            (src / "depth.rs").unlink()
            stem = Path(own).parts[0].removesuffix(".rs")
            (src / "lib.rs").write_text(f"mod {stem};\n", encoding="utf-8")
        (src / own).parent.mkdir(parents=True, exist_ok=True)
        (src / own).write_text(text + declaration, encoding="utf-8")
        for name, body in files:
            (src / name).parent.mkdir(parents=True, exist_ok=True)
            (src / name).write_text(body, encoding="utf-8")
        return root

    def test_a_module_file_beside_the_own_file_is_its_test_module(self):
        root = self.declared("#[cfg(test)]\nmod tests;\n", ("depth/tests.rs", self.SPELLING))
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        public = "#[cfg(test)]\npub(crate) mod tests;\n"
        self.assertEqual(unpinned(self.declared(public, ("depth/tests.rs", self.SPELLING))), [])

    def test_a_module_directory_with_a_mod_file_is_its_test_module(self):
        root = self.declared("#[cfg(test)]\nmod tests;\n", ("depth/tests/mod.rs", self.SPELLING))
        own = root / "crates" / "demo" / "src" / "depth.rs"
        self.assertEqual(out_of_line(own), [own.with_suffix("") / "tests" / "mod.rs"])
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])

    def test_a_module_whose_file_an_attribute_chooses_is_not_read(self):
        for declaration in (
            '#[cfg(test)]\n#[path = "words/shape.rs"]\nmod tests;\n',
            '#[path = "words/shape.rs"]\n#[cfg(test)]\nmod tests;\n',
            '#[cfg(test)]\n#[path="words/shape.rs"]\nmod tests;\n',
        ):
            root = self.declared(declaration, ("words/shape.rs", self.SPELLING))
            self.assertEqual(len(implementations(root)), 1)
            self.assertEqual(len(unpinned(root)), 1, declaration)

    def test_the_module_of_a_lib_file_is_read_from_the_crates_src_directory(self):
        root = self.declared(
            "#[cfg(test)]\nmod tests;\n", ("tests.rs", self.SPELLING), own="lib.rs"
        )
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        for own, name in (("main.rs", "tests.rs"), ("a/mod.rs", "a/tests.rs")):
            root = self.declared("#[cfg(test)]\nmod tests;\n", (name, self.SPELLING), own=own)
            self.assertEqual(len(implementations(root)), 1, own)
            self.assertEqual(unpinned(root), [], own)

    def test_a_file_that_is_no_declared_test_module_is_not_read_as_one(self):
        elsewhere = ("depth/tests.rs", self.SPELLING)
        for declaration in (
            "",
            "mod tests;\n",
            "#[cfg(test)]\nmod other;\n",
            "mod inner {\n    #[cfg(test)]\n    mod tests;\n}\n",
            "#[allow(dead_code)]\nmod tests;\n",
            'const D: &str = "#[cfg(test)] mod tests;";\n',
            'const D: &str = "; #[cfg(test)] mod tests;";\n',
            'macro_rules! m {\n    ($($t:tt)*) => {};\n}\nm!(a; "#[cfg(test)] mod tests;");\n',
            'macro_rules! m {\n    ($($t:tt)*) => {};\n}\nm!{a; r#"#[cfg(test)] mod tests;"#}\n',
        ):
            root = self.declared(declaration, elsewhere)
            self.assertEqual(len(implementations(root)), 1)
            self.assertEqual(len(unpinned(root)), 1, declaration)
        root = self.declared("#[allow(dead_code)]\nmod tests;\n", ("depth/tests.rs", self.SPELLING))
        own = root / "crates" / "demo" / "src" / "depth.rs"
        self.assertEqual(test_files(own), [])
        root = self.declared("#[cfg(test)]\nmod tests;\n", ("depth/tests.rs", self.SPELLING))
        own = root / "crates" / "demo" / "src" / "depth.rs"
        self.assertEqual(test_files(own), [own.with_suffix("") / "tests.rs"])
        root = self.declared("#[cfg(test)]\nmod tests;\n", ("depth/tests.rs", "let shape = 1;\n"))
        self.assertEqual(len(unpinned(root)), 1)
        other = '#[path = "words.rs"]\nmod words;\n#[cfg(test)]\nmod tests;\n'
        self.assertEqual(len(unpinned(self.declared(other, ("words.rs", self.SPELLING)))), 1)
        path = '#[cfg(test)]\n#[path = "words/shape.rs"]\nmod tests;\n'
        root = self.declared(path, ("words/shape.rs", "let shape = 1;\n"), elsewhere)
        self.assertEqual(len(unpinned(root)), 1)

    KINDS = (
        ("lib.rs", (), "tests.rs"),
        ("main.rs", (), "tests.rs"),
        ("bin/tool.rs", (), "bin/tests.rs"),
        ("a/depth.rs", (("lib.rs", "mod a;\n"), ("a/mod.rs", "mod depth;\n")), "a/depth/tests.rs"),
        ("a/mod.rs", (("lib.rs", "mod a;\n"),), "a/tests.rs"),
        ("x/depth.rs", (("lib.rs", '#[path = "x/depth.rs"]\nmod depth;\n'),), "x/tests.rs"),
    )
    ATTRIBUTES = (
        "",
        '#[path = "t.rs"]\n',
        '#[path="t.rs"]\n',
        '#[ path = "t.rs" ]\n',
        '#[path = r"t.rs"]\n',
        '#[r#path = "t.rs"]\n',
        '#[cfg_attr(test, path = "t.rs")]\n',
        '#[cfg_attr(all(test, unix), path = "t.rs")]\n',
    )
    LEAVES = ("test", "x", 'feature = "slow"')
    UNKNOWN = re.compile(r"\bx\b|\bfeature\b")
    CARRIERS = (
        'const _: &str = "{}";',
        'const _: &[u8] = b"{}";',
        "const _: char = '{}';",
        'const _: &str = r#"{}"#;',
        "/* {} */",
        "// {}",
    )

    def predicates(self):
        """Every predicate to depth two over the leaves, built from the guard's own operators, each
        at every arity it evaluates (so `not` takes exactly one argument)."""
        arity = {op: [n for n in range(3) if KLEENE[op]([True] * n) is not None] for op in KLEENE}

        def calls(op, parts, n):
            return [f"{op}({', '.join(args)})" for args in itertools.product(parts, repeat=n)]

        one = [*self.LEAVES] + [
            p for op in KLEENE for n in arity[op] for p in calls(op, self.LEAVES, n)
        ]
        two = [f"{op}({p})" for op in KLEENE if 1 in arity[op] for p in one]
        two += [
            f"{op}({p}, {q})" for op in KLEENE if 2 in arity[op] for p in one for q in self.LEAVES
        ]
        return one, one + two

    def members(self):
        """R8's population, generated: (case, files, root, own, decided). Every module source rustc
        could compile holds a probe naming its place, and the declaring file ends with one that
        every build compiles, so rustc names each source it compiles. A decided member names no
        option but `test` and leaves the guard one candidate file, so the guard must read exactly
        what rustc compiles only under test; any other member it may refuse.
        """
        found = []

        def add(case, files, own, decided, root=None):
            files = {**files, own: files[own] + probe("own")}
            files = {**files, root or own: files[root or own] + probe("root")}
            files = {
                path: text.replace("PROBE ", f"PROBE {len(found)} ") for path, text in files.items()
            }
            found.append((case, files, root or own, own, decided))

        def probe(place):
            return f'compile_error!("PROBE {place}");\n'

        gate = f"#[cfg({self.LEAVES[0]})]\n"
        one, every = self.predicates()
        negated = [*self.LEAVES, *(f"not({leaf})" for leaf in self.LEAVES)]
        attributes = [f"cfg({p})" for p in every]
        attributes += [f"cfg_attr({p}, cfg({q}))" for p in negated for q in negated]
        attributes += [f"cfg_attr({p}, allow(dead_code))" for p in self.LEAVES]
        attributes += ["cfg_attr(test, allow(dead_code), cfg(any()))"]
        runs = []
        for a in attributes:
            known = not self.UNKNOWN.search(a)
            runs += [(f"#[{a}]\n{gate}", "", known), (f"{gate}#[{a}]\n", "", known)]
            runs += [(gate, f"#![{a}]\n", known), (f"#[{a}]\n", "", known)]
        bracket = 'doc = concat!["a"]'
        for p in one:
            runs += [(f"#[cfg({p})]\n#[{bracket}]\n{gate}", "", False)]
            runs += [(gate, f"#![{bracket}]\n#![cfg({p})]\n", False)]
            # No outer attribute gates the module, so only its second inner one decides it.
            runs += [("", f'#![doc = "a"]\n#![cfg({p})]\n', not self.UNKNOWN.search(p))]
        runs += [(gate + vis, "", True) for vis in ("pub ", "pub(crate) ")]
        for outer, inner, known in runs:
            for name in ("tests", "r#tests") if outer.endswith(" ") else ("tests",):
                declaration = f"{outer}mod {name}"
                for place, decides in (("tests.rs", known), ("lib/tests.rs", False)):
                    files = {"lib.rs": declaration + ";\n", place: inner + probe(place)}
                    add(f"lib.rs {declaration}; {inner!r} at {place}", files, "lib.rs", decides)
                files = {"lib.rs": f"{declaration} {{\n{inner}{probe('inline')}}}\n"}
                files["tests.rs"] = probe("tests.rs")
                add(f"lib.rs {declaration} {{ {inner!r} }}", files, "lib.rs", known)
        for wrapper in ("", "#[cfg(any())]\n"):
            for carrier in ("", *(c.format(b) for c in self.CARRIERS for b in "{}")):
                opened = f"{wrapper}mod inner {{\n{carrier}\n{gate}mod tests"
                files = {"lib.rs": opened + ";\n}\n", "inner/tests.rs": probe("inner/tests.rs")}
                add(f"{opened}; }}", {**files, "tests.rs": probe("tests.rs")}, "lib.rs", False)
                inline = f"{opened} {{\n{probe('inline')}}}\n}}\n"
                add(f"{opened} {{ }} }}", {"lib.rs": inline}, "lib.rs", False)
        for own, extra, default in self.KINDS:
            stem, folder = Path(own).with_suffix(""), Path(own).parent
            places = [p / leaf for p in (stem, folder) for leaf in ("tests.rs", "tests/mod.rs")]
            root = next((path for path, _ in extra if path == "lib.rs"), own)
            for attribute in self.ATTRIBUTES:
                chosen = (folder / "t.rs").as_posix() if attribute else default
                stale = [p.as_posix() for p in places if p.as_posix() != chosen]
                for misplaced in (None, *stale):
                    files = {**dict(extra), own: f"{gate}{attribute}mod tests;\n"}
                    files.update({place: probe(place) for place in (chosen, misplaced) if place})
                    decided = not attribute and misplaced is None and root == own
                    add(f"{own} {attribute!r} stale={misplaced}", files, own, decided, root)
        every = list(itertools.product(("lib.rs", "a.rs"), ("ool", "inline"), (0, 1)))
        for outer, inner, pre, decided, crossed in self.lexer_runs():
            for kind, form, crlf in every if crossed else (every[0], every[-1]):
                opened = pre + outer + "mod tests"
                lib = {"a.rs": "mod a;\n"}.get(kind, "")
                files = {"lib.rs": lib, "a.rs": ""} if kind == "a.rs" else {"lib.rs": ""}
                folder = "a/" if kind == "a.rs" else ""
                if form == "ool":
                    files[kind] += opened + ";\n"
                    files[f"{folder}tests.rs"] = inner + probe(f"{folder}tests.rs")
                else:
                    files[kind] += f"{opened} {{\n{inner}{probe('inline')}}}\n"
                    files[f"{folder}tests.rs"] = probe(f"{folder}tests.rs")
                if crlf:
                    files = {path: text.replace("\n", "\r\n") for path, text in files.items()}
                add(
                    f"{kind} {form} crlf={crlf} {opened!r} {inner!r}",
                    files,
                    kind,
                    decided,
                    "lib.rs",
                )
        for opening, lib in itertools.product(self.OPENINGS, self.CHAINS):
            files = {"lib.rs": lib, "a.rs": f"{opening[1]}{gate}mod tests;\n"}
            files["a/tests.rs"] = opening[0] + probe("a/tests.rs")
            add(f"a.rs {opening!r} {lib!r}", files, "a.rs", True, "lib.rs")
        # A pin, not only a refusal, rides on each chain and each opening: a file each chain reaches
        # holds its test module inline, and a module file is test-only by its inner attribute alone.
        for lib in self.CHAINS:
            files = {"lib.rs": lib, "a.rs": f"{gate}mod tests {{\n{probe('inline')}}}\n"}
            add(f"a.rs {gate!r} mod tests {{ }} {lib!r}", files, "a.rs", True, "lib.rs")
        for opening, _ in self.OPENINGS:
            if "cfg(any())" not in opening:
                continue
            inner = opening.replace("cfg(any())", "cfg(test)")
            files = {"lib.rs": "mod a;\n", "a.rs": "mod tests;\n"}
            files["a/tests.rs"] = inner + probe("a/tests.rs")
            add(f"a.rs mod tests; {inner!r}", files, "a.rs", True, "lib.rs")
        for body in (";\n", f" {{\n{probe('inline')}}}\n"):
            opened = "#![cfg(test)]\n#[cfg(any(test, x))]\nmod tests"
            files = {"lib.rs": "mod a;\n", "a.rs": opened + body, "a/tests.rs": probe("a/tests.rs")}
            add(f"a.rs {opened + body!r}", files, "a.rs", True, "lib.rs")
        for sep in self.SEPARATORS:
            split = f"#{sep}[{sep}cfg(test){sep}]\n"
            add(
                f"lib.rs {split!r}",
                {"lib.rs": split + "mod tests;\n", "tests.rs": probe("tests.rs")},
                "lib.rs",
                True,
            )
        return found

    SEPARATORS = (" ", "\n", "\t", "\r", "\x0c", "\u2028", "/**/", "/* a /* b */ c */", "// c\n")
    OPENINGS = (
        ("", ""),
        ("\ufeff#![cfg(any())]\n", ""),
        ("\ufeff#!/bin/tool\n#![cfg(any())]\n", ""),
        ("#!/bin/tool\n#![cfg(any())]\n", ""),
        ("#! [cfg(any())]\n", ""),
        ("#!/**/[cfg(any())]\n", ""),
        ("/*** c */\n#![cfg(any())]\n", ""),
        ("", "#![cfg(not(test))]\n"),
        ("", "# ! [cfg(any())]\n"),
        ("", "//! d\n#![cfg(test)]\n"),
    )
    CHAINS = (
        "mod a;\n",
        "#[cfg(not(test))]\nmod a;\n",
        "#[cfg(test)]\nmod a;\n",
        "# [cfg(any())]\nmod a;\n",
        "#[cfg(x)]\nmod a;\n",
    )

    def literals(self):
        """Every literal form of the Reference with each prefix and 0 to 3 hashes, holding what a
        reader could take for a quote, an escape, a comment or a brace, and the text after it that
        closes a misreading: `#[cfg(any())]` hides behind it only when the literal is misread."""
        found = []
        for prefix in ("", "b", "c"):
            for content in ("\\\\", '\\"', "*/", "}"):
                found.append((f'{prefix}"{content}"', '"'))
        for prefix, hashes in itertools.product(("r", "br", "cr"), range(4)):
            fence = "#" * hashes
            for content in ("\\", *(('a"b',) if hashes else ()), "*/", "}"):
                found.append((f'{prefix}{fence}"{content}"{fence}', '"' + fence))
        return found + [("'\"'", "'"), ("'\\''", "'"), ("b'\"'", "'")]

    def lexer_runs(self):
        """(outer run, inner run, text before the run, decided, crossed), drawn from the Reference's
        token grammar: whitespace and comments at every place between an attribute's `#`, `!` and
        `[`, doc comments of each kind among the attributes, literals of every prefix before the
        run, macro token trees of each delimiter, and each spelling of `cfg` the guard does not
        read. A crossed run is planted in every module kind, form and line ending; the others in
        two plantings that between them take each value of each."""
        gate, found = "#[cfg(test)]\n", []
        for sep, mask in itertools.product(self.SEPARATORS, ({1}, {2}, {3}, {1, 2, 3, 4})):
            one, two, three, four = (sep if i in mask else "" for i in (1, 2, 3, 4))
            for predicate in ("any()", "not(test)"):
                found.append((gate, f"#{one}!{two}[{three}cfg({predicate}){four}]\n", "", True, 1))
                second = f"#{one}[{three}cfg({predicate}){four}]\n"
                found += [(gate + second, "", "", True, 1), (second + gate, "", "", True, 1)]
        macros = "macro_rules! m {\n    ($($t:tt)*) => {};\n}\n"
        for literal, close in self.literals():
            for holder in (
                "const _: () = {{ let _ = {}; }};\n",
                "m!({});\n",
                "m![{}];\n",
                "m!{{{}}}\n",
            ):
                pre = macros + holder.format(literal)
                found.append((f"#[cfg(any())] // {close};\n{gate}", "", pre, True, 0))
                found.append((gate, "", pre, True, 0))
        for doc in ("/// d\n", '/** " */\n', "//// d\n", "/***/\n", '/* /* " */ */\n', '// "\n'):
            found += [
                (doc + gate + "#[cfg(any())]\n", "", "", True, 1),
                (gate + doc, "", "", True, 1),
            ]
        for doc in ("//! d\n", '/*! " */\n', "/* /* */ #![cfg(any())] */\n"):
            found += [(gate, doc + "#![cfg(any())]\n", "", True, 1), (gate, doc, "", True, 1)]
        for call in (
            'm!(a; "#[cfg(test)] mod tests;");\n',
            'm![a; "; #[cfg(test)] mod tests;"];\n',
            "m!{ #[cfg(test)] mod tests; }\n",
            'const D: &str = "; #[cfg(test)] mod tests;";\n',
        ):
            found += [("", "", macros + call, True, 0), (gate, "", macros + call, True, 0)]
        for value in ('"slow"', 'r"slow"', 'r#"slow"#'):
            found += [
                (f"#[cfg_attr(feature = {value}, allow(dead_code))]\n{gate}", "", "", True, 0),
                (gate, f"#![cfg(any(test, feature = {value}))]\n", "", True, 0),
            ]
        for literal in ("true", "false", "all(test, true)", "any(test, false)", "not(false)"):
            found += [
                (f"#[cfg({literal})]\n", "", "", True, 0),
                (gate, f"#![cfg({literal})]\n", "", True, 0),
            ]
        for spelling in ("cfg(r#test)", "r#cfg(any())", "core::prelude::v1::cfg(any())"):
            found += [
                (gate + f"#[{spelling}]\n", "", "", False, 0),
                (f"#[{spelling}]\n", "", "", False, 0),
            ]
        return found

    def compiled(self, src, count):
        """What rustc compiles of each member under every setting of `test`, `x` and a feature:
        (compiled in every run with `--cfg test`, compiled in any run without, members in error).
        The reading fails closed: each run must exit 1 and count exactly the errors it printed, and
        each member not in error must have its root's probe fire in every run, so a rustc that
        compiles nothing, or stops early, is no answer."""
        rustc = shutil.which("rustc")
        if rustc is None:
            self.fail("rustc is not on PATH, so R8's oracle cannot run: this test never skips")
        switches = (("--cfg", "test"), ("--cfg", "x"), ("--cfg", 'feature="slow"'))
        runs = []
        for chosen in itertools.product((False, True), repeat=len(switches)):
            flags = [flag for on, pair in zip(chosen, switches) if on for flag in pair]
            log = (src.parent / f"{len(runs)}.log").open("w", encoding="utf-8")
            command = [rustc, "--edition", "2024", "--crate-type", "lib", "-A", "warnings"]
            command += ["--error-format=json", "--emit=metadata", "-o", f"{log.name}.rmeta", *flags]
            process = subprocess.Popen([*command, str(src / "lib.rs")], cwd=REPO, stderr=log)
            runs.append((chosen[0], process, log))
        under, without, broken, fired = [None] * count, [set() for _ in range(count)], set(), []
        for test, process, log in runs:
            process.wait()
            log.close()
            self.assertEqual(process.returncode, 1, f"rustc exited {process.returncode}, not 1")
            places, errors, aborted = [set() for _ in range(count)], 0, None
            for line in Path(log.name).read_text(encoding="utf-8").splitlines():
                message = json.loads(line) if line.startswith("{") else {}
                if message.get("level") != "error":
                    continue
                abort = re.fullmatch(r"aborting due to (\d+) previous errors?", message["message"])
                if abort:
                    aborted = int(abort.group(1))
                    continue
                errors += 1
                words = message["message"].split(" ", 2)
                if words[0] == "PROBE":
                    places[int(words[1])].add(words[2])
                    continue
                owners = {re.search(r"/m(\d+)/", s["file_name"]) for s in message["spans"]}
                if None in owners or not owners:
                    self.fail(f"rustc failed outside the population: {message['message']}")
                broken |= {int(owner.group(1)) for owner in owners}
            self.assertEqual(aborted, errors, "rustc's count of its errors is not the oracle's")
            fired.append(places)
            for index, compiled in enumerate(places):
                if test:
                    under[index] = compiled if under[index] is None else under[index] & compiled
                else:
                    without[index] |= compiled
        silent = [
            i for i in range(count) if i not in broken and any("root" not in p[i] for p in fired)
        ]
        self.assertEqual(silent[:1], [], f"{len(silent)} member(s) whose root probe did not fire")
        return under, without, broken

    SETTINGS = (
        'impl Setting for Depth {\n    const SHAPE: &\'static str = "a whole depth";\n}\n'
        'impl Setting for Width {\n    const SHAPE: &\'static str = "a whole width";\n}\n'
    )

    def test_every_module_file_choice_is_read_from_rustcs_file_or_refused(self):
        """R8's class, generated from rustc's token grammar and judged by rustc: planted as a crate
        whose own file implements two settings, each member spells `Depth`'s shape in every source
        rustc compiles only under `--cfg test`, whatever else is configured, and `Width`'s in every
        other source a probe stands in. The guard must refuse `Width` always, and pin `Depth` for a
        decided member that has such a source; otherwise it may refuse. A member rustc refuses
        compiles no source, so every source it holds spells `Width`'s shape and none pins."""
        members = self.members()
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        src = Path(directory.name) / "src"
        roots = []
        for index, (_, files, root, _, _) in enumerate(members):
            for path, text in files.items():
                (src / f"m{index}" / path).parent.mkdir(parents=True, exist_ok=True)
                (src / f"m{index}" / path).write_bytes(text.encode("utf-8"))
            roots.append(f'#[path = "m{index}/{root}"]\nmod m{index};\n')
        (src / "lib.rs").write_text("".join(roots), encoding="utf-8")
        under, without, broken = self.compiled(src, len(members))
        wrong, judged, refusing = [], [], []
        for index, (case, files, _, _, decided) in enumerate(members):
            if decided and index in broken:
                wrong.append(f"{case}: a decided member does not compile")
            (refusing if index in broken else judged).append(case)
            only = set() if index in broken else under[index] - without[index] - {"own", "root"}
            spell = {"own": self.SETTINGS, "root": ""}
            width = self.SPELLING.replace("depth", "width")
            tree = Path(directory.name) / f"t{index}"
            for path, text in files.items():
                text = re.sub(
                    r'compile_error!\("PROBE \d+ (\S+)"\);',
                    lambda p: spell.get(p.group(1), self.SPELLING if p.group(1) in only else width),
                    text,
                )
                (tree / "crates" / "demo" / "src" / path).parent.mkdir(parents=True, exist_ok=True)
                (tree / "crates" / "demo" / "src" / path).write_bytes(text.encode("utf-8"))
            (tree / "scripts" / "mutation-rows.d").mkdir(parents=True)
            refused = " ".join(unpinned(tree))
            if "demo::Width " not in refused:
                wrong.append(f"{case}: a source rustc does not compile only under test is read")
            if decided and only and "demo::Depth " in refused:
                wrong.append(f"{case}: refused, and rustc compiles {sorted(only)} only under test")
        examined("R8 member(s) judged against rustc", judged)
        examined("R8 member(s) rustc refuses, each source read as Width's", refusing)
        self.assertGreater(len(judged), 0)
        self.assertEqual(wrong[:1], [], f"{len(wrong)} of {len(members)} members")


class TheGuardIgnoresAnImplementationInAComment(unittest.TestCase):
    """An `impl Setting for` inside a comment is no implementation (A14)."""

    tree = TheGuardJudgesAPlantedTree.tree
    src = TheGuardJudgesAPlantedTree.src

    def test_a_block_comment_holding_an_impl_is_not_examined(self):
        ghost = 'impl Setting for Ghost {\n    const SHAPE: &\'static str = "a ghost shape";\n}\n'
        pinned = self.tree('const X: &str = "a whole depth";')
        for comment in ("/*\n" + ghost + "*/\n", "/* a /* nested */\n" + ghost + "*/\n"):
            root = self.src(pinned, comment)
            self.assertEqual(len(implementations(root)), 1)
            self.assertEqual(unpinned(root), [])
        line = self.src(pinned, "// " + ghost.replace("\n", "\n// "))
        self.assertEqual(len(implementations(line)), 1)
        live = self.src(pinned, ghost)
        self.assertEqual(unpinned(live), ['demo::Ghost (src/more.rs) "a ghost shape"'])
        after = self.src(pinned, "const A: u8 = 1; /* a\n b */ " + ghost)
        self.assertEqual(unpinned(after), ['demo::Ghost (src/more.rs) "a ghost shape"'])
        # `*/` closes at its slash, so `*//*` is two block comments and no line comment.
        joined = self.src(pinned, "/* a *//* b */ " + ghost.replace("\n", " ") + "\n")
        self.assertEqual(unpinned(joined), ['demo::Ghost (src/more.rs) "a ghost shape"'])
        self.assertEqual(lexed("a /* b\nc */ d"), "a     \n     d", "blanking keeps each newline")


class TheGuardResolvesAModuleDeclaredInsideAnInlineModule(unittest.TestCase):
    """A `mod tests;` inside `mod a { ... }` is read from the file rustc reads for it: the module
    directory of the declaring file, then the inline modules' names, then `tests.rs` or
    `tests/mod.rs`, or the `#[path]` it names (A16, #433)."""

    tree = TheGuardJudgesAPlantedTree.tree
    declared = TheGuardReadsOutOfLineTestModules.declared
    SPELLING = TheGuardReadsOutOfLineTestModules.SPELLING
    NO_SHAPE = "let shape = 1;\n"

    def nested(self, names, leaf="tests", attributes="#[cfg(test)]\n"):
        """`mod a { mod b { ... #[cfg(test)] mod <leaf>; } }` for the inline `names`."""
        opened = "".join(f"mod {name} {{\n" for name in names)
        return f"{opened}{attributes}mod {leaf};\n" + "}\n" * len(names)

    def folders(self, own):
        """Where rustc looks for a module of `own`'s inline modules: the crate's `src` for a crate
        root, the file's own directory for any other module file."""
        return "" if own == "lib.rs" else "depth/"

    def test_a_declaration_inside_inline_modules_is_read_from_the_inline_path(self):
        for own in ("lib.rs", "depth.rs"):
            for names in (("a",), ("a", "b"), ("a", "b", "c")):
                folder = self.folders(own) + "".join(f"{name}/" for name in names)
                for leaf in ("tests.rs", "tests/mod.rs"):
                    declaration = self.nested(names)
                    pinned = self.declared(declaration, (folder + leaf, self.SPELLING), own=own)
                    self.assertEqual(unpinned(pinned), [], (own, names, leaf))
                    bare = self.declared(declaration, (folder + leaf, self.NO_SHAPE), own=own)
                    self.assertEqual(len(unpinned(bare)), 1, (own, names, leaf))

    def test_a_file_that_is_not_the_inline_path_is_not_read(self):
        for own in ("lib.rs", "depth.rs"):
            folder = self.folders(own)
            declaration = self.nested(("a", "b"))
            for wrong in ("tests.rs", "a/tests.rs", "b/tests.rs", "a/b/c/tests.rs"):
                root = self.declared(declaration, (folder + wrong, self.SPELLING), own=own)
                self.assertEqual(len(unpinned(root)), 1, (own, wrong))

    def test_a_declaration_with_a_path_attribute_is_read_from_the_path_below_the_inline_path(self):
        for own in ("lib.rs", "depth.rs"):
            folder = self.folders(own)
            for attributes in (
                '#[cfg(test)]\n#[path = "t.rs"]\n',
                '#[path = "t.rs"]\n#[cfg(test)]\n',
                '#[cfg(test)]\n#[path="t.rs"]\n',
            ):
                declaration = self.nested(("a", "b"), "tests", attributes)
                pinned = self.declared(declaration, (folder + "a/b/t.rs", self.SPELLING), own=own)
                self.assertEqual(unpinned(pinned), [], (own, attributes))
                bare = self.declared(declaration, (folder + "a/b/t.rs", self.NO_SHAPE), own=own)
                self.assertEqual(len(unpinned(bare)), 1, (own, attributes))
                for wrong in ("t.rs", "a/t.rs", "a/b/tests.rs"):
                    root = self.declared(declaration, (folder + wrong, self.SPELLING), own=own)
                    self.assertEqual(len(unpinned(root)), 1, (own, attributes, wrong))

    def test_the_attributes_of_the_inline_modules_decide_whether_the_file_is_a_test_module(self):
        file = ("a/tests.rs", self.SPELLING)
        inline = "mod a {\n#![cfg(test)]\n#[cfg(test)]\nmod tests;\n}\n"
        for declaration, read in (
            ("#[cfg(test)]\nmod a {\nmod tests;\n}\n", True),
            ("#[cfg(test)]\nmod a {\n#[cfg(test)]\nmod tests;\n}\n", True),
            ("mod a {\n#![cfg(test)]\nmod tests;\n}\n", True),
            (inline, True),
            ("mod a {\nmod tests;\n}\n", False),
            ("#[cfg(any())]\nmod a {\n#[cfg(test)]\nmod tests;\n}\n", False),
            ("#[cfg(not(test))]\nmod a {\n#[cfg(test)]\nmod tests;\n}\n", False),
            ("mod a {\n#![cfg(not(test))]\n#[cfg(test)]\nmod tests;\n}\n", False),
            ('#[cfg(feature = "slow")]\nmod a {\n#[cfg(test)]\nmod tests;\n}\n', False),
            ("mod a {\nconst X: u8 = 1;\n#[cfg(test)]\nfn tests() {}\n}\n", False),
        ):
            root = self.declared(declaration, file, own="lib.rs")
            self.assertEqual(unpinned(root) == [], read, declaration)

    def test_a_declaration_inside_a_function_body_is_not_read(self):
        declaration = "fn f() {\n#[cfg(test)]\nmod tests;\n}\n"
        root = self.declared(declaration, ("tests.rs", self.SPELLING), own="lib.rs")
        self.assertEqual(len(unpinned(root)), 1)

    def test_a_file_whose_own_inner_attribute_keeps_it_under_test_is_read(self):
        """The inner attributes that open a module's file are attributes of the module, so a
        `#![cfg(test)]` there makes it a test module, and the one declaration that reaches it is
        no rival of itself (#433)."""
        inner = "#![cfg(test)]\n"
        for own in ("lib.rs", "depth.rs"):
            for names in (("a",), ("a", "b")):
                folder = self.folders(own) + "".join(f"{name}/" for name in names)
                for attributes, leaf in (
                    ("", "tests.rs"),
                    ("", "tests/mod.rs"),
                    ('#[path = "t.rs"]\n', "t.rs"),
                    ("#[cfg(test)]\n", "tests.rs"),
                ):
                    declaration = self.nested(names, attributes=attributes)
                    case = (own, names, attributes, leaf)
                    spelled = (folder + leaf, inner + self.SPELLING)
                    self.assertEqual(
                        unpinned(self.declared(declaration, spelled, own=own)), [], case
                    )
                    bare = (folder + leaf, inner + self.NO_SHAPE)
                    self.assertEqual(
                        len(unpinned(self.declared(declaration, bare, own=own))), 1, case
                    )

    def test_a_declaration_whose_file_the_guard_cannot_choose_is_not_read(self):
        """A file is read only where rustc's choice is plain. Below an inline module that carries a
        `#[path]` the directory moves (the Reference's `thread_files` example); of two `#[path]`s
        rustc reads the first; a `cfg_attr` path under an option the guard does not know, or with
        a raw literal, is a choice it does not make. Each shape is refused."""
        for declaration, files in (
            (
                '#[path = "thread_files"]\nmod a {\n'
                '#[cfg(test)]\n#[path = "tests.rs"]\nmod tests;\n}\n',
                (("a/tests.rs", self.SPELLING), ("thread_files/tests.rs", self.NO_SHAPE)),
            ),
            (
                'mod a {\n#[cfg(test)]\n#[path = "one.rs"]\n#[path = "two.rs"]\nmod tests;\n}\n',
                (("a/two.rs", self.SPELLING),),
            ),
            (
                'mod a {\n#[cfg(test)]\n#[cfg_attr(unix, path = "real.rs")]\nmod tests;\n}\n',
                (("a/real.rs", self.SPELLING),),
            ),
            (
                'mod a {\n#[cfg(test)]\n#[cfg_attr(test, path = r"real.rs")]\nmod tests;\n}\n',
                (("a/tests.rs", self.SPELLING), ("a/real.rs", self.NO_SHAPE)),
            ),
        ):
            root = self.declared(declaration, *files, own="lib.rs")
            self.assertEqual(len(unpinned(root)), 1, declaration)
        plain = self.declared(self.nested(("a",)), ("a/tests.rs", self.SPELLING), own="lib.rs")
        self.assertEqual(unpinned(plain), [])


class TheGuardRefusesAMacroThatDeclaresATestModule(unittest.TestCase):
    """A `macro_rules!` body that declares a `cfg(test)` module is a source the text reader cannot
    expand, so the crate file holding it is refused by name instead of read as if the macro were
    not there (A17, #441)."""

    tree = TheGuardJudgesAPlantedTree.tree
    src = TheGuardJudgesAPlantedTree.src
    PINNED = 'const X: &str = "a whole depth";'

    def refused(self, text):
        """The refusals that name `src/more.rs`, in a tree whose one implementation is pinned."""
        root = self.src(self.tree(self.PINNED), text)
        return [line for line in unpinned(root) if "src/more.rs" in line]

    def macro(self, body):
        return "macro_rules! m {\n    ($($n:ident),*) => {\n" + body + "    };\n}\n"

    def test_a_macro_body_with_a_cfg_test_module_is_refused_by_its_file(self):
        for body in (
            "        #[cfg(test)]\n        mod tests;\n",
            "        #[cfg(test)]\n        mod tests {\n            const X: u8 = 1;\n        }\n",
            "        #[cfg(all(test, unix))]\n        mod tests;\n",
            '        #[cfg(any(test, feature = "slow"))]\n        pub mod tests;\n',
            "        #[cfg_attr(test, allow(dead_code))]\n        mod tests;\n",
            "        #[allow(dead_code)]\n        #[cfg(test)]\n        mod tests;\n",
            "        $(#[cfg(test)] mod $n;)*\n",
            "        const A: u8 = 1;\n        #[cfg(test)]\n        mod tests;\n",
        ):
            found = self.refused(self.macro(body))
            self.assertEqual(len(found), 1, body)
            self.assertIn("macro", found[0])

    def test_every_delimiter_and_depth_of_a_macro_body_is_read(self):
        inner = "#[cfg(test)]\nmod tests;\n"
        for text in (
            "macro_rules! m {\n    () => { " + inner + " };\n}\n",
            "macro_rules! m (\n    () => ( " + inner + " );\n);\n",
            "macro_rules! m [\n    () => [ " + inner + " ];\n];\n",
            "mod a {\n    macro_rules! m {\n        () => { " + inner + " };\n    }\n}\n",
            "macro_rules! m {\n    () => { mod a { " + inner + " } };\n}\n",
            "macro_rules! m {\n    () => {};\n    ($x:ident) => { " + inner + " };\n}\n",
        ):
            self.assertEqual(len(self.refused(text)), 1, text)

    def test_a_macro_body_without_a_cfg_test_module_is_not_refused(self):
        for text in (
            self.macro("        mod tests;\n"),
            self.macro("        #[cfg(not(unix))]\n        mod tests;\n"),
            self.macro("        #[cfg(test)]\n        fn check() {}\n"),
            self.macro("        #[cfg(test)]\n        const T: u8 = 1;\n"),
            "#[cfg(test)]\nmod tests {\n    const X: u8 = 1;\n}\n",
            "// macro_rules! m { () => { #[cfg(test)] mod tests; }; }\n",
            'const D: &str = "macro_rules! m { () => { #[cfg(test)] mod tests; }; }";\n',
            "macro_rules! m {\n    () => {};\n}\n#[cfg(test)]\nmod tests {}\n",
        ):
            self.assertEqual(self.refused(text), [], text)
        planted = self.macro("        #[cfg(test)]\n        mod tests;\n")
        self.assertEqual(len(self.refused(planted)), 1, planted)

    def test_the_refusal_names_the_crate_and_the_file(self):
        root = self.src(
            self.tree(self.PINNED), self.macro("        #[cfg(test)]\n        mod t;\n")
        )
        found = [line for line in unpinned(root) if "more.rs" in line]
        self.assertEqual(len(found), 1)
        self.assertTrue(found[0].startswith("demo (src/more.rs)"), found[0])
        self.assertEqual(len(unpinned(root)), 1)

    def test_an_inner_attribute_or_an_attribute_before_a_repetition_is_read(self):
        """`mod x { #![cfg(test)] }` carries its condition inside its braces, and an attribute
        before `$( ... )*` or `$( ... )?` reaches the module the repetition writes (#441)."""
        for body in (
            "        mod x {\n            #![cfg(test)]\n        }\n",
            "        mod x {\n            #![cfg(all(test))]\n        }\n",
            "        mod x {\n            #![cfg_attr(all(), cfg(test))]\n        }\n",
            "        $(mod $n {\n            #![cfg(test)]\n        })*\n",
            "        #[cfg(test)]\n        $(mod $n {})*\n",
            "        #[cfg(test)]\n        $(mod $n {})?\n",
            "        #[cfg_attr(all(), cfg(test))]\n        $(mod $n {})*\n",
            "        #[cfg(test)]\n        $(mod $n;)*\n",
        ):
            found = self.refused(self.macro(body))
            self.assertEqual(len(found), 1, body)
            self.assertIn("macro", found[0])

    def test_an_attribute_of_another_item_or_of_the_matcher_is_not_read(self):
        """An attribute is read back only to the previous item's end (`;` or `}`) or the delimiter
        that opens the body, past a `$(` that opens the module's own repetition: one on a `use`,
        on a function or in the matcher does not make the module a test module."""
        for text in (
            self.macro("        #[cfg(test)]\n        use std::fmt;\n        mod real {}\n"),
            self.macro("        #[cfg(test)]\n        fn helper() {}\n        mod real {}\n"),
            self.macro("        #[cfg(test)]\n        use std::fmt;\n        $(mod $n {})*\n"),
            self.macro("        #[cfg(test)]\n        fn helper() {}\n        $(mod $n {})*\n"),
            "macro_rules! m {\n    (#[cfg(test)] $x:ident) => {\n        mod real {}\n    };\n}\n",
            "macro_rules! m {\n    (#[cfg(test)] $x:ident) => {\n"
            "        $(mod $n {})*\n    };\n}\n",
        ):
            self.assertEqual(self.refused(text), [], text)
        planted = self.macro(
            "        #[cfg(test)]\n        fn f() {}\n        #[cfg(test)]\n        mod real {}\n"
        )
        self.assertEqual(len(self.refused(planted)), 1, planted)


class TheGuardRefusesAModuleFileThatAnotherDeclarationCompilesWithoutTest(unittest.TestCase):
    """A file a `#[cfg(test)] mod tests;` declares is not read when any declaration the guard can
    see also compiles it without `test`: a spelling there is production code (A18, #458)."""

    tree = TheGuardJudgesAPlantedTree.tree
    declared = TheGuardReadsOutOfLineTestModules.declared
    SPELLING = TheGuardReadsOutOfLineTestModules.SPELLING
    TEST = "#[cfg(test)]\nmod tests;\n"

    def judged(self, declaration, own="lib.rs", extra=()):
        folder = "" if own == "lib.rs" else "depth/"
        root = self.declared(declaration, (folder + "tests.rs", self.SPELLING), *extra, own=own)
        return len(unpinned(root))

    def test_a_module_declared_beside_one_with_not_test_is_refused(self):
        for own in ("lib.rs", "depth.rs"):
            for declaration in (
                self.TEST + "#[cfg(not(test))]\nmod tests;\n",
                "#[cfg(not(test))]\nmod tests;\n" + self.TEST,
                self.TEST + "#[cfg(not(test))]\n#[allow(dead_code)]\nmod tests;\n",
                self.TEST + "#[allow(dead_code)]\n#[cfg(not(test))]\nmod tests;\n",
                self.TEST + "#[cfg(not(test))]\npub mod tests;\n",
            ):
                self.assertEqual(self.judged(declaration, own), 1, (own, declaration))
            self.assertEqual(self.judged(self.TEST, own), 0, own)

    def test_a_module_a_path_declaration_names_beside_a_test_module_is_refused(self):
        for own, path in (("lib.rs", "tests.rs"), ("depth.rs", "depth/tests.rs")):
            for declaration in (
                f'#[path = "{path}"]\nmod prod;\n' + self.TEST,
                self.TEST + f'#[path = "{path}"]\nmod prod;\n',
                f'#[path="{path}"]\nmod prod;\n' + self.TEST,
                f'#[cfg(not(test))]\n#[path = "{path}"]\nmod prod;\n' + self.TEST,
                f'#[cfg_attr(not(test), path = "{path}")]\nmod prod;\n' + self.TEST,
                f'#[path = "{path}"]\n#[cfg(not(test))]\nmod prod;\n' + self.TEST,
            ):
                self.assertEqual(self.judged(declaration, own), 1, (own, declaration))

    def test_a_variant_that_compiles_the_file_without_test_is_refused(self):
        for rival in (
            "#[cfg_attr(not(test), allow(dead_code))]\nmod tests;\n",
            "#[cfg(any(not(test), unix))]\nmod tests;\n",
            '#[cfg(feature = "slow")]\nmod tests;\n',
            "#[cfg(all())]\nmod tests;\n",
            "mod tests;\n",
            "#[cfg_attr(test, cfg(any()))]\nmod tests;\n",
        ):
            self.assertEqual(self.judged(self.TEST + rival), 1, rival)

    def test_a_declaration_in_another_file_or_an_inline_module_counts(self):
        root = self.declared(self.TEST, ("depth/tests.rs", self.SPELLING), own="depth.rs")
        src = root / "crates" / "demo" / "src"
        (src / "lib.rs").write_text(
            'mod depth;\n#[cfg(not(test))]\n#[path = "depth/tests.rs"]\nmod prod;\n',
            encoding="utf-8",
        )
        self.assertEqual(len(unpinned(root)), 1)
        (src / "lib.rs").write_text(
            'mod depth;\nmod a {\n#[cfg(not(test))]\n#[path = "../depth/tests.rs"]\nmod prod;\n}\n',
            encoding="utf-8",
        )
        self.assertEqual(len(unpinned(root)), 1)

    def test_a_file_compiled_only_under_test_is_still_read(self):
        for own, path in (("lib.rs", "tests.rs"), ("depth.rs", "depth/tests.rs")):
            for declaration in (
                self.TEST + f'#[cfg(test)]\n#[path = "{path}"]\nmod again;\n',
                self.TEST + "#[cfg(not(test))]\nmod other;\n",
                self.TEST + '#[cfg(not(test))]\n#[path = "other.rs"]\nmod prod;\n',
            ):
                extra = (("other.rs", "let shape = 1;\n"), ("depth/other.rs", "let shape = 1;\n"))
                self.assertEqual(self.judged(declaration, own, extra), 0, (own, declaration))
            rival = self.TEST + "#[cfg(not(test))]\nmod tests;\n"
            self.assertEqual(self.judged(rival, own), 1, (own, rival))

    def test_a_rival_whose_path_the_guard_cannot_read_is_refused(self):
        """A declaration whose file the guard cannot name (an escaped or raw path literal, or one
        below an inline module whose `#[path]` moves the directory) may name any file, so unless
        its own attributes keep it under test it counts against every test module (#458)."""
        for own, path in (("lib.rs", "tests"), ("depth.rs", "depth/tests")):
            for literal in (f'"{path}\\x2ers"', f'r"{path}.rs"', f'r#"{path}.rs"#'):
                rival = f"#[path = {literal}]\nmod prod;\n"
                for declaration in (rival + self.TEST, self.TEST + rival):
                    self.assertEqual(self.judged(declaration, own), 1, (own, declaration))
                kept = self.TEST + f"#[cfg(test)]\n#[path = {literal}]\nmod again;\n"
                self.assertEqual(self.judged(kept, own), 0, (own, kept))
        moved = '#[path = "."]\nmod a {\n#[path = "tests.rs"]\nmod prod;\n}\n'
        self.assertEqual(self.judged(self.TEST + moved), 1, moved)

    def test_a_file_only_test_reaches_by_its_own_attribute_or_a_cfg_attr_path_is_read(self):
        """A file's own `#![cfg(test)]` is an attribute of every module that reaches it, and a
        declaration's one `cfg_attr(P, path = "...")` reaches the file it names only under P and
        the default file only without P, so neither makes a production rival (#458)."""
        inner = "#![cfg(test)]\n"
        for own, folder in (("lib.rs", ""), ("depth.rs", "depth/")):
            path = folder + "tests.rs"
            for declaration, spelling in (
                ("mod tests;\n", inner + self.SPELLING),
                (f'mod tests;\n#[path = "{path}"]\nmod prod;\n', inner + self.SPELLING),
                (self.TEST + f'#[cfg_attr(test, path = "{path}")]\nmod prod;\n', self.SPELLING),
            ):
                files = ((path, spelling), (folder + "prod.rs", "let shape = 1;\n"))
                root = self.declared(declaration, *files, own=own)
                self.assertEqual(unpinned(root), [], (own, declaration))
            plain = self.TEST + f'#[path = "{path}"]\nmod prod;\n'
            self.assertEqual(self.judged(plain, own), 1, (own, plain))
        binary = '#[cfg_attr(not(test), path = "prod.rs")]\nmod tests;\nfn main() {}\n'
        files = (("main.rs", binary), ("prod.rs", "let shape = 1;\n"))
        self.assertEqual(self.judged(self.TEST, "lib.rs", files), 0, binary)

    def test_a_cfg_attr_path_that_does_not_decide_the_file_alone_is_a_rival(self):
        """A `cfg_attr` gates its file only when it is the declaration's one attribute that names
        a path and names it directly: two of them (rustc reads the first that applies) or one
        nested in another `cfg_attr` may reach the file without `test`, so each is refused."""
        for rival in (
            '#[cfg_attr(test, path = "tests.rs")]\n#[cfg_attr(not(test), path = "tests.rs")]\n',
            '#[cfg_attr(test, path = "other.rs")]\n#[path = "tests.rs"]\n',
            '#[path = "tests.rs"]\n#[cfg_attr(test, path = "other.rs")]\n',
        ):
            declaration = self.TEST + rival + "mod prod;\n"
            extra = (("other.rs", "let shape = 1;\n"), ("prod.rs", "let shape = 1;\n"))
            self.assertEqual(self.judged(declaration, "lib.rs", extra), 1, rival)
        nested = (
            '#[cfg_attr(not(test), cfg_attr(feature = "slow", path = "prod.rs"))]\nmod tests;\n'
        )
        files = (("main.rs", nested + "fn main() {}\n"), ("prod.rs", "let shape = 1;\n"))
        self.assertEqual(self.judged(self.TEST, "lib.rs", files), 1, nested)

    def test_a_rival_in_a_binary_or_under_an_attribute_the_guard_cannot_read_is_refused(self):
        """A `src/bin` file is a crate root too, and a tool attribute (`#[rustfmt::cfg]`) that
        names `cfg` is no `cfg`: rustc does not interpret it, so the declaration is compiled in
        every configuration and the file it names is production code."""
        tool = self.TEST + '#[rustfmt::cfg]\n#[path = "tests.rs"]\nmod prod;\n'
        self.assertEqual(self.judged(tool), 1, tool)
        binary = (("bin/tool.rs", '#[path = "../tests.rs"]\nmod prod;\nfn main() {}\n'),)
        self.assertEqual(self.judged(self.TEST, "lib.rs", binary), 1, binary)
        self.assertEqual(self.judged(self.TEST), 0)

    def test_a_plain_path_in_another_file_names_only_the_file_it_spells(self):
        """`#[path = "prod.rs"] mod tests;` in `depth.rs` reads `prod.rs`, not the `tests.rs` the
        crate root's test module reads, so it is no rival of it."""
        other = (("depth.rs", '#[path = "prod.rs"]\nmod tests;\n'), ("prod.rs", "let shape = 1;\n"))
        self.assertEqual(self.judged(self.TEST + "mod depth;\n", "lib.rs", other), 0)
        rival = (("depth.rs", '#[path = "tests.rs"]\nmod prod;\n'),)
        self.assertEqual(self.judged(self.TEST + "mod depth;\n", "lib.rs", rival), 1)

    def test_a_rival_whose_attributes_the_guard_cannot_read_whole_is_refused(self):
        """`unsafe mod` parses, and rustc rejects it wherever it compiles it, so the guard cannot
        read its attributes back to the previous item's end: they count as compiling the file
        without `test`, and a `#[path]` among them may name any file, so every one is refused."""
        for own, path in (("lib.rs", "tests.rs"), ("depth.rs", "depth/tests.rs")):
            for rival in (
                "#[cfg(not(test))]\nunsafe mod tests;\n",
                f'#[cfg(not(test))]\n#[path = "{path}"]\nunsafe mod prod;\n',
            ):
                self.assertEqual(self.judged(self.TEST + rival, own), 1, (own, rival))
            self.assertEqual(self.judged(self.TEST, own), 0, own)


REFUSED_BY_A_MACRO = {
    "441": "macro_rules! body declares a cfg(test) module",
    "pass": "macro invocation passes a cfg(test) module",
    "meta": "macro declares a module under an attribute it passes in",
    "path": "macro declares a module whose path it names",
    "impl": "macro writes an implementation of Setting",
    "block": "block declares an out-of-line module",
    "include": "include! compiles another file's text",
}


class TheGuardReadsAnImplementationByToken(unittest.TestCase):
    """An implementation is found from rustc's tokens: a raw identifier, an alias, an attribute or
    a second item on its line are read, a string that holds the words is not, and one a macro
    writes is refused by its file (A19, #436)."""

    tree = TheGuardJudgesAPlantedTree.tree
    WIDTH = 'const SHAPE: &\'static str = "a whole width";'
    PINS = 'const X: &str = "a whole depth";\nconst Y: &str = "a whole width";\n'
    DEPTH = 'const X: &str = "a whole depth";\n'
    WIDE = 'demo::Wide (src/more.rs) "a whole width"'
    MACRO = f"demo (src/more.rs) {REFUSED_BY_A_MACRO['impl']}"

    def spellings(self):
        """(case, `src/more.rs`, what `lib.rs` adds, kind, lines refused either way). A "read"
        member is refused only when unpinned, a "macro" member always, by its file; the lines
        refused
        either way are an implementation of another trait that a shadowing name binds, read as a
        `Setting` one with no shape (a disclosed false refusal)."""
        body = f" {{\n    {self.WIDTH}\n}}\n"
        plain = "impl Setting for Wide" + body
        found = [
            ("plain", plain, "", "read", []),
            ("raw trait", "impl r#Setting for Wide" + body, "", "read", []),
            ("raw type", "impl Setting for r#Wide" + body, "", "read", []),
            ("path", "impl crate::settings::Setting for Wide" + body, "", "read", []),
            (
                "alias",
                "use crate::settings::Setting as Shaped;\nimpl Shaped for Wide" + body,
                "",
                "read",
                [],
            ),
            (
                "alias in a group",
                "use crate::settings::{Other, Setting as Shaped};\nimpl Shaped for Wide" + body,
                "",
                "read",
                [],
            ),
            (
                "alias chain",
                "use crate::settings::Setting as First;\nuse self::First as Second;\n"
                "impl Second for Wide" + body,
                "",
                "read",
                [],
            ),
            (
                "alias another file binds",
                "use crate::Shaped;\nimpl Shaped for Wide" + body,
                "pub use settings::Setting as Shaped;\n",
                "read",
                [],
            ),
            (
                "raw alias",
                "use crate::settings::Setting as r#Shaped;\nimpl r#Shaped for Wide" + body,
                "",
                "read",
                [],
            ),
            ("attribute on the line", "#[allow(dead_code)] " + plain, "", "read", []),
            ("second item on the line", "pub struct Wide {} " + plain, "", "read", []),
            (
                "generic with an arrow",
                "impl<F: Fn() -> u8> Setting for Wide<F>" + body,
                "",
                "read",
                [],
            ),
            ("where clause", "impl Setting for Wide where u8: Copy" + body, "", "read", []),
            (
                "a string holds the words",
                'pub const D: &str = "\nimpl Setting for Ghost {\n";\n' + plain,
                "",
                "read",
                [],
            ),
            (
                "an inherent and another trait in a macro",
                "macro_rules! make {\n    ($t:ident) => {\n        impl $t {}\n"
                "        impl Display for $t {}\n    };\n}\n" + plain,
                "",
                "read",
                [],
            ),
            (
                "one-line macro",
                "macro_rules! make { ($t:ident) => { impl Setting for $t { "
                f"{self.WIDTH} }} }}; }}\n"
                "make!(Wide);\n",
                "",
                "macro",
                [],
            ),
            (
                "macro at a line start",
                "macro_rules! make {\n    ($t:ident) => {\n        impl Setting for $t {\n"
                f"            {self.WIDTH}\n        }}\n    }};\n}}\nmake!(Wide);\n",
                "",
                "macro",
                [],
            ),
            (
                "macro through $crate",
                "macro_rules! make {\n    ($t:ident) => {\n"
                f"        impl $crate::settings::Setting for $t {{ {self.WIDTH} }}\n    }};\n}}\n",
                "",
                "macro",
                [],
            ),
            (
                "macro trait metavariable",
                "macro_rules! make {\n    ($tr:path, $t:ident) => {\n"
                f"        impl $tr for $t {{ {self.WIDTH} }}\n    }};\n}}\nmake!(Setting, Wide);\n",
                "",
                "macro",
                [],
            ),
            (
                "macro through an alias",
                "use crate::settings::Setting as Shaped;\nmacro_rules! make {\n"
                f"    ($t:ident) => {{ impl Shaped for $t {{ {self.WIDTH} }} }};\n}}\n",
                "",
                "macro",
                [],
            ),
            ("invocation passes it", "wrap! {\n" + plain + "}\n", "", "macro", []),
            (
                "macro trait path a repetition",
                "macro_rules! make {\n    ($($p:ident)::+ ; $t:ident) => {\n"
                f"        impl $($p)::+ for $t {{ {self.WIDTH} }}\n    }};\n}}\n"
                "make!(crate::settings::Setting; Wide);\n",
                "",
                "macro",
                [],
            ),
            (
                "macro trait a repetition of tokens",
                "macro_rules! make {\n    ($($p:tt)*) => {\n"
                f"        impl $($p)* for Wide {{ {self.WIDTH} }}\n    }};\n}}\n"
                "make!(crate::settings::Setting);\n",
                "",
                "macro",
                [],
            ),
            (
                "macro impl keyword passed in",
                "use crate::settings::Setting;\nmacro_rules! make {\n    ($k:tt) => {\n"
                f"        $k Setting for Wide {{ {self.WIDTH} }}\n    }};\n}}\nmake!(impl);\n",
                "",
                "macro",
                [],
            ),
            (
                "macro impl keyword and trait passed in",
                "macro_rules! make {\n    ($k:tt, $tr:path) => {\n"
                f"        $k $tr for Wide {{ {self.WIDTH} }}\n    }};\n}}\n"
                "make!(impl, crate::settings::Setting);\n",
                "",
                "macro",
                [],
            ),
            (
                "macro for passed in",
                "use crate::settings::Setting;\nmacro_rules! make {\n    ($f:tt) => {\n"
                f"        impl Setting $f Wide {{ {self.WIDTH} }}\n    }};\n}}\nmake!(for);\n",
                "",
                "macro",
                [],
            ),
            (
                "invocation passes an impl head",
                "use crate::settings::Setting;\nmacro_rules! make {\n    ($($h:tt)*) => {\n"
                f"        $($h)* Wide {{ {self.WIDTH} }}\n    }};\n}}\nmake!(impl Setting for);\n",
                "",
                "macro",
                [],
            ),
            (
                "a loop and a bound in a macro",
                "macro_rules! each {\n    ($e:expr) => {\n        for x in $e {\n"
                "            let _ = x;\n        }\n"
                "        fn g<F>(_: F) where F: for<'a> Fn(&'a u8) {}\n"
                "    };\n}\n" + plain,
                "",
                "read",
                [],
            ),
            (
                "another trait with its impl passed in",
                "macro_rules! make {\n    ($k:tt) => {\n        $k Other for Wide {}\n    };\n}\n"
                + plain,
                "",
                "read",
                [],
            ),
            (
                "shadowing alias in a module",
                "use crate::settings::Setting as Shaped;\nimpl Shaped for Wide"
                + body
                + "mod other {\n    pub trait Shaped {}\n    impl Shaped for super::Narrow {}\n}\n",
                "",
                "read",
                ["demo::Narrow (src/more.rs) None"],
            ),
            (
                "shadowing the trait's own name",
                plain + "mod other {\n    use other::Thing as Setting;\n"
                "    impl Setting for Narrow {}\n}\n",
                "",
                "read",
                ["demo::Narrow (src/more.rs) None"],
            ),
            (
                "shape only in a comment",
                "impl Setting for Wide {\n    // "
                + self.WIDTH
                + "\n    const SHAPE: &'static str = W;\n}\n",
                "",
                "none",
                ["demo::Wide (src/more.rs) None"],
            ),
        ]
        return found

    def test_every_spelling_is_read_or_refused_as_written(self):
        """Each member's outcome is written by its kind, never by the guard: pinned and unpinned,
        a read implementation is refused only unpinned, a macro's always and by its file."""
        found, expected, judged = {}, {}, []
        for case, text, lib, kind, either in self.spellings():
            for pinned in (True, False):
                root = self.tree(self.PINS if pinned else self.DEPTH)
                src = root / "crates" / "demo" / "src"
                (src / "more.rs").write_text(text, encoding="utf-8")
                if lib:
                    (src / "lib.rs").write_text("mod depth;\n" + lib, encoding="utf-8")
                expected[case, pinned] = sorted(
                    {
                        "read": [] if pinned else [self.WIDE],
                        "macro": [self.MACRO],
                        "none": [],
                    }[kind]
                    + either
                )
                found[case, pinned] = sorted(unpinned(root))
                judged.append((case, pinned))
        self.assertEqual(found, expected, "each member's refusal lines against its kind's outcome")
        examined("implementation spelling(s) judged, pinned and unpinned", judged)


class TheGuardReadsOnlyTheItemsRustcCompilesUnderTest(unittest.TestCase):
    """A pin counts only in an item rustc compiles under `--cfg test`. A generated population of
    item-level `cfg` shapes inside a compiled test module is judged by rustc itself: each member's
    ORACLE TWIN gives the item under test a body that does not type-check, so rustc exits 0 only
    when it strips the item (A20, #449)."""

    PRELUDE = (
        "pub trait Setting {\n    const SHAPE: &'static str;\n}\npub struct Depth;\n"
        'impl Setting for Depth {\n    const SHAPE: &\'static str = "a whole depth";\n}\n'
    )
    ATTRIBUTES = (
        "",
        "#[cfg(any())]\n",
        "#[cfg(all())]\n",
        "#[cfg(test)]\n",
        "#[cfg(not(test))]\n",
        "#[cfg(true)]\n",
        "#[cfg(false)]\n",
        "#[cfg(any(test, false))]\n",
        "#[cfg(all(test, not(test)))]\n",
        "#[cfg(not(any()))]\n",
        "#[allow(dead_code)]\n#[cfg(any())]\n",
        "#[cfg(any())]\n#[allow(dead_code)]\n",
        "/// d\n#[cfg(not(test))]\n",
        "#[cfg(test)]\n#[cfg(not(test))]\n",
        '#[cfg(feature = "slow")]\n',
        '#[cfg(not(feature = "slow"))]\n',
        "#[cfg(unix)]\n",
        "#[cfg_attr(test, cfg(any()))]\n",
        "#[cfg_attr(any(), cfg(any()))]\n",
        "#[cfg_attr(test, allow(dead_code))]\n",
    )
    UNDECIDED = re.compile(r"feature|unix|cfg_attr")
    ITEMS = (
        ("const", 'const X: &str = "a whole depth";', 'const X: u8 = "a whole depth";'),
        ("static", 'static X: &str = "a whole depth";', 'static X: u8 = "a whole depth";'),
        (
            "fn",
            'fn x() -> &\'static str {\n    "a whole depth"\n}',
            'fn x() -> u8 {\n    "a whole depth"\n}',
        ),
        (
            "test",
            '#[test]\nfn x() {\n    assert_eq!("a whole depth".len(), 13);\n}',
            '#[test]\nfn x() {\n    let _: u8 = "a whole depth";\n}',
        ),
        (
            "if-else",
            'const X: &str = if true { "x" } else { "a whole depth" };',
            'const X: u8 = if true { 0 } else { "a whole depth" };',
        ),
    )

    def members(self):
        """(case, member source, oracle twin source, decided): the item under test at module
        level of the test module, in an inline module under an outer or an inner attribute, in a
        second `cfg(test)` module, as a statement and as an associated constant."""
        found = []
        for attribute in self.ATTRIBUTES:
            inner = attribute.replace("#[", "#![").replace("/// ", "//! ")
            decided = not self.UNDECIDED.search(attribute)
            places = [
                (kind, f"{attribute}{item}", f"{attribute}{twin}")
                for kind, item, twin in self.ITEMS
            ]
            item, twin = self.ITEMS[0][1:]
            places += [
                (
                    "nested",
                    f"{attribute}mod inner {{\n{item}\n}}",
                    f"{attribute}mod inner {{\n{twin}\n}}",
                ),
                ("inner", f"mod inner {{\n{inner}{item}\n}}", f"mod inner {{\n{inner}{twin}\n}}"),
                (
                    "deeper",
                    f"#[cfg(test)]\nmod deeper {{\n{attribute}{item}\n}}",
                    f"#[cfg(test)]\nmod deeper {{\n{attribute}{twin}\n}}",
                ),
                (
                    "statement",
                    f'fn holder() {{\n{attribute}let _x: &str = "a whole depth";\n}}',
                    f'fn holder() {{\n{attribute}let _x: u8 = "a whole depth";\n}}',
                ),
                (
                    "associated",
                    "struct Holder;\nimpl Holder {\n"
                    f'{attribute}const X: &\'static str = "a whole depth";\n}}',
                    "struct Holder;\nimpl Holder {\n"
                    f'{attribute}const X: u8 = "a whole depth";\n}}',
                ),
            ]
            for place, member, twin_text in places:
                wrap = "#[cfg(test)]\nmod tests {\n{}\n}\n"
                found.append(
                    (
                        f"{place} {attribute!r}",
                        self.PRELUDE + wrap.replace("{}", member),
                        self.PRELUDE + wrap.replace("{}", twin_text),
                        decided,
                    )
                )
        return found

    def oracle(self, scratch, members):
        """Each twin through rustc, serially, one at a time: "stripped" when it exits 0, "compiled"
        when it fails with E0308 (the twin's type error), and the oracle refuses any other answer.
        The readings are written to a file before the guard reads any member."""
        rustc = shutil.which("rustc")
        if rustc is None:
            self.fail("rustc is not on PATH, so the item oracle cannot run: this test never skips")
        (scratch / "o").mkdir()
        (scratch / "t").mkdir()
        readings = []
        for index, (case, _, twin, _) in enumerate(members):
            source = scratch / "t" / f"{index}.rs"
            source.write_text(twin, encoding="utf-8")
            command = [rustc, "--edition", "2021", "--test", "--crate-type", "lib"]
            command += ["--emit=metadata", "-o", str(scratch / "o" / f"{index}.rmeta"), str(source)]
            run = subprocess.run(command, capture_output=True, text=True, check=False)
            if run.returncode == 0:
                readings.append("stripped")
            elif "E0308" in run.stderr:
                readings.append("compiled")
            else:
                self.fail(
                    f"{case}: rustc answered neither: rc {run.returncode}: {run.stderr[:400]}"
                )
        path = scratch / "readings.json"
        path.write_text(json.dumps(readings), encoding="utf-8")
        return path

    def test_a_pin_counts_only_in_an_item_rustc_compiles_under_test(self):
        """A member rustc strips must be refused; a member rustc compiles whose attributes
        `rustc_keeps` decides must be pinned; one it does not decide is refused, and where rustc
        compiles it that refusal is a false refusal, disclosed by count."""
        members = self.members()
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        scratch = Path(directory.name)
        readings = json.loads(self.oracle(scratch, members).read_text(encoding="utf-8"))
        control = {case: reading for (case, _, _, _), reading in zip(members, readings)}
        self.assertEqual(control["const ''"], "compiled", "the oracle's compiled control")
        self.assertEqual(
            control["const '#[cfg(any())]\\n'"], "stripped", "the oracle's stripped control"
        )
        wrong, judged, disclosed = [], [], []
        for index, ((case, member, _, decided), reading) in enumerate(zip(members, readings)):
            root = scratch / f"m{index}"
            (root / "crates" / "demo" / "src").mkdir(parents=True)
            (root / "scripts" / "mutation-rows.d").mkdir(parents=True)
            (root / "crates" / "demo" / "src" / "lib.rs").write_text(member, encoding="utf-8")
            refused = "demo::Depth " in " ".join(unpinned(root))
            judged.append(case)
            if reading == "stripped" and not refused:
                wrong.append(f"{case}: a pin is read from an item rustc strips")
            elif reading == "compiled" and decided and refused:
                wrong.append(f"{case}: refused, and rustc compiles it where rustc_keeps decides")
            elif not decided and not refused:
                wrong.append(f"{case}: an item rustc_keeps does not decide is read")
            elif reading == "compiled" and refused:
                disclosed.append(case)
        self.assertEqual(wrong, [], f"{len(wrong)} of {len(members)} member(s)")
        examined("item member(s) judged against rustc", judged)
        print(
            f"disclosed {len(disclosed)} false refusal(s): "
            "rustc compiles an item rustc_keeps does not decide"
        )

    FILE_KINDS = ("own", "tests", "module")

    def file_kinds(self):
        """(case, kind, member files, twin files, twin root), the paths relative to the crate: the
        item under test in each kind of file the guard reads a pin from, the implementation's own
        file's test module, a file under `tests/` (a crate root rustc compiles with `--test`) and
        an out-of-line test module's file, under an attribute rustc keeps and one it strips."""
        item, twin = self.ITEMS[0][1:]
        found = []
        for attribute in ("", "#[cfg(test)]\n", "#[cfg(any())]\n", "#[cfg(not(test))]\n"):
            wrap = "#[cfg(test)]\nmod tests {\n{}\n}\n"
            declared = self.PRELUDE + "#[cfg(test)]\nmod tests;\n"
            shapes = {
                "own": (
                    {"src/lib.rs": self.PRELUDE + wrap.replace("{}", attribute + item)},
                    {"src/lib.rs": self.PRELUDE + wrap.replace("{}", attribute + twin)},
                    "src/lib.rs",
                ),
                "tests": (
                    {"src/lib.rs": self.PRELUDE, "tests/pin.rs": f"{attribute}{item}\n"},
                    {"tests/pin.rs": f"{attribute}{twin}\n"},
                    "tests/pin.rs",
                ),
                "module": (
                    {"src/lib.rs": declared, "src/tests.rs": f"{attribute}{item}\n"},
                    {"src/lib.rs": declared, "src/tests.rs": f"{attribute}{twin}\n"},
                    "src/lib.rs",
                ),
            }
            for kind in self.FILE_KINDS:
                files, twins, root = shapes[kind]
                found.append((f"{kind} {attribute!r}", kind, files, twins, root))
        return found

    def test_a_pin_counts_only_in_a_compiled_item_of_every_file_kind(self):
        """A pin in a test file or an out-of-line test module's file counts only in an item rustc
        compiles under `--cfg test`, as one in the implementation's own test module does. Each
        member's twin tree is compiled by rustc from its root, serially, before the guard reads any
        member: a stripped item must be refused and a compiled one pinned, in every kind."""
        members = self.file_kinds()
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        scratch = Path(directory.name)
        rustc = shutil.which("rustc")
        if rustc is None:
            self.fail("rustc is not on PATH, so the item oracle cannot run: this test never skips")
        (scratch / "o").mkdir()
        readings = []
        for index, (case, _, _, twins, root) in enumerate(members):
            for name, text in twins.items():
                (scratch / "t" / str(index) / name).parent.mkdir(parents=True, exist_ok=True)
                (scratch / "t" / str(index) / name).write_text(text, encoding="utf-8")
            command = [rustc, "--edition", "2021", "--test", "--crate-type", "lib"]
            command += ["--emit=metadata", "-o", str(scratch / "o" / f"{index}.rmeta")]
            command.append(str(scratch / "t" / str(index) / root))
            run = subprocess.run(command, capture_output=True, text=True, check=False)
            if run.returncode == 0:
                readings.append("stripped")
            elif "E0308" in run.stderr:
                readings.append("compiled")
            else:
                self.fail(
                    f"{case}: rustc answered neither: rc {run.returncode}: {run.stderr[:400]}"
                )
        (scratch / "readings.json").write_text(json.dumps(readings), encoding="utf-8")
        seen = {(kind, reading) for (_, kind, _, _, _), reading in zip(members, readings)}
        self.assertEqual(
            seen,
            {(kind, reading) for kind in self.FILE_KINDS for reading in ("compiled", "stripped")},
            "the oracle reads a compiled and a stripped member of every file kind",
        )
        wrong, judged = [], []
        for index, ((case, _, files, _, _), reading) in enumerate(zip(members, readings)):
            root = scratch / f"k{index}"
            (root / "scripts" / "mutation-rows.d").mkdir(parents=True)
            for name, text in files.items():
                (root / "crates" / "demo" / name).parent.mkdir(parents=True, exist_ok=True)
                (root / "crates" / "demo" / name).write_text(text, encoding="utf-8")
            refused = "demo::Depth " in " ".join(unpinned(root))
            judged.append(case)
            if reading == "stripped" and not refused:
                wrong.append(f"{case}: a pin is read from an item rustc strips")
            elif reading == "compiled" and refused:
                wrong.append(f"{case}: refused, and rustc compiles the item that pins it")
        self.assertEqual(wrong, [], f"{len(wrong)} of {len(members)} member(s)")
        examined("file-kind member(s) judged against rustc", judged)

    def cargo_members(self):
        """(case, files, decided): a pin in a file under `tests/`, which cargo and rustc compile
        only from a crate root (`tests/<name>.rs`, `tests/<dir>/main.rs`) and through each `mod`
        a compiled file declares: through a declaration under each attribute, in a subdirectory,
        below an inline module, under a file's own inner attribute, and in a file no root
        declares. `PIN` marks the pin's place; paths are relative to the crate."""
        found = []
        for attribute in (
            "",
            "#[cfg(test)]\n",
            "#[cfg(any())]\n",
            "#[cfg(not(test))]\n",
            '#[cfg(feature = "slow")]\n',
            "#[cfg_attr(test, allow(dead_code))]\n",
        ):
            decided = not self.UNDECIDED.search(attribute)
            found += [
                (
                    f"subdirectory {attribute!r}",
                    {"tests/a.rs": f"{attribute}mod support;\n", "tests/support/mod.rs": "PIN"},
                    decided,
                ),
                (
                    f"nested file {attribute!r}",
                    {
                        "tests/a.rs": "mod support;\n",
                        "tests/support/mod.rs": f"{attribute}mod deep;\n",
                        "tests/support/deep.rs": "PIN",
                    },
                    decided,
                ),
                (
                    f"main.rs module {attribute!r}",
                    {
                        "tests/suite/main.rs": f"{attribute}mod helpers;\n",
                        "tests/suite/helpers.rs": "PIN",
                    },
                    decided,
                ),
                (
                    f"below an inline module {attribute!r}",
                    {
                        "tests/a.rs": f"{attribute}mod outer {{\n    mod inner;\n}}\n",
                        "tests/outer/inner.rs": "PIN",
                    },
                    decided,
                ),
            ]
        for inner in ("", "#![cfg(test)]\n", "#![cfg(any())]\n", "#![cfg(not(test))]\n"):
            found += [
                (f"main.rs root {inner!r}", {"tests/suite/main.rs": f"{inner}PIN"}, True),
                (f"a root's inner attribute {inner!r}", {"tests/pin.rs": f"{inner}PIN"}, True),
                (
                    f"a module's inner attribute {inner!r}",
                    {"tests/a.rs": "mod support;\n", "tests/support/mod.rs": f"{inner}PIN"},
                    True,
                ),
                (
                    f"a root's inner attribute over its module {inner!r}",
                    {"tests/a.rs": f"{inner}mod support;\n", "tests/support/mod.rs": "PIN"},
                    True,
                ),
            ]
        return found + [
            (
                "undeclared in a subdirectory",
                {"tests/a.rs": "pub fn f() {}\n", "tests/common/mod.rs": "PIN"},
                True,
            ),
            (
                "undeclared beside main.rs",
                {"tests/suite/main.rs": "pub fn f() {}\n", "tests/suite/other.rs": "PIN"},
                True,
            ),
            (
                "declared through a path attribute",
                {"tests/a.rs": '#[path = "x/y.rs"]\nmod m;\n', "tests/x/y.rs": "PIN"},
                False,
            ),
        ]

    def test_a_tests_file_counts_only_where_cargo_and_rustc_compile_it(self):
        """#449: a file under `tests/` is compiled only as cargo's test-target discovery and rustc
        reach it, from a crate root and through each `mod` a compiled file declares. Each member's
        twin tree is compiled by rustc from every root, serially, before the guard reads any
        member: a file no compiled declaration reaches must be refused, one reached where
        `rustc_keeps` decides must be pinned, and one it does not decide is refused, disclosed by
        count where rustc compiles it."""
        members = self.cargo_members()
        item, twin = self.ITEMS[0][1:]
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        scratch = Path(directory.name)
        rustc = shutil.which("rustc")
        if rustc is None:
            self.fail("rustc is not on PATH, so the item oracle cannot run: this test never skips")
        (scratch / "o").mkdir()
        readings = []
        for index, (case, files, _) in enumerate(members):
            base = scratch / "t" / str(index)
            for name, text in files.items():
                (base / name).parent.mkdir(parents=True, exist_ok=True)
                (base / name).write_text(text.replace("PIN", twin + "\n"), encoding="utf-8")
            answers = []
            for root in sorted([*base.glob("tests/*.rs"), *base.glob("tests/*/main.rs")]):
                command = [rustc, "--edition", "2021", "--test", "--crate-type", "lib"]
                command += ["--emit=metadata", "-o", str(scratch / "o" / f"{index}.rmeta")]
                run = subprocess.run(
                    command + [str(root)], capture_output=True, text=True, check=False
                )
                if run.returncode and "E0308" not in run.stderr:
                    self.fail(
                        f"{case}: rustc answered neither: rc {run.returncode}: {run.stderr[:400]}"
                    )
                answers.append(run.returncode)
            readings.append("compiled" if any(answers) else "stripped")
        (scratch / "readings.json").write_text(json.dumps(readings), encoding="utf-8")
        self.assertEqual(set(readings), {"compiled", "stripped"}, "the oracle reads both")
        wrong, judged, disclosed = [], [], []
        for index, ((case, files, decided), reading) in enumerate(zip(members, readings)):
            root = scratch / f"c{index}"
            (root / "scripts" / "mutation-rows.d").mkdir(parents=True)
            files = {"src/lib.rs": self.PRELUDE, **files}
            for name, text in files.items():
                (root / "crates" / "demo" / name).parent.mkdir(parents=True, exist_ok=True)
                (root / "crates" / "demo" / name).write_text(
                    text.replace("PIN", item + "\n"), encoding="utf-8"
                )
            refused = "demo::Depth " in " ".join(unpinned(root))
            judged.append(case)
            if reading == "stripped" and not refused:
                wrong.append(f"{case}: a pin is read from a file rustc does not compile")
            elif reading == "compiled" and decided and refused:
                wrong.append(
                    f"{case}: refused, and rustc compiles the file where rustc_keeps decides"
                )
            elif not decided and not refused:
                wrong.append(f"{case}: a file only an undecided declaration reaches is read")
            elif reading == "compiled" and refused:
                disclosed.append(case)
        self.assertEqual(wrong, [], f"{len(wrong)} of {len(members)} member(s)")
        examined("tests/ member(s) judged against rustc", judged)
        print(f"disclosed {len(disclosed)} false refusal(s): {sorted(disclosed)}")


class TheGuardRefusesASpellingItDoesNotExpand(unittest.TestCase):
    """Each of #535's spellings fails open unless it is read: an attribute a macro passes in, a
    macro that passes a module through, a `#[path]` rival a macro or a block declares, and
    `include!`. Each is refused by its file, and the same trees without the plant read no refusal
    (A21, #535)."""

    tree = TheGuardJudgesAPlantedTree.tree
    src = TheGuardJudgesAPlantedTree.src
    PINNED = TheGuardRefusesAMacroThatDeclaresATestModule.PINNED
    refused = TheGuardRefusesAMacroThatDeclaresATestModule.refused
    macro = TheGuardRefusesAMacroThatDeclaresATestModule.macro

    def line(self, reason):
        return [f"demo (src/more.rs) {REFUSED_BY_A_MACRO[reason]}"]

    def test_an_attribute_a_macro_passes_in_is_refused_by_its_file(self):
        for text in (
            "macro_rules! m {\n    ($a:meta) => {\n        #[$a]\n        mod prod;\n    };\n}\n",
            "macro_rules! m {\n    ($(#[$m:meta])*) => {\n        $(#[$m])*\n        mod tests;\n"
            "    };\n}\n",
            "macro_rules! m {\n    ($a:meta) => {\n        mod x {\n            #![$a]\n        }\n"
            "    };\n}\n",
            "macro_rules! m {\n    ($p:meta) => {\n        #[cfg($p)]\n"
            "        mod x {}\n    };\n}\n",
            self.macro("        $(#[$m:meta])*\n        mod tests;\n"),
        ):
            self.assertEqual(self.refused(text), self.line("meta"), text)

    def test_a_macro_that_passes_a_module_through_is_refused_by_its_file(self):
        keep = "macro_rules! keep {\n    ($($t:tt)*) => { $($t)* };\n}\n"
        for text, reason in (
            (keep + "keep! {\n    #[cfg(test)]\n    mod tests;\n}\n", "pass"),
            (keep + "keep!(#[cfg(test)] mod tests {});\n", "pass"),
            ("macro_rules! m {\n    () => {};\n}\nm!{ #[cfg(test)] mod tests; }\n", "pass"),
            (
                keep
                + 'keep! {\n    #[cfg(not(test))]\n    #[path = "tests.rs"]\n    mod prod;\n}\n',
                "path",
            ),
        ):
            self.assertEqual(self.refused(text), self.line(reason), text)

    def test_a_path_rival_a_macro_declares_is_refused_by_its_file(self):
        for attributes in (
            '#[cfg(not(test))]\n        #[path = "tests.rs"]\n',
            '#[path = "tests.rs"]\n',
            '#[r#path = "tests.rs"]\n',
        ):
            text = "macro_rules! rival {\n    () => {\n        "
            text += f"{attributes}        mod prod;\n    }};\n}}\n"
            self.assertEqual(self.refused(text), self.line("path"), text)

    def test_a_block_scoped_declaration_is_refused_by_its_file(self):
        """A limit by design (SPEC-192 section 16): rustc's block-scope module paths are not
        modelled, so an out-of-line module in a block that may compile without `test` refuses its
        file, and one `rustc_keeps` proves removed without `test` does not."""
        for text in (
            'fn f() {\n    #[path = "tests.rs"]\n    mod prod;\n}\n',
            "fn f() {\n    #[cfg(not(test))]\n    mod tests;\n}\n",
            "const _: () = {\n    mod prod;\n};\n",
        ):
            self.assertEqual(self.refused(text), self.line("block"), text)
        for text in (
            "fn f() {\n    #[cfg(test)]\n    mod tests;\n}\n",
            "fn f() {\n    mod inline {}\n}\n",
        ):
            self.assertEqual(self.refused(text), [], text)

    def test_an_include_is_refused_by_its_file(self):
        """A limit by design (SPEC-192 section 16): the guard does not follow `include!`, so a
        crate file that holds one is refused."""
        for text in (
            'mod prod {\n    include!("tests.rs");\n}\n',
            'include!("tests.rs");\n',
            'mod prod {\n    std::include!("tests.rs");\n}\n',
        ):
            self.assertEqual(self.refused(text), self.line("include"), text)
        self.assertEqual(self.refused('const S: &str = include_str!("tests.rs");\n'), [])

    def test_an_include_a_use_imports_is_refused_by_its_file(self):
        """#535: a `use` that imports `include` under any name compiles another file's text
        through that name, so its crate file is refused as a spelled `include!` is; so is a `use`
        whose path a macro passes in, and a macro name a macro passes in (`$m!`), since either may
        be `include`. A `use` of another name, a field or an argument named `include` import
        nothing, and each such tree reads 0 refusals while its implementation is examined."""
        for text in (
            'mod prod {\n    use core::include as pull;\n    pull!("tests.rs");\n}\n',
            'mod prod {\n    use std::include as inc;\n    inc!("tests.rs");\n}\n',
            "use std::include;\n",
            "use std::{include as x};\n",
            "use std::{fmt, include as x};\n",
            "pub use ::core::r#include as pull;\n",
            "macro_rules! bring {\n    ($m:ident) => {\n        use core::$m as pull;\n    };\n}\n",
            'macro_rules! call {\n    ($m:ident) => {\n        $m!("tests.rs");\n    };\n}\n'
            "call!(include);\n",
        ):
            self.assertEqual(self.refused(text), self.line("include"), text)
        for text in (
            "use std::fs::read_to_string as pull;\n",
            "pub struct Filter {\n    pub include: bool,\n}\n",
            "macro_rules! log {\n    ($e:expr) => {\n        let _ = (include, $e);\n    };\n}\n",
            "macro_rules! bring {\n    () => {\n        use $crate::Wide as W;\n    };\n}\n",
            "fn g(include: bool) -> bool {\n    include\n}\n",
        ):
            root = self.src(self.tree(self.PINNED), text)
            self.assertEqual(len(implementations(root)), 1, text)
            self.assertEqual(unpinned(root), [], text)

    def test_the_trees_without_a_plant_read_no_refusal(self):
        """The control: the planted tree with nothing planted, and with each plant's harmless
        neighbour, reads 0 refusals, while its one implementation is examined."""
        root = self.src(self.tree(self.PINNED), "pub fn f() {}\n")
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])
        for text in (
            "macro_rules! m {\n    () => {\n        mod real {}\n    };\n}\n",
            "macro_rules! keep {\n    ($($t:tt)*) => { $($t)* };\n}\nkeep!(fn g() {});\n",
            "fn f() {\n    mod inline {}\n}\n",
            'const S: &str = include_str!("tests.rs");\n',
        ):
            self.assertEqual(unpinned(self.src(self.tree(self.PINNED), text)), [], text)


class TheGuardReadsATreeItOverRefused(unittest.TestCase):
    """#536: a module a macro declares that `rustc_keeps` proves is no test-only module, a module
    beside a decoy in the other directory, and a `cfg_attr` path below an inline module are read as
    rustc reads them, while every #441 and #535 arm still refuses beside them (A22)."""

    tree = TheGuardJudgesAPlantedTree.tree
    src = TheGuardJudgesAPlantedTree.src
    declared = TheGuardReadsOutOfLineTestModules.declared
    PINNED = TheGuardRefusesAMacroThatDeclaresATestModule.PINNED
    SPELLING = TheGuardReadsOutOfLineTestModules.SPELLING
    NO_SHAPE = "let shape = 1;\n"
    refused = TheGuardRefusesAMacroThatDeclaresATestModule.refused
    macro = TheGuardRefusesAMacroThatDeclaresATestModule.macro

    def test_a_macro_module_that_is_no_test_only_module_is_read(self):
        for body in (
            "        #[cfg(not(test))]\n        mod tests {}\n",
            "        #[cfg(not(test))]\n        mod tests;\n",
            "        #[cfg(any(test, true))]\n        mod tests {}\n",
            "        #[cfg(all(not(test), unix))]\n        mod x {}\n",
            "        mod x {\n            #![cfg(not(test))]\n        }\n",
        ):
            self.assertEqual(self.refused(self.macro(body)), [], body)
        for body, reason in (
            ('        #[cfg(any(test, feature = "slow"))]\n        mod tests;\n', "441"),
            ("        #[cfg(test)]\n        mod tests;\n", "441"),
            ("        #[cfg_attr(not(test), cfg(any()))]\n        mod x {}\n", "441"),
            ('        #[cfg(not(test))]\n        #[path = "x.rs"]\n        mod prod;\n', "path"),
        ):
            expected = [f"demo (src/more.rs) {REFUSED_BY_A_MACRO[reason]}"]
            self.assertEqual(self.refused(self.macro(body)), expected, body)

    def test_a_module_a_macro_declares_without_test_still_refuses_the_test_file(self):
        """The narrowing reopens no rival: a `cfg(not(test)) mod tests;` a macro writes compiles
        the file the test module reads, so the guard, which cannot name it, refuses the pin."""
        test = "#[cfg(test)]\nmod tests;\n"
        twin = "macro_rules! twin {\n    () => {\n        #[cfg(not(test))]\n        mod tests;\n"
        twin += "    };\n}\ntwin!();\n"
        root = self.declared(test + twin, ("tests.rs", self.SPELLING), own="lib.rs")
        self.assertEqual(unpinned(root), ['demo::Depth (src/lib.rs) "a whole depth"'])
        removed = twin.replace("not(test)", "any()")
        root = self.declared(test + removed, ("tests.rs", self.SPELLING), own="lib.rs")
        self.assertEqual(unpinned(root), [])
        self.assertEqual(len(implementations(root)), 1)

    def test_a_decoy_beside_a_non_mod_rs_file_does_not_refuse_its_module(self):
        """`a.rs` reads its modules below `src/a/`, so a file the crate root's own declaration
        names in `src/` is no candidate for it."""
        nested = "mod n {\n#[cfg(test)]\nmod tests;\n}\n"
        top = "#[cfg(test)]\nmod tests;\n"
        for declaration, ours, decoy in (
            (nested, "a/n/tests.rs", "n/tests.rs"),
            (top, "a/tests.rs", "tests.rs"),
        ):
            for spelled, read in ((ours, True), (decoy, False)):
                files = [
                    (path, self.SPELLING if path == spelled else self.NO_SHAPE)
                    for path in (ours, decoy)
                ]
                root = self.declared(declaration, *files, own="a.rs")
                lib = root / "crates" / "demo" / "src" / "lib.rs"
                lib.write_text("mod a;\n" + declaration, encoding="utf-8")
                self.assertEqual(unpinned(root) == [], read, (declaration, spelled))

    def test_a_cfg_attr_path_below_an_inline_module_is_read(self):
        """Under `test` the predicate chooses the file: the named one when it holds, the default
        one when it fails; an undecided predicate stays refused."""
        for attribute, shaped, plain, read in (
            ('#[cfg_attr(test, path = "real.rs")]\n', "a/real.rs", "a/tests.rs", True),
            ('#[cfg_attr(not(test), path = "prod.rs")]\n', "a/tests.rs", "a/prod.rs", True),
            ('#[cfg_attr(test, path = "real.rs")]\n', "a/tests.rs", "a/real.rs", False),
            ('#[cfg_attr(unix, path = "real.rs")]\n', "a/real.rs", "a/tests.rs", False),
        ):
            declaration = f"mod a {{\n#[cfg(test)]\n{attribute}mod tests;\n}}\n"
            files = ((shaped, self.SPELLING), (plain, self.NO_SHAPE))
            root = self.declared(declaration, *files, own="lib.rs")
            self.assertEqual(unpinned(root) == [], read, (attribute, shaped))

    def test_modules_returns_each_declarations_attributes_and_index_without_its_name(self):
        found = modules("mod a;\n#[cfg(test)]\nmod tests {}\n")[1]
        self.assertEqual([len(each) for each in found], [2, 2])
        self.assertEqual([run for run, _ in found], [[], [["cfg", "(", "test", ")"]]])

    PRELUDE = TheGuardReadsOnlyTheItemsRustcCompilesUnderTest.PRELUDE
    MARKER = 'const _: u8 = "marker";\n'

    def crate(self, files):
        """A planted tree whose crate `demo` holds `files` (paths relative to the crate)."""
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        (root / "scripts" / "mutation-rows.d").mkdir(parents=True)
        for name, text in files.items():
            (root / "crates" / "demo" / name).parent.mkdir(parents=True, exist_ok=True)
            (root / "crates" / "demo" / name).write_text(text, encoding="utf-8")
        return root

    def rustc(self, root, file, test):
        """rustc's own reading of the crate rooted at `crates/demo/<file>`: "compiled" when it
        fails with E0308 (a marker's type error), "clean" when it exits 0. Any other answer fails
        the test, which never skips."""
        rustc = shutil.which("rustc")
        if rustc is None:
            self.fail("rustc is not on PATH, so the oracle cannot run: this test never skips")
        command = [rustc, "--edition", "2021", "--crate-type", "lib"] + ["--test"] * test
        command += ["--emit=metadata", "-o", str(root / "out.rmeta")]
        command.append(str(root / "crates" / "demo" / file))
        run = subprocess.run(command, capture_output=True, text=True, check=False)
        if run.returncode == 0:
            return "clean"
        if "E0308" in run.stderr:
            return "compiled"
        self.fail(f"{file}: rustc answered neither: rc {run.returncode}: {run.stderr[:400]}")

    def test_a_file_a_path_attribute_loads_reads_its_modules_beside_itself(self):
        """`mod a { #[path = "q.rs"] mod b; }` loads `src/a/q.rs`, and rustc reads that file's
        `mod tests;` from `src/a/tests.rs`, never from `src/a/q/tests.rs`: a marker's type error is
        read in the first and not in the second. A pin only in the file rustc compiles is read,
        and one only in the file it never reads is refused."""
        lib = self.PRELUDE.split("pub struct")[0] + 'mod a {\n    #[path = "q.rs"]\n    mod b;\n}\n'
        loaded = (
            "use crate::Setting;\n" + self.PRELUDE.split("}\n", 1)[1] + "#[cfg(test)]\nmod tests;\n"
        )
        judged = []
        for pinned, other, read in (
            ("src/a/tests.rs", "src/a/q/tests.rs", True),
            ("src/a/q/tests.rs", "src/a/tests.rs", False),
        ):
            files = {"src/lib.rs": lib, "src/a/q.rs": loaded, other: "pub fn none() {}\n"}
            twin = self.crate({**files, pinned: self.SPELLING + self.MARKER})
            self.assertEqual(self.rustc(twin, "src/lib.rs", True), "compiled" if read else "clean")
            root = self.crate({**files, pinned: self.SPELLING})
            self.assertEqual(len(implementations(root)), 1)
            self.assertEqual(unpinned(root) == [], read, pinned)
            judged.append(pinned)
        examined("path-loaded module file(s) judged against rustc", judged)

    def test_a_keyword_before_a_bang_opens_no_macro(self):
        """`return !{ ... }` and `if !( ... )` negate a block, and neither keyword names a macro,
        so the `cfg(test)` module inside the block opens no tree the guard refuses: the file reads
        0 refusals while its implementation is examined. rustc compiles it with and without
        `--test`."""
        text = (
            "pub fn gate() -> bool {\n    return !{\n        #[cfg(test)]\n        mod tests {}\n"
            "        false\n    };\n}\n"
            "pub fn gate2(x: bool) -> bool {\n    if !({\n        #[cfg(test)]\n"
            "        mod tests2 {}\n        x\n    }) {\n        return true;\n    }\n"
            "    false\n}\n"
        )
        alone = self.crate({"src/lib.rs": text + self.MARKER})
        self.assertEqual(self.rustc(alone, "src/lib.rs", True), "compiled")
        alone = self.crate({"src/lib.rs": text})
        self.assertEqual(
            [self.rustc(alone, "src/lib.rs", test) for test in (True, False)], ["clean"] * 2
        )
        root = self.src(self.tree(self.PINNED), text)
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])

    def test_an_attribute_inside_a_macro_repetition_is_read_with_its_module(self):
        """`$(#[cfg(any())] mod $n;)*` puts the attribute on every module the repetition writes,
        so rustc removes each of them with or without `test`: rustc exits 0 with no file for the
        module `decl!(zz)` writes, and reads the test file's marker only under `--test`. That
        macro module is no rival to the test module's file, so the pin is read."""
        macro = "macro_rules! decl {\n    ($($n:ident),*) => { $(#[cfg(any())] mod $n;)* };\n}\n"
        lib = self.PRELUDE + macro + "decl!(zz);\n#[cfg(test)]\nmod tests;\n"
        twin = self.crate({"src/lib.rs": lib, "src/tests.rs": self.SPELLING + self.MARKER})
        self.assertEqual(
            [self.rustc(twin, "src/lib.rs", test) for test in (True, False)], ["compiled", "clean"]
        )
        root = self.crate({"src/lib.rs": lib, "src/tests.rs": self.SPELLING})
        self.assertEqual(len(implementations(root)), 1)
        self.assertEqual(unpinned(root), [])


if __name__ == "__main__":
    unittest.main()
