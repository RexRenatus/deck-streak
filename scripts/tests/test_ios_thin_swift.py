"""Every Swift file under `ios/` keeps its role, its doors and its budget (SPEC-347 R11 and A12,
ADR-358 D9).

The app's Swift is thin: the engine decides, and Swift shows what the engine answers. This census
holds that shape over every Swift file git tracks under `ios/`, through the register
`ios/swift-roles.json`, in four readings:

- the register is closed: every file is listed once, with a role its path admits, and the register
  lists no file git does not track;
- doors: of the app's own roles, only `session` imports the engine or the codec or names
  `FileManager`, only `credential` reaches the Keychain, only `config` reads the info dictionary,
  none imports `WebKit`, and none but `wire` names a JSON or property-list coder;
- names no file may hold, harness and test files included; the network names are admitted only in
  the card view's sources and the tests of the card view and its probe (ADR-358 D9), and each
  admission is printed by its file and its name;
- budgets: each counted file's decisions, read with its comments and string text removed (an
  interpolation's contents are code, so they stay), equal the register's count and stay within
  its role's ceiling.

The census is a function of the data it is handed, so every planted control is a dict: the test
hands it the live tree, then planted trees that each break one rule and must be refused by that
rule's name.
"""

import json
import re
import unittest

from _support import REPO, examined
from test_one_static_library import tracked_paths

REGISTER = "ios/swift-roles.json"
# The app's own roles, each with its ceiling: the most decisions one file of that role may hold.
CEILINGS = {"entry": 0, "view": 3, "model": 6, "session": 4, "credential": 4, "config": 2}
APP_ROLES = frozenset(CEILINGS)
# The roles whose decisions are counted exactly: the app's, and the codec's, which has no ceiling.
COUNTED = APP_ROLES | {"wire"}
ROLES = COUNTED | {"isolation", "manifest", "test", "harness"}
# The test targets that sit outside a package's own `Tests/` directory.
TEST_TARGETS = (
    "ios/HarnessTests/",
    "ios/HarnessUITests/",
    "ios/CardProbeTests/",
    "ios/AppTests/",
    "ios/AppUITests/",
)


def imports(module):
    """An import of `module`, whole or of one declaration from it, under any attribute."""
    kinds = r"(?:(?:typealias|struct|class|enum|protocol|let|var|func)\s+)?"
    return re.compile(rf"\bimport\s+{kinds}{module}\b")


# Each door: what crosses it, and the counted roles that may. Read over the code alone, so a
# sentence about a door, in a comment or a string, is not a use of it.
CODERS = re.compile(
    r"\b(?:JSONEncoder|JSONDecoder|JSONSerialization"
    r"|PropertyListEncoder|PropertyListDecoder|PropertyListSerialization)\b"
)
DOORS = {
    "imports DeckStreakFFI": (imports("DeckStreakFFI"), {"session"}),
    "imports HarnessWire": (imports("HarnessWire"), {"session"}),
    "names FileManager": (re.compile(r"\bFileManager\b"), {"session"}),
    "imports Security": (imports("Security"), {"credential"}),
    "names SecItem": (re.compile(r"\bSecItem\w*"), {"credential"}),
    "names a kSec constant": (re.compile(r"\bkSec\w*"), {"credential"}),
    "reads the info dictionary": (
        re.compile(r"\binfoDictionary\b|\bforInfoDictionaryKey\b"),
        {"config"},
    ),
    "imports WebKit": (imports("WebKit"), set()),
    "names a JSON or property-list coder": (CODERS, {"wire"}),
}
# Names no Swift file may hold, read over the whole text, comments and strings included: a network
# client of the app's own, a second store, or a setting that outlives the engine's collection.
FORBIDDEN = {
    "URLSession": re.compile(r"URLSession"),
    "URLRequest": re.compile(r"URLRequest"),
    "NWConnection": re.compile(r"NWConnection\w*"),
    "import Network": imports("Network"),
    "SQLite3": re.compile(r"SQLite3"),
    "sqlite3_": re.compile(r"sqlite3_"),
    "UserDefaults": re.compile(r"UserDefaults"),
    "@AppStorage": re.compile(r"@AppStorage"),
    "NSUbiquitousKeyValueStore": re.compile(r"NSUbiquitousKeyValueStore"),
}
# ADR-358 D9: the network names, admitted in the card view's sources and in the tests of the card
# view and its probe, where a listener proves that a card reaches no network. Nowhere else.
NETWORK_NAMES = frozenset({"NWConnection", "import Network"})
ADMITTED_TESTS = ("ios/CardIsolation/Tests/", "ios/CardProbeTests/")
# R11's decision tokens: the branching keywords, the short-circuit and nil-coalescing operators, a
# ternary, and the calls that decide over a collection.
DECISIONS = re.compile(
    r"\b(?:if|guard|case|while|for|repeat|catch)\b"
    r"|&&|\|\||\?\?| \? "
    r"|\.(?:filter|sorted|sort|reduce|min|max|contains|allSatisfy)\(|\.first\(where:"
)
# A raw string's delimiter, which the stripper does not read: a counted file may not hold one.
RAW_STRING = '#"'


def path_roles(path):
    """The roles a Swift file's path admits (SPEC-347 section 10): a package manifest, a test
    target's file, the card view's sources, the harness's, the codec's, or the app's own."""
    if path.endswith("/Package.swift"):
        return frozenset({"manifest"})
    if "/Tests/" in path or path.startswith(TEST_TARGETS):
        return frozenset({"test"})
    if path.startswith("ios/CardIsolation/Sources/"):
        return frozenset({"isolation"})
    if path.startswith(("ios/Harness/Sources/", "ios/CardProbeHost/")):
        return frozenset({"harness"})
    if path.startswith("ios/HarnessWire/Sources/"):
        return frozenset({"wire"})
    if path.startswith("ios/App/Sources/"):
        return APP_ROLES
    return frozenset()


def code_of(source):
    """A Swift file's code: its comments (nested block comments included) and the text of its
    string literals removed, each replaced by a space, and every interpolation's contents kept,
    because they are code. Lines are kept, so a reader can still split on them. A raw string is
    never read here: a counted file holding one is refused before this is called."""
    kept = []
    # Each frame is [mode, open parentheses]: "code" frames end at their unmatched `)` when they
    # are an interpolation; "string" and "block" frames end at their closing quotes.
    stack = [["code", 0]]
    at, end = 0, len(source)
    while at < end:
        frame = stack[-1]
        if frame[0] == "code":
            if source.startswith("//", at):
                newline = source.find("\n", at)
                at = end if newline == -1 else newline
            elif source.startswith("/*", at):
                depth, at = 1, at + 2
                while at < end and depth:
                    if source.startswith("/*", at):
                        depth, at = depth + 1, at + 2
                    elif source.startswith("*/", at):
                        depth, at = depth - 1, at + 2
                    else:
                        kept.append("\n" if source[at] == "\n" else "")
                        at += 1
                kept.append(" ")
            elif source.startswith('"""', at):
                stack.append(["block", 0])
                kept.append(" ")
                at += 3
            elif source[at] == '"':
                stack.append(["string", 0])
                kept.append(" ")
                at += 1
            elif source[at] == ")" and len(stack) > 1 and frame[1] == 0:
                stack.pop()
                kept.append(" ")
                at += 1
            else:
                frame[1] += {"(": 1, ")": -1}.get(source[at], 0)
                kept.append(source[at])
                at += 1
        elif source.startswith("\\(", at):
            stack.append(["code", 0])
            kept.append(" ")
            at += 2
        elif source[at] == "\\":
            at += 2
        elif frame[0] == "block" and source.startswith('"""', at):
            stack.pop()
            kept.append(" ")
            at += 3
        elif frame[0] == "string" and source[at] == '"':
            stack.pop()
            kept.append(" ")
            at += 1
        else:
            kept.append("\n" if source[at] == "\n" else "")
            at += 1
    return "".join(kept)


class Pairs(list):
    """A JSON object's members, in order, so a path the register lists twice is seen."""


def register_entries(text, problems):
    """The register's entries, {path: entry}, or {} with the reason in `problems`."""
    if text is None:
        problems.append(f"{REGISTER}: the register: missing")
        return {}
    try:
        value = json.loads(text, object_pairs_hook=Pairs)
    except json.JSONDecodeError:
        problems.append(f"{REGISTER}: the register: not JSON")
        return {}
    if not isinstance(value, Pairs):
        problems.append(f"{REGISTER}: the register: not an object of paths")
        return {}
    entries = {}
    for path, entry in value:
        if path in entries:
            problems.append(f"{path}: the register: listed twice")
        entries[path] = dict(entry) if isinstance(entry, Pairs) else entry
    return entries


def entry_problems(path, entry):
    """What one register entry gets wrong in itself, and the role it records when that role is
    one the census knows: (role or None, recorded count or None, problems)."""
    if not isinstance(entry, dict) or not isinstance(entry.get("role"), str):
        return None, None, [f"{path}: the register: its entry names no role"]
    role = entry["role"]
    if role not in ROLES:
        return None, None, [f"{path}: the register: {role!r} is not a role"]
    problems = [
        f"{path}: the register: an unknown key {key!r}"
        for key in sorted(set(entry) - {"role", "decisions"})
    ]
    recorded = entry.get("decisions")
    if role in COUNTED and not (type(recorded) is int and recorded >= 0):
        problems.append(f"{path}: the register: records no decision count for a {role!r} file")
        recorded = None
    elif role not in COUNTED and "decisions" in entry:
        problems.append(
            f"{path}: the register: records a decision count for a {role!r} file, which has none"
        )
        recorded = None
    return role, recorded, problems


def forbidden_problems(path, text, role, judged):
    """The names `path` holds that no file may, past ADR-358 D9's admission, which is recorded."""
    admitted = role == "isolation" or (role == "test" and path.startswith(ADMITTED_TESTS))
    problems = []
    for name, pattern in FORBIDDEN.items():
        for token in sorted({" ".join(match.split()) for match in pattern.findall(text)}):
            if admitted and name in NETWORK_NAMES:
                judged["admissions"].append(f"{path}: {token}")
            else:
                problems.append(f"{path}: the forbidden names: names {token!r}")
    return problems


def who_may(allowed):
    """The roles a door admits, as the refusal says them."""
    if not allowed:
        return "no app file may"
    return "only " + " or ".join(f"a {role!r} file" for role in sorted(allowed)) + " may"


def door_problems(path, code, role, judged):
    """Each door `path`'s code crosses that its role may not."""
    problems = []
    for door, (pattern, allowed) in DOORS.items():
        judged["doors"].append(f"{path}: {door}")
        if role not in allowed and pattern.search(code):
            problems.append(f"{path}: the doors: {door}, which {who_may(allowed)}")
    return problems


def budget_problems(path, code, role, recorded, judged):
    """`path`'s decision count against the register's and its role's ceiling."""
    count = len(DECISIONS.findall(code))
    judged["decisions"].append(f"{path}: {count}")
    problems = []
    if recorded is not None:
        exact = count == recorded
        if not exact:
            problems.append(
                f"{path}: the budgets: {count} counted, the register records {recorded}"
            )
    ceiling = CEILINGS.get(role)
    if ceiling is not None and count > ceiling:
        problems.append(
            f"{path}: the budgets: {count} counted, over the {role!r} ceiling of {ceiling}"
        )
    return problems


def thin_swift_problems(sources, register_text):
    """Every rule of R11 the tree breaks, each named, and what was examined. `sources` maps each
    tracked Swift file under `ios/` to its text; `register_text` is the register's, or None when
    git does not track one."""
    problems = []
    judged = {"files": [], "doors": [], "decisions": [], "admissions": []}
    entries = register_entries(register_text, problems)
    for path in sorted(sources):
        text = sources[path]
        judged["files"].append(path)
        admitted = path_roles(path)
        role, recorded = None, None
        if path in entries:
            role, recorded, shape = entry_problems(path, entries[path])
            problems.extend(shape)
        else:
            problems.append(f"{path}: the register: not listed")
        if role is not None and role not in admitted:
            problems.append(
                f"{path}: the roles: registered {role!r}, which its path does not admit"
            )
            role, recorded = None, None
        if role is None and len(admitted) == 1:
            (role,) = admitted
        problems.extend(forbidden_problems(path, text, role, judged))
        if role not in COUNTED:
            continue
        if RAW_STRING in text:
            problems.append(
                f"{path}: the budgets: holds a raw string literal ({RAW_STRING}), which the census "
                "does not read"
            )
            continue
        code = code_of(text)
        problems.extend(door_problems(path, code, role, judged))
        problems.extend(budget_problems(path, code, role, recorded, judged))
    for path in sorted(set(entries) - set(sources)):
        problems.append(f"{path}: the register: lists a file git does not track")
    return problems, judged


def live_tree():
    """The tracked Swift files under `ios/` with their text, and the register's text, or None
    when git tracks no register: a file on disk that git does not track is never judged."""
    tracked = tracked_paths(REPO)
    swift = [path for path in tracked if path.startswith("ios/") and path.endswith(".swift")]
    sources = {path: (REPO / path).read_text(encoding="utf-8") for path in swift}
    register = (REPO / REGISTER).read_text(encoding="utf-8") if REGISTER in tracked else None
    return sources, register


# A tree that keeps every rule: the controls start from it and each breaks one rule. Its counted
# files hold decisions in comments, in string text and in interpolations, so the stripper is held
# too: a comment or a string that counted, or an interpolation that did not, would refuse it.
APP = "ios/App/Sources/"
WIRE = "ios/HarnessWire/Sources/HarnessWire/Wire.swift"
HARNESS = "ios/Harness/Sources/HarnessApp.swift"
LISTENERS = "ios/CardProbeTests/Listeners.swift"
APP_TEST = "ios/AppTests/CredentialStoreTests.swift"
GOOD_SOURCES = {
    APP + "DeckStreakApp.swift": (
        "import SwiftUI\n\n@main\nstruct DeckStreakApp: App {\n"
        "    var body: some Scene {\n        WindowGroup { DeckListView(names: []) }\n    }\n}\n"
    ),
    APP + "AppModel.swift": (
        "import Observation\n\n/* a /* nested */ if, still a comment */\n@Observable\n"
        "final class AppModel {\n    var names: [String] = []\n    var chosen: String?\n"
        "    func choose(_ name: String) {\n        if names.contains(name) {\n"
        "            chosen = name\n        }\n    }\n}\n"
    ),
    APP + "EngineSession.swift": (
        "import DeckStreakFFI\nimport Foundation\nimport HarnessWire\n\nactor EngineSession {\n"
        "    private var ready = false\n    func open() {\n"
        "        guard !ready else { return }\n"
        "        ready = FileManager.default.temporaryDirectory.path.isEmpty == false\n"
        "    }\n}\n"
    ),
    APP + "SyncCredentialStore.swift": (
        "import Security\n\nenum SyncCredentialStore {\n"
        "    static func delete(service: String) -> Bool {\n"
        "        let query = [kSecClass: kSecClassGenericPassword, kSecAttrService: service]\n"
        "        let status = SecItemDelete(query as CFDictionary)\n"
        "        return status == errSecSuccess || status == errSecItemNotFound\n    }\n}\n"
    ),
    APP + "SyncConfiguration.swift": (
        "import Foundation\n\nenum SyncConfiguration {\n"
        "    static func read(_ key: String) -> String {\n"
        '        Bundle.main.object(forInfoDictionaryKey: key) as? String ?? ""\n    }\n}\n'
    ),
    APP + "DeckListView.swift": (
        "import SwiftUI\n\n// A name is the engine's: if it is long, guard nothing, while it scrolls.\n"
        "struct DeckListView: View {\n    let names: [String]\n    var body: some View {\n"
        "        List(names, id: \\.self) { name in\n"
        '            Text("case for \\(name) of \\(names.first ?? "none, if empty")")\n'
        "        }\n    }\n}\n"
    ),
    APP + "AccountView.swift": (
        "import SwiftUI\n\nstruct AccountView: View {\n    let signedIn: Bool\n"
        "    let busy: Bool\n"
        '    static let note = """\n        if a guard while \\"case\\"\n'
        '        \\(1 > 0 && 2 > 1)\n        """\n'
        "    var body: some View {\n"
        '        Text(signedIn ? "Signed in" : "say \\"if\\"")\n    }\n}\n'
    ),
    WIRE: (
        "struct WireWriter {\n    mutating func varint(_ value: UInt64) {\n"
        "        var rest = value\n        while rest >= 0x80 {\n            rest >>= 7\n"
        "        }\n    }\n}\n"
    ),
    "ios/HarnessWire/Package.swift": (
        '// swift-tools-version:5.9\nimport PackageDescription\n\nlet package = Package(name: "W")\n'
    ),
    HARNESS: (
        "import WebKit\n\nlet files = FileManager.default\n"
        "func both(_ a: Bool, _ b: Bool) -> Bool {\n    if a && b || a { return true }\n"
        "    guard a else { return false }\n    return b\n}\n"
    ),
    "ios/CardIsolation/Sources/CardIsolation/RuleList.swift": (
        'import Network\nimport WebKit\n\nlet source = #"[{"trigger":{}}]"#\n'
        "var open: [NWConnection] = []\n"
    ),
    LISTENERS: (
        "import Network\n\nfinal class Listeners {\n    private var open: [NWConnection] = []\n"
        "    private var groups: [NWConnectionGroup] = []\n}\n"
    ),
    APP_TEST: (
        "import XCTest\n@testable import DeckStreak\n\nfinal class CredentialStoreTests: XCTestCase {\n"
        "    func test_a10() throws {\n        if true { XCTAssertTrue(true) }\n    }\n}\n"
    ),
}
GOOD_REGISTER = {
    APP + "DeckStreakApp.swift": {"role": "entry", "decisions": 0},
    APP + "AppModel.swift": {"role": "model", "decisions": 2},
    APP + "EngineSession.swift": {"role": "session", "decisions": 1},
    APP + "SyncCredentialStore.swift": {"role": "credential", "decisions": 1},
    APP + "SyncConfiguration.swift": {"role": "config", "decisions": 1},
    APP + "DeckListView.swift": {"role": "view", "decisions": 1},
    APP + "AccountView.swift": {"role": "view", "decisions": 2},
    WIRE: {"role": "wire", "decisions": 1},
    "ios/HarnessWire/Package.swift": {"role": "manifest"},
    HARNESS: {"role": "harness"},
    "ios/CardIsolation/Sources/CardIsolation/RuleList.swift": {"role": "isolation"},
    LISTENERS: {"role": "test"},
    APP_TEST: {"role": "test"},
}
# One of each decision token, so a token the census stopped counting changes the count.
EVERY_TOKEN = (
    "if\nguard\ncase\nwhile\nfor\nrepeat\ncatch\na && b\na || b\na ?? b\na ? b : c\n"
    "xs.filter(f)\nxs.sorted(by: f)\nxs.sort(by: f)\nxs.reduce(0, f)\nxs.min()\nxs.max()\n"
    "xs.contains(1)\nxs.first(where: f)\nxs.allSatisfy(f)\n"
)


def register_text(pairs):
    """A register's text from its (path, entry) pairs, in the order given, a repeat included."""
    members = ",\n".join(f"  {json.dumps(path)}: {json.dumps(entry)}" for path, entry in pairs)
    return "{\n" + members + "\n}\n"


def planted(sources=None, entries=None, register=None, missing=False):
    """The good tree, with `sources` and `entries` (path to text or entry; None drops one) in
    place of or beside its own; `register` replaces the register's text, and `missing` drops it."""
    tree, roles = dict(GOOD_SOURCES), dict(GOOD_REGISTER)
    for table, changes in ((tree, sources), (roles, entries)):
        for path, value in (changes or {}).items():
            if value is None:
                table.pop(path)
            else:
                table[path] = value
    if missing:
        return tree, None
    return tree, register_text(sorted(roles.items())) if register is None else register


def edit(path, old, new):
    """The good tree's `path`, with `old` replaced once by `new`."""
    text = GOOD_SOURCES[path]
    assert text.count(old) == 1, f"{path}: {old!r} is not in the planted file once"
    return {path: text.replace(old, new)}


def unlisted(*register_problems):
    """The problems of a tree whose register is unread: its own, then every file unlisted."""
    return list(register_problems) + [
        f"{path}: the register: not listed" for path in sorted(GOOD_SOURCES)
    ]


MODEL = APP + "AppModel.swift"
SESSION = APP + "EngineSession.swift"
CONFIG = APP + "SyncConfiguration.swift"
LIST = APP + "DeckListView.swift"
ACCOUNT = APP + "AccountView.swift"
ENTRY = APP + "DeckStreakApp.swift"
PLANTS = {
    "the good tree": ({}, []),
    "every decision token, each counted once": (
        {"sources": {WIRE: EVERY_TOKEN}, "entries": {WIRE: {"role": "wire", "decisions": 20}}},
        [],
    ),
    "no register": ({"missing": True}, unlisted(f"{REGISTER}: the register: missing")),
    "a register that is not JSON": (
        {"register": "{\n"},
        unlisted(f"{REGISTER}: the register: not JSON"),
    ),
    "a register that is not an object": (
        {"register": "[]\n"},
        unlisted(f"{REGISTER}: the register: not an object of paths"),
    ),
    "a file outside the register": (
        {"sources": {APP + "Extra.swift": "struct Extra {}\n"}},
        [f"{APP}Extra.swift: the register: not listed"],
    ),
    "a file listed twice": (
        {
            "register": register_text(
                [*sorted(GOOD_REGISTER.items()), (MODEL, GOOD_REGISTER[MODEL])]
            )
        },
        [f"{MODEL}: the register: listed twice"],
    ),
    "a register entry for a file git does not track": (
        {"entries": {APP + "Gone.swift": {"role": "view", "decisions": 0}}},
        [f"{APP}Gone.swift: the register: lists a file git does not track"],
    ),
    "an entry with no role": (
        {"entries": {MODEL: {"decisions": 2}}},
        [f"{MODEL}: the register: its entry names no role"],
    ),
    "an unknown role": (
        {"entries": {MODEL: {"role": "helper", "decisions": 2}}},
        [f"{MODEL}: the register: 'helper' is not a role"],
    ),
    "an unknown key": (
        {"entries": {MODEL: {"role": "model", "decisions": 2, "ceiling": 9}}},
        [f"{MODEL}: the register: an unknown key 'ceiling'"],
    ),
    "a counted file with no count": (
        {"entries": {LIST: {"role": "view"}}},
        [f"{LIST}: the register: records no decision count for a 'view' file"],
    ),
    "a count for a file with no budget": (
        {"entries": {HARNESS: {"role": "harness", "decisions": 3}}},
        [f"{HARNESS}: the register: records a decision count for a 'harness' file, which has none"],
    ),
    "a role its path does not admit": (
        {"entries": {LIST: {"role": "harness"}}},
        [f"{LIST}: the roles: registered 'harness', which its path does not admit"],
    ),
    "a path no role admits": (
        {
            "sources": {"ios/Elsewhere/Thing.swift": "struct Thing {}\n"},
            "entries": {"ios/Elsewhere/Thing.swift": {"role": "harness"}},
        },
        [
            "ios/Elsewhere/Thing.swift: the roles: registered 'harness', which its path does not admit"
        ],
    ),
    "the engine imported by the model": (
        {
            "sources": edit(
                MODEL, "import Observation\n", "import Observation\nimport DeckStreakFFI\n"
            )
        },
        [f"{MODEL}: the doors: imports DeckStreakFFI, which only a 'session' file may"],
    ),
    "the codec imported by a view": (
        {"sources": edit(LIST, "import SwiftUI\n", "import SwiftUI\nimport HarnessWire\n")},
        [f"{LIST}: the doors: imports HarnessWire, which only a 'session' file may"],
    ),
    "FileManager in a view": (
        {"sources": edit(ACCOUNT, "    let busy: Bool\n", "    let files = FileManager.default\n")},
        [f"{ACCOUNT}: the doors: names FileManager, which only a 'session' file may"],
    ),
    "Security outside the credential store": (
        {"sources": edit(MODEL, "import Observation\n", "import Observation\nimport Security\n")},
        [f"{MODEL}: the doors: imports Security, which only a 'credential' file may"],
    ),
    "SecItem in the session": (
        {
            "sources": edit(
                SESSION,
                "    func open() {\n",
                "    func wipe() { SecItemDelete(q) }\n    func open() {\n",
            )
        },
        [f"{SESSION}: the doors: names SecItem, which only a 'credential' file may"],
    ),
    "a kSec constant in the config": (
        {
            "sources": edit(
                CONFIG,
                "enum SyncConfiguration {\n",
                "enum SyncConfiguration {\n    static let k = kSecAttrService\n",
            )
        },
        [f"{CONFIG}: the doors: names a kSec constant, which only a 'credential' file may"],
    ),
    "the info dictionary outside the config": (
        {
            "sources": edit(
                MODEL, "    var chosen: String?\n", "    let info = Bundle.main.infoDictionary\n"
            )
        },
        [f"{MODEL}: the doors: reads the info dictionary, which only a 'config' file may"],
    ),
    "WebKit in a view": (
        {"sources": edit(LIST, "import SwiftUI\n", "import SwiftUI\nimport WebKit\n")},
        [f"{LIST}: the doors: imports WebKit, which no app file may"],
    ),
    "a JSON coder in the model": (
        {"sources": edit(MODEL, "    var chosen: String?\n", "    let coder = JSONDecoder()\n")},
        [f"{MODEL}: the doors: names a JSON or property-list coder, which only a 'wire' file may"],
    ),
    "doors named only in a comment and a string": (
        {
            "sources": edit(
                LIST,
                "struct DeckListView: View {\n",
                "// FileManager, SecItem and import Security are named here alone.\n"
                'struct DeckListView: View {\n    let note = "import WebKit and JSONDecoder"\n',
            )
        },
        [],
    ),
    "UserDefaults in a harness file": (
        {
            "sources": edit(
                HARNESS, "let files = FileManager.default\n", "let saved = UserDefaults.standard\n"
            )
        },
        [f"{HARNESS}: the forbidden names: names 'UserDefaults'"],
    ),
    "URLSession in a test": (
        {
            "sources": edit(
                APP_TEST, "        if true", "        _ = URLSession.shared\n        if true"
            )
        },
        [f"{APP_TEST}: the forbidden names: names 'URLSession'"],
    ),
    "URLRequest in the session": (
        {
            "sources": edit(
                SESSION,
                "    private var ready = false\n",
                "    private var ready = false\n    var request: URLRequest?\n",
            )
        },
        [f"{SESSION}: the forbidden names: names 'URLRequest'"],
    ),
    "@AppStorage in a view": (
        {
            "sources": edit(
                ACCOUNT, "    let busy: Bool\n", '    @AppStorage("busy") var busy = false\n'
            )
        },
        [f"{ACCOUNT}: the forbidden names: names '@AppStorage'"],
    ),
    "SQLite3 in the model": (
        {"sources": edit(MODEL, "import Observation\n", "import Observation\nimport SQLite3\n")},
        [f"{MODEL}: the forbidden names: names 'SQLite3'"],
    ),
    "sqlite3_ in a harness file": (
        {"sources": edit(HARNESS, "    return b\n", "    sqlite3_close(nil)\n    return b\n")},
        [f"{HARNESS}: the forbidden names: names 'sqlite3_'"],
    ),
    "NSUbiquitousKeyValueStore in the config": (
        {
            "sources": edit(
                CONFIG,
                "enum SyncConfiguration {\n",
                "enum SyncConfiguration {\n    static let cloud = NSUbiquitousKeyValueStore.default\n",
            )
        },
        [f"{CONFIG}: the forbidden names: names 'NSUbiquitousKeyValueStore'"],
    ),
    "import Network in the session": (
        {"sources": edit(SESSION, "import Foundation\n", "import Foundation\nimport Network\n")},
        [f"{SESSION}: the forbidden names: names 'import Network'"],
    ),
    "NWConnection in an app test": (
        {
            "sources": edit(
                APP_TEST,
                "        if true",
                "        let held: NWConnection? = nil\n        if true",
            )
        },
        [f"{APP_TEST}: the forbidden names: names 'NWConnection'"],
    ),
    "NWConnectionGroup in a harness file": (
        {
            "sources": edit(
                HARNESS, "    return b\n", "    let group: NWConnectionGroup? = nil\n    return b\n"
            )
        },
        [f"{HARNESS}: the forbidden names: names 'NWConnectionGroup'"],
    ),
    "UserDefaults in an admitted test file": (
        {
            "sources": edit(
                LISTENERS,
                "final class Listeners {\n",
                "final class Listeners {\n    let saved = UserDefaults.standard\n",
            )
        },
        [f"{LISTENERS}: the forbidden names: names 'UserDefaults'"],
    ),
    "a forbidden name in a comment": (
        {
            "sources": edit(
                HARNESS, "import WebKit\n", "import WebKit\n// URLSession is not this file's.\n"
            )
        },
        [f"{HARNESS}: the forbidden names: names 'URLSession'"],
    ),
    "a count the register does not record": (
        {"entries": {LIST: {"role": "view", "decisions": 0}}},
        [f"{LIST}: the budgets: 1 counted, the register records 0"],
    ),
    "a register that records more than the file holds": (
        {"entries": {LIST: {"role": "view", "decisions": 2}}},
        [f"{LIST}: the budgets: 1 counted, the register records 2"],
    ),
    "a codec count that drifts": (
        {"entries": {WIRE: {"role": "wire", "decisions": 2}}},
        [f"{WIRE}: the budgets: 1 counted, the register records 2"],
    ),
    "a view over its ceiling": (
        {
            "sources": edit(
                ACCOUNT,
                "    let busy: Bool\n",
                "    let busy: Bool\n    var ready: Bool { signedIn && busy || busy && signedIn }\n",
            ),
            "entries": {ACCOUNT: {"role": "view", "decisions": 5}},
        },
        [f"{ACCOUNT}: the budgets: 5 counted, over the 'view' ceiling of 3"],
    ),
    "an entry with a decision": (
        {
            "sources": edit(
                ENTRY, "DeckListView(names: [])", "if true { DeckListView(names: []) }"
            ),
            "entries": {ENTRY: {"role": "entry", "decisions": 1}},
        },
        [f"{ENTRY}: the budgets: 1 counted, over the 'entry' ceiling of 0"],
    ),
    "a raw string in a counted file": (
        {
            "sources": edit(
                LIST, "    let names: [String]\n", '    let names: [String]\n    let raw = #"if"#\n'
            )
        },
        [f'{LIST}: the budgets: holds a raw string literal (#"), which the census does not read'],
    ),
}


class EverySwiftFileKeepsItsRole(unittest.TestCase):
    def test_every_swift_file_keeps_its_role_its_doors_and_its_budget(self):
        # The behaviour first: every tracked Swift file under `ios/` keeps R11's four readings.
        sources, register = live_tree()
        problems, judged = thin_swift_problems(sources, register)
        counts = ", ".join(f"{len(items)} {what}" for what, items in judged.items())
        with self.subTest(tree="the live tree"):
            self.assertEqual(problems, [], f"examined {counts}")
        for admission in judged["admissions"]:
            print(f"admitted by ADR-358 D9: {admission}")

        # The controls: the good tree is accepted, and each plant is refused by its rule's name.
        for name, (shape, wanted) in examined("planted trees", list(PLANTS.items())):
            with self.subTest(plant=name):
                got = thin_swift_problems(*planted(**shape))[0]
                print(f"planted {name}: {'; '.join(got) if got else 'accepted'}")
                self.assertEqual(got, wanted, name)

        examined("Swift files", judged["files"])
        examined("doors", judged["doors"])
        examined("decision counts", judged["decisions"])


if __name__ == "__main__":
    unittest.main()
