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
without P; and a declaration whose file the guard cannot name (a path literal it cannot read, or
a module below an inline module whose attributes name `path`) may name any file, so unless its
attributes remove it without `test` it refuses every one. A `macro_rules!` body that declares a
`cfg(test)` module, by an outer attribute, by one before a `$( ... )` repetition or by an inner
attribute in its braces, is refused by its file, as the guard does not expand a macro (#433,
#441, #458). A literal that two implementations of one crate share is pinned only by a row on
each implementation's file.
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

IMPL = re.compile(
    r"^\s*impl\b\s*(?:<[^{};]*>)?\s*(?:\$?[\w:]+::)?Setting\s+for\s+(\$?\w+)",
    re.MULTILINE,
)
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


def crate_files(root):
    """Every crate source file the guard reads: `crates/*/src/**/*.rs`."""
    return sorted((root / "crates").glob("*/src/**/*.rs"))


def implementations(root):
    """Every `impl Setting for X` under `crates/*/src`: (crate, file, name, quoted literal, span).

    The literal is the first `const SHAPE = "..."` after the impl line and before the next impl,
    and `span` is where that constant sits in the file, so the impl's own line can be excluded.
    """
    found = []
    for path in crate_files(root):
        text = lexed(path.read_text(encoding="utf-8"))
        crate, *inside = path.relative_to(root / "crates").parts
        starts = [(m.start(), m.group(1)) for m in IMPL.finditer(text)]
        for index, (start, name) in enumerate(starts):
            end = starts[index + 1][0] if index + 1 < len(starts) else len(text)
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
    declared inside an inline module or a block is read by `sites`, not here): (name, outer
    attributes or None, index of the `mod` keyword)."""
    found, pairs, _ = scanned(text)
    own, first = leading(found, pairs, 0, len(found))
    declared, at = [], first
    while at < len(found):
        if found[at][:2] == ("word", "mod") and [t[0] for t in found[at + 1 : at + 3]] in (
            ["word", "punct"],
            ["word", "open"],
        ):
            name = found[at + 1][1].removeprefix("r#")
            declared.append((name, outer(found, pairs, first, at), at))
        at = pairs[at] + 1 if found[at][0] == "open" else at + 1
    return own, declared


def cfg_test_spans(text):
    """The spans of the file's top-level inline test modules (`mod name { ... }` that only
    `--cfg test` compiles, with the file's own inner attributes), read from `scanned`'s tokens.
    One declared inside another module is not read: only a module's file is followed there."""
    found, pairs, _ = scanned(text)
    own, declared = modules(text)
    spans = []
    for _, run, at in declared:
        if found[at + 2][1] != "{":
            continue
        inner, _ = leading(found, pairs, at + 3, pairs[at + 2])
        if test_only(None if run is None else own + run + inner):
            spans.append((found[at][2], found[pairs[at + 2]][3]))
    return spans


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


def below(own, names, name, run):
    """The files rustc could read for `mod name;` declared in `own` below the inline modules
    `names`: the file a plain `#[path]` names, relative to the module directory the inline names
    make, or else `name.rs` or `name/mod.rs` in that directory, which is below `own`'s own folder or
    below the folder beside `own` (a crate root, a `mod.rs` and a file an attribute loaded all read
    beside themselves). Every file a `cfg_attr` path could name counts too, so a rival is never
    missed. Only files that exist are returned, each once."""
    literals, _ = named_paths(run)
    plain = any(unraw(each[0]) == "path" for each in run)
    folders = [own.with_suffix(""), own.parent]
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


def declared(own):
    """(file, attributes) for each out-of-line module (`mod name;`) at any depth of inline modules
    in `own` whose file rustc's choice leaves in no doubt (R8): of every file rustc could read for
    it (`name.rs` or `name/mod.rs`, in `own`'s module directory or beside `own`, since a crate
    root, a `src/bin` file, a `mod.rs` and a file an attribute loaded all read their modules beside
    themselves, and below an inline module's name), exactly one exists and rustc's lexer reads it.
    At the top level no attribute of the declaration may carry `path` in any spelling (`#[path]`, a
    raw identifier, `cfg_attr` under any predicate); below an inline module one plain
    `#[path = "..."]` names the file from the module directory (#433). The attributes are `own`'s
    inner ones, each enclosing inline module's, the declaration's, and that file's inner ones. A
    declaration inside a function body or a macro is not followed."""
    text = own.read_text(encoding="utf-8")
    tokens = scanned(text)[0]
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
                continue
            found = below(own, names, name, run)
        else:
            if run is None or any(w.removeprefix("r#") == "path" for each in run for w in each):
                continue
            found = [
                path
                for folder in (own.with_suffix(""), own.parent)
                for path in (folder / f"{name}.rs", folder / name / "mod.rs")
                if path.is_file()
            ]
        if len(found) == 1:
            try:
                body, pairs, _ = scanned(found[0].read_text(encoding="utf-8"))
            except ValueError:
                continue
            files.append((found[0], inner + run + leading(body, pairs, 0, len(body))[0]))
    return files


def visible(src):
    """(target file or None, attributes or None) for every out-of-line module that any file
    reachable from the crate's roots declares, whatever its attributes and however deep, and for
    every file each could name (`beside`, `below`). The attributes are the whole run from the root
    that keeps the declaration, then the condition its `cfg_attr` path sets on that file (`gated`),
    then that file's own inner attributes (a module's file opens with the module's own
    attributes), or None when unreadable. A declaration whose file the guard cannot name, from a
    path literal it cannot read or from below an inline module whose attributes name `path`, is
    listed with the target None too: it may name any file (#433, #458)."""
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
            tokens = scanned(text)[0]
            found = sites(text)[1]
        except ValueError:
            continue
        for name, names, run, inner, at in found:
            if tokens[at + 2][1] == "{":
                continue
            held = None if chain is None or inner is None or run is None else chain + inner + run
            if names is None or named_paths(run or [])[1]:
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
    (#458). A declaration whose file the guard cannot name (`visible`'s None) and that is compiled
    without `test` refuses every file. A test-only second declaration is no rival, and a rival on
    another file refuses nothing."""
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
    reaches is not in it, so its test modules are not read."""
    roots = [src / "lib.rs", src / "main.rs", *src.glob("bin/*.rs"), *src.glob("bin/*/main.rs")]
    todo, seen = [root for root in roots if root.is_file()], set()
    while todo:
        file = todo.pop()
        if file in seen:
            continue
        seen.add(file)
        try:
            files = declared(file)
        except ValueError:
            continue
        for path, attributes in files:
            try:
                kept = KLEENE["all"]([condition(each, True) for each in attributes])
            except (IndexError, ValueError):
                kept = None
            if kept is True:
                todo.append(path)
    return seen


def spelled_elsewhere(root, crate, file, literal, own_span):
    """True when `literal` is spelled, quoted, in the crate's tests or inside a `#[cfg(test)]`
    module of the implementation's own source file, outside a comment, and never at any `SHAPE`
    constant."""
    base = root / "crates" / crate
    constants = {
        (path, span)
        for holder, path, _, _, span in implementations(root)
        if holder == crate and span
    }
    candidates = sorted((base / "tests").rglob("*.rs"))
    if base / file in reached(base / "src"):
        candidates += [base / file] + out_of_line(base / file)
    for path in candidates:
        text = path.read_text(encoding="utf-8")
        bare = lexed(text)
        spans = cfg_test_spans(text) if path == base / file else [(0, len(bare))]
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
    """The crate files whose `macro_rules!` body declares a module under `cfg(test)`, as refusal
    lines naming the crate and the file (#441). The guard reads tokens and does not expand a macro,
    so a module a macro writes would be a test module it never sees: it refuses the file instead.
    A `mod` in a body is judged by the attributes just before it, back to the previous item's end
    or the body's open delimiter (past a `$(` that opens the module's own repetition, so
    `#[cfg(test)] $(mod $n {})*` counts), and by the inner attributes that open its braces
    (`mod x { #![cfg(test)] }`). It reads `cfg` and `cfg_attr` in any combination with `test` (a
    `not(test)` one too: the guard cannot expand it, so it does not decide)."""
    refused = []
    for path in crate_files(root):
        found, pairs, _ = scanned(path.read_text(encoding="utf-8"))
        hit = False
        for at in range(len(found) - 3):
            if [t[:2] for t in found[at : at + 2]] != [("word", "macro_rules"), ("punct", "!")]:
                continue
            if found[at + 2][0] != "word" or found[at + 3][0] != "open":
                continue
            for mod in range(at + 4, pairs[at + 3]):
                if found[mod][:2] != ("word", "mod"):
                    continue
                words, depth, back = set(), 0, mod - 1
                while back > at + 3:
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
                if brace < len(found) and found[brace][:2] == ("open", "{"):
                    for each in leading(found, pairs, brace + 1, pairs[brace])[0]:
                        words.update(each)
                hit = hit or bool(words & {"cfg", "cfg_attr"} and "test" in words)
        if hit:
            crate, *inside = path.relative_to(root / "crates").parts
            refused.append(
                f"{crate} ({Path(*inside).as_posix()}) macro_rules! body declares a cfg(test) module"
            )
    return refused


def unpinned(root):
    """The implementations whose literal no test spells and no row of their file finds, then each
    crate file whose macro declares a test module (`macro_test_modules`). A literal that two
    implementations of one crate share is pinned only by a row of each one's file."""
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
        decided member that has such a source; otherwise it may refuse."""
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
        wrong, judged = [], []
        for index, (case, files, _, _, decided) in enumerate(members):
            if decided and index in broken:
                wrong.append(f"{case}: a decided member does not compile")
            if index in broken:
                continue
            judged.append(case)
            only = under[index] - without[index] - {"own", "root"}
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
            self.macro("        $(#[$m:meta])*\n        mod tests;\n"),
            "#[cfg(test)]\nmod tests {\n    const X: u8 = 1;\n}\n",
            "// macro_rules! m { () => { #[cfg(test)] mod tests; }; }\n",
            'const D: &str = "macro_rules! m { () => { #[cfg(test)] mod tests; }; }";\n',
            "macro_rules! m {\n    () => {};\n}\n#[cfg(test)]\nmod tests {}\n",
            "macro_rules! m {\n    () => {};\n}\nm!{ #[cfg(test)] mod tests; }\n",
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


if __name__ == "__main__":
    unittest.main()
