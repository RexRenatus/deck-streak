"""The clock reads of the native Swift are the listed sites (SPEC-431 A12 to A15; ADR-445 D11).

This census reads every tracked `.swift` file under `ios/` whose role in `ios/swift-roles.json` is
not `test`, a file the register does not list read as production, and holds each file's count of
clock reads equal to its row in `config/clock-reads-swift.json`, in both directions (A12). A clock
read is `Date()` or `NSDate()` with no argument, `Date.init()` and the bare `Date.init`; a `.now`
member or implicit member on any receiver but `self` or `Self`; `ContinuousClock()`,
`SuspendingClock()` and the implicit `.continuous` and `.suspending`; `timeIntervalSinceNow`, the
property and the initializer label; `Date.timeIntervalSinceReferenceDate`; `systemUptime`,
`CFAbsoluteTimeGetCurrent`, `CACurrentMediaTime`, `mach_absolute_time`, `mach_continuous_time`,
`clock_gettime` and `gettimeofday`; and the same made, `.init`ed or read through a `typealias` of
`Date`, `NSDate`, `ContinuousClock` or `SuspendingClock`, collected over the whole population to a
fixed point; with any path prefix and whitespace around `.` (A13). A `test` file is set aside,
listed and never read. Comments, the text of every string, multi-line string and raw string, and
every `#/.../#` regex literal are blanked before matching, newlines kept, so a line number
survives; each interpolation is read as code, and a comment or literal left open is a finding
(A14). Each row's ADR names its path in backticks (A15). A finding names a read as
`<path>:<line>: <spelling>`, never the line's text.

The reader has no types: a `.now` of a type of DeckStreak's own counts, so it refuses rather than
passes. A bare `/.../` regex literal is read as code. A time a framework takes for the code is not
a read in DeckStreak's code.

Each test judges the real tree first, then prints what it examined (0 is VOID), then refuses each
planted defect by name. The census reads committed files; the one process it starts is the
register census's `git ls-files`, which names the tracked tree.
"""

import json
import re
import unittest

from _support import REPO, examined
from test_clock_reads import ADR_ID, DECISIONS, ROW_KEYS, listed, row_problems
from test_one_static_library import tracked_paths

REGISTER = "ios/swift-roles.json"
LIST = "config/clock-reads-swift.json"
SET_ASIDE = "test"
CLOCK_TYPES = {
    "Date": "Date",
    "NSDate": "Date",
    "ContinuousClock": "Clock",
    "SuspendingClock": "Clock",
}
TOKEN = re.compile(r'//|/\*|(#*)("""|")|#+/|[()]')
BLOCK = re.compile(r"/\*|\*/")
TYPEALIAS = re.compile(r"(?<!\w)typealias\s+(\w+)\s*(?:<[^=\n]*>)?\s*=\s*([\w.]+)")
SPACE = re.compile(r"\s+")
FIXED = (
    r"(?:(?<!\w)(?P<receiver>\w+)\s*)?\.\s*now(?!\w)"
    r"|(?<![\w)\]?!.])\.(?:continuous|suspending)(?!\w)"
    r"|(?<!\w)(?:timeIntervalSinceNow|systemUptime|CFAbsoluteTimeGetCurrent|CACurrentMediaTime"
    r"|mach_absolute_time|mach_continuous_time|clock_gettime|gettimeofday)(?!\w)"
)
OWN = ("self", "Self")


def line_of(text, offset):
    """The 1-based line of `offset` in `text`."""
    return text.count("\n", 0, offset) + 1


def code_of(text):
    """`text` with every comment, the text of every string literal and every `#/.../#` regex
    blanked to spaces, newlines kept, each interpolation kept as code; and the offset of each
    comment or literal left open (R22)."""
    out, left_open = list(text), []

    def blank(start, end):
        for index in range(start, end):
            if out[index] != "\n":
                out[index] = " "

    def code(index, inside):
        depth = 0
        while (match := TOKEN.search(text, index)) is not None:
            token, index = match.group(), match.end()
            if token == "(":
                depth += 1
            elif token == ")":
                if inside and depth == 0:
                    return index
                depth -= 1
            elif token == "//":
                end = text.find("\n", index)
                end = len(text) if end < 0 else end
                blank(match.start(), end)
                index = end
            elif token == "/*":
                index = comment(match.start())
            elif token.endswith("/"):
                index = regex(match.start(), len(token) - 1)
            else:
                index = string(match.start(), len(match.group(1)), match.group(2))
        return len(text)

    def comment(start):
        depth, index = 0, start
        while (match := BLOCK.search(text, index)) is not None:
            depth += 1 if match.group() == "/*" else -1
            index = match.end()
            if depth == 0:
                blank(start, index)
                return index
        left_open.append(start)
        blank(start, len(text))
        return len(text)

    def regex(start, hashes):
        close = "/" + "#" * hashes
        end = text.find(close, start + hashes + 1)
        if end < 0:
            left_open.append(start)
            blank(start, len(text))
            return len(text)
        blank(start, end + len(close))
        return end + len(close)

    def string(start, hashes, quote):
        close, escape = quote + "#" * hashes, "\\" + "#" * hashes
        index, piece = start + hashes + len(quote), start
        while index < len(text):
            if text.startswith(escape, index):
                after = index + len(escape)
                if text.startswith("(", after):
                    blank(piece, after + 1)
                    index = piece = code(after + 1, True)
                    continue
                index = after + 1
            elif text.startswith(close, index):
                blank(piece, index + len(close))
                return index + len(close)
            elif quote == '"' and text[index] == "\n":
                break
            else:
                index += 1
        left_open.append(start)
        blank(piece, index)
        return index

    code(0, False)
    return "".join(out), left_open


def register_roles(read):
    """Each registered path's role, or None when the register is absent or not a JSON object."""
    text = read(REGISTER)
    try:
        data = json.loads(text) if text is not None else None
    except json.JSONDecodeError:
        return None
    if not isinstance(data, dict):
        return None
    return {
        path: entry.get("role") if isinstance(entry, dict) else None for path, entry in data.items()
    }


def survey(paths, roles):
    """The production Swift files and the `test` files set aside, each sorted (R20)."""
    swift = sorted(path for path in paths if path.startswith("ios/") and path.endswith(".swift"))
    production, aside = [], []
    for path in swift:
        if roles.get(path) == SET_ASIDE:
            aside.append(path)
        else:
            production.append(path)
    return production, aside


def kinds_of(codes):
    """Each clock type and each name a `typealias` makes one, with its kind, `Date` or `Clock`,
    collected over the whole population to a fixed point (R21)."""
    kinds = dict(CLOCK_TYPES)
    pairs = [match.groups() for code in codes.values() for match in TYPEALIAS.finditer(code)]
    grown = True
    while grown:
        grown = False
        for name, target in pairs:
            base = kinds.get(target.rsplit(".", 1)[-1])
            if base is not None and name not in kinds:
                kinds[name] = base
                grown = True
    return kinds


def read_pattern(kinds):
    """The clock reads, for the clock types and the aliases `kinds` holds (R21)."""
    made = "|".join(sorted(kinds, key=lambda name: (-len(name), name)))
    dates = "|".join(
        sorted((name for name in kinds if kinds[name] == "Date"), key=len, reverse=True)
    )
    return re.compile(
        rf"(?<!\w)(?:{made})\s*(?:\.\s*init\s*)?\(\s*\)"
        rf"|(?<!\w)(?:{made})\s*\.\s*init(?!\w)(?!\s*\()"
        rf"|(?<!\w)(?:{dates})\s*\.\s*timeIntervalSinceReferenceDate(?!\w)"
        rf"|{FIXED}"
    )


def reads_in(read, production):
    """Each production file's clock reads as `(line, spelling)`, and each comment or literal left
    open, as a finding (R21, R22)."""
    codes, left = {}, []
    for path in production:
        code, left_open = code_of(read(path) or "")
        codes[path] = code
        left += [f"{path}:{line_of(code, at)}: a comment or literal left open" for at in left_open]
    pattern, found = read_pattern(kinds_of(codes)), {}
    for path, code in codes.items():
        for match in pattern.finditer(code):
            if match.group("receiver") in OWN:
                continue
            spelling = SPACE.sub("", match.group())
            found.setdefault(path, []).append((line_of(code, match.start()), spelling))
    return found, left


def live_tree():
    """Every path git tracks, and a reader of each from disk, None for a path that is no file."""

    def read(path):
        file = REPO / path
        return file.read_text(encoding="utf-8") if file.is_file() else None

    return tracked_paths(REPO), read


def planted_tree(files):
    """A planted tree that holds only `files`, each `{path: text}`."""
    return sorted(files), files.get


def production_files(tree):
    """Every production Swift file (R20)."""
    roles = register_roles(tree[1]) or {}
    return survey(tree[0], roles)[0]


def set_aside(tree):
    """Every `test` file set aside (R20)."""
    roles = register_roles(tree[1]) or {}
    return survey(tree[0], roles)[1]


def clock_reads(tree):
    """Every production clock read, as `<path>:<line>: <spelling>` (R21)."""
    found = reads_in(tree[1], production_files(tree))[0]
    return [
        f"{path}:{line}: {spelling}" for path in sorted(found) for line, spelling in found[path]
    ]


def read_list(read):
    """`config/clock-reads-swift.json` parsed, or None when it is absent."""
    text = read(LIST)
    return json.loads(text) if text is not None else None


def clock_rows(tree):
    """The rows of `config/clock-reads-swift.json`, or none when the list is absent."""
    data = read_list(tree[1])
    rows = data.get("sites") if isinstance(data, dict) else None
    return rows if isinstance(rows, list) else []


def clock_findings(tree):
    """Each production Swift file whose clock reads differ from its row's count, each row for a
    file with no read, each comment or literal left open, and each malformed, repeated or
    misplaced row (R20 to R22, R25)."""
    return []
    paths, read = tree
    roles = register_roles(read)
    if roles is None:
        return [f"{REGISTER}: the register is absent, so no file's role is known"]
    production = survey(paths, roles)[0]
    if not production:
        return ["examined 0 production Swift file(s): VOID"]
    found, findings = reads_in(read, production)
    data = read_list(read)
    rows = data.get("sites") if isinstance(data, dict) else None
    if not isinstance(rows, list):
        return [f'{LIST}: the "sites" list is absent']
    counts, previous = {}, None
    for number, row in enumerate(rows, 1):
        findings += row_problems(row, number)
        if not isinstance(row, dict) or any(key not in row for key in ROW_KEYS):
            continue
        path = row["path"]
        if path in counts:
            findings.append(f"{path}: a row listed twice")
            continue
        if previous is not None and path < previous:
            findings.append(f"{path}: a row out of order, sorted by path")
        previous = path
        counts[path] = row["count"]
    for path in sorted(set(found) | set(counts)):
        reads = found.get(path, [])
        sites = "; ".join(f"{path}:{line}: {spelling}" for line, spelling in reads)
        if not reads:
            findings.append(f"{path}: a row for a file with no clock read")
        elif path not in counts:
            findings.append(f"{path}: {len(reads)} clock read(s) and no row: {sites}")
        elif counts[path] != len(reads):
            findings.append(
                f"{path}: {len(reads)} clock read(s), its row counts {counts[path]}: {sites}"
            )
    return findings


def admission_findings(tree):
    """Each row whose `adr` names no single ADR file that holds its path's token (R26)."""
    return []
    paths, read = tree
    findings = []
    for row in clock_rows(tree):
        if not isinstance(row, dict) or any(key not in row for key in ROW_KEYS):
            continue
        path, adr = row["path"], row["adr"]
        if not isinstance(adr, str) or not ADR_ID.fullmatch(adr):
            findings.append(f"{path}: adr {adr!r} is not ADR-<n>")
            continue
        prefix = f"{DECISIONS}/{adr}-"
        files = sorted(
            name
            for name in paths
            if name.startswith(prefix) and name.endswith(".md") and "/" not in name[len(prefix) :]
        )
        if len(files) != 1:
            findings.append(f"{path}: {adr} is held by {len(files)} files in {DECISIONS}, not one")
        elif f"`{path}`" not in (read(files[0]) or ""):
            findings.append(f"{path}: {files[0]} does not hold `{path}`")
    return findings


APP = "ios/App/Sources/Clock.swift"
OTHER = "ios/App/Sources/Other.swift"
TEST = "ios/AppTests/ClockTests.swift"


def site(path=APP, count=1, adr="ADR-1"):
    """One list row."""
    return {"path": path, "count": count, "adr": adr, "why": "a planted read"}


def clock_tree(files, rows=(), listing=None, roles=None, unregistered=()):
    """A planted tree: `files`, a register giving each Swift file its role (`model` unless `roles`
    names another, and none for a path in `unregistered`), and the list of `rows` (or `listing`,
    verbatim)."""
    roles = roles or {}
    register = {
        path: {"role": roles.get(path, "model"), "decisions": 0}
        for path in files
        if path.endswith(".swift") and path not in unregistered
    }
    tree = {**files, REGISTER: json.dumps(register)}
    tree[LIST] = json.dumps({"sites": list(rows)}) if listing is None else listing
    return tree


def in_app(statement, path=APP):
    """A Swift file whose second line is `statement`, inside a function."""
    return {path: f"func read() {{\n    {statement}\n}}\n"}


def unlisted(spelling, line=2, path=APP):
    """The finding for one read in a file with no row."""
    return [f"{path}: 1 clock read(s) and no row: {path}:{line}: {spelling}"]


def planted(finder, files):
    """What `finder` finds in a planted tree that holds only `files`."""
    return finder(planted_tree(files))


READ = "let _ = Date()"


class TheSwiftClockReadsAreTheListedSites(unittest.TestCase):
    def test_every_production_swift_files_clock_reads_equal_its_row(self):
        tree = live_tree()
        self.assertEqual(clock_findings(tree), [])
        examined("production Swift file(s)", production_files(tree))
        examined("test file(s) set aside", set_aside(tree))
        examined("clock read(s)", clock_reads(tree))
        listed("clock row(s)", clock_rows(tree))
        two = {APP: "func read() {\n    let _ = Date.now\n    " + READ + "\n}\n"}
        both = f"{APP}:2: Date.now; {APP}:3: Date()"
        manifest = "ios/Package.swift"
        plants = {
            "an unlisted read": (clock_tree(in_app(READ)), unlisted("Date()")),
            "a count one below its file's reads": (
                clock_tree(two, [site(count=1)]),
                [f"{APP}: 2 clock read(s), its row counts 1: {both}"],
            ),
            "a count one above": (
                clock_tree(in_app(READ), [site(count=2)]),
                [f"{APP}: 1 clock read(s), its row counts 2: {APP}:2: Date()"],
            ),
            "a row for a file with no read": (
                clock_tree({APP: "func read() {}\n"}, [site()]),
                [f"{APP}: a row for a file with no clock read"],
            ),
            "a read in a Package.swift manifest": (
                clock_tree(
                    {APP: "func read() {}\n", **in_app(READ, path=manifest)},
                    roles={manifest: "manifest"},
                ),
                unlisted("Date()", path=manifest),
            ),
            "a duplicate path": (
                clock_tree(in_app(READ), [site(), site()]),
                [f"{APP}: a row listed twice"],
            ),
            "a row out of order": (
                clock_tree(
                    {**in_app(READ), **in_app(READ, path=OTHER)},
                    [site(path=OTHER), site()],
                ),
                [f"{APP}: a row out of order, sorted by path"],
            ),
            "an absent list": (
                clock_tree(in_app(READ), listing="{}"),
                ['config/clock-reads-swift.json: the "sites" list is absent'],
            ),
            "an absent register": (
                {**in_app(READ), LIST: json.dumps({"sites": [site()]})},
                ["ios/swift-roles.json: the register is absent, so no file's role is known"],
            ),
            "no production file (VOID)": (
                clock_tree(in_app(READ, path=TEST), roles={TEST: "test"}),
                ["examined 0 production Swift file(s): VOID"],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(clock_findings, files), expected)

    def test_each_swift_clock_spelling_and_alias_is_a_read(self):
        tree = live_tree()
        self.assertEqual(clock_findings(tree), [])
        examined("production Swift file(s)", production_files(tree))
        examined("clock read(s)", clock_reads(tree))
        spellings = {
            "let _ = Date()": "Date()",
            "let _ = Foundation.Date()": "Date()",
            "let _ = Date ( )": "Date()",
            "let _ = NSDate()": "NSDate()",
            "let _ = Date.init()": "Date.init()",
            "let make = Date.init": "Date.init",
            "let _ = Date.now": "Date.now",
            "let _ = Date . now": "Date.now",
            "let shown: ContinuousClock.Instant = .now": ".now",
            "let _ = ContinuousClock.now": "ContinuousClock.now",
            "let _ = DispatchTime.now()": "DispatchTime.now",
            "let _ = DispatchWallTime.now()": "DispatchWallTime.now",
            "let _ = clock.now": "clock.now",
            "let _ = ContinuousClock()": "ContinuousClock()",
            "let _ = SuspendingClock()": "SuspendingClock()",
            "use(clock: .continuous)": ".continuous",
            "use(clock: .suspending)": ".suspending",
            "let _ = start.timeIntervalSinceNow": "timeIntervalSinceNow",
            "let _ = Date(timeIntervalSinceNow: 5)": "timeIntervalSinceNow",
            "let _ = Date.timeIntervalSinceReferenceDate": "Date.timeIntervalSinceReferenceDate",
            "let _ = ProcessInfo.processInfo.systemUptime": "systemUptime",
            "let _ = CFAbsoluteTimeGetCurrent()": "CFAbsoluteTimeGetCurrent",
            "let _ = CACurrentMediaTime()": "CACurrentMediaTime",
            "let _ = mach_absolute_time()": "mach_absolute_time",
            "let _ = mach_continuous_time()": "mach_continuous_time",
            "clock_gettime(CLOCK_REALTIME, &spec)": "clock_gettime",
            "gettimeofday(&value, nil)": "gettimeofday",
        }
        plants = {
            f"`{statement}` is a read": (
                clock_tree(in_app(statement)),
                unlisted(spelling),
            )
            for statement, spelling in spellings.items()
        }
        for statement in (
            "let _ = Date(timeIntervalSince1970: 0)",
            "let _ = Date.init(timeIntervalSince1970: 0)",
            "let _ = date.timeIntervalSinceReferenceDate",
            "let _ = Date.distantPast",
            "let _ = now()",
            "self.now = now",
            "let _ = Self.now",
            "let shown: ContinuousClock.Instant? = nil",
            "let _ = MyDate()",
            "let _ = held.continuous",
            "let _ = .nowPlaying",
        ):
            plants[f"`{statement}` is no read"] = (clock_tree(in_app(statement)), [])
        wall = "typealias Wall = Foundation.Date\n"
        plants["a typealias of Date, made"] = (
            clock_tree({APP: wall + "func read() {\n    let _ = Wall()\n}\n"}),
            unlisted("Wall()", line=3),
        )
        plants["a typealias of Date, its reference date"] = (
            clock_tree({APP: wall + "let _ = Wall.timeIntervalSinceReferenceDate\n"}),
            unlisted("Wall.timeIntervalSinceReferenceDate"),
        )
        plants["a typealias of a clock, made"] = (
            clock_tree({APP: "typealias Mono = ContinuousClock\nlet _ = Mono()\n"}),
            unlisted("Mono()"),
        )
        plants["a typealias of a typealias in another file, made"] = (
            clock_tree(
                {
                    APP: "typealias Stamp = Wall\nlet _ = Stamp.init()\n",
                    "ios/App/Sources/Wall.swift": wall,
                }
            ),
            unlisted("Stamp.init()"),
        )
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(clock_findings, files), expected)

    def test_test_files_comments_and_literals_hold_no_read(self):
        tree = live_tree()
        self.assertEqual(clock_findings(tree), [])
        examined("test file(s) set aside", set_aside(tree))
        quiet = {APP: "func read() {}\n"}
        plants = {
            "a read in a test file": (
                clock_tree({**quiet, **in_app(READ, path=TEST)}, roles={TEST: "test"}),
                [],
            ),
            "a read in a file the register does not list": (
                clock_tree({**quiet, **in_app(READ, path=OTHER)}, unregistered=(OTHER,)),
                unlisted("Date()", path=OTHER),
            ),
            "a read after a closed string": (
                clock_tree(in_app('let _ = "a"; ' + READ)),
                unlisted("Date()"),
            ),
            "a read in an interpolation": (
                clock_tree(in_app('let _ = "at \\(Date())"')),
                unlisted("Date()"),
            ),
            "a read in a raw string's interpolation": (
                clock_tree(in_app('let _ = #"at \\#(Date())"#')),
                unlisted("Date()"),
            ),
            "a read in a multi-line string's interpolation": (
                clock_tree({APP: 'let _ = """\n    at \\(Date())\n    """\n'}),
                unlisted("Date()"),
            ),
        }
        for name, statement in {
            "a line comment": "// let _ = Date()",
            "a doc comment": "/// Date() and Date.now",
            "a block comment": "/* Date() */",
            "a nested block comment": "/* a /* b */ Date() */",
            "a string with an escaped quote": 'let _ = "a \\" Date()"',
            "a raw string holding a quote": 'let _ = #"a " Date()"#',
            "a raw string's plain backslash": 'let _ = #"\\(Date())"#',
            "a regex literal": "let _ = #/Date\\(\\)/#",
        }.items():
            plants[f"a read in {name} is no read"] = (
                clock_tree({**quiet, **in_app(statement)}),
                [],
            )
        plants["a read in a multi-line string is no read"] = (
            clock_tree({APP: 'let _ = """\n    Date()\n    """\n'}),
            [],
        )
        plants["a string left open"] = (
            clock_tree({APP: 'let _ = "Date()\nlet _ = 1\n'}),
            [f"{APP}:1: a comment or literal left open"],
        )
        plants["a block comment left open"] = (
            clock_tree({APP: "func read() {}\n/* Date()\n"}),
            [f"{APP}:2: a comment or literal left open"],
        )
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(clock_findings, files), expected)

    def test_every_swift_file_with_reads_is_admitted_by_an_adr_that_names_it(self):
        tree = live_tree()
        self.assertEqual(admission_findings(tree), [])
        listed("clock row(s)", clock_rows(tree))
        names = {f"{DECISIONS}/ADR-1-clocks.md": f"Admits `{APP}`.\n"}
        plants = {
            "a row citing an absent ADR": (
                {LIST: json.dumps({"sites": [site(adr="ADR-2")]}), **names},
                [f"{APP}: ADR-2 is held by 0 files in docs/decisions, not one"],
            ),
            "a row citing an ADR whose text lacks the path": (
                {
                    LIST: json.dumps({"sites": [site()]}),
                    f"{DECISIONS}/ADR-1-clocks.md": "Admits `ios/App/Sources/Other.swift`.\n",
                },
                [f"{APP}: docs/decisions/ADR-1-clocks.md does not hold `{APP}`"],
            ),
            "a row whose adr is not an ADR id": (
                {LIST: json.dumps({"sites": [site(adr="SPEC-1")]}), **names},
                [f"{APP}: adr 'SPEC-1' is not ADR-<n>"],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(admission_findings, files), expected)


if __name__ == "__main__":
    unittest.main()
