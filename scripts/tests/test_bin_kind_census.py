# SPDX-License-Identifier: AGPL-3.0-or-later
"""The mutation runner's `bin` killer kind reads what the compiler builds (SPEC-039 section 19,
issue #405).

The class rule: the kind's census, selection and refusal read the crate's source files, test
targets and binaries exactly as the compiler and cargo define them. No file the compiler does not
build is read as a module, no target cargo builds is missed, and every count a message states
equals cargo's own. Anything the reader cannot decide is refused by name.

The killer is a GENERATED population with a COMPILER ORACLE, never a hand list. Two axis tables
are crossed into scratch crates:

- binary layouts x test layouts, judged against `cargo metadata --no-deps` (the targets and the
  binaries cargo itself reports);
- module layouts x crate-root positions, judged against `rustc --test --emit=dep-info` (the source
  files the compiler itself reads).

A new row in any axis table joins the population by itself, and the `examined` figure is derived
from the tables, so a member dropped or added without the tables changing fails the count. The
oracle runs cargo and rustc in scratch crates only, never on the workspace. Three planted tests,
one per gap the issue names, read the same reader without the oracle.

Generated families close five classes (SPEC-039 section 21): every spelling cargo compares a
declared path by, `..` and an absolute path among them; every dotfile and dot directory cargo's
inference skips; every edition a manifest may state or inherit, since 2015 switches inference off
beside a declared table; and every delimiter of a macro invocation that holds a `mod`.
"""

import concurrent.futures
import functools
import itertools
import json
import os
import posixpath
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

PACKAGE = "fix"
PACKAGE_HEADER = f'[package]\nname = "{PACKAGE}"\nversion = "0.1.0"\n'
#: The edition axis every binary layout and test layout is crossed with: no key is 2015.
EDITIONS = {"edition 2021": 'edition = "2021"\n', "no edition key, so 2015": ""}
#: The editions a member inherits from a workspace manifest at the root, crossed with the binaries.
INHERITED_EDITIONS = ("2015", "2021")
BASE_MANIFEST = PACKAGE_HEADER + EDITIONS["edition 2021"]
MAIN = "fn main() {}\n"
TEST_FILE = "#[test]\nfn passes() {}\n"
LEAF = "pub fn leaf() {}\n"
KILLER = "bin::tests::passes"


def runner_module():
    """The runner imported as a module, so the tests call its readers directly."""
    scripts = str(REPO / "scripts")
    if scripts not in sys.path:
        sys.path.insert(0, scripts)
    import mutation_rows

    return mutation_rows


# A layout is (package keys, manifest tables, files). The package keys sit under `[package]`, the
# tables follow it, and every member also carries a library so that a layout with no binary is
# still a valid manifest for cargo.
BINARY_LAYOUTS = {
    "main": ("", "", {"src/main.rs": MAIN}),
    "main and a [[bin]] at its path": (
        "",
        '[[bin]]\nname = "fixbin"\npath = "src/main.rs"\n',
        {"src/main.rs": MAIN},
    ),
    "a [[bin]] elsewhere": (
        "",
        '[[bin]]\nname = "tool"\npath = "app/tool.rs"\n',
        {"app/tool.rs": MAIN},
    ),
    "a [[bin]] of the package name": ("", f'[[bin]]\nname = "{PACKAGE}"\n', {"src/main.rs": MAIN}),
    "a bin file": ("", "", {"src/bin/a.rs": MAIN}),
    "a bin directory with a module file": (
        "",
        "",
        {"src/bin/a/main.rs": MAIN, "src/bin/a/util.rs": LEAF},
    ),
    "a bin directory named like a file": ("", "", {"src/bin/odd.rs/main.rs": MAIN}),
    "a [[bin]] at a dotted path": (
        "",
        '[[bin]]\nname = "fixbin"\npath = "./src/main.rs"\n',
        {"src/main.rs": MAIN},
    ),
    "a [[bin]] named for a bin file, at another path": (
        "",
        '[[bin]]\nname = "a"\npath = "app/a.rs"\n',
        {"app/a.rs": MAIN, "src/bin/a.rs": MAIN},
    ),
    "two bin files": ("", "", {"src/bin/a.rs": MAIN, "src/bin/b.rs": MAIN}),
    "main and a bin file": ("", "", {"src/main.rs": MAIN, "src/bin/a.rs": MAIN}),
    "two bin files and a directory with no main": (
        "",
        "",
        {"src/bin/a.rs": MAIN, "src/bin/b.rs": MAIN, "src/bin/c/util.rs": LEAF},
    ),
    "two bin files and a directory with a main and a module": (
        "",
        "",
        {
            "src/bin/a.rs": MAIN,
            "src/bin/b.rs": MAIN,
            "src/bin/c/main.rs": MAIN,
            "src/bin/c/util.rs": LEAF,
        },
    ),
    "autobins off, main and a bin file": (
        "autobins = false\n",
        "",
        {"src/main.rs": MAIN, "src/bin/a.rs": MAIN},
    ),
    "autobins off, a [[bin]] beside a bin file": (
        "autobins = false\n",
        '[[bin]]\nname = "a"\n',
        {"src/bin/a.rs": MAIN, "src/bin/b.rs": MAIN},
    ),
    "a [[bin]] named for a bin file, and main": (
        "",
        '[[bin]]\nname = "a"\n',
        {"src/main.rs": MAIN, "src/bin/a.rs": MAIN},
    ),
    "two declared": (
        "",
        '[[bin]]\nname = "fixbin"\npath = "src/main.rs"\n\n[[bin]]\nname = "other"\n'
        'path = "src/other.rs"\n',
        {"src/main.rs": MAIN, "src/other.rs": MAIN},
    ),
}

TEST_LAYOUTS = {
    "no test target": ("", "", {}),
    "tests/bin.rs": ("", "", {"tests/bin.rs": TEST_FILE}),
    "tests/bin/main.rs": ("", "", {"tests/bin/main.rs": TEST_FILE}),
    "a [[test]] named for a test file, at another path": (
        "",
        '[[test]]\nname = "other"\npath = "spec/o.rs"\n',
        {"spec/o.rs": TEST_FILE, "tests/other.rs": TEST_FILE},
    ),
    "a [[test]] at a dotted path of a test file": (
        "",
        '[[test]]\nname = "x"\npath = "./tests/bin.rs"\n',
        {"tests/bin.rs": TEST_FILE},
    ),
    "tests/other.rs": ("", "", {"tests/other.rs": TEST_FILE}),
    "a [[test]] bin at another path": (
        "",
        '[[test]]\nname = "bin"\npath = "tests/other.rs"\n',
        {"tests/other.rs": TEST_FILE},
    ),
    "a [[test]] bin at its default path": (
        "",
        '[[test]]\nname = "bin"\n',
        {"tests/bin.rs": TEST_FILE},
    ),
    "a [[test]] of another name at tests/bin.rs": (
        "",
        '[[test]]\nname = "other"\npath = "tests/bin.rs"\n',
        {"tests/bin.rs": TEST_FILE},
    ),
    "autotests off, tests/bin.rs": ("autotests = false\n", "", {"tests/bin.rs": TEST_FILE}),
    "autotests off, a [[test]] bin": (
        "autotests = false\n",
        '[[test]]\nname = "bin"\npath = "tests/x.rs"\n',
        {"tests/x.rs": TEST_FILE},
    ),
}


def spellings(path):
    """Each way a manifest may spell the crate-relative `path`: as written, with a leading `./`, an
    interior `.`, a doubled separator, from the crate's absolute directory (`@CRATE@`, written when
    the member is laid out) and through a `..`. Cargo compares paths by component, so every one but
    the `..` names the file it was written for."""
    first, rest = path.split("/", 1)
    return {
        "as written": path,
        "with a leading ./": f"./{path}",
        "with an interior .": f"{first}/./{rest}",
        "with a doubled separator": f"{first}//{rest}",
        "from the crate's absolute directory": f"@CRATE@/{path}",
        "through a ..": f"{first}/../{path}",
    }


#: A declared target at a path cargo also infers a target from: (kind, name, path, files).
DECLARED_AT_AN_INFERRED_PATH = {
    "a [[bin]] at main": ("bin", "fixbin", "src/main.rs", {"src/main.rs": MAIN}),
    "a [[bin]] at a bin file": ("bin", "tool", "src/bin/a.rs", {"src/bin/a.rs": MAIN}),
    "a [[test]] at tests/bin.rs": ("test", "other", "tests/bin.rs", {"tests/bin.rs": TEST_FILE}),
}
for _shape, (_kind, _name, _path, _files) in DECLARED_AT_AN_INFERRED_PATH.items():
    _layouts = BINARY_LAYOUTS if _kind == "bin" else TEST_LAYOUTS
    for _spelling, _written in spellings(_path).items():
        _layouts[f"{_shape}, spelled {_spelling}"] = (
            "",
            f'[[{_kind}]]\nname = "{_name}"\npath = "{_written}"\n',
            _files,
        )

#: Entries cargo's inference skips because their names start with a dot, beside each companion.
DOT_ENTRIES = {
    "a dotfile": {"src/bin/.hidden.rs": MAIN},
    "a dot directory with a main": {"src/bin/.d/main.rs": MAIN},
}
DOT_COMPANIONS = {
    "alone": {},
    "beside main": {"src/main.rs": MAIN},
    "beside a bin file": {"src/bin/a.rs": MAIN},
}
for (_entry, _files), (_companion, _beside) in itertools.product(
    DOT_ENTRIES.items(), DOT_COMPANIONS.items()
):
    BINARY_LAYOUTS[f"{_entry} under src/bin, {_companion}"] = ("", "", {**_files, **_beside})
TEST_LAYOUTS["a dotfile tests/.bin.rs"] = ("", "", {"tests/.bin.rs": TEST_FILE})
TEST_LAYOUTS["a dot directory tests/.bin/main.rs"] = ("", "", {"tests/.bin/main.rs": TEST_FILE})

#: Where a binary's root file sits; its directory is where the compiler looks for its modules.
ROOTS = {
    "src/main.rs": {"src/main.rs": None},
    "src/bin/a.rs": {"src/bin/a.rs": None},
    "src/bin/a/main.rs": {"src/bin/a/main.rs": None},
}


def at(home, relative):
    """`relative`, resolved against the crate-relative directory `home`, in normal form."""
    return posixpath.normpath(posixpath.join(home, relative))


def module_layouts(home):
    """name -> (the lines the root file opens with, its files, the files the compiler builds that
    the runner deliberately leaves unread, whether the layout is undecidable to the runner).

    `home` is the directory of the root file, where rustc looks for the root's own modules. The
    unread set is the files reached by a `#[path]` attribute: the runner follows none, so a killer
    there is refused by the census rather than read from a file the compiler does not build. An
    undecidable layout given as text, as the macro family's are, is one whose refusal names it.
    """
    return {
        "a file module": (
            "mod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a pub(crate) file module": (
            "pub(crate) mod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a #[path] module beside a stray default file": (
            '#[path = "impl/x.rs"]\nmod x;\n',
            {at(home, "impl/x.rs"): LEAF, at(home, "x.rs"): LEAF},
            {at(home, "impl/x.rs")},
            False,
        ),
        "a #[path] module with no stray file": (
            '#[path = "impl/x.rs"]\nmod x;\n',
            {at(home, "impl/x.rs"): LEAF},
            {at(home, "impl/x.rs")},
            False,
        ),
        "nested and relative #[path] modules beside stray files": (
            '#[path = "deep/er/x.rs"]\nmod x;\n#[path = "../shared_y.rs"]\nmod y;\n',
            {
                at(home, "deep/er/x.rs"): LEAF,
                at(home, "../shared_y.rs"): LEAF,
                at(home, "x.rs"): LEAF,
                at(home, "y.rs"): LEAF,
            },
            {at(home, "deep/er/x.rs"), at(home, "../shared_y.rs")},
            False,
        ),
        "an inline module holding a file module, beside a stray file": (
            "mod outer {\n    mod inner;\n}\n",
            {at(home, "outer/inner.rs"): LEAF, at(home, "inner.rs"): LEAF},
            set(),
            False,
        ),
        "a cfg(test) file module": (
            "#[cfg(test)]\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a cfg(not(test)) file module the compiler never builds": (
            "#[cfg(not(test))]\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a cfg(feature) file module the runner cannot decide": (
            '#[cfg(feature = "f")]\nmod x;\n',
            {at(home, "x.rs"): LEAF},
            set(),
            True,
        ),
        "a cfg(test) inline module holding a file module, beside a stray file": (
            "#[cfg(test)]\nmod outer {\n    mod inner;\n}\n",
            {at(home, "outer/inner.rs"): LEAF, at(home, "inner.rs"): LEAF},
            set(),
            False,
        ),
        "a #[path] inline module holding a file module": (
            '#[path = "dir"]\nmod outer {\n    mod inner;\n}\n',
            {
                at(home, "dir/inner.rs"): LEAF,
                at(home, "outer/inner.rs"): LEAF,
                at(home, "inner.rs"): LEAF,
            },
            {at(home, "dir/inner.rs")},
            False,
        ),
        "a declaration only in a comment and a string": (
            '// mod ghost;\n/*\nmod ghost;\n*/\nconst S: &str = "mod ghost;";\n'
            'const R: &str = r#"\nmod ghost;\n"#;\n',
            {at(home, "ghost.rs"): LEAF},
            set(),
            False,
        ),
        "a nested empty block comment before a declaration": (
            "/* a /**/ b */\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a block comment glued to a declaration": (
            "/* c */mod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "comments inside a declaration": (
            "mod // a\n/* b */ x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a cfg(not(test)) inline module holding a file module, beside stray files": (
            "#[cfg(not(test))]\nmod outer {\n    mod inner;\n}\n",
            {
                at(home, "outer/inner.rs"): LEAF,
                at(home, "inner.rs"): LEAF,
                at(home, "skip/inner.rs"): LEAF,
            },
            set(),
            False,
        ),
        "a cfg(not(test)) function holding a file module the runner cannot decide": (
            "#[cfg(not(test))]\nfn f() {\n    mod x;\n}\n",
            {at(home, "x.rs"): LEAF},
            set(),
            True,
        ),
        "a cfg(not(test)) field before a declaration": (
            "struct S {\n    #[cfg(not(test))]\n    a: u8,\n}\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a cfg(not(test)) pub file module the compiler never builds": (
            "#[cfg(not(test))]\npub mod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a #[path] pub file module beside a stray default file": (
            '#[path = "impl/x.rs"]\npub mod x;\n',
            {at(home, "impl/x.rs"): LEAF, at(home, "x.rs"): LEAF},
            {at(home, "impl/x.rs")},
            False,
        ),
        "a raw string closing right before a semicolon": (
            '#[cfg(not(test))]\nconst S: &str = r"a";\nmod x;\n',
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "file modules that declare file modules": (
            "mod p;\nmod q;\n",
            {
                at(home, "p/mod.rs"): "mod r;\n",
                at(home, "p/r.rs"): LEAF,
                at(home, "r.rs"): LEAF,
                at(home, "q.rs"): "mod s;\n",
                at(home, "q/s.rs"): LEAF,
                at(home, "s.rs"): LEAF,
            },
            set(),
            False,
        ),
        "a cfg(any(test, unix)) file module": (
            "#[cfg(any(test, unix))]\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a cfg(all(test, not(test))) file module the compiler never builds": (
            "#[cfg(all(test, not(test)))]\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a cfg(not(any(test, unix))) file module the compiler never builds": (
            "#[cfg(not(any(test, unix)))]\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a cfg(all(test, unix)) file module the runner cannot decide": (
            "#[cfg(all(test, unix))]\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            True,
        ),
        "a cfg(not(feature)) file module the runner cannot decide": (
            '#[cfg(not(feature = "f"))]\nmod x;\n',
            {at(home, "x.rs"): LEAF},
            set(),
            True,
        ),
        "a cfg_attr path the runner cannot decide": (
            '#[cfg_attr(test, path = "impl/x.rs")]\nmod x;\n',
            {at(home, "impl/x.rs"): LEAF, at(home, "x.rs"): LEAF},
            set(),
            True,
        ),
        "an inner cfg attribute the runner cannot decide": (
            "#![cfg(test)]\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            True,
        ),
        "an include! the runner cannot follow": (
            'include!("inc.rs");\nmod x;\n',
            {at(home, "inc.rs"): LEAF, at(home, "x.rs"): LEAF},
            set(),
            True,
        ),
        "a raw identifier module the runner cannot decide": (
            "mod r#match;\n",
            {at(home, "match.rs"): LEAF},
            set(),
            True,
        ),
        "a #[path] module inside a function body": (
            'fn f() {\n    #[path = "impl/x.rs"]\n    mod x;\n}\n',
            {at(home, "impl/x.rs"): LEAF},
            {at(home, "impl/x.rs")},
            False,
        ),
        "a #[path] module inside an inline module": (
            'mod outer {\n    #[path = "impl/x.rs"]\n    mod x;\n}\n',
            {at(home, "outer/impl/x.rs"): LEAF, at(home, "outer/x.rs"): LEAF},
            {at(home, "outer/impl/x.rs")},
            False,
        ),
        "lexemes that look like declarations": (
            "const C: char = '{';\nconst Q: char = '\\'';\nfn f<'a>(x: &'a str) -> &'a str { x }\nconst B: &[u8] = b\"mod ghost;\";\nconst R: &[u8] = br#\"\nmod ghost;\n\"#;\n/* outer /* nested */ mod ghost; */\nmod x;\n",
            {at(home, "x.rs"): LEAF, at(home, "ghost.rs"): LEAF},
            set(),
            False,
        ),
        "raw strings and escapes that quote a declaration": (
            'const D: char = \'\\"\';\nconst S: &str = r#"a "mod ghost;" b"#;\nconst H: &str = r##"x "# mod ghost; "##;\nconst E: &str = "\\\\";\nconst Z: &[u8] = cr"mod ghost;";\nmod x;\n',
            {at(home, "x.rs"): LEAF, at(home, "ghost.rs"): LEAF},
            set(),
            False,
        ),
        "an escaped quote in a character and in a string": (
            'const D: char = \'\\"\';\nmod x;\nconst T: &str = "q";\nconst Y: &str = "\\" mod ghost; ";\n',
            {at(home, "x.rs"): LEAF, at(home, "ghost.rs"): LEAF},
            set(),
            False,
        ),
        "an identifier named include and a stray hash in a macro body": (
            "fn include() {}\nmod outer {\n    macro_rules! m { () => { # { } }; }\n    mod x;\n}\n",
            {at(home, "outer/x.rs"): LEAF, at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "an inner cfg_attr before a module": (
            "#![cfg_attr(not(test), no_std)]\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "two cfgs on one module, the false one first": (
            "#[cfg(not(test))]\n#[cfg(test)]\nmod x;\nmod y;\n",
            {at(home, "x.rs"): LEAF, at(home, "y.rs"): LEAF},
            set(),
            False,
        ),
        "a false cfg on a module does not reach the next module": (
            '#[cfg(not(test))]\nmod a;\n#[path = "impl/p.rs"]\nmod b;\nmod x;\n',
            {at(home, "a.rs"): LEAF, at(home, "impl/p.rs"): LEAF, at(home, "x.rs"): LEAF},
            {at(home, "impl/p.rs")},
            False,
        ),
        "a cfg on a function with a body does not reach the next module": (
            "#[cfg(not(test))]\nfn f() {}\nmod x;\n",
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
        "a skipped inline module holding blocks and a declaration": (
            "#[cfg(not(test))]\nmod gone {\n    fn f() { if true { } }\n    mod inner;\n}\nmod x;\n",
            {at(home, "x.rs"): LEAF, at(home, "gone/inner.rs"): LEAF, at(home, "inner.rs"): LEAF},
            set(),
            False,
        ),
        "attributes of other items before a module": (
            '#[cfg(not(test))]\nfn unrelated() {}\n#[cfg(not(test))]\nconst K: u8 = 1;\n#[allow(dead_code)]\n#[doc = "x"]\npub(in crate) mod x;\n',
            {at(home, "x.rs"): LEAF},
            set(),
            False,
        ),
    } | macro_layouts(home)


#: The delimiters a macro invocation's tokens may sit in.
MACRO_DELIMITERS = {"parentheses": ("(", ")"), "brackets": ("[", "]"), "braces": ("{", "}")}
#: What the reader names when it refuses a `mod` among a macro invocation's tokens.
MACRO_REFUSAL = "holds a mod in a macro invocation, which only its expansion decides"


def macro_layouts(home):
    """The macro family, as module layouts: under every delimiter, a `mod` among an invocation's
    tokens, whether the macro discards them (`stringify!`, by name or by path, or a declared macro)
    or a `macro_rules!` definition expands them into a declaration. Each is refused by name, since
    only the expansion decides what it declares. Beside them, under every delimiter, a group nested
    in an invocation before its `mod`, an invocation holding none before a real declaration, and
    an invocation inside a module the test build drops, which the reader reads."""
    declared = "macro_rules! m {\n    ($($t:tt)*) => {};\n}\n"
    ghost = {at(home, "ghost.rs"): LEAF}
    built = {at(home, "x.rs"): LEAF}
    found = {}
    for word, (opened, closed) in MACRO_DELIMITERS.items():
        end = "" if opened == "{" else ";"
        holding = {
            "stringify!": f"const S: &str = stringify!{opened}mod ghost;{closed};\n",
            "a path to stringify!": f"const S: &str = core::stringify!{opened}mod ghost;{closed};\n",
            "a declared macro": f"{declared}m!{opened}mod ghost;{closed}{end}\n",
        }
        for spelling, text in holding.items():
            found[f"{spelling} in {word} holding a declaration"] = (
                text,
                ghost,
                set(),
                MACRO_REFUSAL,
            )
        for inner, (inner_opened, inner_closed) in MACRO_DELIMITERS.items():
            found[f"stringify! in {word} holding a group in {inner}, then a declaration"] = (
                f"const S: &str = stringify!{opened}{inner_opened}a{inner_closed} mod ghost;"
                f"{closed};\n",
                ghost,
                set(),
                MACRO_REFUSAL,
            )
        found[f"stringify! in {word} holding no declaration, before one"] = (
            f"const S: &str = stringify!{opened}a{closed};\nmod x;\n",
            built,
            set(),
            False,
        )
        found[f"stringify! in {word} holding a declaration, in a module the test build drops"] = (
            f"#[cfg(not(test))]\nmod gone {{\n    const S: &str = stringify!{opened}mod ghost;"
            f"{closed};\n}}\nmod x;\n",
            built,
            set(),
            False,
        )
        found[f"a macro_rules! definition in {word} that expands to a declaration"] = (
            f"macro_rules! m {opened}\n    () => {{ mod x; }};\n{closed}{end}\nm!();\n",
            built,
            set(),
            MACRO_REFUSAL,
        )
    return found


def workspace_manifest(edition):
    """A workspace manifest at a member's root whose package table states `edition`."""
    return (
        f'[workspace]\nmembers = ["crates/{PACKAGE}"]\nresolver = "2"\n\n'
        f'[workspace.package]\nedition = "{edition}"\n'
    )


class Member:
    """One generated crate laid out under `<root>/crates/fix`, removed when the test ends. Its
    manifest states `edition` (a line under `[package]`), `@CRATE@` in a table is the crate's
    absolute directory, and `workspace`, when given, is the manifest written at the root."""

    def __init__(self, case, name, parts, edition=EDITIONS["edition 2021"], workspace=None):
        scratch = tempfile.TemporaryDirectory()
        case.addCleanup(scratch.cleanup)
        self.name = name
        self.root = Path(scratch.name).resolve()
        self.crate = self.root / "crates" / PACKAGE
        package_keys = "".join(keys for keys, _tables, _files in parts)
        tables = "".join(f"\n{tables}" for _keys, tables, _files in parts if tables)
        manifest = PACKAGE_HEADER + edition + package_keys + tables
        self.write("Cargo.toml", manifest.replace("@CRATE@", self.crate.as_posix()))
        if workspace is not None:
            (self.root / "Cargo.toml").write_text(workspace, encoding="utf-8")
        self.write("src/lib.rs", LEAF)
        for _keys, _tables, files in parts:
            for relative, text in files.items():
                self.write(relative, text)

    def write(self, relative, text):
        path = self.crate / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


@functools.cache
def toolchain(tool):
    """The path of the toolchain's own `tool`, the one the rustup proxy runs in a scratch crate, or
    the bare name where no rustup resolves it. The proxy's start costs more than the metadata it
    serves, and the oracle runs once per member."""
    with tempfile.TemporaryDirectory() as away:
        try:
            done = subprocess.run(
                ["rustup", "which", tool],
                cwd=away,
                capture_output=True,
                text=True,
                check=False,
                timeout=120,
            )
        except OSError:
            return tool
    found = done.stdout.strip()
    return found if done.returncode == 0 and found else tool


def cargo_targets(member):
    """(binaries, test target names) as `cargo metadata --no-deps` reports them: the oracle."""
    done = subprocess.run(
        [toolchain("cargo"), "metadata", "--no-deps", "--format-version", "1", "--offline"],
        cwd=member.crate,
        capture_output=True,
        text=True,
        check=False,
        timeout=120,
    )
    if done.returncode != 0:
        raise AssertionError(f"{member.name}: cargo refused the layout: {done.stderr}")
    targets = json.loads(done.stdout)["packages"][0]["targets"]
    binaries = sorted(
        (t["name"], Path(t["src_path"]).resolve().relative_to(member.crate).as_posix())
        for t in targets
        if "bin" in t["kind"]
    )
    tests = sorted(t["name"] for t in targets if "test" in t["kind"])
    return binaries, tests


def compiler_sources(member, root_file):
    """The source files `rustc --test` reads from `root_file`: the oracle for the module walk."""
    out = member.root / "dep-info"
    out.mkdir(exist_ok=True)
    done = subprocess.run(
        [toolchain("rustc"), "--edition", "2021", "--test", "--crate-name", "fixbin"]
        + ["--emit=dep-info"]
        + ["--out-dir", str(out), str(member.crate / root_file)],
        cwd=member.crate,
        capture_output=True,
        text=True,
        check=False,
        timeout=120,
    )
    if done.returncode != 0:
        raise AssertionError(f"{member.name}: rustc refused the layout: {done.stderr}")
    text = (out / "fixbin.d").read_text(encoding="utf-8")
    return {
        (member.crate / line[:-1]).resolve()
        for line in text.splitlines()
        if line.endswith(":") and line[:-1].endswith(".rs")
    }


def killer_row(runner):
    """The row a `bin::` killer of the fixture crate names."""
    return runner.Row(
        "S00000-BIN",
        "MUTATIONS",
        "src/main.rs",
        "x",
        "y",
        KILLER,
        PACKAGE,
        "a generated member",
    )


class TheBinKindReadsWhatTheCompilerBuilds(unittest.TestCase):
    """SPEC-039 section 19 (issue #405): the class rule, decided by a generated population."""

    def refusal(self, runner, member):
        """The refusal `locate_killer` raises for the member, or None when it resolves."""
        try:
            runner.locate_killer(member.root, killer_row(runner))
        except runner.KillerUnresolved as refusal:
            return str(refusal)
        return None

    def test_every_layout_agrees_with_cargo_and_the_compiler_or_is_refused_by_name(self):
        runner = runner_module()
        members = []
        for (binary, b_parts), (test, t_parts), (edition, line) in itertools.product(
            BINARY_LAYOUTS.items(), TEST_LAYOUTS.items(), EDITIONS.items()
        ):
            member = Member(
                self,
                f"binary layout [{binary}] with test layout [{test}], {edition}",
                [b_parts, t_parts],
                edition=line,
            )
            members.append(("targets", member))
        for (binary, b_parts), inherited in itertools.product(
            BINARY_LAYOUTS.items(), INHERITED_EDITIONS
        ):
            member = Member(
                self,
                f"binary layout [{binary}], edition {inherited} inherited from the workspace",
                [b_parts],
                edition="edition.workspace = true\n",
                workspace=workspace_manifest(inherited),
            )
            members.append(("targets", member))
        for root in ROOTS:
            home = posixpath.dirname(root)
            for name, (prelude, files, unread, undecidable) in module_layouts(home).items():
                member = Member(
                    self,
                    f"module layout [{name}] under the root {root}",
                    [("", "", {root: prelude + MAIN, **files})],
                )
                member.unread, member.undecidable, member.root_file = unread, undecidable, root
                members.append(("modules", member))
        derived = (
            len(BINARY_LAYOUTS) * len(TEST_LAYOUTS) * len(EDITIONS)
            + len(BINARY_LAYOUTS) * len(INHERITED_EDITIONS)
            + len(ROOTS) * len(module_layouts("src"))
        )
        self.assertEqual(len(examined("generated crate layouts", members)), derived)

        # The oracle runs in scratch crates that share nothing, so its runs overlap; each verdict
        # is read inside its member's subtest, where a refusal by cargo or rustc fails that member.
        with concurrent.futures.ThreadPoolExecutor(
            max_workers=min(16, os.cpu_count() or 1)
        ) as pool:
            oracles = [
                pool.submit(cargo_targets, member)
                if kind == "targets"
                else pool.submit(compiler_sources, member, member.root_file)
                for kind, member in members
            ]
        agreed = refused = 0
        for (kind, member), oracle in zip(members, oracles, strict=True):
            with self.subTest(member=member.name):
                if kind == "targets":
                    agreed += self.judge_targets(runner, member, oracle.result())
                else:
                    outcome = self.judge_modules(runner, member, oracle.result())
                    agreed, refused = agreed + outcome[0], refused + outcome[1]
        print(f"agreed with the oracle {agreed}, refused by name {refused}")
        self.assertEqual(agreed + refused, derived, "a member was neither agreed nor refused")
        # The presence controls: the population holds both outcomes, so neither arm is vacuous.
        self.assertGreater(refused, 0)
        self.assertGreater(agreed, refused)

    def judge_targets(self, runner, member, oracle):
        binaries, tests = oracle
        refusal = self.refusal(runner, member)
        if "bin" in tests:
            self.assertIn("has a test target bin, which the bin kind shadows", refusal or "")
        elif len(binaries) != 1:
            self.assertIn(f"holds {len(binaries)} binaries", refusal or "")
        else:
            self.assertIsNone(refusal, f"cargo builds exactly one binary: {binaries}")
            killer = runner.locate_killer(member.root, killer_row(runner))
            name, path = binaries[0]
            self.assertEqual((killer.binary, killer.file), (name, f"crates/{PACKAGE}/{path}"))
        return 1

    def judge_modules(self, runner, member, oracle):
        try:
            read = {p.resolve() for p in runner.module_sources(member.crate / member.root_file)}
        except runner.KillerUnresolved as refusal:
            self.assertTrue(str(refusal), "a refusal must name its reason")
            self.assertTrue(member.undecidable, f"refused a layout the compiler decides: {refusal}")
            if isinstance(member.undecidable, str):
                self.assertIn(member.undecidable, str(refusal))
            return 0, 1
        expected = oracle - {(member.crate / p).resolve() for p in member.unread}
        self.assertEqual(
            sorted(p.relative_to(member.crate).as_posix() for p in read),
            sorted(p.relative_to(member.crate).as_posix() for p in expected),
        )
        return 1, 0


class ThePlantedShapesOfTheIssue(unittest.TestCase):
    """One planted shape per gap issue #405 names, read without the oracle."""

    def member(self, *parts):
        return Member(self, "planted", list(parts))

    def test_a_path_module_contributes_no_source_file_so_a_stray_default_is_not_read(self):
        stray = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn only_in_the_stray() {}\n}\n"
        member = self.member(
            ("", "", {"src/main.rs": 'fn main() {}\n#[path = "impl/x.rs"]\nmod x;\n'}),
            ("", "", {"src/impl/x.rs": LEAF, "src/x.rs": stray}),
        )
        runner = runner_module()
        sources = [
            p.relative_to(member.crate).as_posix()
            for p in runner.module_sources(member.crate / "src/main.rs")
        ]
        self.assertEqual(sources, ["src/main.rs"])
        row = killer_row(runner)
        named = runner.Row(
            row.id,
            row.table,
            row.target,
            row.find,
            row.replace,
            "bin::x::tests::only_in_the_stray",
            row.crate,
            row.description,
        )
        with self.assertRaisesRegex(runner.KillerUnresolved, "declares only_in_the_stray 0 times"):
            runner.resolve_killer(member.root, named)

    def test_a_declared_test_target_named_bin_is_refused_at_any_path(self):
        member = self.member(
            ("", "", {"src/main.rs": MAIN}),
            (
                "",
                '[[test]]\nname = "bin"\npath = "tests/other.rs"\n',
                {"tests/other.rs": TEST_FILE},
            ),
        )
        runner = runner_module()
        with self.assertRaisesRegex(
            runner.KillerUnresolved, "has a test target bin, which the bin kind shadows"
        ):
            runner.locate_killer(member.root, killer_row(runner))
        # The control: the same crate without the declared target resolves.
        plain = self.member(("", "", {"src/main.rs": MAIN}))
        self.assertEqual(runner.locate_killer(plain.root, killer_row(runner)).binary, PACKAGE)

    def test_the_refusal_counts_binaries_as_cargo_does_and_never_a_module_file(self):
        bins = {"src/bin/a.rs": MAIN, "src/bin/b.rs": MAIN, "src/bin/c/util.rs": LEAF}
        member = self.member(("", "", bins))
        runner = runner_module()
        with self.assertRaisesRegex(runner.KillerUnresolved, "holds 2 binaries,"):
            runner.locate_killer(member.root, killer_row(runner))
        # The control: a directory that holds a main.rs is a binary, and the count moves to three.
        third = self.member(("", "", dict(bins, **{"src/bin/c/main.rs": MAIN})))
        with self.assertRaisesRegex(runner.KillerUnresolved, "holds 3 binaries,"):
            runner.locate_killer(third.root, killer_row(runner))

    def test_what_the_reader_cannot_decide_is_refused_by_name(self):
        runner = runner_module()
        refusals = {
            'autobins = "yes"\n': (
                ("", "", {"src/main.rs": MAIN}),
                "sets autobins to something other than a boolean",
            ),
            "": (
                ("", '[[bin]]\npath = "src/main.rs"\n', {"src/main.rs": MAIN}),
                "declares a bin with no name or path",
            ),
        }
        for keys, (layout, pattern) in refusals.items():
            with self.subTest(pattern=pattern):
                member = self.member((keys, layout[1], layout[2]))
                with self.assertRaisesRegex(runner.KillerUnresolved, pattern):
                    runner.locate_killer(member.root, killer_row(runner))
        # A table with no path and no file to infer one from still names its target: under edition
        # 2015, where the table switches inference off, a binary defaults to the package's own root
        # (cargo refuses this layout under 2021), and a declared test named bin shadows with no file.
        unpathed = Member(
            self,
            "planted",
            [("", '[[bin]]\nname = "tool"\n', {"src/main.rs": MAIN})],
            edition=EDITIONS["no edition key, so 2015"],
        )
        found = runner.locate_killer(unpathed.root, killer_row(runner))
        self.assertEqual((found.binary, found.file), ("tool", f"crates/{PACKAGE}/src/main.rs"))
        ghost = self.member(("", '[[test]]\nname = "bin"\n', {"src/main.rs": MAIN}))
        with self.assertRaisesRegex(runner.KillerUnresolved, "has a test target bin"):
            runner.locate_killer(ghost.root, killer_row(runner))
        nothing = self.member(("autobins = false\n", "", {"src/main.rs": MAIN}))
        with self.assertRaisesRegex(runner.KillerUnresolved, "holds 0 binaries,"):
            runner.locate_killer(nothing.root, killer_row(runner))
        missing = self.member(("", '[[bin]]\nname = "t"\npath = "app/t.rs"\n', {}))
        with self.assertRaisesRegex(
            runner.KillerUnresolved, "declares the binary t at app/t.rs, which does not exist"
        ):
            runner.locate_killer(missing.root, killer_row(runner))
        # An edition the reader cannot read decides whether a declared table switches inference
        # off, so it is refused by name: one of the wrong type, one inherited from no workspace,
        # from a workspace that states none, and from a workspace manifest that does not parse.
        table = ("", '[[bin]]\nname = "fixbin"\npath = "src/main.rs"\n', {"src/main.rs": MAIN})
        for edition, workspace in (
            ("edition = 2021\n", None),
            ("edition.workspace = true\n", None),
            ("edition.workspace = true\n", "[workspace]\n"),
            ("edition.workspace = true\n", "[workspace\n"),
        ):
            with self.subTest(edition=edition, workspace=workspace):
                unread = Member(self, "planted", [table], edition=edition, workspace=workspace)
                with self.assertRaisesRegex(
                    runner.KillerUnresolved,
                    f"^crates/{PACKAGE} sets an edition the reader cannot decide$",
                ):
                    runner.locate_killer(unread.root, killer_row(runner))
        block = self.member(("", "", {"src/main.rs": "fn main() {\n    mod x;\n}\n"}))
        with self.assertRaisesRegex(runner.KillerUnresolved, "declares mod x inside a block"):
            runner.module_sources(block.crate / "src/main.rs")
        for text, pattern in {
            '#![cfg(feature = "f")]\nmod x;\n': "carries an inner cfg attribute",
            "#![cfg(test)]\n": "carries an inner cfg attribute",
            "mod x fn;\n": "a mod declaration the reader",
            "#[cfg = all(test)]\nmod x;\n": "holds a cfg on mod x the reader cannot decide",
            '#[path = "q.rs"]\nfn f() {\n    mod a;\n}\n': "declares mod a inside a block",
            "#[cfg(not(test))]\nfn f() {\n    mod a;\n}\n": "declares mod a inside a block",
        }.items():
            with self.subTest(text=text):
                planted = self.member(("", "", {"src/main.rs": text, "src/x.rs": LEAF}))
                with self.assertRaisesRegex(runner.KillerUnresolved, pattern):
                    runner.module_sources(planted.crate / "src/main.rs")
        for predicate in ("not()", "not(test, test)", 'not(feature = "f")'):
            with self.subTest(predicate=predicate):
                self.assertIsNone(runner.cfg_value(runner.rust_tokens(predicate)))
        weird = self.member(("", "", {"src/main.rs": "mod ;\n"}))
        with self.assertRaisesRegex(runner.KillerUnresolved, "a mod declaration the reader"):
            runner.module_sources(weird.crate / "src/main.rs")

    def test_a_hash_not_opening_an_attribute_changes_nothing_the_reader_returns(self):
        runner = runner_module()
        shapes = {
            "a hash as the last token": ("mod x;\n#", ["src/main.rs", "src/x.rs"]),
            "an inner hash bang as the last token": ("mod x;\n#!", ["src/main.rs", "src/x.rs"]),
            "a hash before a declaration": ("# mod x;\n", ["src/main.rs", "src/x.rs"]),
            "a hash bang before a declaration": ("#!mod x;\n", ["src/main.rs", "src/x.rs"]),
            "an inner attribute before a path module": (
                '#![allow(dead_code)]\n#[path = "impl/x.rs"]\nmod x;\n',
                ["src/main.rs"],
            ),
            "a hash before a bracket on the next line": (
                '#\n[path = "impl/x.rs"]\nmod x;\n',
                ["src/main.rs"],
            ),
        }
        for name, (text, expected) in shapes.items():
            with self.subTest(shape=name):
                member = self.member(("", "", {"src/main.rs": text, "src/x.rs": LEAF}))
                try:
                    read = runner.module_sources(member.crate / "src/main.rs")
                except IndexError as raised:
                    self.fail(f"the reader ran off the end of the tokens: {raised!r}")
                sources = [p.relative_to(member.crate).as_posix() for p in read]
                self.assertEqual(sources, expected)

    def test_a_nested_block_comment_is_dropped_whole_and_nothing_after_it_is(self):
        runner = runner_module()
        """Rust nests block comments, so the tokens of source with one are exactly the tokens
        outside it: depth two and three, one never closed, a lone slash after an opener, and
        one directly followed by an attribute."""
        shapes = {
            "depth two": ("a /* x /* y */ z */ b", ["a", "b"]),
            "depth three": ("a /* /* /* */ */ */ b", ["a", "b"]),
            "never closed": ("a /* /* x */ b", ["a"]),
            "a slash after the opener": ("a /*/ x */ b", ["a", "b"]),
            "followed by an attribute": ("/* /* */ */ #[a] b", ["#", "[", "a", "]", "b"]),
        }
        for name, (text, expected) in shapes.items():
            with self.subTest(shape=name):
                self.assertEqual(runner.rust_tokens(text), expected)
        print(f"nested block comment shapes examined {len(shapes)}")

    def test_a_block_comment_left_open_drops_every_character_to_the_end_of_the_text(self):
        """A block comment that never closes runs to the end of the text, so the tokens are those
        before it, however short or long the tail after its opener. The scan's bound is decided
        at the end of a text, so the tails are every string of slashes, stars, a letter, a space
        and a newline up to five characters that closes no comment: a bound that stops the scan
        short leaves a tail character behind as a token."""
        runner = runner_module()
        heads = {
            "": [],
            "a ": ["a"],
            "x = 1;\n": ["x", "=", "1", ";"],
            "/* c */ b ": ["b"],
        }
        texts = []
        for (head, expected), length in itertools.product(heads.items(), range(6)):
            for letters in itertools.product("x /*\n", repeat=length):
                tail = "".join(letters)
                if "*/" not in tail:
                    texts.append((f"{head}/*{tail}", expected))
        for text, expected in examined("block comments left open", texts):
            if runner.rust_tokens(text) != expected:
                self.fail(f"{text!r} reads {runner.rust_tokens(text)}, where {expected} is due")


class TheCfgPredicatesAgreeWithTheCompiler(unittest.TestCase):
    """SPEC-039 section 19: a predicate the reader decides is the value rustc gives it."""

    ATOMS = (
        "test",
        "unix",
        "windows",
        'feature = "f"',
        "zzz",
        "all",
        'any = "x"',
        'test = "x"',
    )

    def predicates(self):
        """Every predicate over the atoms under not, all and any to a depth of two."""
        level = list(self.ATOMS)
        found = list(level)
        for _ in range(2):
            grown = []
            for first in level:
                grown.append(f"not({first})")
                grown.append(f"all({first})")
                grown.append(f"any({first})")
                for second in self.ATOMS:
                    grown.append(f"all({first}, {second})")
                    grown.append(f"any({first}, {second})")
            found += grown
            level = [g for g in grown if g.startswith("not(") or len(g) < 24]
            if len(found) > 3000:
                break
        found += ["all()", "any()", "all(test,)", "any(test,)", "not(not(test))"]
        return sorted(set(found))

    def test_every_decided_predicate_is_what_rustc_builds_and_the_rest_is_undecided(self):
        runner = runner_module()
        predicates = self.predicates()
        with tempfile.TemporaryDirectory() as raw:
            crate = Path(raw)
            lines = []
            for number, predicate in enumerate(predicates):
                lines.append(f"#[cfg({predicate})]\nmod m{number};\n")
                (crate / f"m{number}.rs").write_text("", encoding="utf-8")
            (crate / "main.rs").write_text("".join(lines), encoding="utf-8")
            done = subprocess.run(
                ["rustc", "--edition", "2021", "--test", "--crate-name", "preds"]
                + ["--emit=dep-info", "--out-dir", str(crate), str(crate / "main.rs")],
                cwd=crate,
                capture_output=True,
                text=True,
                check=False,
                timeout=300,
            )
            self.assertEqual(done.returncode, 0, done.stderr)
            built = {
                line[:-1]
                for line in (crate / "preds.d").read_text(encoding="utf-8").splitlines()
                if line.endswith(":")
            }
        decided = 0
        for number, predicate in enumerate(predicates):
            value = runner.cfg_value(runner.rust_tokens(predicate))
            truth = any(name.endswith(f"m{number}.rs") for name in built)
            over_test_only = self.over_test_only(runner.rust_tokens(predicate))
            with self.subTest(predicate=predicate):
                if over_test_only:
                    self.assertIs(value, truth)
                if value is not None:
                    decided += 1
                    self.assertIs(value, truth)
        examined("predicates", predicates)
        print(f"{decided} decided, every one equal to rustc's value")

    @staticmethod
    def over_test_only(tokens):
        """True when every name in the predicate is `test` or a not, all or any call."""
        for index, token in enumerate(tokens):
            after = tokens[index + 1 : index + 2]
            if token in ("not", "all", "any") and after == ["("]:
                continue
            if token == "test" and after != ["="]:
                continue
            if token not in ("(", ")", ","):
                return False
        return True


if __name__ == "__main__":
    unittest.main()
