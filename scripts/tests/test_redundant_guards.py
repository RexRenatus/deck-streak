"""A condition that an enclosing condition already decides is reported at warn (SPEC-665).

A guard under another condition that can never change the outcome, because the enclosing
condition has already decided it, is dead logic: a reader takes it for a live branch, and an edit
to one copy leaves the other stale. This census reads every shipped source file of the
repository's languages (Rust, Python, TypeScript and Svelte, Swift and shell) as text, never
running, compiling or importing one (R1 to R3). It finds each condition that sits under an
enclosing one (R8) and is decided by it in one of three forms: the same expression or its negation
(R5), a comparison of the same name against a literal whose result the enclosing range fixes (R6),
and a presence check that the enclosing binding or early exit already proves (R7).

A value is followed only inside one function body, and only while nothing between the two checks,
or anywhere in a loop body that encloses the inner check and begins after the enclosing one,
assigns it, shadows it, borrows it mutably or hands it to a call; a condition that holds a call is
never decided, and a decided check on an attribute path is counted as undecided (R9, R10). So every
count is a lower bound. The census starts at warn (R13 to R15): an unlisted site prints
`WARN redundant-guard <key>` and passes, and a listed entry no site matches fails with
`STALE redundant-guard <key>` at both levels. Moving LEVEL to fail is its own delivery.
"""

import ast
import os
import re
import unittest
from pathlib import Path

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "redundant-guards"
RULE = "redundant-guard"
# R13: the level, and today's sites, each key with the reason it is kept (R12 spells a key).
LEVEL = "warn"
LISTED = {}

# R2: the generated directories a checkout may hold beside its tracked sources.
PRUNED = {
    ".build",
    ".git",
    ".svelte-kit",
    ".venv",
    "DerivedData",
    "__pycache__",
    "node_modules",
    "target",
}
# R3: the test runners' own file patterns.
WEB_TEST = re.compile(r"\.(?:test|spec)\.(?:js|ts)$|^web/app/tests/|/__tests__/")
IOS_TEST = re.compile(r"^ios/[^/]*Tests/|^ios/[^/]+/Tests/")
RUST_TEST_FILE = re.compile(r"^\s*#!\[cfg\(test\)\]", re.M)
RUST_TEST_MODULE = re.compile(
    r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;"
)

# R9, R10: the calls whose result never differs between two reads, the bodies that leave a
# block, and the keywords that open a function body.
PURE = {
    "is_some",
    "is_none",
    "is_ok",
    "is_err",
    "is_empty",
    "len",
    "matches",
    "isArray",
}
EXITS = {
    "return",
    "continue",
    "break",
    "throw",
    "panic",
    "unreachable",
    "fatalError",
    "exit",
}
BOUNDARY = {
    "rust": {"fn"},
    "typescript": {"function"},
    "svelte": {"function"},
    "swift": {"func", "init"},
}
# R9: the keywords whose body may run an inner check again after a later assignment.
LOOP_HEADS = {"while", "for", "loop", "do", "repeat"}
OPS = {"<", "<=", ">", ">=", "==", "!=", "===", "!=="}
ASSIGN = {"=", "+=", "-=", "*=", "/=", "%=", "|=", "&=", "^=", "<<=", ">>=", "++", "--"}
BINDERS = {"++", "--", "let", "mut", "var", "const", "&", "for", "inout"}
OPENERS = {
    "if",
    "while",
    "for",
    "switch",
    "catch",
    "return",
    "typeof",
    "guard",
    "match",
    "await",
}
NOT_ROOTS = {
    "let",
    "Some",
    "Ok",
    "Err",
    "None",
    "nil",
    "true",
    "false",
    "null",
    "undefined",
    "typeof",
}
TOKEN = re.compile(
    r"\s+|(?P<id>[A-Za-z_$][\w$]*!?)|(?P<num>\d[\w.]*)"
    r"|(?P<op>===|!==|\.\.=|<<=|>>=|&&|\|\||==|!=|<=|>=|=>|->|::|\?\?|\?\.|\+=|-=|\*=|/=|%=|\|=|&=|\^=|\+\+|--|\.\.|.)",
    re.S,
)
RUST_CHAR = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'")
RUST_RAW = re.compile(r'(?:b|br|r)(#*)"')
# R7: a fact that proves a value present, and the checks it decides.
PRESENT = re.compile(
    r"^(?:let (?:mut )?(?:Some|Ok) \( .* \) = (?P<a>.+)|(?P<b>.+) \. is_(?:some|ok) \( \)"
    r"|(?P<c>.+) (?:!==|!=) (?:undefined|null|nil)|let (?P<d>[\w$]+)|let [\w$]+ = (?P<e>.+)"
    r"|(?P<f>.+) is not None)$"
)
ABSENT_CHECK = re.compile(
    r"^(?:(?P<b>.+) \. is_(?:none|err) \( \)|(?P<c>.+) (?:===|==) (?:undefined|null|nil)|(?P<f>.+) is None)$"
)
PRESENT_CHECK = re.compile(
    r"^(?:(?P<b>.+) \. is_(?:some|ok) \( \)|(?P<c>.+) (?:!==|!=) (?:undefined|null|nil)|(?P<f>.+) is not None)$"
)
SCRIPT = re.compile(r"<script[^>]*>(.*?)</script>", re.S)
SVELTE_BLOCK = re.compile(r"\{(#if|:else if|:else|/if|#each|/each)\s*([^}]*)\}")
SHELL_IF = re.compile(r"^(if|elif)\s+(.*?);?\s*then\s*$|^(if|elif)\s+(.*?)\s*;\s*then")
SHELL_TEST = re.compile(r"^!?\s*(?:\[\[?\s|test\s)")
SHELL_KEEPS = re.compile(r"^(?:then|do|done|fi|else|echo|printf)\b|^$|^#|^[{}]$")
SHELL_FUNCTION = re.compile(r"^(?:function\s+)?([A-Za-z_][\w-]*)\s*\(\s*\)")


def language_of(rel):
    """The reader a repository path is read by, or None for test code and other files (R2, R3)."""
    name = rel.rsplit("/", 1)[-1]
    if name.endswith(".rs"):
        shipped = re.match(r"crates/[^/]+/(?:src/|build\.rs$)", rel) or rel.startswith("tools/")
        return "rust" if shipped else None
    if name.endswith(".py"):
        if re.match(r"scripts/[^/]+\.py$", rel) or rel.startswith("deploy/"):
            return "python"
        return "python" if rel.startswith("tools/") and not name.startswith("test_") else None
    if name.endswith((".ts", ".js", ".mjs", ".cjs", ".svelte")):
        if not rel.startswith("web/") or WEB_TEST.search(rel):
            return None
        return "svelte" if name.endswith(".svelte") else "typescript"
    if name.endswith(".swift"):
        return "swift" if rel.startswith("ios/") and not IOS_TEST.search(rel) else None
    return "shell" if name.endswith(".sh") else None


def test_modules(rel, text):
    """The Rust files a file makes test code: itself under `#![cfg(test)]`, and each out-of-line
    module it declares under `#[cfg(test)]` (R3)."""
    found = {rel} if RUST_TEST_FILE.search(text) else set()
    folder, _, name = rel.rpartition("/")
    stem = name.removesuffix(".rs")
    base = folder if stem in ("mod", "lib", "main") else f"{folder}/{stem}"
    for module in RUST_TEST_MODULE.findall(text):
        found |= {f"{base}/{module}.rs", f"{base}/{module}/mod.rs"}
    return found


def population(root=REPO):
    """Every shipped source file R2 names, as (path, language), less test code and fixtures."""
    fixtures = FIXTURES.relative_to(REPO).as_posix() + "/"
    listed = []
    for top, folders, files in os.walk(root):
        folders[:] = sorted(name for name in folders if name not in PRUNED)
        for name in sorted(files):
            rel = (Path(top) / name).relative_to(root).as_posix()
            language = language_of(rel)
            if language and not rel.startswith(fixtures):
                listed.append((rel, language))
    tests = set()
    for rel, language in listed:
        if language == "rust":
            tests |= test_modules(rel, (root / rel).read_text(encoding="utf-8", errors="replace"))
    return sorted(item for item in listed if item[0] not in tests)


class Census:
    """The sites found and the counts behind them (R4, R12)."""

    def __init__(self):
        self.under, self.undecided, self.sites, self.unread = 0, 0, [], []

    def site(self, where, line, form, context, outer):
        path, language, function = where
        self.sites.append((path, line, language, form, context, outer, function))


def census(root, files):
    """Read each (path, language) under root by its language's reader (R1, R4)."""
    found = Census()
    for rel, language in examined("file(s)", files):
        text = (root / rel).read_text(encoding="utf-8", errors="replace")
        try:
            READERS[language](found, rel, text)
        except (SyntaxError, RecursionError, IndexError, ValueError) as error:
            found.unread.append(f"unread {rel}: {type(error).__name__}")
    return found


def conditions(found):
    """Print the count of conditions read under an enclosing condition, refusing zero (R4)."""
    examined("condition(s) under an enclosing condition", range(found.under))
    return found


def keyed(sites):
    """Each site under its key `<file> <function> <form> #<ordinal>`, ordinals in line order (R12)."""
    counts, out = {}, {}
    for site in sorted(sites):
        stem = f"{site[0]} {site[6]} {site[3]}"
        counts[stem] = counts.get(stem, 0) + 1
        out[f"{stem} #{counts[stem]}"] = site
    return out


def verdict(sites, listed, level):
    """The printed lines and the failures for the found sites against a listing (R14)."""
    if level not in ("warn", "fail"):
        return [], [f"LEVEL {level!r} is neither warn nor fail"]
    found = keyed(sites)
    warned, failed = [], []
    for key, site in found.items():
        if key in listed:
            continue
        detail = f"{RULE} {key} (line {site[1]}, {site[2]}, {site[4]} under line {site[5]})"
        if level == "warn":
            warned.append(f"WARN {detail}")
        else:
            failed.append(f"FAIL {detail}")
    failed += [f"STALE {RULE} {key}" for key in listed if key not in found]
    return warned, failed


def tokens(text, lang):
    """[(kind, text, line)] with comments dropped and each string one token of kind 'str'."""
    out, i, n, line = [], 0, len(text), 1
    while i < n:
        c, two = text[i], text[i : i + 2]
        if two == "//":
            j = text.find("\n", i)
            i = n if j < 0 else j
            continue
        if two == "/*":
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j) and lang in ("rust", "swift"):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            line += text.count("\n", i, j)
            i = j
            continue
        raw = RUST_RAW.match(text, i) if lang == "rust" and c in "br" else None
        if raw and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            j = text.find('"' + raw.group(1), raw.end())
            j = n if j < 0 else j + 1 + len(raw.group(1))
            out.append(("str", text[i:j], line))
            line += text.count("\n", i, j)
            i = j
            continue
        if lang == "rust" and c == "'":
            char = RUST_CHAR.match(text, i)
            if char:
                out.append(("str", char.group(0), line))
                i = char.end()
                continue
        if c == '"' or (lang == "ts" and c in "'`"):
            if lang == "swift" and text.startswith('"""', i):
                j = text.find('"""', i + 3)
                j = n if j < 0 else j + 3
            else:
                j = i + 1
                while j < n and text[j] != c:
                    j += 2 if text[j] == "\\" else 1
                j += 1
            out.append(("str", text[i:j], line))
            line += text.count("\n", i, j)
            i = j
            continue
        match = TOKEN.match(text, i)
        if match.lastgroup is None:
            line += match.group(0).count("\n")
        else:
            out.append((match.lastgroup, match.group(0), line))
        i = match.end()
    return out


def depth_split(toks, seps):
    parts, cur, depth = [], [], 0
    for tok in toks:
        if tok[1] in "([{":
            depth += 1
        elif tok[1] in ")]}":
            depth -= 1
        if depth == 0 and tok[1] in seps:
            parts.append(cur)
            cur = []
        else:
            cur.append(tok)
    parts.append(cur)
    return [part for part in parts if part]


def strip_parens(toks):
    while len(toks) >= 2 and toks[0][1] == "(" and toks[-1][1] == ")":
        depth = 0
        for k, tok in enumerate(toks):
            depth += tok[1] == "("
            depth -= tok[1] == ")"
            if depth == 0 and k < len(toks) - 1:
                return toks
        toks = toks[1:-1]
    return toks


def norm(toks):
    return " ".join(tok[1] for tok in strip_parens(toks))


def negated(text):
    return text[2:] if text.startswith("! ") else "! " + text


def roots(toks):
    return {tok[1] for tok in toks if tok[0] == "id" and tok[1] not in NOT_ROOTS}


def has_call(toks):
    """R10: a conjunct that calls anything outside PURE is never decided."""
    if toks and toks[0][1] in ("let", "var", "case"):
        eq = [k for k, tok in enumerate(toks) if tok[1] == "="]
        toks = toks[eq[0] + 1 :] if eq else []
    for k in range(len(toks) - 1):
        if toks[k][0] == "id" and toks[k + 1][1] == "(" and toks[k][1].rstrip("!") not in PURE:
            if toks[k][1] not in ("if", "while", "let", "typeof"):
                return True
    return False


def comparison(toks):
    """R6: (path, op, ('n', number) or ('s', text)) for `path op literal` or `literal op path`."""
    toks = strip_parens(toks)
    for k, tok in enumerate(toks):
        if tok[1] in OPS and 0 < k < len(toks) - 1:
            op = {"===": "==", "!==": "!="}.get(tok[1], tok[1])
            for path, lit, flip in (
                (toks[:k], toks[k + 1 :], False),
                (toks[k + 1 :], toks[:k], True),
            ):
                if (
                    len(lit) == 1
                    and lit[0][0] in ("num", "str")
                    and all(x[0] == "id" or x[1] == "." for x in path)
                ):
                    if lit[0][0] == "num":
                        try:
                            value = (
                                "n",
                                float(lit[0][1].replace("_", "").rstrip("uifsz")),
                            )
                        except ValueError:
                            return None
                    else:
                        value = ("s", lit[0][1])
                    if flip:
                        op = {"<": ">", ">": "<", "<=": ">=", ">=": "<="}.get(op, op)
                    return (" ".join(x[1] for x in path), op, value)
            return None
    return None


def holds(op, x, v):
    return {
        "<": x < v,
        "<=": x <= v,
        ">": x > v,
        ">=": x >= v,
        "==": x == v,
        "!=": x != v,
    }[op]


def range_decides(fact, inner):
    """R6: True or False when the fact (path op a) fixes the inner (same path op w), else None."""
    (p1, o1, v1), (p2, o2, v2) = fact, inner
    if p1 != p2 or v1[0] != v2[0]:
        return None
    if v1[0] == "s":
        if o1 == "==" and o2 in ("==", "!="):
            return (v1[1] == v2[1]) == (o2 == "==")
        return None
    a, w = v1[1], v2[1]
    probes = {a - 1, a - 0.5, a, a + 0.5, a + 1, w - 1, w - 0.5, w, w + 0.5, w + 1}
    if o1 in ("<", "<=", ">", ">="):
        probes |= {a - 1e9, a + 1e9}
    seen = {holds(o2, x, w) for x in probes if holds(o1, x, a)}
    return seen.pop() if len(seen) == 1 else None


def present_of(text):
    match = PRESENT.match(text)
    return next((group for group in match.groups() if group), None) if match else None


def decide(facts, inner):
    """(form, enclosing line) when a fact decides the conjunct, else None (R5 to R7)."""
    text = norm(inner)
    cmp = comparison(inner)
    for ftext, ftoks, fline in facts:
        if text == ftext or negated(text) == ftext:
            return ("same-expression", fline)
        fcmp = comparison(ftoks) if ftoks else None
        if cmp and fcmp and range_decides(fcmp, cmp) is not None:
            return ("range", fline)
        present = present_of(ftext)
        for check in (ABSENT_CHECK, PRESENT_CHECK):
            match = check.match(text)
            if present and match and next(g for g in match.groups() if g) == present:
                return ("presence", fline)
    return None


def judge(found, where, context, facts, conjunct, live):
    """Count a conjunct under enclosing facts; record it when a live fact decides it (R8 to R10)."""
    if not facts:
        return
    found.under += 1
    if has_call(conjunct):
        return
    hit = decide(live, conjunct)
    if (
        hit
        and where[1] in ("typescript", "svelte", "swift")
        and any(t[1] in (".", "?.") for t in conjunct)
    ):
        found.undecided += 1
    elif hit:
        found.site(where, conjunct[0][2], hit[0], context, hit[1])


def touched(toks, lo, hi):
    """R9: names assigned, shadowed, borrowed mutably or handed to a call in toks[lo:hi]."""
    out = set()
    for k in range(max(lo, 0), min(hi, len(toks))):
        tok = toks[k]
        nxt = toks[k + 1][1] if k + 1 < len(toks) else ""
        prv = toks[k - 1][1] if k > 0 else ""
        if tok[0] != "id":
            continue
        if nxt in ASSIGN or prv in BINDERS:
            out.add(tok[1])
        if (
            nxt in (".", "?.")
            and k + 3 < len(toks)
            and toks[k + 3][1] == "("
            and toks[k + 2][1] not in PURE
        ):
            out.add(tok[1])
        if prv in ("(", ",") or nxt in (",", ")"):
            depth, j = 0, k - 1
            while j >= max(lo, 0):
                s = toks[j][1]
                if s in ")]}":
                    depth += 1
                elif s in "([{":
                    if depth == 0:
                        break
                    depth -= 1
                j -= 1
            opener = toks[j - 1] if j > 0 and j >= lo and toks[j][1] == "(" else None
            if (
                opener
                and (opener[0] == "id" or opener[1] in (")", "]"))
                and opener[1] not in OPENERS
            ):
                out.add(tok[1])
    return out


def brace_walk(found, path, language, toks):
    """R8, R9: walk a brace language's tokens; facts reach only the bodies they enclose."""
    rust, swift = language == "rust", language == "swift"
    heads = ("if", "while", "guard") if swift else ("if", "while")

    def close(k):
        depth = 0
        for j in range(k, len(toks)):
            depth += toks[j][1] in "([{"
            depth -= toks[j][1] in ")]}"
            if depth == 0:
                return j
        return len(toks) - 1

    def item_end(j, hi):
        depth = 0
        while j < hi:
            s = toks[j][1]
            if depth == 0 and s == ";":
                return j
            if depth == 0 and s == "{":
                return close(j)
            depth += s in "(["
            depth -= s in ")]"
            j += 1
        return hi

    def cond_span(k):
        if language in ("typescript", "svelte"):
            if k + 1 < len(toks) and toks[k + 1][1] == "(":
                end = close(k + 1)
                return k + 2, end, end + 1
            return None
        j, depth = k + 1, 0
        while j < len(toks):
            s = toks[j][1]
            if depth == 0 and (s == "{" or (swift and s == "else" and toks[k][1] == "guard")):
                return k + 1, j, j
            depth += s in "(["
            depth -= s in ")]"
            j += 1
        return None

    def body_end(o):
        if o < len(toks) and toks[o][1] == "{":
            return close(o)
        j, depth = o, 0
        while j < len(toks) and not (depth == 0 and toks[j][1] == ";"):
            depth += toks[j][1] in "([{"
            depth -= toks[j][1] in ")]}"
            j += 1
        return j

    def negation(parts, end):
        if len(parts) == 1 and not has_call(parts[0]):
            return [(negated(norm(parts[0])), None, parts[0][0][2], end)]
        return []

    names = {}
    for k, tok in enumerate(toks):
        if tok[1] in BOUNDARY[language] and not (k and toks[k - 1][1] == "."):
            named = k + 1 < len(toks) and toks[k + 1][0] == "id" and tok[1] != "init"
            name, depth = toks[k + 1][1] if named else tok[1], 0
            for j in range(k + 1, len(toks)):
                s = toks[j][1]
                if depth == 0 and s in ("{", ";"):
                    if s == "{":
                        names[j] = name
                    break
                depth += s in "(["
                depth -= s in ")]"

    def loop_body(k):
        """R9: where a loop's body opens; a `while` or `for` body opens after its condition."""
        if toks[k][1] in ("while", "for"):
            span = cond_span(k)
            return span[2] if span else None
        return k + 1 if k + 1 < len(toks) and toks[k + 1][1] == "{" else None

    spans = []
    for k, tok in enumerate(toks):
        if tok[1] in LOOP_HEADS and not (k > 0 and toks[k - 1][1] in (".", "#")):
            opening = loop_body(k)
            if opening is not None and opening < len(toks):
                end = body_end(opening)
                spans.append((k, end, touched(toks, opening, end)))

    def looped(point, at):
        """R9: names touched in a loop body begun after `point` whose condition or body holds `at`."""
        out = set()
        for head, hi, inside in spans:
            if point < head and head < at <= hi:
                out |= inside
        return out

    def block(lo, hi, facts, function):
        k, facts = lo, list(facts)
        while k < hi:
            s = toks[k][1]
            if rust and s == "#" and k + 1 < hi and toks[k + 1][1] == "[":
                shut = close(k + 1)
                skip = norm(toks[k + 2 : shut]) in ("cfg ( test )", "test")
                k = (item_end(shut + 1, hi) if skip else shut) + 1
                continue
            if s in heads and not (k > 0 and toks[k - 1][1] in (".", "#")):
                span = cond_span(k)
                if not span:
                    k += 1
                    continue
                c0, c1, bo = span
                parts = depth_split(toks[c0:c1], ("&&", ",") if swift else ("&&",))
                where, seen = (path, language, function), []
                for part in parts:
                    every = [f[:3] for f in facts]
                    live = [
                        f[:3]
                        for f in facts
                        if not (roots(part) & (touched(toks, f[3], c0) | looped(f[3], c0)))
                    ]
                    judge(
                        found,
                        where,
                        "chain" if seen else "nested",
                        every + seen,
                        part,
                        live + seen,
                    )
                    if not has_call(part):
                        seen.append((norm(part), part, part[0][2]))
                new = [(text, part, line, c1) for text, part, line in seen]
                if s == "guard":
                    end = body_end(c1 + 1)
                    block(c1 + 2, end, [], function)
                    facts += new
                    k = end + 1
                    continue
                end = body_end(bo)
                braced = toks[bo][1] == "{"
                inner = toks[bo + 1 : end] if braced else toks[bo:end]
                early = s == "if" and inner and inner[0][1].rstrip("!") in EXITS
                block(bo + 1 if braced else bo, end, facts + new, function)
                k = end + 1
                if k < hi and toks[k][1] == "else":
                    if k + 1 < hi and toks[k + 1][1] == "if":
                        k += 1
                        continue
                    other = body_end(k + 1)
                    block(k + 2, other, facts + negation(parts, c1), function)
                    k = other + 1
                elif early:
                    facts += negation(parts, end)
                continue
            if s == "{":
                shut = close(k)
                prev = toks[k - 1][1] if k > 0 else ""
                carry = prev in ("else", "loop", "unsafe", "do", "repeat") or (
                    rust and any(toks[j][1] == "for" for j in range(max(lo, k - 12), k))
                )
                block(
                    k + 1,
                    shut,
                    facts if carry and k not in names else [],
                    names.get(k, function),
                )
                k = shut + 1
                continue
            if s in BOUNDARY[language]:
                facts = []
            k += 1

    block(0, len(toks), [], "-")


def rust_read(found, path, text):
    brace_walk(found, path, "rust", tokens(text, "rust"))


def typescript_read(found, path, text):
    brace_walk(found, path, "typescript", tokens(text, "ts"))


def swift_read(found, path, text):
    brace_walk(found, path, "swift", tokens(text, "swift"))


def each_bound(cond):
    """R9: the names an `{#each}` block binds afresh on every pass (its item, index and key)."""
    toks = tokens(cond, "ts")
    at = max((k for k, tok in enumerate(toks) if tok[1] == "as"), default=len(toks))
    return {tok[1] for tok in toks[at + 1 :] if tok[0] == "id"}


def svelte_read(found, path, text):
    """R8, R9: the script blocks as TypeScript, then the template's `{#if}` and `{#each}` blocks."""
    keep = [" " if ch != "\n" else "\n" for ch in text]
    for block in SCRIPT.finditer(text):
        keep[block.start(1) : block.end(1)] = list(block.group(1))
    brace_walk(found, path, "svelte", tokens("".join(keep), "ts"))
    stack = []
    for block in SVELTE_BLOCK.finditer(text):
        kind, cond = block.group(1), block.group(2)
        line = text.count("\n", 0, block.start()) + 1
        if kind in ("/if", "/each"):
            if stack:
                stack.pop()
            continue
        if kind == "#each":
            stack.append(each_bound(cond))
            continue
        if kind == ":else":
            if stack and isinstance(stack[-1], list):
                stack[-1] = []
            continue
        if kind == ":else if" and stack:
            stack.pop()
        parts = depth_split([(a, b, line) for a, b, _ in tokens(cond, "ts")], ("&&",))
        facts, live, hidden = [], [], set()
        for level in reversed(stack):
            if isinstance(level, set):
                hidden |= level
                continue
            facts = level + facts
            live = [f for f in level if not (roots(f[1]) & hidden)] + live
        seen = []
        for part in parts:
            judge(
                found,
                (path, "svelte", "template"),
                "chain" if seen else "nested",
                facts + seen,
                part,
                live + seen,
            )
            if not has_call(part):
                seen.append((norm(part), part, line))
        stack.append(seen)


PY_OPS = {
    ast.Lt: "<",
    ast.LtE: "<=",
    ast.Gt: ">",
    ast.GtE: ">=",
    ast.Eq: "==",
    ast.NotEq: "!=",
}
PY_EXITS = (ast.Return, ast.Raise, ast.Continue, ast.Break)


def python_read(found, path, text):
    """R8, R9: walk a Python module's statements; a fact's stop is any store or call in its scope."""

    def operand(node):
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            kind = "str"
        elif (
            isinstance(node, ast.Constant)
            and isinstance(node.value, (int, float))
            and not isinstance(node.value, bool)
        ):
            kind = "num"
        else:
            kind = "id"
        return [(kind, ast.unparse(node), node.lineno)]

    def as_toks(node):
        if isinstance(node, ast.Compare) and len(node.ops) == 1 and type(node.ops[0]) in PY_OPS:
            op = PY_OPS[type(node.ops[0])]
            return operand(node.left) + [("op", op, node.lineno)] + operand(node.comparators[0])
        return [("id", text_of(node), node.lineno)]

    def text_of(node):
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.Not):
            return "! " + ast.unparse(node.operand)
        return ast.unparse(node)

    def calls(node):
        return any(
            isinstance(n, ast.Call)
            and not (isinstance(n.func, ast.Name) and n.func.id in ("isinstance", "len"))
            for n in ast.walk(node)
        )

    def names(node):
        return {n.id for n in ast.walk(node) if isinstance(n, ast.Name)}

    def assigned(stmts):
        out = set()
        for stmt in stmts:
            for n in ast.walk(stmt):
                if isinstance(n, ast.Name) and isinstance(n.ctx, (ast.Store, ast.Del)):
                    out.add(n.id)
                if isinstance(n, ast.Call):
                    for arg in [*n.args, *[keyword.value for keyword in n.keywords]]:
                        out |= names(arg)
                    if isinstance(n.func, ast.Attribute):
                        out |= names(n.func.value)
        return out

    def fact(node):
        toks = as_toks(node) if isinstance(node, ast.Compare) else None
        return (text_of(node), toks, node.lineno, [])

    def judge_py(context, facts, node, function):
        if not facts:
            return
        found.under += 1
        if calls(node):
            return
        live = [f[:3] for f in facts if not (names(node) & assigned(f[3]))]
        text = text_of(node)
        hit = decide(
            live,
            as_toks(node) if isinstance(node, ast.Compare) else [("id", text, node.lineno)],
        )
        if not hit:
            hit = next(
                (("same-expression", f[2]) for f in live if f[0] in (text, negated(text))),
                None,
            )
        if hit and any(isinstance(n, ast.Attribute) for n in ast.walk(node)):
            found.undecided += 1
        elif hit:
            found.site((path, "python", function), node.lineno, hit[0], context, hit[1])

    def chain(node, function):
        seen = []
        for part in node.values:
            if seen:
                judge_py("chain", seen, part, function)
            if not calls(part):
                seen.append(fact(part))

    def walk(stmts, facts, function):
        facts = list(facts)
        for i, stmt in enumerate(stmts):
            if isinstance(stmt, (ast.FunctionDef, ast.AsyncFunctionDef)):
                walk(stmt.body, [], stmt.name)
                continue
            if isinstance(stmt, ast.ClassDef):
                walk(stmt.body, [], function)
                continue
            for child in ast.iter_child_nodes(stmt):
                if isinstance(child, ast.stmt) or (
                    isinstance(stmt, (ast.If, ast.While)) and child is stmt.test
                ):
                    continue
                for node in ast.walk(child):
                    if isinstance(node, ast.BoolOp) and isinstance(node.op, ast.And):
                        chain(node, function)
            if isinstance(stmt, (ast.If, ast.While)):
                test = stmt.test
                parts = (
                    test.values
                    if isinstance(test, ast.BoolOp) and isinstance(test.op, ast.And)
                    else [test]
                )
                seen = []
                for part in parts:
                    judge_py("chain" if seen else "nested", facts + seen, part, function)
                    if not calls(part):
                        seen.append(fact(part))
                walk(
                    stmt.body,
                    facts + [(t, toks, line, stmt.body) for t, toks, line, _ in seen],
                    function,
                )
                neg = []
                if len(parts) == 1 and not calls(parts[0]):
                    neg = [(negated(text_of(parts[0])), None, parts[0].lineno, stmt.orelse)]
                walk(stmt.orelse, facts + neg, function)
                exits = len(stmt.body) == 1 and isinstance(stmt.body[0], PY_EXITS)
                if isinstance(stmt, ast.If) and exits and neg and not stmt.orelse:
                    facts.append((neg[0][0], None, neg[0][2], stmts[i + 1 :]))
                continue
            for body in (
                getattr(stmt, "body", None),
                getattr(stmt, "orelse", None),
                getattr(stmt, "finalbody", None),
            ):
                if isinstance(body, list) and body and isinstance(body[0], ast.stmt):
                    walk(body, facts, function)
            for handler in getattr(stmt, "handlers", None) or []:
                walk(handler.body, facts, function)
            for case in getattr(stmt, "cases", None) or []:
                walk(case.body, facts, function)

    walk(ast.parse(text).body, [], "-")


def shell_read(found, path, text):
    """R11: `if` and `elif` tests in brackets, nested or chained by `&&`; any other command clears."""
    stack, function = [], "-"
    for number, raw in enumerate(text.splitlines(), 1):
        line = raw.split(" #")[0].strip()
        opened = SHELL_FUNCTION.match(line)
        if opened:
            function = opened.group(1)
        if re.match(r"fi\b", line):
            if stack:
                stack.pop()
            continue
        if re.match(r"else\b", line):
            if stack:
                stack[-1] = []
            continue
        test = SHELL_IF.match(line)
        if not test:
            if not SHELL_KEEPS.match(line):
                for level in stack:
                    level.clear()
            continue
        cond = (test.group(2) or test.group(4) or "").strip()
        if "elif" in (test.group(1), test.group(3)) and stack:
            stack.pop()
        facts, seen = [f for level in stack for f in level], []
        for part in (p.strip() for p in re.split(r"\s&&\s", cond)):
            pure = bool(SHELL_TEST.match(part)) and "$(" not in part and "`" not in part
            if facts or seen:
                found.under += 1
                hit = decide(facts + seen, [("id", part, number)]) if pure else None
                if hit:
                    found.site(
                        (path, "shell", function),
                        number,
                        hit[0],
                        "chain" if seen else "nested",
                        hit[1],
                    )
            if pure:
                seen.append((part, None, number))
        stack.append(seen)


READERS = {
    "rust": rust_read,
    "python": python_read,
    "typescript": typescript_read,
    "svelte": svelte_read,
    "swift": swift_read,
    "shell": shell_read,
}


def marked(path, marker):
    """The (line, word) of each fixture line that ends with a `planted:` or `accepted:` comment."""
    lines = path.read_text(encoding="utf-8").splitlines()
    return [
        (number, match.group(1))
        for number, line in enumerate(lines, 1)
        for match in [re.search(rf" {marker}: ([a-z-]+)", line)]
        if match
    ]


class PlantedSitesAreFound(unittest.TestCase):
    def assert_planted(self, name, language, undecided=0):
        rel = f"scripts/tests/fixtures/redundant-guards/{name}"
        planted, accepted = (
            marked(REPO / rel, "planted"),
            marked(REPO / rel, "accepted"),
        )
        self.assertGreater(len(planted), 0, f"{name} plants no site")
        self.assertGreater(len(accepted), 0, f"{name} plants no accepted line")
        found = conditions(census(REPO, [(rel, language)]))
        self.assertEqual(found.unread, [])
        self.assertEqual(sorted((site[1], site[3]) for site in found.sites), planted)
        self.assertEqual(found.undecided, undecided)
        for site in found.sites:
            self.assertEqual((site[0], site[2]), (rel, language))

    def test_rust_planted_sites_are_found(self):
        self.assert_planted("rust-sites.rs.fixture", "rust")

    def test_python_planted_sites_are_found(self):
        self.assert_planted("python-sites.py.fixture", "python", undecided=1)

    def test_typescript_planted_sites_are_found(self):
        self.assert_planted("typescript-sites.ts.fixture", "typescript")

    def test_svelte_planted_sites_are_found(self):
        self.assert_planted("svelte-sites.svelte.fixture", "svelte")

    def test_swift_planted_sites_are_found(self):
        self.assert_planted("swift-sites.swift.fixture", "swift")

    def test_shell_planted_sites_are_found(self):
        self.assert_planted("shell-sites.sh.fixture", "shell")

    def test_the_population_reads_each_language_and_no_test_code(self):
        shipped = {
            "crates/api/src/routes.rs": "rust",
            "crates/api/build.rs": "rust",
            "tools/probe/src/main.rs": "rust",
            "scripts/release.py": "python",
            "deploy/hooks/apply.py": "python",
            "tools/oracle/oracle.py": "python",
            "web/app/src/lib/store.ts": "typescript",
            "web/app/src/lib/format.js": "typescript",
            "web/app/src/routes/+page.svelte": "svelte",
            "ios/App/Sources/Model.swift": "swift",
            "scripts/build-web.sh": "shell",
        }
        for rel, language in shipped.items():
            self.assertEqual(language_of(rel), language, rel)
        test_code = (
            "crates/api/tests/routes.rs",
            "crates/api/benches/load.rs",
            "scripts/tests/test_release.py",
            "tools/oracle/test_oracle.py",
            "web/app/src/lib/store.test.ts",
            "web/app/src/lib/store.spec.js",
            "web/app/tests/flow.ts",
            "web/app/src/__tests__/flow.ts",
            "ios/AppTests/ModelTests.swift",
            "ios/Kit/Tests/KitTests.swift",
            "vendor/lib.js",
            "scripts/tests/fixtures/redundant-guards/shell-sites.sh.fixture",
        )
        for rel in test_code:
            self.assertIsNone(language_of(rel), rel)
        declared = "#[cfg(test)]\nmod tests;\n"
        self.assertEqual(
            test_modules("crates/api/src/lib.rs", declared),
            {"crates/api/src/tests.rs", "crates/api/src/tests/mod.rs"},
        )
        self.assertEqual(
            test_modules("crates/api/src/store.rs", declared),
            {"crates/api/src/store/tests.rs", "crates/api/src/store/tests/mod.rs"},
        )
        self.assertEqual(
            test_modules("crates/api/src/t.rs", "#![cfg(test)]\nfn f() {}\n"),
            {"crates/api/src/t.rs"},
        )
        self.assertEqual(test_modules("crates/api/src/lib.rs", "mod tests;\n"), set())

    def test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does(
        self,
    ):
        site = (
            "crates/api/src/lib.rs",
            7,
            "rust",
            "same-expression",
            "nested",
            5,
            "check",
        )
        key = "crates/api/src/lib.rs check same-expression #1"
        detail = f"redundant-guard {key} (line 7, rust, nested under line 5)"
        self.assertEqual(verdict([site], {}, "warn"), ([f"WARN {detail}"], []))
        self.assertEqual(verdict([site], {}, "fail"), ([], [f"FAIL {detail}"]))
        self.assertEqual(verdict([site], {key: "kept"}, "fail"), ([], []))
        stale = {"crates/api/src/lib.rs check range #1": "gone"}
        for level in ("warn", "fail"):
            self.assertEqual(
                verdict([], stale, level),
                ([], ["STALE redundant-guard crates/api/src/lib.rs check range #1"]),
            )
        self.assertEqual(verdict([], {}, "loud"), ([], ["LEVEL 'loud' is neither warn nor fail"]))
        twice = [site, ("crates/api/src/lib.rs", 9) + site[2:]]
        self.assertEqual(list(keyed(twice)), [key, key.replace("#1", "#2")])

    def test_an_empty_population_is_refused(self):
        with self.assertRaisesRegex(AssertionError, r"examined 0 file\(s\)"):
            census(REPO, [])
        with self.assertRaisesRegex(
            AssertionError, r"examined 0 condition\(s\) under an enclosing condition"
        ):
            conditions(Census())


class TheRepositoryListsTodaysRedundantGuards(unittest.TestCase):
    def test_every_shipped_source_is_read_and_each_redundant_guard_is_listed(self):
        files = population()
        found = conditions(census(REPO, files))
        self.assertEqual(
            sorted({language for _, language in files}),
            ["python", "rust", "shell", "svelte", "swift", "typescript"],
        )
        self.assertEqual(found.unread, [], "\n".join(found.unread))
        print(f"undecided {found.undecided}")
        warned, failed = verdict(found.sites, LISTED, LEVEL)
        print("\n".join(warned))
        self.assertEqual(LEVEL, "warn")
        self.assertEqual(failed, [], "\n".join(failed))


if __name__ == "__main__":
    unittest.main()
