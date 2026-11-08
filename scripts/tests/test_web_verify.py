"""The web app's phone-size verification held to the tree (SPEC-639 A1, A2, A3, A5, A10, A11, A13;
ADR-662 D2 to D4, D7).

`web/app/playwright.config.ts` keeps its default project, `chromium`, which ignores `tests/verify/`,
and adds six phone projects, each a `phone('<name>', <size>, '<scheme>', <text scale>)` call over
`PHONE` (390 by 844) or `SMALL_PHONE` (360 by 740), with touch. Its screenshot expectation allows no
margin: `threshold: 0`, `maxDiffPixels: 0` and no `maxDiffPixelRatio`. `tests/verify/states.ts`
holds, under `STATES`, every route of `src/lib/routes.ts` with its `first`, `empty` and `error`
states, and the review screen its `question` and `answer`; every value in it and in `fixtures.ts`
is synthetic. `a11y-themes.spec.ts` names the five WCAG tags, turns `target-size` on by name and
asserts it ran. A rule is disabled only through `exceptions.ts`, whose every entry names its rule,
route, reason and issue. Every route, state and 390-wide project has a committed Linux baseline
under `tests/verify/__screenshots__/`, and so has the planted page.

Each check returns one `"<reason>: <detail>"` line per fault. The tests run each over this
repository and over fixture trees in a temporary directory: a complete planted tree as the control,
and a planted fault for each reason. The standard library only; nothing is imported from the tree.
"""

import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

APP = Path("web/app")
CONFIG = APP / "playwright.config.ts"
ROUTES_FILE = APP / "src/lib/routes.ts"
VERIFY = APP / "tests/verify"
STATES = VERIFY / "states.ts"
FIXTURES = VERIFY / "fixtures.ts"
EXCEPTIONS = VERIFY / "exceptions.ts"
AUDIT = VERIFY / "a11y-themes.spec.ts"
SHOTS = VERIFY / "__screenshots__"
REVIEW = "/study/review"
LEARNER_STATES = ("first", "empty", "error")
REVIEW_STATES = ("question", "answer")
SIZES = {"PHONE": (390, 844), "SMALL_PHONE": (360, 740)}
MATRIX = {
    ("phone-light", "PHONE", "light", "1"),
    ("phone-dark", "PHONE", "dark", "1"),
    ("phone-light-text200", "PHONE", "light", "2"),
    ("phone-dark-text200", "PHONE", "dark", "2"),
    ("phone-small-light", "SMALL_PHONE", "light", "1"),
    ("phone-small-dark", "SMALL_PHONE", "dark", "1"),
}
SNAPSHOT_PROJECTS = ("phone-light", "phone-dark", "phone-light-text200", "phone-dark-text200")
PLANTED_SHOT = "phone-light/verify/planted.spec.ts/planted-linux.png"
TAGS = ("wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa")
PHONE_CALL = re.compile(r"phone\(\s*'([\w-]+)',\s*(\w+),\s*'(\w+)',\s*([\d.]+)\s*\)")
SIZE_CONST = re.compile(r"const (\w+) = \{ width: (\d+), height: (\d+) \}")
KEPT_PROJECT = re.compile(r"name: 'chromium',\s*testIgnore: [^,}]*verify")
SCREENSHOT = re.compile(r"toHaveScreenshot: \{([^}]*)\}")
ZERO = r"\s*0\s*(?:,|$)"
EMAIL = re.compile(r"[\w.+-]+@[\w-]+\.[A-Za-z]{2,}")
ACCOUNT = re.compile(r"(?<![\w.])\d{7,}(?![\w.])")
LITERAL = re.compile(r"'[^']*'|\b\d+(?:\.\d+)?\b")
TARGET_SIZE_ON = re.compile(r"'target-size': \{ enabled: true \}")
TARGET_SIZE_RAN = re.compile(r"toContain\('target-size'\)")
DISABLE = re.compile(r"disableRules\((?!disabledFor\()|enabled: false")
ENTRY_FIELDS = ("rule", "route", "reason", "issue")
ISSUE = re.compile(r"^#\d+$")


def read(root, path):
    """The text of `path` under `root`, or None when the file is absent."""
    file = root / path
    return file.read_text(encoding="utf-8") if file.is_file() else None


def braced(text, start):
    """The text inside the braces or brackets that open at `start`, quoted strings skipped."""
    pairs = {"{": "}", "[": "]"}
    stack, i, quote = [pairs[text[start]]], start + 1, None
    while i < len(text) and stack:
        ch = text[i]
        if quote:
            quote = None if ch == quote and text[i - 1] != "\\" else quote
        elif ch in "'\"`":
            quote = ch
        elif ch in pairs:
            stack.append(pairs[ch])
        elif ch == stack[-1]:
            stack.pop()
        i += 1
    return text[start + 1 : i - 1]


def keys(body):
    """The keys at the top level of an object's body, in order."""
    found, depth, quote, token = [], 0, None, ""
    for i, ch in enumerate(body):
        if quote:
            token = token + ch if ch != quote else token
            quote = None if ch == quote else quote
            continue
        if ch in "'\"":
            quote, token = ch, ""
        elif ch in "{[(":
            depth += 1
        elif ch in "}])":
            depth -= 1
        elif ch == ":" and depth == 0 and token:
            found.append(token)
            token = ""
        elif ch.isalnum() or ch in "_$":
            token = token + ch if body[i - 1].isalnum() or body[i - 1] in "_$" else ch
        elif ch == ",":
            token = ""
    return found


def routes(root):
    """The route table's paths, or None when the table is absent."""
    text = read(root, ROUTES_FILE)
    if text is None:
        return None
    start = text.index("ROUTES = [") + len("ROUTES = ")
    return re.findall(r"'(/[^']*)'", braced(text, start))


def state_table(root):
    """`{route: [state, ...]}` from `STATES`, or None when the table is absent."""
    text = read(root, STATES)
    if text is None or "export const STATES" not in text:
        return None
    body = braced(text, text.index("= {", text.index("export const STATES")) + 2)
    table = {}
    for match in re.finditer(r"'(/[^']*)':\s*\{", body):
        prefix = body[: match.start()]
        depth = prefix.count("{") - prefix.count("}") + prefix.count("[") - prefix.count("]")
        if depth == 0:
            table[match.group(1)] = keys(braced(body, match.end() - 1))
    return table


def slug(route):
    """A route's name in a baseline's file name."""
    return "today" if route == "/" else route.strip("/").replace("/", "-")


def matrix_findings(root):
    """The kept project and the six phone projects of SPEC-639 R1."""
    text = read(root, CONFIG)
    if text is None:
        return [f"missing: {CONFIG}"]
    found = []
    sizes = {name: (int(w), int(h)) for name, w, h in SIZE_CONST.findall(text)}
    projects = set(PHONE_CALL.findall(text))
    for name, size, scheme, scale in sorted(MATRIX - projects):
        found.append(f"matrix: missing {name} ({size}, {scheme}, text {scale})")
    for name, size, scheme, scale in sorted(projects - MATRIX):
        found.append(f"matrix: unexpected {name} ({size}, {scheme}, text {scale})")
    for size, expected in SIZES.items():
        if sizes.get(size) != expected:
            found.append(f"matrix: {size} is {sizes.get(size)}, not {expected}")
    if "hasTouch: true" not in text:
        found.append("matrix: the phone projects have no touch")
    if not KEPT_PROJECT.search(text):
        found.append("matrix: no kept chromium project ignoring tests/verify/")
    return found


def state_findings(root):
    """A state for every route (SPEC-639 R2)."""
    table, table_routes = state_table(root), routes(root)
    if table is None:
        return [f"missing: {STATES}"]
    found = []
    for route in table_routes or []:
        wanted = LEARNER_STATES + (REVIEW_STATES if route == REVIEW else ())
        for state in wanted:
            if state not in table.get(route, []):
                found.append(f"states: {route} has no {state} state")
    for route in sorted(set(table) - set(table_routes or [])):
        found.append(f"states: {route} is not in the route table")
    return found


def synthetic_findings(root):
    """No real-looking value in a fixture (SPEC-639 R2): no email and no account-sized number."""
    found = []
    for path in (STATES, FIXTURES):
        text = read(root, path)
        if text is None:
            found.append(f"missing: {path}")
            continue
        for value in EMAIL.findall(text):
            found.append(f"synthetic: {path.name} holds an email-shaped value {value}")
        for value in ACCOUNT.findall(text):
            found.append(f"synthetic: {path.name} holds an account-sized number {value}")
    return found


def audit_findings(root):
    """The audit's five tags, `target-size` on by name and its ran-assertion (SPEC-639 R3)."""
    text = read(root, AUDIT)
    if text is None:
        return [f"missing: {AUDIT}"]
    found = []
    if ".withTags(" not in text:
        found.append("audit: axe runs with no tags")
    for tag in TAGS:
        if f"'{tag}'" not in text:
            found.append(f"audit: the tag {tag} is missing")
    if not TARGET_SIZE_ON.search(text):
        found.append("audit: target-size is not turned on by name")
    if not TARGET_SIZE_RAN.search(text):
        found.append("audit: nothing asserts that target-size ran")
    return found


def baseline_findings(root):
    """A committed Linux baseline for every route, state and 390-wide project (SPEC-639 R4)."""
    table = state_table(root)
    if table is None:
        return [f"missing: {STATES}"]
    wanted = [
        f"{project}/verify/snapshots.spec.ts/{slug(route)}-{state}-linux.png"
        for project in SNAPSHOT_PROJECTS
        for route, states in sorted(table.items())
        for state in states
    ] + [PLANTED_SHOT]
    return [f"baseline: missing {name}" for name in wanted if not (root / SHOTS / name).is_file()]


def exception_findings(root):
    """No rule disabled outside `exceptions.ts`, and each entry there whole (SPEC-639 R5)."""
    text = read(root, EXCEPTIONS)
    if text is None:
        return [f"missing: {EXCEPTIONS}"]
    found = []
    for path in sorted((root / VERIFY).glob("*.ts")):
        if path.name != EXCEPTIONS.name and DISABLE.search(path.read_text(encoding="utf-8")):
            found.append(f"exceptions: {path.name} disables a rule outside the table")
    start = text.index("= [", text.index("export const EXCEPTIONS")) + 2
    table = braced(text, start)
    for match in re.finditer(r"\{", table):
        entry = braced(table, match.start())
        fields = dict(re.findall(r"(\w+): '([^']*)'", entry))
        for field in ENTRY_FIELDS:
            if not fields.get(field):
                found.append(f"exceptions: an entry names no {field}")
        if fields.get("issue") and not ISSUE.match(fields["issue"]):
            found.append(f"exceptions: the issue {fields['issue']} is not an issue number")
    return found


def margin_findings(root):
    """The screenshot expectation allows no margin (SPEC-639 R4 as amended, A13)."""
    text = read(root, CONFIG)
    if text is None:
        return [f"missing: {CONFIG}"]
    match = SCREENSHOT.search(text)
    options = match.group(1) if match else ""
    found = []
    if not re.search(r"\bthreshold:" + ZERO, options, re.M):
        found.append("margin: the screenshot threshold is not 0")
    if not re.search(r"\bmaxDiffPixels:" + ZERO, options, re.M):
        found.append("margin: maxDiffPixels is not 0")
    spec_texts = [p.read_text(encoding="utf-8") for p in sorted((root / VERIFY).glob("*.ts"))]
    if any("maxDiffPixelRatio" in part for part in [text] + spec_texts):
        found.append("margin: a maxDiffPixelRatio is set")
    return found


ROUTES_TEXT = """export const ROUTES = [
  '/',
  '/score',
  '/study/review'
] as const;
"""

CONFIG_TEXT = """import { defineConfig } from '@playwright/test';

const PHONE = { width: 390, height: 844 };
const SMALL_PHONE = { width: 360, height: 740 };

function phone(name: string, viewport: Size, colorScheme: Scheme, textScale: number) {
  return { name, testMatch: 'verify/**', use: { viewport, hasTouch: true, colorScheme, textScale } };
}

export default defineConfig({
  testDir: 'tests',
  expect: { toHaveScreenshot: { threshold: 0, maxDiffPixels: 0 } },
  projects: [
    { name: 'chromium', testIgnore: 'verify/**' },
    phone('phone-light', PHONE, 'light', 1),
    phone('phone-dark', PHONE, 'dark', 1),
    phone('phone-light-text200', PHONE, 'light', 2),
    phone('phone-dark-text200', PHONE, 'dark', 2),
    phone('phone-small-light', SMALL_PHONE, 'light', 1),
    phone('phone-small-dark', SMALL_PHONE, 'dark', 1)
  ]
});
"""

STATES_TEXT = """export const STATES: StateTable = {
  '/': {
    first: { '**/api/me': { json: { study_day: '2001-02-03' } } },
    empty: { '**/api/me': { json: { study_day: null } } },
    error: { '**/api/me': { status: 500 } }
  },
  '/score': {
    first: { '**/api/score': { json: { total: 64, reviews: 12 } } },
    empty: { '**/api/score': { json: { total: 0, reviews: 0 } } },
    error: { '**/api/score': { status: 500 } }
  },
  '/study/review': {
    first: {},
    empty: {},
    error: {},
    question: { side: 'question' },
    answer: { side: 'answer' }
  }
};
"""

FIXTURES_TEXT = """export const LAUNCH = { initData: 'auth_date=1&hash=synthetic', user: 'Learner A' };
"""

EXCEPTIONS_TEXT = """export const EXCEPTIONS: readonly RecordedException[] = [
  { rule: 'region', route: '/score', reason: 'a planted reason', issue: '#1' }
];
"""

AUDIT_TEXT = """const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];
const results = await new AxeBuilder({ page })
  .withTags(WCAG_22_AA)
  .options({ rules: { 'target-size': { enabled: true } } })
  .disableRules(disabledFor(route))
  .analyze();
expect(ran(results)).toContain('target-size');
"""


def plant(tmp, **override):
    """A complete fixture tree under `tmp`, each named file replaced by its override (None drops)."""
    root = Path(tmp)
    files = {
        ROUTES_FILE: ROUTES_TEXT,
        CONFIG: CONFIG_TEXT,
        STATES: STATES_TEXT,
        FIXTURES: FIXTURES_TEXT,
        EXCEPTIONS: EXCEPTIONS_TEXT,
        AUDIT: AUDIT_TEXT,
    }
    names = {"routes": ROUTES_FILE, "config": CONFIG, "states": STATES, "fixtures": FIXTURES}
    names.update({"exceptions": EXCEPTIONS, "audit": AUDIT})
    for key, text in override.items():
        if key in names:
            files[names[key]] = text
    for path, text in files.items():
        if text is not None:
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_text(text, encoding="utf-8")
    shots = [
        f"{project}/verify/snapshots.spec.ts/{slug(route)}-{state}-linux.png"
        for project in SNAPSHOT_PROJECTS
        for route, states in (state_table(root) or {}).items()
        for state in states
    ] + [PLANTED_SHOT]
    for name in shots:
        if name != override.get("drop_shot"):
            (root / SHOTS / name).parent.mkdir(parents=True, exist_ok=True)
            (root / SHOTS / name).write_bytes(b"\x89PNG")
    return root


CHECKS = (
    matrix_findings,
    state_findings,
    synthetic_findings,
    audit_findings,
    baseline_findings,
    exception_findings,
    margin_findings,
)


class TheTreeIsHeld(unittest.TestCase):
    """Each check over this repository."""

    def test_the_config_holds_the_kept_project_and_the_six_phone_projects(self):
        text = read(REPO, CONFIG) or ""
        examined("phone project calls in the config", PHONE_CALL.findall(text))
        self.assertEqual(matrix_findings(REPO), [])

    def test_every_route_has_its_learner_states(self):
        examined("routes in the route table", routes(REPO) or [])
        examined("routes in the state table", (state_table(REPO) or {}).items())
        self.assertEqual(state_findings(REPO), [])

    def test_every_fixture_value_is_synthetic(self):
        text = (read(REPO, STATES) or "") + (read(REPO, FIXTURES) or "")
        examined("literal values in the state table and fixtures", LITERAL.findall(text))
        self.assertEqual(synthetic_findings(REPO), [])

    def test_the_audit_runs_the_five_tags_and_target_size(self):
        text = read(REPO, AUDIT) or ""
        examined("tags named in the audit", [tag for tag in TAGS if f"'{tag}'" in text])
        self.assertEqual(audit_findings(REPO), [])

    def test_every_route_state_and_project_has_a_baseline(self):
        examined("baseline images", (REPO / SHOTS).rglob("*-linux.png"))
        self.assertEqual(baseline_findings(REPO), [])

    def test_no_rule_is_disabled_outside_the_exceptions_table(self):
        examined("verify modules", (REPO / VERIFY).glob("*.ts"))
        self.assertEqual(exception_findings(REPO), [])

    def test_the_screenshot_expectation_allows_no_margin(self):
        examined(
            "screenshot expectations in the config", SCREENSHOT.findall(read(REPO, CONFIG) or "")
        )
        self.assertEqual(margin_findings(REPO), [])


class PlantedTreesAreRefused(unittest.TestCase):
    """Each check over a fixture tree: the complete control, and one planted fault per reason."""

    def findings(self, check, **override):
        with tempfile.TemporaryDirectory() as tmp:
            return check(plant(tmp, **override))

    def test_the_complete_planted_tree_passes_every_check(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            self.assertEqual(len(examined("baseline images", (root / SHOTS).rglob("*.png"))), 45)
            for check in examined("checks", CHECKS):
                self.assertEqual(check(root), [], check.__name__)

    def test_a_planted_missing_phone_project_is_refused(self):
        for name, _, _, _ in examined("phone projects", sorted(MATRIX)):
            line = next(line for line in CONFIG_TEXT.splitlines() if f"'{name}'" in line)
            config = CONFIG_TEXT.replace(line + "\n", "")
            found = self.findings(matrix_findings, config=config)
            self.assertEqual(len(found), 1, found)
            self.assertTrue(found[0].startswith(f"matrix: missing {name} "), found)
        found = self.findings(matrix_findings, config=CONFIG_TEXT.replace("390", "400", 1))
        self.assertEqual(found, ["matrix: PHONE is (400, 844), not (390, 844)"])
        kept = CONFIG_TEXT.replace("testIgnore: 'verify/**'", "testMatch: '**'")
        found = self.findings(matrix_findings, config=kept)
        self.assertEqual(found, ["matrix: no kept chromium project ignoring tests/verify/"])

    def test_a_planted_route_with_no_state_is_refused(self):
        routes_text = ROUTES_TEXT.replace("'/score',", "'/score',\n  '/wallet',")
        found = self.findings(state_findings, routes=routes_text)
        self.assertEqual(
            found, [f"states: /wallet has no {state} state" for state in LEARNER_STATES]
        )
        states = STATES_TEXT.replace("    question: { side: 'question' },\n", "")
        found = self.findings(state_findings, states=states)
        self.assertEqual(found, ["states: /study/review has no question state"])
        found = self.findings(state_findings, states=None)
        self.assertEqual(found, [f"missing: {STATES}"])

    def test_a_planted_real_looking_fixture_value_is_refused(self):
        email = FIXTURES_TEXT.replace("Learner A", "learner@example.org")
        found = self.findings(synthetic_findings, fixtures=email)
        self.assertEqual(
            found, ["synthetic: fixtures.ts holds an email-shaped value learner@example.org"]
        )
        account = STATES_TEXT.replace("reviews: 12", "reviews: 12, serial: 7654321")
        found = self.findings(synthetic_findings, states=account)
        self.assertEqual(found, ["synthetic: states.ts holds an account-sized number 7654321"])

    def test_a_planted_audit_without_target_size_is_refused(self):
        off = AUDIT_TEXT.replace(
            "  .options({ rules: { 'target-size': { enabled: true } } })\n", ""
        )
        found = self.findings(audit_findings, audit=off)
        self.assertEqual(found, ["audit: target-size is not turned on by name"])
        unasserted = AUDIT_TEXT.replace("expect(ran(results)).toContain('target-size');\n", "")
        found = self.findings(audit_findings, audit=unasserted)
        self.assertEqual(found, ["audit: nothing asserts that target-size ran"])

    def test_a_planted_audit_missing_a_wcag_tag_is_refused(self):
        for tag in examined("WCAG tags", TAGS):
            audit = AUDIT_TEXT.replace(f"'{tag}', ", "").replace(f", '{tag}'", "")
            found = self.findings(audit_findings, audit=audit)
            self.assertEqual(found, [f"audit: the tag {tag} is missing"])

    def test_a_planted_missing_baseline_is_refused(self):
        shot = "phone-dark-text200/verify/snapshots.spec.ts/study-review-answer-linux.png"
        found = self.findings(baseline_findings, drop_shot=shot)
        self.assertEqual(found, [f"baseline: missing {shot}"])
        found = self.findings(baseline_findings, drop_shot=PLANTED_SHOT)
        self.assertEqual(found, [f"baseline: missing {PLANTED_SHOT}"])

    def test_a_planted_bare_disable_is_refused(self):
        bare = AUDIT_TEXT.replace("disabledFor(route)", "['color-contrast']")
        found = self.findings(exception_findings, audit=bare)
        self.assertEqual(
            found, ["exceptions: a11y-themes.spec.ts disables a rule outside the table"]
        )
        off = AUDIT_TEXT.replace("enabled: true", "enabled: false")
        found = self.findings(exception_findings, audit=off)
        self.assertEqual(
            found, ["exceptions: a11y-themes.spec.ts disables a rule outside the table"]
        )
        for field in examined("exception entry fields", ENTRY_FIELDS):
            entry = re.sub(rf"{field}: '[^']*'", f"{field}: ''", EXCEPTIONS_TEXT)
            found = self.findings(exception_findings, exceptions=entry)
            self.assertEqual(found, [f"exceptions: an entry names no {field}"])
        found = self.findings(exception_findings, exceptions=EXCEPTIONS_TEXT.replace("#1", "1"))
        self.assertEqual(found, ["exceptions: the issue 1 is not an issue number"])

    def test_a_planted_screenshot_margin_is_refused(self):
        margins = {
            "threshold: 0,": ("threshold: 0.2,", "margin: the screenshot threshold is not 0"),
            "maxDiffPixels: 0 ": ("maxDiffPixels: 10 ", "margin: maxDiffPixels is not 0"),
            "maxDiffPixels: 0 }": (
                "maxDiffPixels: 0, maxDiffPixelRatio: 0.001 }",
                "margin: a maxDiffPixelRatio is set",
            ),
        }
        for old, (new, reason) in examined("margin operands", margins.items()):
            found = self.findings(margin_findings, config=CONFIG_TEXT.replace(old, new))
            self.assertEqual(found, [reason])


if __name__ == "__main__":
    unittest.main()
