"""The one alert path: the alert template unit and its script (SPEC-031 A3, A4, A7; R3, R6;
ADR-031, ADR-038), and the alert unit's own refusal of an empty credential (SPEC-066 A5,
R3; ADR-067).

The script runs as its unit runs it, with stubs first on its PATH. `curl` and `journalctl` record
their argument vector, standard input and environment, and answer as the real ones would; every other
command the script may call is wrapped to record the same before it runs the real one, so no command
line the script makes escapes the record. The credentials directory holds a synthetic value for each
credential the unit loads, and MONITOR_UNIT and MONITOR_SERVICE_RESULT are set on the child as
systemd sets them for an OnFailure= unit. Nothing reads the host's journal or reaches Telegram.
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import _units
from _support import REPO, examined

SYSTEMD = REPO / "deploy" / "systemd"
SCRIPT = REPO / "deploy" / "scripts" / "alert-telegram.sh"
SLO = REPO / "deploy" / "slo.json"
OWNER_SOURCE = REPO / "crates" / "identity" / "src" / "owner.rs"
# The alert template every unit pages through (SPEC-032 R2), and its one instance form.
ALERT_TEMPLATE = "deck-streak-alert@.service"
ON_FAILURE = "deck-streak-alert@%n.service"
SENDMESSAGE = "https://api.telegram.org/bot{token}/sendMessage"
# The prefixes systemd reads before an ExecStart= path; `-` counts a failure as a success
# (systemd.service(5)).
EXEC_PREFIX = re.compile(r"[-@:+!|]*")
# What the alert template's refusals say each directive breaks (SPEC-066 R3).
PAGES = "is named, and a page that fails must not start a page about the page"
ONE_START = "where the template runs its script once"
COUNTS_A_FAILURE = "counts a failure as a success"
SKIPS = "can skip the start, which leaves the instance inactive, not failed"
NAMED = "is named, and the alert template names none"
DIRECT = "skips the failed state on a restart"
RESTARTS = "restarts the refusal"
UNLOADS = "can unload the failed instance, which systemctl --failed then no longer lists"
UNREAD = "is empty or not a known value, which the check refuses"
OFF_LIST = "is not on this unit's list of keys, and is refused"
STOPS = (
    "is refused, as every condition and assertion is, since one can stop the start and leave the "
    "instance inactive, not failed"
)

# Synthetic values: a token of the Bot API's shape whose id has seven digits, never the public
# scrub's shape; the scrub's own placeholder id for the owner; and another for any credential the
# unit loads beyond the two R3 names.
TOKEN = "7654321:" + "Synthetic0" * 4
OWNER = "123456789"
DECOY = "987654321"
FAILED_UNIT = "deck-streak-api.service"
RESULT = "oom-kill"
INVOCATION = "a1b2c3d4e5f60718293a4b5c6d7e8f90"
# Seven error lines the stub journal answers with; the page quotes the last five.
JOURNAL_LINES = [f"error line {number} of the failed run" for number in range(1, 8)]
# The commands the script may run besides the two stubs, each wrapped to record its command line.
WRAPPED = ("cat", "head", "tail", "sed", "awk", "tr", "cut", "env", "iconv", "grep", "wc", "od")
# curl's options that take a parameter, so a parameter is never read as the URL.
CURL_PARAMETERS = {
    "--config",
    "-K",
    "--data-urlencode",
    "--data",
    "-d",
    "--url",
    "--max-time",
    "-m",
    "--retry",
    "--retry-delay",
    "--retry-max-time",
    "--connect-timeout",
    "--output",
    "-o",
    "--header",
    "-H",
}
# The escapes curl's configuration reads inside double quotes (curl's cmdline-opts/config.md).
CURL_ESCAPES = {"\\": "\\", '"': '"', "t": "\t", "n": "\n", "r": "\r", "v": "\v"}

STUB = """#!{python}
import json, os, sys
name = os.path.basename(sys.argv[0])
stdin = sys.stdin.buffer.read().decode("utf-8", "surrogateescape")
record = {{"command": name, "argv": sys.argv[1:], "stdin": stdin, "env": dict(os.environ)}}
with open(os.path.join(os.environ["STUB_LOG"], "calls.jsonl"), "a", encoding="utf-8") as out:
    out.write(json.dumps(record) + "\\n")
if name == "journalctl" and os.environ.get("STUB_JOURNAL"):
    with open(os.environ["STUB_JOURNAL"], encoding="utf-8") as journal:
        sys.stdout.write(journal.read())
elif name == "curl":
    sys.stdout.write('{{"ok":true,"result":{{"message_id":1}}}}')
"""

WRAPPER = """#!{python}
import json, os, sys
record = {{"command": {name!r}, "argv": sys.argv[1:], "stdin": None, "env": dict(os.environ)}}
with open(os.path.join(os.environ["STUB_LOG"], "calls.jsonl"), "a", encoding="utf-8") as out:
    out.write(json.dumps(record) + "\\n")
os.execv({real!r}, [{real!r}] + sys.argv[1:])
"""


def unit_file(path):
    """A unit file through the census's reader, `_units.assignments`, which refuses a line it
    cannot read: each section's assignments in order, where an empty assignment clears the list
    (systemd.syntax(7))."""
    sections = {}
    for section, key, value, _ in _units.assignments(_units.unit_text(path), Path(path).name):
        values = sections.setdefault(section, {}).setdefault(key, [])
        if value:
            values.append(value)
        else:
            values.clear()
    return sections


def values(unit, section, key):
    return unit.get(section, {}).get(key, [])


def identity_ids():
    """The credential ids the identity context declares, by constant."""
    text = OWNER_SOURCE.read_text(encoding="utf-8")
    found = {}
    for constant in ("OWNER_USER_ID", "TELEGRAM_BOT_TOKEN"):
        match = re.search(rf'pub const {constant}: &str = "([a-z0-9-]+)";', text)
        if match is None:
            raise AssertionError(f"{OWNER_SOURCE.relative_to(REPO)} declares no {constant}")
        found[constant] = match.group(1)
    return found


def loaded_credentials():
    """The credential ids the alert template unit loads, in its order."""
    unit = unit_file(SYSTEMD / ALERT_TEMPLATE)
    return [value.partition(":")[0] for value in values(unit, "Service", "LoadCredential")]


def synthetic(ident):
    """The synthetic value a credential holds in a run: the token, the owner, or a decoy."""
    ids = identity_ids()
    return {ids["TELEGRAM_BOT_TOKEN"]: TOKEN, ids["OWNER_USER_ID"]: OWNER}.get(ident, DECOY)


class Run:
    """One run of an alert script, and every command it ran."""

    def __init__(self, done, log):
        self.returncode = done.returncode
        self.stdout = done.stdout
        self.stderr = done.stderr
        path = Path(log) / "calls.jsonl"
        lines = path.read_text(encoding="utf-8").splitlines() if path.exists() else []
        self.all_calls = [json.loads(line) for line in lines]

    def calls(self, command):
        return [call for call in self.all_calls if call["command"] == command]


def run_alert(instance, environment, journal=JOURNAL_LINES, script=SCRIPT, planted=None):
    """Runs `script` as the alert unit runs it, `alert-telegram.sh %i`, with the stubs first on its
    PATH and a credentials directory holding each credential the unit loads: its synthetic value
    and a newline, or exactly the content `planted` names for its id."""
    original = os.environ.get("PATH", "/usr/bin:/bin")
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        stubs, log, credentials = root / "bin", root / "log", root / "credentials"
        for folder in (stubs, log, credentials):
            folder.mkdir()
        for name in ("curl", "journalctl"):
            stub = stubs / name
            stub.write_text(STUB.format(python=sys.executable), encoding="utf-8")
            stub.chmod(0o755)
        for name in WRAPPED:
            real = shutil.which(name, path=original)
            if real is None:
                continue
            wrapper = stubs / name
            wrapper.write_text(
                WRAPPER.format(python=sys.executable, name=name, real=real), encoding="utf-8"
            )
            wrapper.chmod(0o755)
        for ident in loaded_credentials():
            content = (planted or {}).get(ident, synthetic(ident) + "\n")
            (credentials / ident).write_text(content, encoding="utf-8")
        journal_file = root / "journal.txt"
        journal_file.write_text("".join(f"{line}\n" for line in journal), encoding="utf-8")
        env = {
            "PATH": f"{stubs}:{original}",
            "STUB_LOG": str(log),
            "STUB_JOURNAL": str(journal_file),
            "CREDENTIALS_DIRECTORY": str(credentials),
            **environment,
        }
        done = subprocess.run(
            [str(script), instance],
            env=env,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
        )
        return Run(done, log)


def config_value(text):
    """A parameter of curl's configuration: a quoted value with its escapes undone, or a word."""
    text = text.strip()
    if not text.startswith('"'):
        return text.split()[0] if text else ""
    out, index = [], 1
    while index < len(text):
        char = text[index]
        if char == "\\" and index + 1 < len(text):
            following = text[index + 1]
            # A backslash before any other character is dropped (curl's config.md).
            out.append(CURL_ESCAPES.get(following, following))
            index += 2
            continue
        if char == '"':
            break
        out.append(char)
        index += 1
    return "".join(out)


def config_options(text):
    """Each (option, parameter) of a curl configuration, one per line, `#` lines skipped."""
    options = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        match = re.match(r"(?:--?)?([A-Za-z0-9-]+)\s*[=:]?\s*(.*)$", line)
        options.append((match.group(1), config_value(match.group(2))))
    return options


def request(call):
    """The URLs and the form fields one curl call sent, read from its command line and from the
    configuration it read on standard input together, wherever each was given."""
    argv = call["argv"]
    options = []
    index = 0
    while index < len(argv):
        word = argv[index]
        if word in CURL_PARAMETERS and index + 1 < len(argv):
            parameter = argv[index + 1]
            if word in ("--config", "-K") and parameter == "-":
                options += config_options(call["stdin"] or "")
            else:
                options.append((word.lstrip("-"), parameter))
            index += 2
            continue
        if word.startswith(("http://", "https://")):
            options.append(("url", word))
        index += 1
    urls = [value for name, value in options if name == "url"]
    fields = {}
    for name, value in options:
        if name in ("data-urlencode", "data", "d"):
            key, _, content = value.partition("=")
            fields[key] = content
    return urls, fields


def option(argv, name):
    """The parameter of `name` in an argument vector, written `name=value` or `name value`."""
    for index, word in enumerate(argv):
        if word.startswith(f"{name}="):
            return word.partition("=")[2]
        if word == name and index + 1 < len(argv):
            return argv[index + 1]
    return None


def leaks(calls, secrets):
    """Every command line and environment of `calls` that carries one of `secrets`."""
    found = []
    for call in calls:
        for secret in secrets:
            if any(secret in word for word in call["argv"]):
                found.append(f"{call['command']}: a secret is on its command line")
            if any(secret in value for value in call["env"].values()):
                found.append(f"{call['command']}: a secret is in its environment")
    return found


def on_failure_refusals(name, unit, template):
    """Why `unit` does not page through `template` on failure with the failed unit's name."""
    wanted = template.replace("@.service", "@%n.service")
    named = values(unit, "Unit", "OnFailure")
    targets = [word for value in named for word in value.split()]
    if wanted in targets:
        return []
    return [f"{name} names OnFailure={' '.join(targets) or 'nothing'}, not {wanted}"]


def timer_target(path, services):
    """The service a timer starts: its Unit=, else the service of its own name, whose template
    ships when the name is an instance's (systemd.timer(5))."""
    unit = unit_file(path)
    named = values(unit, "Timer", "Unit")
    target = named[-1] if named else path.name.removesuffix(".timer") + ".service"
    if target in services:
        return target
    prefix, at, _ = target.partition("@")
    return f"{prefix}@.service" if at else target


def alert_template_refusals(path):
    """Why the alert template at `path` would not stay failed when its script refuses a credential
    (SPEC-066 R3), one line each, and none for a template that stays failed. It names no
    `OnFailure=`, since a page that fails must not start a page about the page (SPEC-031). It
    counts no refusal a success: one `ExecStart=`, with no `-` prefix; no `ExecCondition=`, since
    one that exits 1 to 254 skips the start and leaves the instance inactive, not failed; no
    `SuccessExitStatus=` at all; and no `RestartMode=direct`, which skips the failed state on a
    restart. It names no `[Unit]` condition or assertion, an empty one included, since an unmet one
    stops the start and leaves the instance inactive, not failed. It restarts none: no `Restart=`
    other than `no`, and no `RestartForceExitStatus=` at all. And it is never unloaded while
    failed: no `CollectMode=` other than `inactive` (systemd.service(5), systemd.unit(5)). Each of
    `RestartMode=`, `Restart=` and `CollectMode=` is read at every assignment, with no reset
    applied, and one that is empty or not a known value is refused, so the check never decides
    which of two is in force. A template the reader refuses is refused whole, with the reader's
    line."""
    name = Path(path).name
    try:
        read = list(_units.assignments(_units.unit_text(path), name))
    except _units.Refused as refusal:
        return [str(refusal)]
    template = unit_file(path)
    refused = []

    def every(key):
        section, _ = _units.ENUMS[key]
        return [value for at, named, value, _ in read if (at, named) == (section, key)]

    def refuse(why):
        refused.append(f"{name}: {why}")

    for target in values(template, "Unit", "OnFailure"):
        refuse(f"OnFailure={target} {PAGES}")
    for section, key, value, _ in read:
        if section == "Unit" and key.startswith(_units.STOPS_A_START):
            refuse(f"{key}={value} {STOPS}")
    starts = values(template, "Service", "ExecStart")
    if len(starts) != 1:
        refuse(f"{len(starts)} ExecStart= lines, {ONE_START}")
    for start in starts:
        if "-" in EXEC_PREFIX.match(start).group(0):
            refuse(f"ExecStart={start} {COUNTS_A_FAILURE}")
    for command in values(template, "Service", "ExecCondition"):
        refuse(f"ExecCondition={command} {SKIPS}")
    for key in ("SuccessExitStatus", "RestartForceExitStatus"):
        for statuses in values(template, "Service", key):
            refuse(f"{key}={statuses} {NAMED}")
    for mode in every("RestartMode"):
        if mode == "direct":
            refuse(f"RestartMode={mode} {DIRECT}")
    for restart in every("Restart"):
        if restart in _units.ENUMS["Restart"][1] and restart != "no":
            refuse(f"Restart={restart} {RESTARTS}")
    for mode in every("CollectMode"):
        if mode in _units.ENUMS["CollectMode"][1] and mode != "inactive":
            refuse(f"CollectMode={mode} {UNLOADS}")
    for key, (_, known) in _units.ENUMS.items():
        for value in every(key):
            if value not in known:
                refuse(f"{key}={value} {UNREAD}")
    pairs = [(at, named) for at, named, _, _ in read]
    for (at, named, value, _), off in zip(read, _units.off_list(pairs, _units.ALERT_KEYS), strict=True):
        if off:
            refuse(f"[{at}] {named}={value} {OFF_LIST}")
    return refused


def planted_template(scratch, anchor, line, keep):
    """The alert template written into `scratch` with `line` after its one line that starts
    `anchor`, or in its place when `keep` is false."""
    lines = (SYSTEMD / ALERT_TEMPLATE).read_bytes().decode("utf-8").split("\n")
    (at,) = [number for number, text in enumerate(lines) if text.startswith(anchor)]
    lines[at : at + 1] = [lines[at], line] if keep else [line]
    path = scratch / ALERT_TEMPLATE
    path.write_bytes("\n".join(lines).encode("utf-8"))
    return path


class TheAlertScriptNamesTheFailure(unittest.TestCase):
    def test_the_alert_script_reads_its_credentials_and_names_the_unit_and_result(self):
        run = run_alert(
            FAILED_UNIT,
            {
                "MONITOR_UNIT": FAILED_UNIT,
                "MONITOR_SERVICE_RESULT": RESULT,
                "MONITOR_EXIT_STATUS": "KILL",
            },
        )
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        sent = examined("request(s) the page sent", run.calls("curl"))
        self.assertEqual(len(sent), 1, "the page is one request")
        urls, fields = request(sent[0])
        # The token and the chat both came from the credentials directory: the chat is the owner's
        # private chat, whose id is the owner's user id (R3).
        self.assertEqual(fields.get("chat_id"), OWNER, "the page went to another chat")
        self.assertEqual(urls, [SENDMESSAGE.format(token=TOKEN)])
        # The unit the credentials were loaded for is the one R3 names, and no other.
        ids = identity_ids()
        self.assertEqual(
            sorted(loaded_credentials()), sorted([ids["OWNER_USER_ID"], ids["TELEGRAM_BOT_TOKEN"]])
        )
        text = fields.get("text", "")
        self.assertIn(FAILED_UNIT, text)
        self.assertIn(RESULT, text)
        # The failed unit's own error lines: the unit is named to the journal, at error priority.
        asked = examined("journal read(s)", run.calls("journalctl"))
        self.assertEqual(option(asked[0]["argv"], "--unit"), FAILED_UNIT)
        self.assertEqual(option(asked[0]["argv"], "--priority"), "err")

        # With no MONITOR_UNIT, the instance the unit was started as names the failed unit.
        other = "deck-streak-bot.service"
        run = run_alert(other, {"MONITOR_SERVICE_RESULT": "exit-code"})
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        _, fields = request(run.calls("curl")[0])
        self.assertIn(other, fields.get("text", ""))
        self.assertIn("exit-code", fields.get("text", ""))
        self.assertEqual(option(run.calls("journalctl")[0]["argv"], "--unit"), other)

    def test_the_page_quotes_the_failed_runs_last_five_error_lines(self):
        # systemd names the failed run, so the page quotes that run's lines and no older ones.
        run = run_alert(
            FAILED_UNIT,
            {
                "MONITOR_UNIT": FAILED_UNIT,
                "MONITOR_SERVICE_RESULT": "exit-code",
                "MONITOR_INVOCATION_ID": INVOCATION,
            },
        )
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        asked = run.calls("journalctl")[0]["argv"]
        self.assertIn(f"_SYSTEMD_INVOCATION_ID={INVOCATION}", asked)
        self.assertEqual(option(asked, "--priority"), "err")
        self.assertEqual(option(asked, "--lines"), "5")
        text = request(run.calls("curl")[0])[1]["text"]
        quoted = examined("quoted line(s)", [line for line in JOURNAL_LINES if line in text])
        self.assertEqual(quoted, JOURNAL_LINES[-5:], "the last five lines, in order")

    def test_the_page_stays_within_3500_bytes_and_whole_characters(self):
        # Five lines of a three-byte character overflow the bound, and byte 3500 falls inside one.
        long_lines = ["漢" * 400] * 5
        run = run_alert(
            FAILED_UNIT,
            {"MONITOR_UNIT": FAILED_UNIT, "MONITOR_SERVICE_RESULT": "exit-code"},
            journal=long_lines,
        )
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        text = request(run.calls("curl")[0])[1]["text"]
        encoded = text.encode("utf-8", "surrogateescape")
        try:
            encoded.decode("utf-8")
        except UnicodeDecodeError as error:
            self.fail(f"the page is not UTF-8: {error}")
        self.assertLessEqual(len(encoded), 3500)
        self.assertGreater(len(encoded), 3490, "the text was cut short of the bound")
        # Cut at a character boundary: the text is the start of the whole page, and nothing else.
        whole = f"DeckStreak: {FAILED_UNIT} failed (exit-code)\n" + "\n".join(long_lines)
        self.assertGreater(len(whole.encode("utf-8")), 3500, "the lines do not overflow the bound")
        self.assertTrue(whole.startswith(text), "the cut text is not the start of the page")

    def test_a_json_error_line_reaches_the_page_as_written(self):
        # A role's error lines are JSON, full of double quotes, and a message may hold a backslash
        # or a tab: each reaches the page as written, through curl's quoted configuration.
        line = '{"level":"ERROR","message":"a \\"quoted\\" word, a back\\\\slash","target":"x"}'
        run = run_alert(
            FAILED_UNIT,
            {
                "MONITOR_UNIT": FAILED_UNIT,
                "MONITOR_SERVICE_RESULT": "exit-code",
                "MONITOR_EXIT_STATUS": "1",
            },
            journal=[line, "a\ttab"],
        )
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        text = request(run.calls("curl")[0])[1]["text"]
        self.assertEqual(
            text, f"DeckStreak: {FAILED_UNIT} failed (exit-code, status 1)\n{line}\na\ttab"
        )


class TheTokenStaysOffTheCommandLine(unittest.TestCase):
    def test_the_bot_token_never_appears_on_the_command_line(self):
        run = run_alert(
            FAILED_UNIT,
            {"MONITOR_UNIT": FAILED_UNIT, "MONITOR_SERVICE_RESULT": RESULT},
        )
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        calls = examined("command(s) the script ran", run.all_calls)
        # No command line and no environment of anything the script ran carries the token, or the
        # owner's id, which is a credential too.
        self.assertEqual(leaks(calls, [TOKEN, OWNER]), [])
        # The token reached curl all the same: in the configuration curl read on standard input.
        sent = run.calls("curl")
        self.assertEqual(len(sent), 1)
        self.assertEqual(option(sent[0]["argv"], "--config"), "-")
        self.assertIn(TOKEN, sent[0]["stdin"])
        self.assertEqual(request(sent[0])[0], [SENDMESSAGE.format(token=TOKEN)])
        # The census refuses a planted script that puts the token in its URL on the command line.
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "planted-alert.sh"
            planted.write_text(
                "#!/bin/sh\nset -eu\n"
                'token="$(cat "$CREDENTIALS_DIRECTORY/telegram-bot-token")"\n'
                'curl --silent "https://api.telegram.org/bot$token/sendMessage" > /dev/null\n',
                encoding="utf-8",
            )
            planted.chmod(0o755)
            run = run_alert(FAILED_UNIT, {"MONITOR_UNIT": FAILED_UNIT}, script=planted)
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertEqual(leaks(run.all_calls, [TOKEN]), ["curl: a secret is on its command line"])


class EveryUnitPagesThroughTheTemplate(unittest.TestCase):
    def test_every_unit_names_the_alert_template_on_failure(self):
        template = json.loads(SLO.read_text(encoding="utf-8"))["alerting"]["unit"]
        self.assertEqual(template, ALERT_TEMPLATE)
        units = {path.name: unit_file(path) for path in sorted(SYSTEMD.glob("*.service"))}
        self.assertIn(template, units, "the alerting unit is not shipped")
        services = examined("service(s) that page on failure", [n for n in units if n != template])
        refusals = [
            refusal
            for name in services
            for refusal in on_failure_refusals(name, units[name], template)
        ]
        self.assertEqual(refusals, [])
        # Every timer-activated unit is one of them: each timer starts a shipped service.
        timers = examined("timer(s)", sorted(SYSTEMD.glob("*.timer")))
        for timer in timers:
            self.assertIn(timer_target(timer, units), services, timer.name)
        # The template itself names none: a page that fails must not page about the page.
        self.assertEqual(values(units[template], "Unit", "OnFailure"), [])
        # Planted units: one with no OnFailure=, one naming the template with no specifier.
        with tempfile.TemporaryDirectory() as scratch:
            silent = Path(scratch) / "silent.service"
            silent.write_text("[Unit]\nDescription=planted\n[Service]\nExecStart=/bin/true\n")
            bare = Path(scratch) / "bare.service"
            bare.write_text("[Unit]\nOnFailure=deck-streak-alert.service\n[Service]\n")
            planted = {path.name: unit_file(path) for path in (silent, bare)}
        self.assertEqual(
            [
                r
                for name, unit in planted.items()
                for r in on_failure_refusals(name, unit, template)
            ],
            [
                f"silent.service names OnFailure=nothing, not {ON_FAILURE}",
                f"bare.service names OnFailure=deck-streak-alert.service, not {ON_FAILURE}",
            ],
        )


class AnEmptyCredentialFailsTheAlertUnit(unittest.TestCase):
    def test_an_empty_credential_fails_the_alert_unit_before_any_request(self):
        # The alert template names no OnFailure=, so it cannot page about itself (SPEC-031): its
        # own refusal is its failed state, with one error line naming the credential (SPEC-066 R3).
        # Each credential the template loads, empty in each form, the other holding its value.
        environment = {"MONITOR_UNIT": FAILED_UNIT, "MONITOR_SERVICE_RESULT": RESULT}
        cases = [(ident, form) for ident in loaded_credentials() for form in ("", "\n")]
        for ident, form in examined("empty credential case(s)", cases):
            where = f"{ident} holding {form!r}"
            run = run_alert(FAILED_UNIT, environment, planted={ident: form})
            self.assertEqual(run.returncode, 1, f"{where}: {run.stdout}{run.stderr}")
            refusal = (
                f"<3>the credential {ident} is empty in the credentials directory: no page is sent"
            )
            self.assertEqual(run.stderr.splitlines(), [refusal], where)
            # Refused before anything is asked or sent: no journal read and no request.
            asked = [call["command"] for call in run.all_calls]
            self.assertEqual([c for c in asked if c in ("curl", "journalctl")], [], where)
            # What it wrote names the id and carries no value of either credential.
            for value in (TOKEN, OWNER):
                self.assertNotIn(value, run.stdout + run.stderr, where)
        # The route is the failed instance: the template still names no OnFailure=, and a page
        # about it is a second route's (#285). It loads its two credentials, and nothing in it
        # counts the refusal a success, restarts it or unloads the failed instance.
        template = unit_file(SYSTEMD / ALERT_TEMPLATE)
        self.assertEqual(values(template, "Unit", "OnFailure"), [])
        self.assertEqual(len(values(template, "Service", "LoadCredential")), 2)
        self.assertEqual(alert_template_refusals(SYSTEMD / ALERT_TEMPLATE), [])
        # Planted templates: the alert template with one line added after its ExecStart= or its
        # Description=, or its ExecStart= given the `-` prefix, each refused for what it breaks.
        (start,) = values(template, "Service", "ExecStart")
        name = ALERT_TEMPLATE

        def off(section, *assigned):
            # The lines a key off the alert's list adds, each after the refusals of what it breaks.
            return [f"{name}: [{section}] {a} {OFF_LIST}" for a in assigned]

        plants = [
            ("ExecStart=", f"ExecStart=-{start}", False, f"ExecStart=-{start} {COUNTS_A_FAILURE}"),
            ("ExecStart=", "ExecStart=/bin/true", True, f"2 ExecStart= lines, {ONE_START}"),
            ("ExecStart=", "ExecCondition=/bin/true", True, f"ExecCondition=/bin/true {SKIPS}"),
            ("ExecStart=", "SuccessExitStatus=2", True, f"SuccessExitStatus=2 {NAMED}"),
            ("ExecStart=", "RestartForceExitStatus=2", True, f"RestartForceExitStatus=2 {NAMED}"),
            ("ExecStart=", "RestartMode=direct", True, f"RestartMode=direct {DIRECT}"),
            ("ExecStart=", "Restart=on-failure", True, f"Restart=on-failure {RESTARTS}"),
            (
                "Description=",
                "CollectMode=inactive-or-failed",
                True,
                f"CollectMode=inactive-or-failed {UNLOADS}",
            ),
            ("Description=", f"OnFailure={ON_FAILURE}", True, f"OnFailure={ON_FAILURE} {PAGES}"),
        ]
        off_planted = {
            "ExecCondition=/bin/true": ("Service", "ExecCondition=/bin/true"),
            "SuccessExitStatus=2": ("Service", "SuccessExitStatus=2"),
            "RestartForceExitStatus=2": ("Service", "RestartForceExitStatus=2"),
            "RestartMode=direct": ("Service", "RestartMode=direct"),
            "Restart=on-failure": ("Service", "Restart=on-failure"),
            "CollectMode=inactive-or-failed": ("Unit", "CollectMode=inactive-or-failed"),
            f"OnFailure={ON_FAILURE}": ("Unit", f"OnFailure={ON_FAILURE}"),
        }
        for anchor, line, keep, refusal in examined("planted alert template(s)", plants):
            with tempfile.TemporaryDirectory() as scratch:
                path = planted_template(Path(scratch), anchor, line, keep)
                extra = off(*off_planted[line]) if line in off_planted else []
                self.assertEqual(alert_template_refusals(path), [f"{name}: {refusal}"] + extra, line)
        # A reset or an unknown value of Restart=, RestartMode= or CollectMode= is refused beside
        # what it follows (SPEC-066 R3).
        unread = "is empty or not a known value, which the check refuses"
        reset = [
            (
                "ExecStart=",
                "Restart=on-failure\nRestartSec=1d\nRestart=",
                [f"Restart=on-failure {RESTARTS}", f"Restart= {unread}"],
                off("Service", "Restart=on-failure", "RestartSec=1d", "Restart="),
            ),
            (
                "ExecStart=",
                "RestartMode=direct\nRestartMode=",
                [f"RestartMode=direct {DIRECT}", f"RestartMode= {unread}"],
                off("Service", "RestartMode=direct", "RestartMode="),
            ),
            (
                "Description=",
                "CollectMode=inactive-or-failed\nCollectMode=",
                [f"CollectMode=inactive-or-failed {UNLOADS}", f"CollectMode= {unread}"],
                off("Unit", "CollectMode=inactive-or-failed", "CollectMode="),
            ),
            (
                "ExecStart=",
                "Restart=On-Failure",
                [f"Restart=On-Failure {unread}"],
                off("Service", "Restart=On-Failure"),
            ),
        ]
        for anchor, line, refusals, extra in examined(
            "planted alert template(s) with a reset", reset
        ):
            with tempfile.TemporaryDirectory() as scratch:
                path = planted_template(Path(scratch), anchor, line, True)
                self.assertEqual(
                    alert_template_refusals(path), [f"{name}: {r}" for r in refusals] + extra, line
                )
        # Every [Unit] condition and assertion is refused, an empty one included (SPEC-066 R3).
        stops = (
            "is refused, as every condition and assertion is, since one can stop the start and "
            "leave the instance inactive, not failed"
        )
        stopped = [
            (
                "ConditionPathExists=/nonexistent",
                [f"ConditionPathExists=/nonexistent {stops}"],
                off("Unit", "ConditionPathExists=/nonexistent"),
            ),
            (
                "AssertPathExists=/nonexistent",
                [f"AssertPathExists=/nonexistent {stops}"],
                off("Unit", "AssertPathExists=/nonexistent"),
            ),
            (
                "ConditionPathExists=/nonexistent\nConditionPathExists=",
                [f"ConditionPathExists=/nonexistent {stops}", f"ConditionPathExists= {stops}"],
                off("Unit", "ConditionPathExists=/nonexistent", "ConditionPathExists="),
            ),
        ]
        for line, refusals, extra in examined("planted alert template(s) with a condition", stopped):
            with tempfile.TemporaryDirectory() as scratch:
                path = planted_template(Path(scratch), "Description=", line, True)
                self.assertEqual(
                    alert_template_refusals(path), [f"{name}: {r}" for r in refusals] + extra, line
                )
        # Planted lines the reader refuses, after ExecStart=, each refused whole with its line: one
        # ending in a backslash, a comment's included, and a control character other than a tab or
        # whitespace outside ASCII (SPEC-066 R3).
        lines = (SYSTEMD / ALERT_TEMPLATE).read_bytes().decode("utf-8").split("\n")
        (after,) = [n + 2 for n, text in enumerate(lines) if text.startswith("ExecStart=")]
        backslash = "ends in a backslash, which the reader refuses"
        misread = [
            ("# a note \\\nExecCondition=/bin/true", backslash),
            ("X-Note=kept \\\\\nExecCondition=/bin/true", backslash),
        ] + [
            (
                f"SuccessExitStatus={char}1 X-Y=z",
                f"holds U+{ord(char):04X}, a character the reader refuses",
            )
            for char in "\x0b\x0c\x85\u2028\u2029"
        ]
        for line, why in examined("planted alert template(s) the reader refuses", misread):
            with tempfile.TemporaryDirectory() as scratch:
                path = planted_template(Path(scratch), "ExecStart=", line, True)
                self.assertEqual(
                    alert_template_refusals(path), [f"{name}:{after}: {why}"], repr(line)
                )


    def test_a_key_off_the_alert_templates_list_is_refused_by_name(self):
        # The alert template holds only the keys `_units.ALERT_KEYS` lists, each in the section the
        # list gives it: a key off the list, in any section, is refused by its key (SPEC-066 R3).
        # The committed template holds none. Planted, one line at a time after the template's
        # Description= (in [Unit]) or its ExecStart= (in [Service]): the directives that make a
        # start depend on another unit; a key with no standard meaning; and a key the list holds in
        # the other section.
        self.assertEqual(alert_template_refusals(SYSTEMD / ALERT_TEMPLATE), [])
        name = ALERT_TEMPLATE
        plants = [
            ("Description=", "Requisite=missing.service", "Unit", "Requisite", "missing.service"),
            ("Description=", "Requires=missing.service", "Unit", "Requires", "missing.service"),
            ("Description=", "BindsTo=missing.service", "Unit", "BindsTo", "missing.service"),
            ("Description=", "X-Note=kept", "Unit", "X-Note", "kept"),
            ("Description=", "User=nobody", "Unit", "User", "nobody"),
            ("ExecStart=", "X-Note=kept", "Service", "X-Note", "kept"),
            ("ExecStart=", "Wants=network-online.target", "Service", "Wants", "network-online.target"),
        ]
        got = {}
        for anchor, line, section, key, value in examined("planted alert template(s)", plants):
            with tempfile.TemporaryDirectory() as scratch:
                path = planted_template(Path(scratch), anchor, line, True)
                got[line] = alert_template_refusals(path)
        self.assertEqual(
            got,
            {
                line: [f"{name}: [{section}] {key}={value} {OFF_LIST}"]
                for _, line, section, key, value in plants
            },
        )


if __name__ == "__main__":
    unittest.main()
