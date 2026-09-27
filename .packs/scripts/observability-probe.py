#!/usr/bin/env python3
"""observability-probe.py -- judge a service's logs, traces, metrics, SLOs, alerts and memory watch.

SPEC-V2-2220 / ADR-V2-2220. The pack is `skills/packs/observability`: tracing spans and fields,
structured logs into journald, metrics, SLOs with error budgets and burn-rate alerts delivered on
the Telegram alert path, and a memory watch (MemoryHigh and MemoryMax, OOM signals) for services on
a small VM. Every class is a STATIC read of `--root`: the Rust sources, the `deploy/` systemd units,
and one checkable declaration, `deploy/slo.json`. Nothing is run, queried or fetched.

It composes, and copies nothing. The Rust workspace (crates, binaries, the sources a binary is
built from, comment- and string-aware scanning) is `rust-service-probe.py`'s model, and the units
are `durable-unit-lint.py`'s parser; both are imported from beside this file. Vendor the three
together.

usage:
  observability-probe.py [--root DIR] [--slo FILE] check CLASS
  observability-probe.py classes

CLASS is one of the row ids of `skills/packs/observability/checks.json`:
  obs.subscriber-installed   every service binary installs a tracing subscriber
  obs.structured-logs        every service binary logs JSON or journald-native fields
  obs.journal-priority       a level reaches journald's PRIORITY, not one flat 6 (advisory)
  obs.log-level              the level comes from the environment, and no unit runs debug (advisory)
  obs.no-print-logging       no service binary logs with println!, eprintln! or dbg! (advisory)
  obs.no-secret-fields       no span or event records a token, password, key or init data
  obs.sensitive-headers      logged headers are marked sensitive first
  obs.http-trace-layer       every HTTP service traces its requests (TraceLayer)
  obs.request-events-visible request spans and response events are at INFO or above (advisory)
  obs.span-route             the request span records the matched route (advisory)
  obs.request-id             requests carry a set and propagated x-request-id (advisory)
  obs.metric-names           metric names follow Prometheus naming (advisory)
  obs.metric-labels          no metric label is a high-cardinality id
  obs.slo-declared           deploy/slo.json declares well-formed SLOs over the services
  obs.error-budget-policy    the error budget has a policy: actions on exhaustion, an escalation
  obs.burn-rate-alerts       every SLO pages on a coherent multiwindow burn rate that can fire
  obs.slo-measurable         an evaluator runs, and each SLI's source produces its events
  obs.slo-window-weeks       an SLO window is a whole number of weeks (advisory)
  obs.alert-short-window     a short window is a twelfth of its long window (advisory)
  obs.low-traffic            a single failed event cannot page a low-traffic SLO (advisory)
  obs.alert-route            alerts go to one template unit on the Telegram path
  obs.failure-alerts         every service and scheduled job pages that path on failure
  obs.alert-names-result     the alert names the failed unit's result, oom-kill included (advisory)
  obs.memory-watch           a scheduled watch reads each service's memory accounting
  obs.oom-policy             no service keeps running after an OOM kill (advisory)

Output is one line per finding, `<class>: <finding>`, then `examined N`, the population the class
read. Exit 0 is green, 1 a finding, 2 a usage error, and 3 VOID: nothing to judge (no Rust
workspace, or no deploy/ unit for a class that judges units), an unreadable input, or a missing
sibling script, which is never a pass.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
import types
from collections.abc import Callable
from pathlib import Path

HERE = Path(__file__).resolve().parent
EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3
SLO_FILE = Path("deploy") / "slo.json"
SLO_SCHEMA = "phx.slo.v1"


def load_rust_service() -> types.ModuleType:
    """rust-service-probe.py, imported from beside this file (it imports durable-unit-lint.py)."""
    name = "rust_service_probe"
    if name in sys.modules:
        return sys.modules[name]
    path = HERE / "rust-service-probe.py"
    if not path.is_file():
        print(
            "observability-probe: VOID rust-service-probe.py is not beside this script"
        )
        print("examined 0")
        raise SystemExit(EXIT_VOID)
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


rs = load_rust_service()
Void = rs.Void
Outcome = rs.Outcome
Tree = rs.Tree
any_names = rs.any_names
any_says = rs.any_says
matching = rs.matching


# --------------------------------------------------------------------------- populations


def service_binaries(tree) -> list[tuple[object, str, list, list]]:
    """Every workspace binary a service unit runs, long-running or scheduled:
    (crate, binary, sources, units). With no unit naming a workspace binary, every HTTP service
    (a binary that calls axum::serve) stands in: it is certainly a service, where a CLI that
    prints to its terminal by design is not."""
    by_binary: dict[str, list] = {}
    targets: dict[str, tuple[object, str]] = {}
    for unit in tree.service_units():
        target = tree.unit_binary(unit)
        if target is None:
            continue
        crate, name = target
        targets[name] = target
        by_binary.setdefault(name, []).append(unit)
    if not targets:
        for crate, name, _ in tree.http_services():
            targets[name] = (crate, name)
            by_binary[name] = []
    return [
        (crate, name, tree.binary_sources(crate, name), by_binary[name])
        for name, (crate, name) in sorted(targets.items())
    ]


def population(count: int, tree) -> int:
    """A class's examined count: its population, else the manifests it read to find none."""
    return count or tree.workspace.manifests_read


def compact(sources: list) -> list[str]:
    return [re.sub(r"\s+", "", source.body) for source in sources]


def has_any(sources: list, *tokens: str) -> bool:
    texts = compact(sources)
    wanted = [re.sub(r"\s+", "", token) for token in tokens]
    return any(token in text for text in texts for token in wanted)


# --------------------------------------------------------------------------- stage `logs`


SUBSCRIBER_NAME = re.compile(r"\btracing_subscriber\b|\btracing_journald\b")
STATEMENT_START = re.compile(r"[;{}]")


def subscriber_statements(sources: list) -> list[str]:
    """Every statement (whitespace removed) that names tracing-subscriber or tracing-journald:
    where a subscriber is built, configured and installed."""
    statements: list[str] = []
    for source in sources:
        body = source.body
        for mention in SUBSCRIBER_NAME.finditer(body):
            starts = [
                m.end() for m in STATEMENT_START.finditer(body, 0, mention.start())
            ]
            start = starts[-1] if starts else 0
            end = rs.statement_end(body, mention.start())
            statements.append(re.sub(r"\s+", "", body[start:end]))
    return statements


def installs_subscriber(sources: list) -> bool:
    """A subscriber is installed: `set_global_default(..)`, or a tracing-subscriber statement
    that ends in `.init()` or `.try_init()`. An `.init()` on anything else installs nothing."""
    if has_any(sources, "set_global_default("):
        return True
    return any(
        ".init()" in statement or ".try_init()" in statement
        for statement in subscriber_statements(sources)
    )


def structured(sources: list) -> bool:
    """JSON from a tracing-subscriber statement, or a tracing-journald layer. A `.json()` on an
    HTTP response is not a log format."""
    return has_any(sources, "tracing_journald") or any(
        ".json()" in statement for statement in subscriber_statements(sources)
    )


def check_subscriber_installed(tree) -> Outcome:
    outcome = Outcome()
    binaries = service_binaries(tree)
    outcome.examined = population(len(binaries), tree)
    for crate, name, sources, units in binaries:
        if not installs_subscriber(sources):
            ran_by = ", ".join(unit.rel for unit in units) or crate.rel
            outcome.findings.append(
                f"binary {name} ({ran_by}) installs no tracing subscriber: every event it emits "
                "is discarded"
            )
    return outcome


def check_structured_logs(tree) -> Outcome:
    outcome = Outcome()
    binaries = service_binaries(tree)
    outcome.examined = population(len(binaries), tree)
    for crate, name, sources, _ in binaries:
        if not installs_subscriber(sources):
            continue
        if not structured(sources):
            outcome.findings.append(
                f"binary {name} ({crate.rel}) logs plain text: use tracing-subscriber's .json() or "
                "tracing-journald, so each field is a field and a CR/LF in input cannot forge a line"
            )
    return outcome


SD_PREFIX = re.compile(r"^<[0-7]>")


def check_journal_priority(tree) -> Outcome:
    outcome = Outcome()
    binaries = service_binaries(tree)
    outcome.examined = population(len(binaries), tree)
    false_words = rs.durable().FALSE
    for crate, name, sources, units in binaries:
        if not installs_subscriber(sources) or has_any(sources, "tracing_journald"):
            continue
        prefixed = any(
            SD_PREFIX.match(literal.value) for s in sources for literal in s.literals
        )
        if not prefixed:
            outcome.findings.append(
                f"binary {name} ({crate.rel}) writes its events to stdout with no <N> priority "
                "prefix and no tracing-journald layer: journald files every line at the unit's "
                "SyslogLevel (info), so `journalctl -p err` sees no error (i343)"
            )
            continue
        for unit in units:
            setting = (unit.last("Service", "SyslogLevelPrefix") or "").strip().lower()
            if setting in false_words:
                outcome.findings.append(
                    f"{unit.rel} sets SyslogLevelPrefix={setting}, so journald keeps binary "
                    f"{name}'s <N> prefixes as text and files every line at SyslogLevel"
                )
    return outcome


VERBOSE_FILTER = re.compile(
    r"(?:LevelFilter\s*::\s*(?:TRACE|DEBUG)|with_max_level\s*\(\s*(?:tracing\s*::\s*)?"
    r"(?:Level|LevelFilter)\s*::\s*(?:TRACE|DEBUG)\s*\))"
)
ENV_FILTER = ("from_default_env", "try_from_default_env", ".from_env", "from_env_lossy")
VERBOSE_RUST_LOG = re.compile(r"\bRUST_LOG=[^\s\"']*\b(?:trace|debug)\b", re.IGNORECASE)


def check_log_level(tree) -> Outcome:
    outcome = Outcome()
    binaries = service_binaries(tree)
    outcome.examined = population(len(binaries), tree)
    lint = rs.durable()
    for crate, name, sources, units in binaries:
        if installs_subscriber(sources) and not has_any(sources, *ENV_FILTER):
            outcome.findings.append(
                f"binary {name} ({crate.rel}) takes its log level from no environment filter "
                "(EnvFilter::try_from_default_env): an incident cannot raise it without a build"
            )
        for source in sources:
            for match in VERBOSE_FILTER.finditer(source.body):
                outcome.findings.append(
                    f"{source.place(match.start())}: a hard-coded {match.group(0).split('::')[-1].strip(' )')} "
                    "level in a service floods the journal"
                )
        for unit in units:
            if not lint.long_running(unit):
                continue
            for value in unit.values("Service", "Environment"):
                if VERBOSE_RUST_LOG.search(value):
                    outcome.findings.append(
                        f"{unit.rel}: Environment={value} runs a long-running service at debug or "
                        "trace; journald drops what passes its RateLimitBurst (10000 per 30s)"
                    )
    return outcome


PRINT_MACRO = re.compile(
    r"(?<![\w:])(?:std\s*::\s*)?(?:e?println|e?print|dbg)\s*!\s*\("
)


def check_no_print_logging(tree) -> Outcome:
    outcome = Outcome()
    binaries = service_binaries(tree)
    outcome.examined = population(len(binaries), tree)
    reported: set[tuple[str, int]] = set()
    for crate, name, sources, _ in binaries:
        for source in sources:
            for match in PRINT_MACRO.finditer(source.bare):
                key = (source.rel, match.start())
                if key in reported:
                    continue
                reported.add(key)
                macro = match.group(0).split("!")[0].strip()
                outcome.findings.append(
                    f"{source.place(match.start())}: {macro}! in service {name} bypasses the "
                    "subscriber: no level, no fields, no span"
                )
    return outcome


SECRET_PARTS = frozenset(
    {
        "token",
        "secret",
        "password",
        "passwd",
        "passphrase",
        "credential",
        "credentials",
        "apikey",
        "authorization",
        "cookie",
        "dsn",
        "privkey",
    }
)
SECRET_PAIRS = frozenset(
    {
        ("api", "key"),
        ("private", "key"),
        ("secret", "key"),
        ("access", "key"),
        ("signing", "key"),
        ("init", "data"),
        ("initdata", ""),
        ("session", "id"),
        ("session", "token"),
        ("database", "url"),
        ("db", "url"),
        ("connection", "string"),
        ("auth", "header"),
    }
)
NEUTRAL_LAST = frozenset(
    {
        "count",
        "counts",
        "len",
        "length",
        "kind",
        "type",
        "name",
        "names",
        "ref",
        "path",
        "file",
        "dir",
        "directory",
        "expiry",
        "expires",
        "ttl",
        "hash",
        "hashed",
        "digest",
        "fingerprint",
        "present",
        "set",
        "used",
        "limit",
        "redacted",
        "masked",
        "valid",
        "ok",
    }
)


def secret_like(name: str) -> bool:
    """Whether a field or parameter name holds what OWASP says never to log: a session id, an
    access token, a password, a key, a connection string, or Telegram's init data."""
    parts = [part for part in re.split(r"[_.]+", name.strip().lower()) if part]
    if not parts:
        return False
    pairs = set(zip(parts, parts[1:]))
    if parts[-1] in NEUTRAL_LAST and not (pairs & {("session", "id")}):
        return False
    if "initdata" in parts:
        return True
    return bool(set(parts) & SECRET_PARTS) or bool(pairs & SECRET_PAIRS)


INSTRUMENT = re.compile(r"#\s*\[\s*(?:tracing\s*::\s*)?instrument\b")
FN_AFTER = re.compile(r"\bfn\s+(\w+)\s*(?:<)?")
EVENT_MACRO = re.compile(
    r"(?<![\w])(?:tracing\s*::\s*)?(?:trace|debug|info|warn|error|event|span|trace_span|"
    r"debug_span|info_span|warn_span|error_span)\s*!\s*\("
)
FIELD = re.compile(r"^\s*[?%]?\s*([A-Za-z_][\w.]*)\s*(?:=|$)")
INLINE_ARG = re.compile(r"\{([A-Za-z_]\w*)(?::[^}]*)?\}")


def split_arguments(text: str) -> list[str]:
    """Top-level comma-separated arguments of a call's parenthesised text (brackets respected)."""
    arguments: list[str] = []
    depth = 0
    current: list[str] = []
    for char in text:
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        if char == "," and depth == 0:
            arguments.append("".join(current))
            current = []
            continue
        current.append(char)
    if "".join(current).strip():
        arguments.append("".join(current))
    return arguments


def argument_spans(bare: str, start: int, end: int) -> list[tuple[int, int]]:
    """Offsets of the top-level arguments between `start` and `end`, split on `bare`, where a
    comma inside a string or a comment is already blank. Slice `code` and `bare` with the same
    spans and the two views of one argument cannot drift apart."""
    spans: list[tuple[int, int]] = []
    depth = 0
    first = start
    for index in range(start, end):
        char = bare[index]
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        elif char == "," and depth == 0:
            spans.append((first, index))
            first = index + 1
    if end > first:
        spans.append((first, end))
    return spans


def parameter_names(bare: str, open_paren: int) -> list[str]:
    inner = bare[open_paren + 1 : matching(bare, open_paren) - 1]
    names = []
    for argument in split_arguments(inner):
        head = argument.split(":")[0].strip()
        head = re.sub(r"^(?:mut\s+|&\s*(?:mut\s+)?)", "", head)
        if re.fullmatch(r"[A-Za-z_]\w*", head) and head != "self":
            names.append(head)
    return names


def instrument_findings(source) -> list[str]:
    findings: list[str] = []
    for attribute in INSTRUMENT.finditer(source.bare):
        bracket = source.bare.find("[", attribute.start())
        close = matching(source.bare, bracket)
        text = source.bare[bracket:close]
        function = FN_AFTER.search(source.bare, close)
        if function is None:
            continue
        paren = source.bare.find("(", function.end() - 1)
        if source.bare[function.end() - 1] == "<":
            paren = source.bare.find("(", matching(source.bare, function.end() - 1))
        params = parameter_names(source.bare, paren)
        if "skip_all" in text:
            recorded: list[str] = []
        else:
            skipped_match = re.search(r"\bskip\s*\(([^)]*)\)", text)
            skipped = {
                part.strip()
                for part in (skipped_match.group(1) if skipped_match else "").split(",")
            }
            recorded = [param for param in params if param not in skipped]
        for param in recorded:
            if secret_like(param):
                findings.append(
                    f"{source.place(attribute.start())}: #[instrument] on {function.group(1)} "
                    f"records the argument {param}; add it to skip(..)"
                )
        fields = re.search(r"\bfields\s*\(", text)
        if fields:
            start = text.find("(", fields.start())
            inner = text[start + 1 : matching(text, start) - 1]
            for argument in split_arguments(inner):
                field_match = FIELD.match(argument)
                if field_match and secret_like(field_match.group(1).split(".")[-1]):
                    findings.append(
                        f"{source.place(attribute.start())}: #[instrument] on {function.group(1)} "
                        f"records the field {field_match.group(1)}"
                    )
    return findings


def event_findings(source) -> list[str]:
    findings: list[str] = []
    for macro in EVENT_MACRO.finditer(source.bare):
        paren = macro.end() - 1
        close = matching(source.bare, paren)
        spans = argument_spans(source.bare, paren + 1, close - 1)
        seen_message = False
        for low, high in spans:
            text, bare = source.code[low:high], source.bare[low:high]
            if not text.strip():
                continue
            stripped = text.strip()
            if not seen_message and stripped.startswith(('"', 'r"', 'r#"')):
                seen_message = True
                for name in INLINE_ARG.findall(stripped):
                    if secret_like(name):
                        findings.append(
                            f"{source.place(macro.start())}: the message interpolates {name}"
                        )
                continue
            if seen_message:
                ident = re.fullmatch(r"\s*&?\s*([A-Za-z_][\w.]*)\s*", bare)
                if ident and secret_like(ident.group(1).split(".")[-1]):
                    findings.append(
                        f"{source.place(macro.start())}: the message formats {ident.group(1)}"
                    )
                continue
            if re.match(r"\s*(?:target|parent|name)\s*:", bare) or re.match(
                r"\s*(?:tracing\s*::\s*)?Level\s*::", bare
            ):
                continue
            field_match = FIELD.match(bare)
            if field_match and secret_like(field_match.group(1).split(".")[-1]):
                findings.append(
                    f"{source.place(macro.start())}: the event records the field {field_match.group(1)}"
                )
    return findings


def check_no_secret_fields(tree) -> Outcome:
    outcome = Outcome()
    for source in tree.all_sources():
        outcome.examined += 1
        for finding in instrument_findings(source) + event_findings(source):
            outcome.findings.append(
                finding
                + ": OWASP's Logging Cheat Sheet never logs tokens, passwords or keys"
            )
    outcome.examined = population(outcome.examined, tree)
    return outcome


INCLUDE_HEADERS = re.compile(r"\.include_headers\s*\(\s*true\s*\)")
SENSITIVE_LAYERS = (
    "SetSensitiveRequestHeadersLayer",
    "SetSensitiveHeadersLayer",
    ".sensitive_headers(",
    ".sensitive_request_headers(",
)


def check_sensitive_headers(tree) -> Outcome:
    outcome = Outcome()
    services = tree.http_services()
    outcome.examined = population(len(services), tree)
    for crate, name, sources in services:
        logged = [
            (source, match.start())
            for source in sources
            for match in INCLUDE_HEADERS.finditer(source.bare)
        ]
        if logged and not has_any(sources, *SENSITIVE_LAYERS):
            source, offset = logged[0]
            outcome.findings.append(
                f"{source.place(offset)}: {name} logs request headers with none marked sensitive: "
                "Authorization, cookies and the webhook secret token reach the journal "
                "(SetSensitiveRequestHeadersLayer, applied before TraceLayer)"
            )
    return outcome


# --------------------------------------------------------------------------- stage `traces`


TRACE_LAYERS = ("TraceLayer::new_for_http(", "TraceLayer::new(", ".trace_for_http(")


def check_http_trace_layer(tree) -> Outcome:
    outcome = Outcome()
    services = tree.http_services()
    outcome.examined = population(len(services), tree)
    for crate, name, sources in services:
        if not has_any(sources, *TRACE_LAYERS):
            outcome.findings.append(
                f"binary {name} ({crate.rel}) serves HTTP with no TraceLayer: no request span, "
                "latency or status reaches the journal, so no SLI can be computed"
            )
    return outcome


LOUD = re.compile(r"Level\s*::\s*(?:INFO|WARN|ERROR)\b")
QUIET = re.compile(r"Level\s*::\s*(?:DEBUG|TRACE)\b")


def call_arguments(source, name: str) -> list[str]:
    """The bare text inside every `.name(` call in a source."""
    out = []
    for call in re.finditer(r"\.\s*" + re.escape(name) + r"\s*\(", source.bare):
        paren = call.end() - 1
        out.append(source.bare[paren : matching(source.bare, paren)])
    return out


def visible(sources: list, method: str) -> bool:
    """Whether some `.method(..)` configures its level at INFO or above, or is a closure."""
    for source in sources:
        for text in call_arguments(source, method):
            if "|" in text:
                return True
            if LOUD.search(text) and not QUIET.search(text):
                return True
    return False


def traced_services(tree) -> list[tuple[object, str, list]]:
    return [
        (crate, name, sources)
        for crate, name, sources in tree.http_services()
        if has_any(sources, *TRACE_LAYERS)
    ]


def request_events_problems(sources: list) -> list[str]:
    problems = []
    if not visible(sources, "on_response"):
        problems.append(
            "its response events keep tower-http's DefaultOnResponse level, DEBUG, so a service "
            "at `info` logs no request"
        )
    if not visible(sources, "make_span_with"):
        problems.append(
            "its request span keeps DefaultMakeSpan's DEBUG level, so an INFO response event "
            "carries no method or path"
        )
    return problems


def check_request_events_visible(tree) -> Outcome:
    outcome = Outcome()
    services = traced_services(tree)
    outcome.examined = population(len(services), tree)
    for crate, name, sources in services:
        for problem in request_events_problems(sources):
            outcome.findings.append(f"binary {name} ({crate.rel}): {problem}")
    return outcome


def check_span_route(tree) -> Outcome:
    outcome = Outcome()
    services = traced_services(tree)
    outcome.examined = population(len(services), tree)
    for crate, name, sources in services:
        if not has_any(sources, "MatchedPath"):
            outcome.findings.append(
                f"binary {name} ({crate.rel}) spans each request by its raw URI: record axum's "
                "MatchedPath as http.route, which is low-cardinality and carries no query string"
            )
    return outcome


def check_request_id(tree) -> Outcome:
    outcome = Outcome()
    services = tree.http_services()
    outcome.examined = population(len(services), tree)
    for crate, name, sources in services:
        sets = has_any(
            sources, "SetRequestIdLayer", ".set_x_request_id(", ".set_request_id("
        )
        propagates = has_any(
            sources,
            "PropagateRequestIdLayer",
            ".propagate_x_request_id(",
            ".propagate_request_id(",
        )
        if not (sets and propagates):
            missing = "set" if not sets else "propagated"
            outcome.findings.append(
                f"binary {name} ({crate.rel}): no request id is {missing} (SetRequestIdLayer and "
                "PropagateRequestIdLayer), so a log line cannot be tied to its request"
            )
    return outcome


# --------------------------------------------------------------------------- stage `metrics`


METRICS_MACRO = re.compile(
    r"(?<![\w])(?:metrics\s*::\s*)?(counter|gauge|histogram|describe_counter|describe_gauge|"
    r"describe_histogram)\s*!\s*\("
)
PROMETHEUS_MACRO = re.compile(
    r"(?<![\w])(?:prometheus\s*::\s*)?register_(int_counter|counter|int_gauge|gauge|histogram)"
    r"(_vec)?(?:_with_registry)?\s*!\s*\("
)
BAD_UNITS = re.compile(
    r"_(?:ms|millis|milliseconds|us|micros|microseconds|ns|nanos|nanoseconds|minutes|hours|"
    r"kb|mb|gb|kilobytes|megabytes|gigabytes)(?:_total)?$"
)
METRIC_NAME = re.compile(r"^[a-z_:][a-z0-9_:]*$")
HIGH_CARDINALITY = frozenset(
    {
        "email",
        "ip",
        "ip_address",
        "remote_addr",
        "url",
        "uri",
        "path",
        "query",
        "username",
        "user",
        "chat",
        "token",
        "session",
    }
)


def first_literal(source, open_paren: int):
    close = matching(source.bare, open_paren)
    for literal in source.literals:
        if open_paren < literal.start < close:
            return literal
    return None


def metrics(tree) -> list[tuple[object, int, str, str, list[str]]]:
    """(source, offset, kind, name, label keys) for every metric registration in production."""
    found = []
    for source in tree.all_sources():
        for macro in METRICS_MACRO.finditer(source.bare):
            paren = macro.end() - 1
            name = first_literal(source, paren)
            if name is None:
                continue
            close = matching(source.bare, paren)
            keys = [
                literal.value
                for literal in source.literals
                if paren < literal.start < close
                and re.match(r"\s*=>", source.bare[literal.end : literal.end + 4])
            ]
            kind = macro.group(1).replace("describe_", "")
            found.append((source, macro.start(), kind, name.value, keys))
        for macro in PROMETHEUS_MACRO.finditer(source.bare):
            paren = macro.end() - 1
            name = first_literal(source, paren)
            if name is None:
                continue
            close = matching(source.bare, paren)
            keys = []
            if macro.group(2):
                bracket = source.bare.find("[", name.end, close)
                if bracket >= 0:
                    end = matching(source.bare, bracket)
                    keys = [
                        literal.value
                        for literal in source.literals
                        if bracket < literal.start < end
                    ]
            kind = (
                "counter"
                if "counter" in macro.group(1)
                else macro.group(1).replace("int_", "")
            )
            found.append((source, macro.start(), kind, name.value, keys))
    return found


def check_metric_names(tree) -> Outcome:
    outcome = Outcome()
    found = metrics(tree)
    outcome.examined = population(len(found), tree)
    prefixes: dict[str, str] = {}
    for source, offset, kind, name, _ in found:
        where = source.place(offset)
        if not METRIC_NAME.match(name):
            outcome.findings.append(f"{where}: metric {name!r} is not lower snake case")
        if kind == "counter" and not name.endswith("_total"):
            outcome.findings.append(f"{where}: counter {name} does not end in _total")
        if kind != "counter" and name.endswith("_total"):
            outcome.findings.append(
                f"{where}: {kind} {name} ends in _total, which marks a counter"
            )
        if BAD_UNITS.search(name):
            outcome.findings.append(
                f"{where}: metric {name} is not in a base unit (seconds, bytes)"
            )
        prefixes.setdefault(name.split("_")[0], where)
    if len(prefixes) > 1:
        outcome.findings.append(
            "metric names carry more than one application prefix: "
            + ", ".join(sorted(prefixes))
        )
    return outcome


def high_cardinality(key: str) -> bool:
    lowered = key.lower()
    return lowered == "id" or lowered.endswith("_id") or lowered in HIGH_CARDINALITY


def check_metric_labels(tree) -> Outcome:
    outcome = Outcome()
    found = metrics(tree)
    outcome.examined = population(len(found), tree)
    for source, offset, _, name, keys in found:
        for key in keys:
            if high_cardinality(key):
                outcome.findings.append(
                    f"{source.place(offset)}: metric {name} is labelled by {key}, an unbounded set "
                    "of values: every value is a new series held in memory"
                )
    return outcome


# --------------------------------------------------------------------------- deploy/slo.json

SLO_KEYS = frozenset(
    {
        "schema",
        "note",
        "slos",
        "error_budget_policy",
        "alerting",
        "evaluator",
        "memory_watch",
    }
)
OBJECTIVE_KEYS = frozenset(
    {
        "id",
        "unit",
        "sli",
        "objective",
        "window_days",
        "rationale",
        "alerts",
        "expected_events_per_hour",
        "low_traffic",
        "note",
    }
)
SLI_KEYS = frozenset(
    {"kind", "source", "good", "total", "threshold_ms", "probe_unit", "note"}
)
ALERT_KEYS = frozenset(
    {
        "severity",
        "long_window",
        "short_window",
        "burn_rate",
        "budget_consumed",
        "route",
        "note",
    }
)
POLICY_KEYS = frozenset(
    {"on_exhaustion", "escalation", "postmortem_budget_fraction", "note"}
)
ALERTING_KEYS = frozenset({"unit", "channel", "note"})
EVALUATOR_KEYS = frozenset({"unit", "timer", "note"})
WATCH_KEYS = frozenset({"unit", "timer", "units", "note"})
SLI_KINDS = frozenset({"availability", "latency"})
SLI_SOURCES = frozenset({"journal", "metrics", "probe"})
SEVERITIES = frozenset({"page", "ticket"})
MITIGATIONS = frozenset({"synthetic", "grouped", "longer-window", "lower-objective"})
DURATION = re.compile(r"^(\d+(?:\.\d+)?)(m|h|d|w)$")
HOURS = {"m": 1 / 60, "h": 1.0, "d": 24.0, "w": 168.0}
SLUG = re.compile(r"^[a-z0-9][a-z0-9-]*$")
MEMORY_READS = (
    "memory.events",
    "memory.pressure",
    "memory.current",
    "memory.peak",
    "MemoryCurrent",
    "MemoryPeak",
    "oom_kill",
)
TELEGRAM = ("api.telegram.org", "sendMessage")


def hours(text: object) -> float | None:
    if not isinstance(text, str):
        return None
    found = DURATION.match(text.strip())
    if not found:
        return None
    return float(found.group(1)) * HOURS[found.group(2)]


class Deployment:
    """The units under deploy/ and the SLO declaration beside them."""

    def __init__(self, tree, slo_path: Path | None) -> None:
        self.tree = tree
        self.subject = tree.subject
        self.path = slo_path or (tree.root / SLO_FILE)
        self.rel = (
            self.path.relative_to(tree.root).as_posix()
            if tree.root in self.path.parents
            else str(self.path)
        )
        if not self.subject.units:
            raise Void(
                "no deploy/ unit: the SLO, alert and memory rows judge a deployed service"
            )
        self.declaration: dict | None = None
        if self.path.is_file():
            try:
                loaded = json.loads(self.path.read_text(encoding="utf-8"))
            except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
                raise Void(f"cannot read {self.rel}: {error}") from error
            if not isinstance(loaded, dict):
                raise Void(f"{self.rel} is not a JSON object")
            self.declaration = loaded

    @property
    def units(self) -> dict:
        return self.subject.units

    def long_running(self) -> list:
        lint = rs.durable()
        return [unit for unit in self.subject.services if lint.long_running(unit)]

    def scheduled(self) -> list:
        activated = self.subject.timer_activated()
        return [unit for unit in self.subject.services if unit.name in activated]

    def slos(self) -> list[dict]:
        entries = (self.declaration or {}).get("slos")
        return (
            [entry for entry in entries if isinstance(entry, dict)]
            if isinstance(entries, list)
            else []
        )

    def section(self, key: str) -> dict:
        value = (self.declaration or {}).get(key)
        return value if isinstance(value, dict) else {}

    def unit_text(self, name: str) -> str:
        """The unit's commands and the deploy/ scripts they run, with shell comment lines
        dropped: a variable a comment names is not a variable the script reads."""
        unit = self.units.get(name)
        if unit is None:
            return ""
        lines = self.subject.script_text(unit).splitlines()
        return "\n".join(line for line in lines if not line.lstrip().startswith("#"))

    def timer_starts(self, timer: object, unit: str) -> bool:
        """Whether `timer` names a deploy/ timer whose target is `unit`."""
        found = self.units.get(timer) if isinstance(timer, str) else None
        return (
            found is not None
            and found.kind == "timer"
            and self.subject.timer_target(found) == unit
        )

    def missing(self, outcome: Outcome) -> bool:
        if self.declaration is None:
            outcome.findings.append(
                f"no {self.rel}: the services declare no SLO, error budget, alert path or memory "
                "watch"
            )
            return True
        return False


def deployment(tree, context: dict) -> Deployment:
    return Deployment(tree, context.get("slo"))


def unknown_keys(where: str, entry: dict, allowed: frozenset[str]) -> list[str]:
    return [
        f"{where} has an unknown key {key!r}" for key in sorted(set(entry) - allowed)
    ]


def validate_slo(deploy: Deployment) -> list[str]:
    findings: list[str] = []
    body = deploy.declaration or {}
    rel = deploy.rel
    if body.get("schema") != SLO_SCHEMA:
        findings.append(f"{rel} schema is {body.get('schema')!r}, not {SLO_SCHEMA!r}")
    findings += unknown_keys(rel, body, SLO_KEYS)
    entries = body.get("slos")
    if not isinstance(entries, list) or not entries:
        findings.append(f"{rel} declares no SLO in `slos`")
        return findings
    seen: set[str] = set()
    for index, entry in enumerate(entries):
        where = f"{rel} slos[{index}]"
        if not isinstance(entry, dict):
            findings.append(f"{where} is not an object")
            continue
        findings += unknown_keys(where, entry, OBJECTIVE_KEYS)
        ident = entry.get("id")
        if not isinstance(ident, str) or not SLUG.match(ident):
            findings.append(f"{where} id {ident!r} is not a slug")
        elif ident in seen:
            findings.append(f"{where} repeats the id {ident}")
        else:
            seen.add(ident)
            where = f"{rel} slo {ident}"
        unit = entry.get("unit")
        if not isinstance(unit, str) or unit not in deploy.units:
            findings.append(f"{where} names unit {unit!r}, which deploy/ does not ship")
        sli = entry.get("sli")
        if not isinstance(sli, dict):
            findings.append(f"{where} has no sli object")
        else:
            findings += unknown_keys(f"{where} sli", sli, SLI_KEYS)
            if sli.get("kind") not in SLI_KINDS:
                findings.append(
                    f"{where} sli kind {sli.get('kind')!r} is not one of {sorted(SLI_KINDS)}"
                )
            if sli.get("source") not in SLI_SOURCES:
                findings.append(
                    f"{where} sli source {sli.get('source')!r} is not one of {sorted(SLI_SOURCES)}"
                )
            for key in ("good", "total"):
                if not isinstance(sli.get(key), str) or not sli[key].strip():
                    findings.append(f"{where} sli does not say which events are {key}")
            if sli.get("kind") == "latency":
                threshold = sli.get("threshold_ms")
                if (
                    not isinstance(threshold, (int, float))
                    or isinstance(threshold, bool)
                    or threshold <= 0
                ):
                    findings.append(
                        f"{where} is a latency SLI with no threshold_ms: a ratio of requests under "
                        "a threshold, never an average"
                    )
        objective = entry.get("objective")
        if (
            not isinstance(objective, (int, float))
            or isinstance(objective, bool)
            or not 0 < objective < 1
        ):
            findings.append(
                f"{where} objective {objective!r} is not a ratio strictly between 0 and 1"
            )
        window = entry.get("window_days")
        if not isinstance(window, int) or isinstance(window, bool) or window <= 0:
            findings.append(
                f"{where} window_days {window!r} is not a positive whole number of days"
            )
        rationale = entry.get("rationale")
        if not isinstance(rationale, str) or not rationale.strip():
            findings.append(f"{where} records no rationale for its numbers")
        alerts = entry.get("alerts")
        if not isinstance(alerts, list) or not alerts:
            findings.append(f"{where} declares no alert")
            continue
        for number, alert in enumerate(alerts):
            place = f"{where} alerts[{number}]"
            if not isinstance(alert, dict):
                findings.append(f"{place} is not an object")
                continue
            findings += unknown_keys(place, alert, ALERT_KEYS)
            if alert.get("severity") not in SEVERITIES:
                findings.append(
                    f"{place} severity {alert.get('severity')!r} is not page or ticket"
                )
            for key in ("long_window", "short_window"):
                if hours(alert.get(key)) is None:
                    findings.append(
                        f"{place} {key} {alert.get(key)!r} is not a duration like 5m, 1h, 3d"
                    )
            burn = alert.get("burn_rate")
            if (
                not isinstance(burn, (int, float))
                or isinstance(burn, bool)
                or burn <= 0
            ):
                findings.append(f"{place} burn_rate {burn!r} is not a positive number")
            consumed = alert.get("budget_consumed")
            if (
                not isinstance(consumed, (int, float))
                or isinstance(consumed, bool)
                or not 0 < consumed <= 1
            ):
                findings.append(
                    f"{place} budget_consumed {consumed!r} is not a fraction in (0, 1]"
                )
            if not isinstance(alert.get("route"), str) or not alert["route"].strip():
                findings.append(f"{place} names no route")
    return findings


def check_slo_declared(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    outcome.findings += validate_slo(deploy)
    covered = {entry.get("unit") for entry in deploy.slos()}
    serving = {name for _, name, _ in tree.http_services()}
    for unit in deploy.long_running():
        target = tree.unit_binary(unit)
        if target is not None and target[1] in serving and unit.name not in covered:
            outcome.findings.append(
                f"{unit.rel} runs HTTP service {target[1]}, and no SLO in {deploy.rel} covers it"
            )
    return outcome


def check_error_budget_policy(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    policy = (deploy.declaration or {}).get("error_budget_policy")
    if not isinstance(policy, dict):
        outcome.findings.append(
            f"{deploy.rel} has no error_budget_policy: an error budget with no agreed action when "
            "it is spent is only a number"
        )
        return outcome
    outcome.findings += unknown_keys(
        f"{deploy.rel} error_budget_policy", policy, POLICY_KEYS
    )
    actions = policy.get("on_exhaustion")
    if (
        not isinstance(actions, list)
        or not actions
        or not all(isinstance(action, str) and action.strip() for action in actions)
    ):
        outcome.findings.append(
            f"{deploy.rel} error_budget_policy names no action to take when the budget is spent"
        )
    escalation = policy.get("escalation")
    if not isinstance(escalation, str) or not escalation.strip():
        outcome.findings.append(
            f"{deploy.rel} error_budget_policy names no escalation for a disputed calculation"
        )
    fraction = policy.get("postmortem_budget_fraction")
    if fraction is not None and (
        not isinstance(fraction, (int, float))
        or isinstance(fraction, bool)
        or not 0 < fraction <= 1
    ):
        outcome.findings.append(
            f"{deploy.rel} postmortem_budget_fraction {fraction!r} is not a fraction in (0, 1]"
        )
    return outcome


def numeric(value: object) -> float | None:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    return float(value)


def check_burn_rate_alerts(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    for entry in deploy.slos():
        name = entry.get("id", "?")
        objective = numeric(entry.get("objective"))
        window = numeric(entry.get("window_days"))
        alerts = [
            alert for alert in entry.get("alerts") or [] if isinstance(alert, dict)
        ]
        if not any(alert.get("severity") == "page" for alert in alerts):
            outcome.findings.append(
                f"slo {name} has no page alert: its budget can burn out unseen"
            )
        if objective is None or window is None or not 0 < objective < 1 or window <= 0:
            continue
        budget = 1 - objective
        window_hours = window * 24
        for alert in alerts:
            long_h, short_h = (
                hours(alert.get("long_window")),
                hours(alert.get("short_window")),
            )
            burn, consumed = (
                numeric(alert.get("burn_rate")),
                numeric(alert.get("budget_consumed")),
            )
            if None in (long_h, short_h, burn, consumed):
                continue
            label = f"slo {name} {alert.get('severity')} alert {alert.get('long_window')}/{alert.get('short_window')}"
            if short_h >= long_h:
                outcome.findings.append(
                    f"{label}: the short window is not shorter than the long one"
                )
            if long_h > window_hours:
                outcome.findings.append(
                    f"{label}: the long window exceeds the {window:g}-day SLO window"
                )
            expected = consumed * window_hours / long_h
            if abs(burn - expected) > 0.02 * expected:
                outcome.findings.append(
                    f"{label}: burn_rate {burn:g} does not spend {consumed:g} of the budget in "
                    f"{alert.get('long_window')} (that is {expected:.4g})"
                )
            if burn * budget > 1:
                outcome.findings.append(
                    f"{label}: burn rate {burn:g} over a {budget:.4g} budget needs an error ratio of "
                    f"{burn * budget:.3g}, more than every event failing: it can never fire (i397)"
                )
    return outcome


def check_slo_window_weeks(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    for entry in deploy.slos():
        window = entry.get("window_days")
        if isinstance(window, int) and not isinstance(window, bool) and window % 7:
            outcome.findings.append(
                f"slo {entry.get('id', '?')} window_days {window} is not a whole number of weeks: "
                "the window's count of weekends moves as it rolls"
            )
    return outcome


def check_alert_short_window(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    for entry in deploy.slos():
        for alert in entry.get("alerts") or []:
            if not isinstance(alert, dict):
                continue
            long_h, short_h = (
                hours(alert.get("long_window")),
                hours(alert.get("short_window")),
            )
            if long_h is None or short_h is None:
                continue
            ideal = long_h / 12
            if abs(short_h - ideal) > 0.25 * ideal:
                outcome.findings.append(
                    f"slo {entry.get('id', '?')} alert {alert.get('long_window')}/"
                    f"{alert.get('short_window')}: the short window is not about a twelfth of the "
                    "long one, so the alert resets late or fires on a blip"
                )
    return outcome


def check_low_traffic(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    for entry in deploy.slos():
        name = entry.get("id", "?")
        objective = numeric(entry.get("objective"))
        if objective is None or not 0 < objective < 1:
            continue
        mitigation = entry.get("low_traffic")
        if isinstance(mitigation, str) and mitigation in MITIGATIONS:
            continue
        rate = numeric(entry.get("expected_events_per_hour"))
        if rate is None:
            outcome.findings.append(
                f"slo {name} states no expected_events_per_hour, so whether one failure pages it "
                "cannot be judged"
            )
            continue
        for alert in entry.get("alerts") or []:
            if not isinstance(alert, dict) or alert.get("severity") != "page":
                continue
            long_h, burn = (
                hours(alert.get("long_window")),
                numeric(alert.get("burn_rate")),
            )
            if long_h is None or burn is None or burn <= 0:
                continue
            events = rate * long_h
            needed = 1 / (burn * (1 - objective))
            if events < needed:
                outcome.findings.append(
                    f"slo {name}: about {events:.3g} events in {alert.get('long_window')}, and one "
                    f"failure among fewer than {needed:.3g} pages it; declare low_traffic "
                    f"({', '.join(sorted(MITIGATIONS))})"
                )
                break
    return outcome


def check_slo_measurable(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    evaluator = deploy.section("evaluator")
    unit, timer = evaluator.get("unit"), evaluator.get("timer")
    if not isinstance(unit, str) or unit not in deploy.units:
        outcome.findings.append(
            f"{deploy.rel} evaluator unit {unit!r} is not shipped in deploy/: nothing computes the "
            "burn rates, so no SLO alert can fire (i397)"
        )
    elif not deploy.timer_starts(timer, unit):
        outcome.findings.append(
            f"{deploy.rel} evaluator timer {timer!r} is not a deploy/ timer that starts {unit}"
        )
    for entry in deploy.slos():
        sli = entry.get("sli") if isinstance(entry.get("sli"), dict) else {}
        name = entry.get("id", "?")
        target_unit = deploy.units.get(entry.get("unit"))
        target = tree.unit_binary(target_unit) if target_unit is not None else None
        source = sli.get("source")
        if source == "journal":
            if target is None:
                outcome.findings.append(
                    f"slo {name} reads the journal of {entry.get('unit')!r}, which runs no workspace "
                    "binary this probe can read"
                )
                continue
            crate, binary = target
            sources = tree.binary_sources(crate, binary)
            if not has_any(sources, *TRACE_LAYERS):
                outcome.findings.append(
                    f"slo {name} reads request events from the journal, and {binary} has no TraceLayer"
                )
                continue
            for problem in request_events_problems(sources):
                outcome.findings.append(
                    f"slo {name}: {binary} {problem}; the SLI counts no traffic"
                )
        elif source == "metrics":
            if target is None or not any(
                entry_source in tree.binary_sources(*target)
                for entry_source, *_ in metrics(tree)
            ):
                outcome.findings.append(
                    f"slo {name} reads metrics, and {entry.get('unit')!r} registers none"
                )
        elif source == "probe":
            probe = sli.get("probe_unit")
            if not isinstance(probe, str) or probe not in deploy.units:
                outcome.findings.append(
                    f"slo {name} reads a synthetic probe, and its probe_unit {probe!r} is not shipped"
                )
    return outcome


# --------------------------------------------------------------------------- stage `alerts`


INSTANCE = re.compile(r"^([\w.:-]+)@%[nNpPiIjJ]\.service$")


def alert_template(deploy: Deployment) -> str | None:
    unit = deploy.section("alerting").get("unit")
    return unit if isinstance(unit, str) else None


def check_alert_route(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    alerting = deploy.section("alerting")
    outcome.findings += unknown_keys(f"{deploy.rel} alerting", alerting, ALERTING_KEYS)
    template = alert_template(deploy)
    channel = alerting.get("channel")
    if template is None or not template.endswith("@.service"):
        outcome.findings.append(
            f"{deploy.rel} alerting.unit {template!r} is not a template unit (name@.service): "
            "OnFailure= needs an instance per failed unit"
        )
    elif template not in deploy.units:
        outcome.findings.append(
            f"{deploy.rel} alerting.unit {template} is not shipped in deploy/"
        )
    elif not any(token in deploy.unit_text(template) for token in TELEGRAM):
        outcome.findings.append(
            f"{template}'s command never reaches the Telegram Bot API (api.telegram.org sendMessage)"
        )
    if channel != "telegram":
        outcome.findings.append(
            f"{deploy.rel} alerting.channel is {channel!r}, not the telegram path"
        )
    for entry in deploy.slos():
        for alert in entry.get("alerts") or []:
            if isinstance(alert, dict) and alert.get("route") != channel:
                outcome.findings.append(
                    f"slo {entry.get('id', '?')} routes an alert to {alert.get('route')!r}, not "
                    f"the one alert path {channel!r}"
                )
    return outcome


def check_failure_alerts(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    template = alert_template(deploy) if deploy.declaration is not None else None
    watched = {unit.name: unit for unit in deploy.long_running() + deploy.scheduled()}
    if template is not None:
        watched.pop(template, None)
    outcome.examined = len(deploy.units)
    for name, unit in sorted(watched.items()):
        if name.split("@")[0] + "@.service" == template:
            continue
        targets = unit.words("Unit", "OnFailure")
        routed = False
        for target in targets:
            instance = INSTANCE.match(target)
            if instance is None:
                continue
            base = instance.group(1) + "@.service"
            if (template is None and base in deploy.units) or base == template:
                routed = True
        if not routed:
            wanted = (
                template.replace("@.service", "@%n.service")
                if template
                else "alert@%n.service"
            )
            outcome.findings.append(
                f"{unit.rel} has no OnFailure={wanted}: it can reach failed, oom-kill included, "
                "and nobody is paged"
            )
    return outcome


def check_alert_names_result(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    template = alert_template(deploy)
    if template is None or template not in deploy.units:
        return outcome
    text = deploy.unit_text(template)
    if "MONITOR_SERVICE_RESULT" not in text:
        outcome.findings.append(
            f"{template} never reads $MONITOR_SERVICE_RESULT, so an alert cannot say oom-kill, "
            "watchdog or timeout (systemd 251 and later pass it to OnFailure= units)"
        )
    if "MONITOR_UNIT" not in text and "%i" not in text and "%I" not in text:
        outcome.findings.append(
            f"{template} never names the failed unit ($MONITOR_UNIT or %i)"
        )
    return outcome


# --------------------------------------------------------------------------- stage `memory`


def check_memory_watch(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    outcome.examined = len(deploy.units)
    if deploy.missing(outcome):
        return outcome
    watch = deploy.section("memory_watch")
    if not watch:
        outcome.findings.append(
            f"{deploy.rel} declares no memory_watch: MemoryHigh throttling and OOM kills go unseen "
            "until a service dies (i345)"
        )
        return outcome
    outcome.findings += unknown_keys(f"{deploy.rel} memory_watch", watch, WATCH_KEYS)
    unit, timer = watch.get("unit"), watch.get("timer")
    if not isinstance(unit, str) or unit not in deploy.units:
        outcome.findings.append(f"memory_watch unit {unit!r} is not shipped in deploy/")
        return outcome
    if not deploy.timer_starts(timer, unit):
        outcome.findings.append(
            f"memory_watch timer {timer!r} is not a deploy/ timer that starts {unit}"
        )
    if not any(token in deploy.unit_text(unit) for token in MEMORY_READS):
        outcome.findings.append(
            f"{unit}'s command reads no memory accounting (memory.events, memory.pressure, "
            "memory.current or MemoryCurrent)"
        )
    scope = watch.get("units")
    for service in deploy.long_running():
        if scope == "all" or (isinstance(scope, list) and service.name in scope):
            continue
        outcome.findings.append(
            f"{service.rel} is long-running, and the memory watch does not cover it"
        )
    return outcome


def check_oom_policy(tree, context: dict) -> Outcome:
    outcome = Outcome()
    deploy = deployment(tree, context)
    services = deploy.long_running()
    outcome.examined = len(deploy.units)
    for service in services:
        if (service.last("Service", "OOMPolicy") or "").strip() == "continue":
            outcome.findings.append(
                f"{service.rel} sets OOMPolicy=continue: an OOM kill leaves it running, it never "
                "reaches failed, and OnFailure= never pages (i345)"
            )
    return outcome


# --------------------------------------------------------------------------- the command line


def tree_only(function: Callable) -> Callable:
    return lambda tree, context: function(tree)


CHECKS: dict[str, Callable] = {
    "obs.subscriber-installed": tree_only(check_subscriber_installed),
    "obs.structured-logs": tree_only(check_structured_logs),
    "obs.journal-priority": tree_only(check_journal_priority),
    "obs.log-level": tree_only(check_log_level),
    "obs.no-print-logging": tree_only(check_no_print_logging),
    "obs.no-secret-fields": tree_only(check_no_secret_fields),
    "obs.sensitive-headers": tree_only(check_sensitive_headers),
    "obs.http-trace-layer": tree_only(check_http_trace_layer),
    "obs.request-events-visible": tree_only(check_request_events_visible),
    "obs.span-route": tree_only(check_span_route),
    "obs.request-id": tree_only(check_request_id),
    "obs.metric-names": tree_only(check_metric_names),
    "obs.metric-labels": tree_only(check_metric_labels),
    "obs.slo-declared": check_slo_declared,
    "obs.error-budget-policy": check_error_budget_policy,
    "obs.burn-rate-alerts": check_burn_rate_alerts,
    "obs.slo-measurable": check_slo_measurable,
    "obs.slo-window-weeks": check_slo_window_weeks,
    "obs.alert-short-window": check_alert_short_window,
    "obs.low-traffic": check_low_traffic,
    "obs.alert-route": check_alert_route,
    "obs.failure-alerts": check_failure_alerts,
    "obs.alert-names-result": check_alert_names_result,
    "obs.memory-watch": check_memory_watch,
    "obs.oom-policy": check_oom_policy,
}
CLASSES = tuple(CHECKS)


def run_check(root: Path, name: str, context: dict) -> int:
    try:
        outcome = CHECKS[name](Tree(root), context)
    except Void as void:
        print(f"{name}: VOID {void}")
        print("examined 0")
        return EXIT_VOID
    for finding in outcome.findings:
        print(f"{name}: {finding}")
    print(f"examined {outcome.examined}")
    if outcome.examined == 0:
        return EXIT_VOID
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


def parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--root", type=Path, default=argparse.SUPPRESS)
    common.add_argument("--slo", type=Path, default=argparse.SUPPRESS)
    top = argparse.ArgumentParser(
        description="Judge a service's logs, traces, metrics, SLOs, alerts and memory watch.",
        parents=[common],
    )
    verbs = top.add_subparsers(dest="verb", required=True)
    check = verbs.add_parser("check", parents=[common], help="run one class")
    check.add_argument("klass", metavar="CLASS", choices=CLASSES)
    verbs.add_parser("classes", help="print the classes, one per line")
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.verb == "classes":
        for name in CLASSES:
            print(name)
        return EXIT_GREEN
    root = Path(getattr(args, "root", Path("."))).resolve()
    if not root.is_dir():
        print(f"observability-probe: --root {root} is not a directory", file=sys.stderr)
        return EXIT_USAGE
    if rs.tomllib is None:
        print(f"{args.klass}: VOID Python 3.11 or newer is required (tomllib)")
        print("examined 0")
        return EXIT_VOID
    slo = getattr(args, "slo", None)
    context = {
        "slo": (root / slo).resolve()
        if slo is not None and not slo.is_absolute()
        else slo
    }
    return run_check(root, args.klass, context)


if __name__ == "__main__":
    sys.exit(main())
