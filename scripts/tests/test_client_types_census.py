"""The client types census (SPEC-654, ADR-676).

It reads, as text, the request-handler routes of `crates/api/src`, the web client's read bindings in
`web/app/src/lib/api.ts` with the modules their parsers come from, and every Swift file under `ios/`,
and pairs each route with each client type that mirrors it, by method and path. A pair is equal
when the two key sets are the same, drifted when they differ (each differing key named) and
unmeasured when one side's key set is empty; a route no client type mirrors and a client type with
no route are unmatched. A finding is any pair that is not equal. At `LEVEL` warn an unlisted finding
prints its WARN line and passes, and at fail it fails; a listed entry no finding matches is stale and
fails at both levels. Every key set is a lower bound: a key built outside the forms below is unseen.

Match forms:
  SERVER-ROUTE   `.route(CONST, method(handler)..)` in a file directly under `crates/api/src`, CONST
                 resolved by `const CONST: &str = ".."` in the same file.
  SERVER-ANSWER  every `"key":` inside a `json!(..)` the handler reaches (its body and the same
                 file's functions it calls, transitively), less an object whose only key is
                 `reason`; every `["key"] =` assignment; the members of a `const NAME: [&str; N]`
                 the body names; and the fields of a `Json(T {..})` whose struct is in the file.
  WEB-BINDING    every `read(PATH, PARSER)` call in api.ts: a binding's body, an awaited read in an
                 async binding and a read whose answer is chained alike. PATH is a literal, a
                 template whose each `${..}` is one `{..}` segment, a const exported by the
                 parser's module, or a call, read as that module's `_PATH` const. The parser is
                 PARSER when it is a `parseX` name, else the first `parseX` its text names.
  WEB-READ       every key destructured from, or read by `.key` or `['key']` on, a value cast
                 `as Record<string, unknown>` in parseX and the same module's functions it calls; a
                 computed index reads the members of a const list the body names.
  SWIFT-MIRROR   a `struct T: ..` conforming to `Decodable` or `Codable` in a Swift file whose text
                 holds exactly one `"/api/.."` literal; its keys are its `let` and `var` names, and it
                 mirrors GET of that path.
  PAIR           a route and a client type with the same method and path, where a `{..}`
                 segment meets any other `{..}` segment.
"""

import re
import unittest
from pathlib import Path

from _support import REPO

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "client-types"
API_SRC = "crates/api/src"
WEB_LIB = "web/app/src/lib"
IOS = "ios"
WARN = "warn"
FAIL = "fail"
LEVEL = WARN

NO_CLIENT = (
    "no read( call in api.ts and no Swift type mirrors this route; "
    "the web client sends it by hand, from another module, or not at all"
)
LISTED = {
    "DELETE /api/identities/{id} server": ("unmatched", NO_CLIENT),
    "DELETE /api/session server": ("unmatched", NO_CLIENT),
    "GET /api/analytics/days server": ("unmatched", NO_CLIENT),
    "GET /api/chests server": ("unmatched", NO_CLIENT),
    "GET /api/companion/v1/manifest.json server": ("unmatched", NO_CLIENT),
    "GET /api/companion/v1/module.wasm server": ("unmatched", NO_CLIENT),
    "GET /api/companion/v1/weights.bin server": ("unmatched", NO_CLIENT),
    "GET /api/governor web": (
        "drifted unread=lapse,standby",
        "the client reads the verdict at api.ts:250, never the standby and lapse flags",
    ),
    "GET /api/identities web": (
        "unmeasured",
        "the answer is a list typed in another crate and the parser reads outside the named forms",
    ),
    "GET /api/insights web": (
        "unmeasured",
        "the parser reads its keys through a helper and optional chaining",
    ),
    "GET /api/insights/{id} web": (
        "unmeasured",
        "read at api.ts:264 by a parser that reads its keys through a helper",
    ),
    "GET /api/livez server": ("unmatched", NO_CLIENT),
    "GET /api/milestone server": ("unmatched", NO_CLIENT),
    "GET /api/notifications/feed web": (
        "drifted missing=kind,text,tier",
        "the items are a type of the notifications crate, which this census does not read",
    ),
    "GET /api/readyz server": ("unmatched", NO_CLIENT),
    "GET /api/score web": (
        "drifted unread=reviews",
        "the client shows no review count",
    ),
    "GET /api/streak web": (
        "drifted missing=current,freeze_cap,freezes,heat,longest",
        "read at api.ts:248; the handler puts these keys in a map, outside SERVER-ANSWER",
    ),
    "GET /api/sync/minimum-client server": ("unmatched", NO_CLIENT),
    "POST /api/chests/{id}/open server": ("unmatched", NO_CLIENT),
    "POST /api/chests/{id}/pick server": ("unmatched", NO_CLIENT),
    "POST /api/data/export server": ("unmatched", NO_CLIENT),
    "POST /api/inbox/captures server": ("unmatched", NO_CLIENT),
    "POST /api/insights/{id}/run server": ("unmatched", NO_CLIENT),
    "POST /api/launch server": ("unmatched", NO_CLIENT),
    "POST /api/link/code server": ("unmatched", NO_CLIENT),
    "POST /api/link/redeem server": ("unmatched", NO_CLIENT),
    "POST /api/passkeys/register/finish server": ("unmatched", NO_CLIENT),
    "POST /api/passkeys/register/start server": ("unmatched", NO_CLIENT),
    "POST /api/passkeys/sign-in/finish server": ("unmatched", NO_CLIENT),
    "POST /api/passkeys/sign-in/start server": ("unmatched", NO_CLIENT),
    "POST /api/session server": ("unmatched", NO_CLIENT),
    "POST /api/sync/seal-key server": ("unmatched", NO_CLIENT),
    "POST /api/tokens/activate server": ("unmatched", NO_CLIENT),
    "PUT /api/decks/{id}/sensitive server": ("unmatched", NO_CLIENT),
}

KEY = r"[A-Za-z_][A-Za-z0-9_]*"
RUST_FN = re.compile(r"^\s*(?:pub(?:\([a-z]+\))?\s+)?(?:async\s+)?fn\s+([a-z_0-9]+)", re.M)
TS_FN = re.compile(r"^(?:export\s+)?(?:async\s+)?function\s+([A-Za-z0-9_]+)", re.M)
SWIFT_STRUCT = re.compile(
    r"^\s*(?:(?:public|internal|fileprivate|private)\s+)?struct\s+([A-Z]\w*)\s*:([^{]*)\{",
    re.M,
)
RECORD = r"as\s+Record<string,\s*unknown>"


def blank(text):
    """The text with comments and string contents blanked, so brace and paren counts see code."""
    out = []
    i, n = 0, len(text)
    while i < n:
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("".join(c if c == "\n" else " " for c in text[i:j]))
            i = j
        elif text[i] == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append('"' + "_" * (j - i - 1) + '"')
            i = j + 1
        elif text[i] == "'" and i + 2 < n and text[i + 2] == "'" and text[i + 1] in "{}()[]":
            out.append("'_'")
            i += 3
        else:
            out.append(text[i])
            i += 1
    return "".join(out)


def block_end(code, start):
    """The index just past the brace block that opens at or after `start`."""
    depth = 0
    for j in range(code.index("{", start), len(code)):
        if code[j] == "{":
            depth += 1
        elif code[j] == "}":
            depth -= 1
            if depth == 0:
                return j + 1
    return len(code)


def paren_end(code, at):
    depth = 0
    for j in range(at, len(code)):
        if code[j] in "([{":
            depth += 1
        elif code[j] in ")]}":
            depth -= 1
            if depth == 0:
                return j + 1
    return len(code)


def line_of(text, at):
    return text.count("\n", 0, at) + 1


def answer_keys(text, code, lo, hi):
    """SERVER-ANSWER's keys inside one function's span."""
    keys = set()
    for m in re.finditer(r"json!\(", code[lo:hi]):
        start = lo + m.end() - 1
        found = set(re.findall(r'"(' + KEY + r')"\s*:', text[start : paren_end(code, start)]))
        if found == {"reason"}:
            continue
        keys |= found
    body = text[lo:hi]
    keys |= set(re.findall(r'\["(' + KEY + r')"\]\s*=[^=]', body))
    for name, members in re.findall(
        r"const\s+([A-Z_0-9]+)\s*:\s*\[&str;\s*\d+\]\s*=\s*\[([^\]]*)\]", text
    ):
        if re.search(r"\b" + name + r"\b", body):
            keys |= set(re.findall(r'"(' + KEY + r')"', members))
    for name in re.findall(r"Json\(\s*([A-Z][A-Za-z0-9_]+)\s*\{", body):
        m = re.search(r"struct\s+" + name + r"\s*\{", text)
        if m:
            fields = text[m.end() : text.index("}", m.end())]
            keys |= set(
                re.findall(
                    r"^\s*(?:pub(?:\([a-z]+\))?\s+)?([a-z_][a-z0-9_]*)\s*:",
                    fields,
                    re.M,
                )
            )
    return keys


def server(root):
    """SERVER-ROUTE and SERVER-ANSWER over the files directly under `crates/api/src`."""
    folder = Path(root) / API_SRC
    files = sorted(p for p in folder.glob("*.rs") if p.is_file()) if folder.is_dir() else []
    routes = []
    for path in files:
        text = path.read_text(encoding="utf-8")
        code = blank(text)
        consts = dict(re.findall(r'const\s+([A-Z_0-9]+)\s*:\s*&str\s*=\s*"([^"]*)"', text))
        fns = {m.group(1): (m.start(), block_end(code, m.end())) for m in RUST_FN.finditer(code)}
        handlers = set(re.findall(r"\b(?:get|post|put|delete|patch)\(\s*([a-z_0-9]+)\s*\)", code))
        for rm in re.finditer(r"\.route\(\s*([A-Z_0-9]+)\s*,\s*", code):
            spec = code[rm.end() : paren_end(code, rm.start() + len(".route")) - 1]
            for mm in re.finditer(r"(get|post|put|delete|patch)\(\s*([a-z_0-9]+)\s*\)", spec):
                keys, seen, todo = set(), set(), [mm.group(2)]
                while todo:
                    name = todo.pop()
                    if name in seen or name not in fns:
                        continue
                    seen.add(name)
                    lo, hi = fns[name]
                    keys |= answer_keys(text, code, lo, hi)
                    for callee in re.findall(r"\b([a-z_][a-z0-9_]*)\b", code[lo + 3 : hi]):
                        if callee in fns and callee not in seen and callee not in handlers:
                            todo.append(callee)
                routes.append(
                    dict(
                        method=mm.group(1).upper(),
                        path=consts.get(rm.group(1), "?" + rm.group(1)),
                        where=f"{API_SRC}/{path.name}:{line_of(text, rm.start())}",
                        keys=keys,
                    )
                )
    return files, routes


def ts_module(path):
    text = path.read_text(encoding="utf-8")
    code = blank(text.replace("'", '"').replace("`", '"'))
    fns = {}
    for m in TS_FN.finditer(code):
        fns[m.group(1)] = (
            m.start(),
            block_end(code, paren_end(code, code.index("(", m.end() - 1))),
        )
    consts = dict(re.findall(r"export\s+const\s+([A-Z_0-9]+)\s*=\s*'([^']*)'", text))
    return text, code, fns, consts


def web_reads(text, code, fns, start):
    """WEB-READ's keys from the parser `start` and the same module's functions it calls."""
    keys, seen, todo = set(), set(), [start]
    while todo:
        name = todo.pop()
        if name in seen or name not in fns:
            continue
        seen.add(name)
        lo, hi = fns[name]
        body = text[lo:hi]
        records = set(
            re.findall(
                r"(?:const|let)\s+([a-zA-Z_]\w*)\s*=\s*[^;=]*?\s+" + RECORD + r"\s*;",
                body,
            )
        )
        for dm in re.finditer(
            r"(?:const|let)\s*\{([^}]*)\}\s*=\s*([^;]*?" + RECORD + r"|[a-zA-Z_]\w*)\s*;",
            body,
        ):
            source = dm.group(2).strip()
            if "Record<string" in source or source in records:
                for part in dm.group(1).split(","):
                    key = part.split(":")[0].split("=")[0].strip()
                    if re.fullmatch(KEY, key):
                        keys.add(key)
        keys |= set(re.findall(r"\([^()]*?\s+" + RECORD + r"\)\.(" + KEY + ")", body))
        for record in records:
            keys |= set(re.findall(r"\b" + re.escape(record) + r"\.(" + KEY + ")", body))
            keys |= set(re.findall(r"\b" + re.escape(record) + r"\[\s*'(" + KEY + r")'\s*\]", body))
            if re.search(r"\b" + re.escape(record) + r"\[\s*[a-z]\w*\s*\]", body):
                for listed, members in re.findall(
                    r"const\s+([A-Z_0-9]+)(?:\s*:[^=]*)?\s*=\s*\[([^\]]*)\]", text
                ):
                    if re.search(r"\b" + listed + r"\b", body):
                        keys |= set(re.findall(r"'(" + KEY + ")'", members))
        for callee in re.findall(r"\b([a-zA-Z_]\w*)\b", code[lo + 8 : hi]):
            if callee in fns and callee not in seen:
                todo.append(callee)
    return keys


def split_args(code, lo, hi):
    """The spans of the top-level arguments between `lo` and `hi` in blanked code."""
    depth, start, spans = 0, lo, []
    for j in range(lo, hi):
        if code[j] in "([{":
            depth += 1
        elif code[j] in ")]}":
            depth -= 1
        elif code[j] == "," and depth == 0:
            spans.append((start, j))
            start = j + 1
    spans.append((start, hi))
    return spans


def shape(path):
    """The path with each `{..}` segment emptied, so a template's `${..}` meets a route's `{id}`."""
    return re.sub(r"\{[^}]*\}", "{}", path)


def web(root):
    """WEB-BINDING and WEB-READ over api.ts and the modules its parsers come from."""
    lib = Path(root) / WEB_LIB
    api = lib / "api.ts"
    if not api.is_file():
        return []
    text = api.read_text(encoding="utf-8")
    code = blank(text.replace("'", '"').replace("`", '"'))
    imports = {}
    for im in re.finditer(r"import\s*(?:type\s*)?\{([^}]*)\}\s*from\s*'\./([^']+)'", text):
        for part in im.group(1).split(","):
            name = part.replace("type ", "").strip()
            if name:
                imports[name] = im.group(2)
    modules, bindings = {}, []
    for bm in re.finditer(r"(?<![\w.$])read\(", code):
        spans = split_args(code, bm.end(), paren_end(code, bm.end() - 1) - 1)
        arg, given = ([text[lo:hi].strip() for lo, hi in spans] + [""])[:2]
        named = re.search(r"\b(parse[A-Z]\w*)\b", given)
        parser = named.group(1) if named else ""
        module = imports.get(parser, "api")
        if module not in modules:
            modules[module] = ts_module(lib / (module + ".ts"))
        mtext, mcode, fns, consts = modules[module]
        if arg[:1] in ("'", "`"):
            path = re.sub(r"\$\{[^}]*\}", "{}", arg[1:-1])
        elif arg in consts:
            path = consts[arg]
        elif "(" in arg:
            path = next((v for k, v in consts.items() if k.endswith("_PATH")), "?" + arg)
        else:
            path = "?" + arg
        bindings.append(
            dict(
                side="web",
                method="GET",
                path=path,
                where=f"{WEB_LIB}/api.ts:{line_of(text, bm.start())}",
                keys=web_reads(mtext, mcode, fns, parser),
            )
        )
    return bindings


def swift(root):
    """SWIFT-MIRROR over every Swift file under `ios/`."""
    folder = Path(root) / IOS
    files = sorted(p for p in folder.rglob("*.swift") if p.is_file()) if folder.is_dir() else []
    mirrors = []
    for path in files:
        text = path.read_text(encoding="utf-8")
        paths = set(re.findall(r'"(/api/[^"\s]*)"', text))
        if len(paths) != 1:
            continue
        code = blank(text)
        for m in SWIFT_STRUCT.finditer(code):
            if not re.search(r"\b(?:Codable|Decodable)\b", m.group(2)):
                continue
            body = text[m.end() : block_end(code, m.end() - 1) - 1]
            mirrors.append(
                dict(
                    side="swift",
                    method="GET",
                    path=next(iter(paths)),
                    where=f"{path.relative_to(root).as_posix()}:{line_of(text, m.start())}",
                    keys=set(
                        re.findall(
                            r"^\s*(?:public\s+)?(?:let|var)\s+([a-z_]\w*)\s*:",
                            body,
                            re.M,
                        )
                    ),
                )
            )
    return files, mirrors


def census(root):
    """Every pair's finding, keyed `METHOD PATH side`, and the count of equal pairs."""
    server(root)
    web(root)
    swift(root)
    return {}, 0


def verdict(findings, listed, level):
    """The printed lines and the failures: an unlisted finding warns at warn and fails at fail, and a
    listed entry no finding matches is stale and fails at both."""
    return [], []


class PlantedTrees(unittest.TestCase):
    def test_an_equal_pair_reads_no_finding(self):
        findings, equal = census(FIXTURES / "equal")
        self.assertEqual(equal, 2)
        self.assertEqual(findings, {})

    def test_a_drifted_field_is_named_warns_at_warn_and_fails_at_fail(self):
        findings, equal = census(FIXTURES / "drifted")
        self.assertEqual(
            findings,
            {
                "GET /api/tally web": (
                    "drifted unread=extra",
                    "web/app/src/lib/api.ts:4",
                )
            },
        )
        self.assertEqual(equal, 0)
        line = "GET /api/tally web: drifted unread=extra at web/app/src/lib/api.ts:4"
        self.assertEqual(verdict(findings, {}, WARN), (["WARN " + line], []))
        self.assertEqual(verdict(findings, {}, FAIL), ([], ["FAIL " + line]))

    def test_an_unmatched_client_type_and_its_route_are_found(self):
        findings, equal = census(FIXTURES / "unmatched")
        self.assertEqual(
            findings,
            {
                "GET /api/elsewhere web": ("unmatched", "web/app/src/lib/api.ts:4"),
                "GET /api/tally server": (
                    "unmatched",
                    "crates/api/src/tally_routes.rs:4",
                ),
            },
        )
        self.assertEqual(equal, 0)

    def test_a_listed_finding_reads_listed_and_a_stale_entry_fails_at_both_levels(self):
        findings = {"GET /api/tally web": ("drifted unread=extra", "web/app/src/lib/api.ts:4")}
        listed = {"GET /api/tally web": ("drifted unread=extra", "planted")}
        self.assertEqual(
            verdict(findings, listed, FAIL),
            (["LISTED GET /api/tally web: drifted unread=extra (planted)"], []),
        )
        stale = ["STALE GET /api/tally web: listed drifted unread=extra, found none"]
        self.assertEqual(verdict({}, listed, WARN), ([], stale))
        self.assertEqual(verdict({}, listed, FAIL), ([], stale))

    def test_an_empty_population_is_void(self):
        with self.assertRaises(AssertionError) as caught:
            census(FIXTURES / "empty")
        self.assertEqual(
            str(caught.exception),
            "examined 0 handler file(s): the population is empty, so nothing was judged",
        )

    def test_a_read_inside_an_async_binding_is_paired(self):
        findings, equal = census(FIXTURES / "awaited-read")
        self.assertEqual(findings, {})
        self.assertEqual(equal, 1)

    def test_a_read_with_a_template_path_is_paired(self):
        findings, equal = census(FIXTURES / "template-path")
        self.assertEqual(findings, {})
        self.assertEqual(equal, 1)


class TheTree(unittest.TestCase):
    def test_the_tree_holds_todays_listing_at_warn(self):
        findings, equal = census(REPO)
        self.assertEqual(LEVEL, WARN)
        lines, failed = verdict(findings, LISTED, LEVEL)
        self.assertEqual(failed, [])
        self.assertGreater(equal, 0)
        print("\n".join(lines))


if __name__ == "__main__":
    unittest.main()
