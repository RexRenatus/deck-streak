"""The deploy templates: every unit, timer and the Caddy block is hardened, fits the host budget,
and names no private value (SPEC-032; ADR-007, ADR-010, ADR-025, ADR-032, ADR-038).

The units are read through the vendored durable lint's own parser, so the tests and the pack read
a unit exactly the same way. Every enumerating test prints `examined N` and refuses zero, and every
absence it asserts is paired with a planted template it must refuse. No test writes a template
instance name literally (SPEC-032 R10): each is built at run time from its template and its id.
"""

import importlib.util
import ipaddress
import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

DEPLOY = REPO / "deploy"
SYSTEMD = DEPLOY / "systemd"
CADDY = DEPLOY / "caddy" / "deck-streak.caddy"
BUDGET = DEPLOY / "host-budget.json"
ENV_EXAMPLE = DEPLOY / "deck-streak.env.example"
LINT = REPO / ".packs" / "scripts" / "durable-unit-lint.py"
SCRUB = REPO / "scripts" / "public-scrub.py"
WIRING = REPO / ".packs" / "wiring.json"
ADR = REPO / "docs" / "decisions" / "ADR-032-deploy-templates-and-the-host-budget.md"

# The one release binary every service runs, from the release root's `current` link (R2).
BINARY = "/usr/local/lib/deck-streak/current/bin/deckstreakd"
# The one required settings file of every service (R3), named without the optional `-`.
ENVIRONMENT_FILE = "/etc/deck-streak/deck-streak.env"
# The credential socket the private rail serves (ADR-038).
SOCKET = "/run/deck-streak-credentials/socket"
# The alert template every service pages through on failure (R2; SPEC-031 ships it).
ON_FAILURE = "deck-streak-alert@%n.service"
# The job template, whose instances the timers start (R1).
JOB_TEMPLATE = "deck-streak-job"

# Where the code declares each credential id a role reads, by the constant's name.
CREDENTIAL_SOURCES = {
    "OWNER_USER_ID": REPO / "crates" / "identity" / "src" / "owner.rs",
    "TELEGRAM_BOT_TOKEN": REPO / "crates" / "identity" / "src" / "owner.rs",
    "SYNC_USERNAME": REPO / "crates" / "ingest" / "src" / "settings.rs",
    "SYNC_PASSWORD": REPO / "crates" / "ingest" / "src" / "settings.rs",
}
# Which credentials each service's role reads: the api's owner gate (SPEC-024, SPEC-025), the bot's
# transport, owner gate and `/sync` (SPEC-026 R1, R11), and the `sync` job's syncer (SPEC-022,
# SPEC-027). The job template carries the sync's pair for every instance, and the private rail's map
# answers them for the `sync` instance alone (ADR-038).
ROLE_CREDENTIALS = {
    "deck-streak-api.service": ("OWNER_USER_ID", "TELEGRAM_BOT_TOKEN"),
    "deck-streak-bot.service": (
        "OWNER_USER_ID",
        "TELEGRAM_BOT_TOKEN",
        "SYNC_USERNAME",
        "SYNC_PASSWORD",
    ),
    f"{JOB_TEMPLATE}@.service": ("SYNC_USERNAME", "SYNC_PASSWORD"),
}
# The role each service runs (R2); the job template's `%i` is its instance, the job's id.
ROLES = {
    "deck-streak-api.service": "api",
    "deck-streak-bot.service": "bot",
    f"{JOB_TEMPLATE}@.service": "job %i",
}
# The settings a role requires, which the committed example must therefore name (SPEC-025 R6,
# SPEC-022 R5); systemd sets STATE_DIRECTORY and CREDENTIALS_DIRECTORY itself.
REQUIRED_SETTINGS = ("DECKSTREAK_API_LISTEN", "DECKSTREAK_SYNC_ENDPOINT")
SYSTEMD_SETS = ("STATE_DIRECTORY", "CREDENTIALS_DIRECTORY")

# R1: the lifecycle of the two long-running services.
DAEMON_SERVICE = {
    "Type": "notify",
    "WatchdogSec": "90",
    "Restart": "on-failure",
    "RestartSec": "15",
    "TimeoutStartSec": "180",
    "TimeoutStopSec": "30",
    "OOMPolicy": "kill",
}
DAEMON_UNIT = {"StartLimitBurst": "5", "StartLimitIntervalSec": "300"}
# R1: the job template yields to the daemons, and ADR-032's delivery bounds a hung run.
JOB_SERVICE = {
    "Type": "oneshot",
    "Nice": "10",
    "IOSchedulingClass": "idle",
    "TimeoutStartSec": "30min",
}
# ADR-032's delivery: the daemons' CPU and task caps.
DAEMON_CAPS = {
    "deck-streak-api.service": {"CPUQuota": "100%", "TasksMax": "64"},
    "deck-streak-bot.service": {"CPUQuota": "50%", "TasksMax": "64"},
}
# R2: the identity and hardening of every service, each at the value the pack's rows score.
HARDENING = {
    "User": "deck-streak",
    "Group": "deck-streak",
    "StateDirectory": "deck-streak",
    "UMask": "0077",
    "ProtectSystem": "strict",
    "ProtectHome": "yes",
    "PrivateTmp": "yes",
    "PrivateDevices": "yes",
    "NoNewPrivileges": "yes",
    "SystemCallFilter": "@system-service",
    "SystemCallArchitectures": "native",
    "RestrictAddressFamilies": "AF_UNIX AF_INET AF_INET6",
    "CapabilityBoundingSet": "",
    "ProtectKernelTunables": "yes",
    "ProtectKernelModules": "yes",
    "ProtectKernelLogs": "yes",
    "ProtectControlGroups": "yes",
    "ProtectClock": "yes",
    "ProtectHostname": "yes",
    "ProtectProc": "invisible",
    "ProcSubset": "pid",
    "RestrictNamespaces": "yes",
    "RestrictRealtime": "yes",
    "RestrictSUIDSGID": "yes",
    "LockPersonality": "yes",
    "MemoryDenyWriteExecute": "yes",
    "EnvironmentFile": ENVIRONMENT_FILE,
}
# The security headers R6 names, at the values the Caddy block sends.
HEADERS = {
    "Content-Security-Policy": (
        "frame-ancestors https://web.telegram.org; object-src 'none'; base-uri 'self'"
    ),
    "Strict-Transport-Security": "max-age=31536000; includeSubDomains",
    "X-Content-Type-Options": "nosniff",
    "X-Robots-Tag": "noindex",
}
# Referrer policies that never send a URL to another origin (web-security `ws.referrer-policy`).
KEEPS_URLS = {"no-referrer", "same-origin", "strict-origin", "strict-origin-when-cross-origin"}
ONE_YEAR = 31_536_000
# Every advisory the templates depart from, by unit and reason, each waived with its why (R4).
WAIVED = {
    (f"{JOB_TEMPLATE}@sync.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@maintenance.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@maintenance.timer", "calendar-not-persistent"),
    (f"{JOB_TEMPLATE}@liveness.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@liveness.timer", "calendar-not-persistent"),
}
# Every advisory whose subject waits on an open issue, by where the lint reports it and its reason:
# SPEC-021's Litestream template is committed before the unit that runs `litestream replicate`,
# which the backups issue builds with the daily backup and the restore drill (#44). Each must still
# fire, so an entry that outlives its reason fails rather than passes. SPEC-021's journald drop-in
# sets `SystemMaxUse=`, so the journal's size cap no longer departs from its advisory.
WAITING = {("deploy", "litestream-unit-missing"): "#44"}


def load_lint():
    """The vendored durable lint as a module, registered before it runs, because its dataclasses
    resolve their annotations through `sys.modules`."""
    if "durable_unit_lint" in sys.modules:
        return sys.modules["durable_unit_lint"]
    spec = importlib.util.spec_from_file_location("durable_unit_lint", LINT)
    module = importlib.util.module_from_spec(spec)
    sys.modules["durable_unit_lint"] = module
    spec.loader.exec_module(module)
    return module


def lint_json(root, *args):
    """The durable lint's JSON report over `root`, and its exit."""
    done = subprocess.run(
        [sys.executable, str(LINT), *args, "--root", str(root), "--format", "json"],
        capture_output=True,
        text=True,
        check=False,
    )
    return json.loads(done.stdout), done.returncode


def subject(root=REPO):
    """Every unit under `root`'s deploy/, parsed as systemd reads it."""
    return load_lint().load_subject(Path(root))


def services(root=REPO):
    return examined("service unit(s) under deploy/", sorted(subject(root).services, key=name))


def name(unit):
    return unit.name


def last(unit, section, key):
    value = unit.last(section, key)
    return None if value is None else value.strip()


def size(text):
    """A systemd byte size such as 128M, in bytes."""
    found = load_lint().size_bytes(text)
    if found is None:
        raise AssertionError(f"{text!r} is not a byte size")
    return found


def budget():
    return json.loads(BUDGET.read_text(encoding="utf-8"))


def adr_budget():
    """ADR-032's budget table, by unit: (memory_high, memory_max)."""
    rows = re.findall(r"(?m)^\| `([^`]+)` \| (\d+M) \| (\d+M) \|", ADR.read_text(encoding="utf-8"))
    return {unit: (high, ceiling) for unit, high, ceiling in rows}


def credential_ids():
    """Each credential id the code declares, by its constant's name."""
    found = {}
    for constant, source in CREDENTIAL_SOURCES.items():
        match = re.search(rf'pub const {constant}: &str = "([a-z0-9-]+)";', source.read_text())
        if match is None:
            raise AssertionError(f"{source.relative_to(REPO)} declares no {constant}")
        found[constant] = match.group(1)
    return found


def declared_settings():
    """Every setting the workspace's code names: a `DECKSTREAK_*` constant in `crates/*/src`."""
    found = set()
    for source in (REPO / "crates").glob("*/src/**/*.rs"):
        found.update(
            re.findall(r'pub const [A-Z_]+: &str = "(DECKSTREAK_[A-Z_]+)";', source.read_text())
        )
    return found


def env_example():
    """The committed example's active `KEY=VALUE` lines, as (line, key, value)."""
    return load_lint().env_assignments(ENV_EXAMPLE)


def credential_lines(root):
    """Every credential directive of every template under `root`, as (file, line, key, value)."""
    keys = ("LoadCredential", "LoadCredentialEncrypted", "SetCredential")
    keys += ("SetCredentialEncrypted", "ImportCredential")
    found = []
    for path in sorted(Path(root).rglob("*")):
        if path.suffix not in (".service", ".timer", ".conf") or not path.is_file():
            continue
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            key, equals, value = line.strip().partition("=")
            if equals and key in keys:
                found.append((path.relative_to(root).as_posix(), number, key, value))
    return found


def socket_form_refusals(lines, ids):
    """Every credential line that is not `LoadCredential=<a DeckStreak id>:<the socket>`."""
    refused = []
    for where, number, key, value in lines:
        ident, _, path = value.partition(":")
        if key != "LoadCredential":
            refused.append(f"{where}:{number}: {key}= is refused; use LoadCredential= (ADR-038)")
        elif ident not in ids:
            refused.append(f"{where}:{number}: {ident!r} is not a DeckStreak credential id")
        elif path != SOCKET:
            refused.append(f"{where}:{number}: {ident} is read from {path!r}, not the socket")
    return refused


def environment_refusals(unit, ids):
    """Every way `unit` passes a secret through its environment."""
    lint = load_lint()
    refused = []
    for value in unit.values("Service", "Environment") + unit.values("Service", "PassEnvironment"):
        for word in value.split():
            variable = word.partition("=")[0].strip("\"'")
            if lint.SECRET_NAME.search(variable) or variable.lower().replace("_", "-") in ids:
                refused.append(f"{unit.rel}: {variable} passes a secret through the environment")
    return refused


# --- the Caddyfile, read as Caddy's lexer reads it -------------------------------------------


class Directive:
    """One Caddyfile line: its tokens, the line it starts on, and the block it opens."""

    def __init__(self, tokens, line):
        self.tokens = tokens
        self.line = line
        self.children = []

    def find(self, *head):
        """Every child directive whose tokens start with `head`."""
        return [child for child in self.children if tuple(child.tokens[: len(head)]) == head]

    def one(self, *head):
        found = self.find(*head)
        if len(found) != 1:
            raise AssertionError(f"{len(found)} `{' '.join(head)}` directive(s), not one")
        return found[0]


def words(line):
    """A line's tokens: quoted strings keep their spaces, and a `#` that begins a token begins a
    comment."""
    tokens, current, quoted, index = [], "", False, 0
    while index < len(line):
        char = line[index]
        if quoted:
            if char == "\\" and index + 1 < len(line) and line[index + 1] == '"':
                current += '"'
                index += 1
            elif char == '"':
                quoted = False
            else:
                current += char
        elif char == '"':
            quoted = True
        elif char.isspace():
            if current:
                tokens.append(current)
            current = ""
        elif char == "#" and not current:
            break
        else:
            current += char
        index += 1
    if current:
        tokens.append(current)
    return tokens


def caddy_tree(text):
    """The Caddyfile as a tree of directives; a heredoc's body is one token."""
    root = Directive([], 0)
    stack = [root]
    lines = text.splitlines()
    number = 0
    while number < len(lines):
        tokens = words(lines[number])
        start = number + 1
        if tokens and tokens[-1].startswith("<<"):
            marker = tokens[-1][2:]
            body = []
            number += 1
            while not lines[number].strip().startswith(marker):
                body.append(lines[number])
                number += 1
            closing = lines[number]
            indent = closing[: len(closing) - len(closing.lstrip())]
            text_body = "\n".join(line.removeprefix(indent) for line in body)
            tokens = [*tokens[:-1], text_body, *words(closing.strip()[len(marker) :])]
        number += 1
        if not tokens:
            continue
        if tokens == ["}"]:
            stack.pop()
            continue
        opens = tokens[-1] == "{"
        directive = Directive(tokens[:-1] if opens else tokens, start)
        stack[-1].children.append(directive)
        if opens:
            stack.append(directive)
    if len(stack) != 1:
        raise AssertionError(f"{len(stack) - 1} block(s) left open")
    return root


def site():
    """The one site block of the Caddy template."""
    blocks = caddy_tree(CADDY.read_text(encoding="utf-8")).children
    sites = [block for block in blocks if block.children]
    if len(sites) != 1:
        raise AssertionError(f"{len(sites)} site block(s) in {CADDY.relative_to(REPO)}, not one")
    return sites[0]


def header_operations(block):
    """The site-level `header` operations, by field: `-Server` removes, anything else sets."""
    operations = {}
    for header in block.find("header"):
        lines = [header.tokens[1:]] if len(header.tokens) > 1 else []
        lines += [child.tokens for child in header.children]
        for tokens in lines:
            operations[tokens[0]] = " ".join(tokens[1:])
    return operations


def policy(text):
    """A Content-Security-Policy, by directive."""
    directives = {}
    for part in text.split(";"):
        tokens = part.split()
        if tokens:
            directives[tokens[0].lower()] = tokens[1:]
    return directives


def handle(block, *matcher):
    """The `handle` block with exactly this matcher (none for the fallback)."""
    return next(
        (h for h in block.find("handle") if tuple(h.tokens[1:]) == matcher),
        None,
    )


def route_constant(constant):
    source = REPO / "crates" / "api" / "src" / "health.rs"
    match = re.search(rf'pub const {constant}: &str = "([^"]+)";', source.read_text())
    if match is None:
        raise AssertionError(f"crates/api/src/health.rs declares no {constant}")
    return match.group(1)


def scrub(*subjects):
    return subprocess.run(
        [sys.executable, str(SCRUB), "--root", str(REPO), "--no-tree"]
        + [argument for path in subjects for argument in ("--subject", str(path))],
        capture_output=True,
        text=True,
        check=False,
    )


class TheTemplatesPassTheDurableLint(unittest.TestCase):
    def test_the_durable_lint_finds_no_blocking_defect_in_the_templates(self):
        files = examined(
            "unit file(s) under deploy/",
            [p for p in DEPLOY.rglob("*") if p.suffix in (".service", ".timer", ".slice")],
        )
        report, _ = lint_json(REPO, "lint")
        self.assertEqual(report["examined"]["units"], len(files), report["examined"])
        wiring = json.loads(WIRING.read_text(encoding="utf-8"))["packs"]["durable-services"]
        deferred = wiring.get("deferred_rows", {})
        blocking = examined(
            "blocking check(s)", [c for c in report["checks"] if c["severity"] == "blocking"]
        )
        refused = {
            check["id"]: check["findings"]
            for check in blocking
            if check["verdict"] != "pass" and check["id"] not in deferred
        }
        self.assertEqual(refused, {}, "the durable lint refused the templates")
        # Only the rows whose subject the backups issue builds wait, and the pack is enforced (R8).
        self.assertEqual(
            deferred,
            {"backup.copies": "#44", "backup.offsite": "#44", "backup.restore-drill": "#44"},
        )
        self.assertEqual(wiring["state"], "enforced")

    def test_every_departure_from_an_advisory_is_waived_with_its_why(self):
        report, _ = lint_json(REPO, "lint")
        advisories = examined(
            "advisory check(s)", [c for c in report["checks"] if c["severity"] == "advisory"]
        )
        unwaived = {
            (finding["unit"], check["reason"])
            for check in advisories
            for finding in check["findings"]
        }
        self.assertEqual(
            unwaived, set(WAITING), "an advisory departure is neither waived nor waiting"
        )
        waived = {
            (Path(item["unit"]).name, check["reason"])
            for check in advisories
            for item in check["waived"]
        }
        self.assertEqual(waived, WAIVED)
        for check in advisories:
            for item in check["waived"]:
                self.assertGreater(len(item["why"].split()), 5, f"{item['unit']}: a thin why")


class TheTemplatesFitTheHostBudget(unittest.TestCase):
    def test_every_unit_ceiling_matches_the_host_budget_and_high_is_below_max(self):
        units = services()
        entries = budget()["units"]
        self.assertEqual(sorted(entries), [unit.name for unit in units], "budget against units")
        decided = adr_budget()
        for unit in units:
            entry = entries[unit.name]
            high = last(unit, "Service", "MemoryHigh")
            ceiling = last(unit, "Service", "MemoryMax")
            self.assertEqual((high, ceiling), (entry["memory_high"], entry["memory_max"]), unit.rel)
            self.assertEqual((high, ceiling), decided.get(unit.name), f"{unit.rel} against ADR-032")
            self.assertLess(
                size(high), size(ceiling), f"{unit.rel}: MemoryHigh must throttle first"
            )

    def test_the_daemons_and_the_largest_job_fit_the_stack_share(self):
        lint = load_lint()
        units = services()
        share = budget()["memory"]
        self.assertIn(f'"memory": "{share}"', ADR.read_text(encoding="utf-8"), "ADR-032's share")
        ceilings = {}
        for unit in units:
            value = last(unit, "Service", "MemoryMax")
            self.assertIsNotNone(
                value, f"{unit.rel} has no MemoryMax, so it cannot be shown to fit"
            )
            ceilings[unit.name] = size(value)
        daemons = [u.name for u in units if lint.long_running(u)]
        oneshots = [u.name for u in units if not lint.long_running(u)]
        self.assertEqual(daemons, ["deck-streak-api.service", "deck-streak-bot.service"])
        self.assertEqual(oneshots, [f"{JOB_TEMPLATE}@.service"])
        worst = sum(ceilings[unit] for unit in daemons) + max(ceilings[unit] for unit in oneshots)
        # ADR-032's arithmetic: 128 + 96 for the daemons, and the job's 384.
        self.assertEqual(worst, size("608M"))
        self.assertLessEqual(worst, size(share), "the worst case exceeds DeckStreak's share")
        # The daemons' CPU quotas fit the share's CPUs.
        quotas = [
            int(last(u, "Service", "CPUQuota").rstrip("%")) for u in units if u.name in daemons
        ]
        self.assertLessEqual(sum(quotas), 100 * budget()["cpus"], quotas)
        report, code = lint_json(REPO, "check", "--id", "resources.budget")
        self.assertEqual((code, report["checks"][0]["verdict"]), (0, "pass"), report["checks"])


class TheCaddyBlock(unittest.TestCase):
    def test_the_caddy_policy_admits_telegram_web_and_sends_the_security_headers(self):
        block = site()
        operations = header_operations(block)
        for field, value in HEADERS.items():
            self.assertEqual(operations.get(field), value, f"the block's {field}")
        self.assertIn("-Server", operations, "the Server header is not removed")
        directives = policy(operations["Content-Security-Policy"])
        self.assertEqual(directives["frame-ancestors"], ["https://web.telegram.org"])
        self.assertEqual(directives["object-src"], ["'none'"])
        self.assertEqual(directives["base-uri"], ["'self'"])
        # The script policy is the page's own meta policy, with its build's hashes (SPEC-028 R14):
        # a script-src or default-src here would block the scripts that policy admits.
        self.assertNotIn("script-src", directives)
        self.assertNotIn("default-src", directives)
        config = (REPO / "web" / "app" / "svelte.config.js").read_text(encoding="utf-8")
        self.assertRegex(config, r"'script-src': \['self', 'https://telegram\.org'\]")
        sent = operations["Strict-Transport-Security"].split(";")
        hsts = dict(part.strip().partition("=")[::2] for part in sent)
        self.assertGreaterEqual(int(hsts["max-age"]), ONE_YEAR)
        self.assertIn("includeSubDomains", hsts)
        # The same referrer policy as the page's own meta element, which keeps URLs home.
        page = (REPO / "web" / "app" / "src" / "app.html").read_text(encoding="utf-8")
        meta = re.search(r'<meta name="referrer" content="([a-z-]+)"', page).group(1)
        self.assertEqual(operations.get("Referrer-Policy"), meta)
        self.assertIn(meta, KEEPS_URLS)
        # No switch that weakens the edge: automatic HTTPS stays on, TLS stays at 1.2 or above.
        text = CADDY.read_text(encoding="utf-8")
        for weakening in ("auto_https", "tls internal", "tls1.0", "tls1.1"):
            self.assertNotIn(weakening, text)

    def test_the_caddy_block_serves_the_spa_proxies_the_api_and_hides_health(self):
        block = site()
        self.assertEqual(block.tokens, ["{$DECKSTREAK_HOST}"])
        api = handle(block, "/api/*")
        self.assertIsNotNone(api, "no handle for /api/*")
        self.assertEqual(
            api.one("reverse_proxy").tokens, ["reverse_proxy", "{$DECKSTREAK_API_UPSTREAM}"]
        )
        # The health routes the API serves are answered 404 from outside (ADR-025), inside the API's
        # own handle, where respond runs before reverse_proxy in Caddy's directive order.
        health = [route_constant("LIVEZ"), route_constant("READYZ")]
        matchers = [child for child in api.children if child.tokens[0].startswith("@")]
        self.assertEqual([m.tokens[1:] for m in matchers], [["path", *health]])
        self.assertEqual(api.one("respond").tokens, ["respond", matchers[0].tokens[0], "404"])
        # The SPA: files from the web root, the fallback page for every route the client renders.
        spa = handle(block)
        self.assertIsNotNone(spa, "no fallback handle")
        self.assertEqual(spa.one("root").tokens, ["root", "*", "{$DECKSTREAK_WEB_ROOT}"])
        fallback = re.search(
            r"fallback: '([^']+)'", (REPO / "web" / "app" / "svelte.config.js").read_text()
        ).group(1)
        self.assertEqual(
            spa.one("try_files").tokens, ["try_files", "{path}", "{path}.html", f"/{fallback}"]
        )
        self.assertEqual(spa.one("file_server").tokens, ["file_server"])
        # robots.txt disallows everything, answered before the build's own file is reached.
        robots = handle(block, "/robots.txt")
        self.assertIsNotNone(robots, "no handle for /robots.txt")
        answer = robots.one("respond").tokens
        self.assertEqual(answer[2:], ["200"])
        self.assertEqual(answer[1].splitlines(), ["User-agent: *", "Disallow: /"])
        # Nothing answers outside the handles, so a route cannot slip past the order above.
        self.assertEqual(
            sorted(child.tokens[0] for child in block.children), ["handle"] * 3 + ["header"]
        )
        # The upstream is the API's own loopback listener (ADR-007), which the example names.
        listen = dict((key, value) for _, key, value in env_example())["DECKSTREAK_API_LISTEN"]
        host = listen.rpartition(":")[0].strip("[]")
        self.assertTrue(ipaddress.ip_address(host).is_loopback, listen)


class NoPrivateValue(unittest.TestCase):
    def test_no_deploy_template_names_a_private_value(self):
        files = examined("file(s) under deploy/", [p for p in DEPLOY.rglob("*") if p.is_file()])
        done = scrub(DEPLOY)
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn(f"examined {len(files)} file(s)", done.stdout)
        # A planted template carrying a private-range address is refused by name and line.
        address = ".".join(str(octet) for octet in (10, 24, 36, 48))
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "planted.service"
            planted.write_text(f"[Service]\nEnvironment=DECKSTREAK_API_LISTEN={address}:8080\n")
            done = scrub(scratch)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("planted.service:2: ipv4", done.stdout)
        self.assertNotIn(address, done.stdout)


class NoSecretInTheEnvironment(unittest.TestCase):
    def test_no_unit_passes_a_secret_through_its_environment(self):
        ids = credential_ids()
        # The one settings file every service reads names no secret, only settings a role reads.
        settings = examined("setting(s) in the committed example", env_example())
        declared = declared_settings()
        lint = load_lint()
        for number, key, value in settings:
            where = f"{ENV_EXAMPLE.relative_to(REPO)}:{number}"
            self.assertIsNone(lint.SECRET_NAME.search(key), f"{where}: {key} names a secret")
            self.assertNotIn(key, SYSTEMD_SETS, f"{where}: systemd sets {key}")
            self.assertIn(key, declared | {"RUST_LOG"}, f"{where}: no role reads {key}")
        self.assertLessEqual(set(REQUIRED_SETTINGS), {key for _, key, _ in settings})
        units = services()
        for unit in units:
            self.assertEqual(environment_refusals(unit, set(ids.values())), [], unit.rel)
            # Each credential the role reads reaches the unit as a credential instead.
            loaded = {value.partition(":")[0] for value in unit.values("Service", "LoadCredential")}
            wanted = {ids[constant] for constant in ROLE_CREDENTIALS[unit.name]}
            self.assertEqual(loaded, wanted, f"{unit.rel}: the credentials its role reads")
        # The pack's secrets rows agree, and refuse a planted unit that passes the bot token.
        for row in ("secrets.environment-literal", "secrets.env-file", "secrets.credentials"):
            report, code = lint_json(REPO, "check", "--id", row)
            self.assertEqual(code, 0, report["checks"])
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "deploy" / "systemd" / "planted.service"
            planted.parent.mkdir(parents=True)
            token = "".join(("73", "91", "a2c4e6"))
            planted.write_text(
                "[Unit]\nDescription=planted\n\n[Service]\nExecStart=/bin/true\n"
                f"Environment=DECKSTREAK_BOT_TOKEN={token}\n"
            )
            report, code = lint_json(scratch, "check", "--id", "secrets.environment-literal")
            planted_units = subject(scratch).services
        self.assertEqual(code, 1, report["checks"])
        self.assertEqual(
            [environment_refusals(u, set(ids.values())) for u in planted_units],
            [
                [
                    "deploy/systemd/planted.service: DECKSTREAK_BOT_TOKEN passes a secret through the "
                    "environment"
                ]
            ],
        )


class CredentialsComeFromTheSocket(unittest.TestCase):
    def test_every_credential_line_has_the_socket_form_and_encrypted_is_refused(self):
        ids = set(credential_ids().values())
        lines = examined("credential line(s) under deploy/", credential_lines(DEPLOY))
        self.assertEqual(socket_form_refusals(lines, ids), [])
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "planted.service"
            planted.write_text(
                "[Service]\n"
                f"LoadCredentialEncrypted=telegram-bot-token:{SOCKET}\n"
                f"LoadCredential=telegram-bot-token:{SOCKET}\n"
            )
            found = credential_lines(scratch)
        self.assertEqual(len(found), 2)
        self.assertEqual(
            socket_form_refusals(found, ids),
            [
                "planted.service:2: LoadCredentialEncrypted= is refused; use LoadCredential= (ADR-038)"
            ],
        )


class TheServicesRunTheirRoles(unittest.TestCase):
    def test_every_service_runs_its_role_with_the_lifecycle_r1_names(self):
        for unit in services():
            role = ROLES[unit.name]
            self.assertEqual(unit.values("Service", "ExecStart"), [f"{BINARY} {role}"], unit.rel)
            self.assertEqual(last(unit, "Unit", "OnFailure"), ON_FAILURE, unit.rel)
            identifier = unit.name.removesuffix(".service") if "@" not in unit.name else "%N"
            self.assertEqual(last(unit, "Service", "SyslogIdentifier"), identifier, unit.rel)
            if role.startswith("job"):
                for key, value in JOB_SERVICE.items():
                    self.assertEqual(last(unit, "Service", key), value, f"{unit.rel} {key}")
                self.assertEqual(unit.values("Install", "WantedBy"), [], "the timers start it")
                continue
            for key, value in DAEMON_SERVICE.items():
                self.assertEqual(last(unit, "Service", key), value, f"{unit.rel} {key}")
            for key, value in DAEMON_UNIT.items():
                self.assertEqual(last(unit, "Unit", key), value, f"{unit.rel} {key}")
            for key, value in DAEMON_CAPS[unit.name].items():
                self.assertEqual(last(unit, "Service", key), value, f"{unit.rel} {key}")
            self.assertEqual(unit.values("Install", "WantedBy"), ["multi-user.target"], unit.rel)
        timers = examined("job timer(s)", subject().timers)
        for timer in timers:
            self.assertTrue(timer.name.startswith(f"{JOB_TEMPLATE}@"), timer.rel)
            self.assertEqual(timer.values("Install", "WantedBy"), ["timers.target"], timer.rel)
            # R10: a timer starts the service of its own name, which systemd's default Unit= is.
            self.assertIsNone(last(timer, "Timer", "Unit"), timer.rel)

    def test_every_service_carries_the_hardening_r2_names(self):
        for unit in services():
            for key, value in HARDENING.items():
                self.assertEqual(
                    unit.values("Service", key), [value] if value else [], f"{unit.rel} {key}"
                )
            self.assertTrue(unit.assigned("Service", "CapabilityBoundingSet"), unit.rel)
            self.assertEqual(unit.values("Service", "AmbientCapabilities"), [], unit.rel)


if __name__ == "__main__":
    unittest.main()
