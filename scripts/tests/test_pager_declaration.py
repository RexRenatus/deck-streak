"""SPEC-585 A2 to A5 (R2 to R9; ADR-604 D1 to D5, D9): the pager's declaration,
`docs/monitoring/pager.json`, held to the alert path it reads, the timers whose silence it hears, the
journal's window it keeps and the request log it counts.

The module reads files only and executes no deploy script: the declaration, `deploy/slo.json`, the
alert template and its journal sender's text, the SLO evaluator's and the daily backup's timers and
services, `privacy.json`, the evaluator's journal fixture and the two routers that open the request
span. The monitoring service matches the pages outside the tree, and the host's hand-run verify
proves them (SPEC-585 section 8).
"""

import json
import re
import unittest

from _support import REPO, examined

DECLARATION = REPO / "docs" / "monitoring" / "pager.json"
SLO = REPO / "deploy" / "slo.json"
SYSTEMD = REPO / "deploy" / "systemd"
TEMPLATE = SYSTEMD / "deck-streak-alert@.service"
SENDER = REPO / "deploy" / "scripts" / "alert-journal.sh"
EVALUATOR_SERVICE = SYSTEMD / "deck-streak-slo.service"
EVALUATOR_TIMER = SYSTEMD / "deck-streak-slo.timer"
BACKUP_SERVICE = SYSTEMD / "deck-streak-backup.service"
BACKUP_TIMER = SYSTEMD / "deck-streak-backup.timer"
INVENTORY = REPO / "privacy.json"
FIXTURE = REPO / "scripts" / "tests" / "fixtures" / "journal" / "api-burn.jsonl"
ROUTERS = (
    REPO / "crates" / "api" / "src" / "router.rs",
    REPO / "crates" / "mcp" / "src" / "server.rs",
)

# The event the trace layer writes for every request it answers, and the keys it and its span carry.
RESPONSE = "finished processing request"
SPAN_KEYS = {"method", "name", "request_id", "route"}
EVENT_KEYS = {"latency", "level", "message", "span", "spans", "status", "target"}
# The span both routers open for a request: its method, its matched route and its request id.
REQUEST_SPAN = re.compile(
    r'info_span!\(\s*"request",\s*method = %request\.method\(\),\s*route,\s*request_id,\s*\)'
)
# The four labels a request counter carries, and where each is read from (ADR-604 D5).
LABELS = {
    "unit": "the unit",
    "route": "span.route",
    "method": "span.method",
    "status_class": "status",
}


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def shipped():
    """The names of the units the install ships."""
    return {path.name for path in SYSTEMD.iterdir()}


def prefix(unit):
    """A template unit's name up to and including its `@`: every instance's name starts with it."""
    return unit[: unit.index("@") + 1]


def page(declaration, page_id):
    """The one page of `page_id` among the pages the declaration names; none or two are refused."""
    pages = examined("page(s) declared", declaration["pages"])
    found = [each for each in pages if each["id"] == page_id]
    if len(found) != 1:
        raise AssertionError(f"{len(found)} page(s) named {page_id!r}, not one")
    return found[0]


def subject(each):
    """What a page matches by name: its unit, or the prefix its units' or lines' names start with."""
    for key in ("unit", "unit_prefix", "identifier_prefix"):
        if key in each:
            return each[key]
    raise AssertionError(f"the page {each['id']!r} names no unit and no prefix")


def period_minutes(timer):
    """A timer's period in minutes, from its one `OnCalendar=` line: `*-*-* *:00/N:00 UTC` is N,
    and `*-*-* HH:MM:SS UTC`, once a day, is 1440; any other calendar is refused."""
    calendars = [
        line.removeprefix("OnCalendar=")
        for line in timer.read_text(encoding="utf-8").splitlines()
        if line.startswith("OnCalendar=")
    ]
    if len(calendars) != 1:
        raise AssertionError(f"{timer.name}: {len(calendars)} OnCalendar= line(s), not one")
    every = re.fullmatch(r"\*-\*-\* \*:00/(\d+):00 UTC", calendars[0])
    if every is not None:
        return int(every.group(1))
    if re.fullmatch(r"\*-\*-\* \d\d:\d\d:\d\d UTC", calendars[0]) is not None:
        return 24 * 60
    raise AssertionError(f"{timer.name}: {calendars[0]!r} is neither every N minutes nor daily")


def response_events():
    """Each response event of the journal fixture. A `MESSAGE` the journal stored as a list of byte
    values is decoded as UTF-8, and a leading `<n>` priority prefix is dropped; a line whose message
    is no JSON object naming the response is another event, never a response."""
    events = []
    for line in FIXTURE.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        message = json.loads(line)["MESSAGE"]
        if isinstance(message, list):
            message = bytes(message).decode("utf-8")
        if not isinstance(message, str):
            continue
        try:
            event = json.loads(re.sub(r"^<\d>", "", message))
        except ValueError:
            continue
        if isinstance(event, dict) and event.get("message") == RESPONSE:
            events.append(event)
    return events


class ThePagerIsDeclared(unittest.TestCase):
    def test_the_pager_carries_the_route_every_slo_alert_names(self):
        declaration = read_json(DECLARATION)
        slo = read_json(SLO)
        self.assertEqual(declaration["schema"], "deckstreak.pager.v1")
        routes = examined(
            "SLO alert route(s)",
            [alert["route"] for each in slo["slos"] for alert in each["alerts"]],
        )
        self.assertEqual({*routes, slo["alerting"]["channel"]}, {declaration["route"]})
        self.assertEqual(declaration["route"], "pager")

    def test_the_pager_matches_the_alert_templates_line_at_error_priority(self):
        declaration = read_json(DECLARATION)
        unit = read_json(SLO)["alerting"]["unit"]
        line = page(declaration, "an-alert-line")
        self.assertEqual(line["kind"], "line")
        self.assertEqual(unit, "deck-streak-alert@.service")
        self.assertIn(unit, shipped())
        self.assertEqual(line["identifier_prefix"], prefix(unit))
        self.assertIn("SyslogIdentifier=%N", TEMPLATE.read_text(encoding="utf-8").splitlines())
        self.assertEqual(line["priority_at_most"], 3)
        self.assertEqual(line["opens_with"], "DeckStreak: ")
        sender = SENDER.read_text(encoding="utf-8")
        self.assertIn("<3>", sender)
        self.assertIn("DeckStreak: ", sender)

    def test_a_failed_alert_instance_pages_without_its_own_line(self):
        declaration = read_json(DECLARATION)
        unit = read_json(SLO)["alerting"]["unit"]
        failed = page(declaration, "an-alert-instance-failed")
        self.assertEqual(failed["kind"], "unit-failed")
        self.assertEqual(failed["unit_prefix"], prefix(unit))
        template = TEMPLATE.read_text(encoding="utf-8").splitlines()
        starts = examined(
            "ExecStart= line(s) of the alert template",
            [line for line in template if line.startswith("ExecStart=")],
        )
        self.assertEqual([line for line in starts if "alert-journal.sh" not in line], [])
        self.assertEqual([line for line in template if line.startswith("OnFailure=")], [])


class ThePagerHearsSilence(unittest.TestCase):
    def test_the_evaluators_silence_pages_within_four_of_its_periods(self):
        evaluator = read_json(SLO)["evaluator"]
        silent = page(read_json(DECLARATION), "the-evaluator-is-silent")
        self.assertEqual(silent["kind"], "absent")
        self.assertEqual((silent["unit"], silent["timer"]), (evaluator["unit"], evaluator["timer"]))
        self.assertEqual(
            (evaluator["unit"], evaluator["timer"]), (EVALUATOR_SERVICE.name, EVALUATOR_TIMER.name)
        )
        period = period_minutes(EVALUATOR_TIMER)
        self.assertEqual(period, 5)
        self.assertLessEqual(2 * period, silent["minutes"])
        self.assertLessEqual(silent["minutes"], 4 * period)

    def test_a_daily_backup_two_hours_late_pages(self):
        late = page(read_json(DECLARATION), "the-daily-backup-is-late")
        self.assertEqual(late["kind"], "absent")
        self.assertEqual((late["unit"], late["timer"]), (BACKUP_SERVICE.name, BACKUP_TIMER.name))
        self.assertEqual({late["unit"], late["timer"]} - shipped(), set())
        period = period_minutes(BACKUP_TIMER)
        self.assertEqual(period, 24 * 60)
        self.assertLess(period, late["minutes"])
        self.assertLessEqual(late["minutes"], period + 2 * 60)


class TheShippedLinesAreBounded(unittest.TestCase):
    def test_only_deck_streak_lines_ship_and_none_outlives_the_journal(self):
        declaration = read_json(DECLARATION)
        self.assertEqual(declaration["shipped"]["units"], "deck-streak-*")
        pages = examined("page(s) declared", declaration["pages"])
        named = [subject(each) for each in pages]
        self.assertEqual([name for name in named if not name.startswith("deck-streak-")], [])
        retention = declaration["retention"]
        self.assertEqual(retention["period"], read_json(INVENTORY)["logs"]["retention"])
        self.assertIs(retention["set_before_any_line_ships"], True)

    def test_a_page_reaches_a_person_by_two_channels_of_two_kinds(self):
        delivery = read_json(DECLARATION)["delivery"]
        self.assertGreaterEqual(delivery["channels_at_least"], 2)
        self.assertGreaterEqual(delivery["kinds_at_least"], 2)
        self.assertIn("email", delivery["kinds_include"])


class TheRequestCountersCarryNoIdentity(unittest.TestCase):
    def test_the_request_counters_carry_no_request_id_and_no_identity(self):
        counters = read_json(DECLARATION)["counters"]
        self.assertEqual(counters["labels"], LABELS)
        self.assertEqual(counters["units"], ["deck-streak-api.service", "deck-streak-mcp.service"])
        self.assertEqual(set(counters["units"]) - shipped(), set())
        self.assertIn("span.request_id", counters["never"])
        events = examined("response event(s) of the journal fixture", response_events())
        spans = examined(
            "span(s) of the response events",
            [span for event in events for span in [event["span"], *event["spans"]]],
        )
        self.assertEqual([sorted(span) for span in spans if set(span) != SPAN_KEYS], [])
        self.assertEqual([sorted(event) for event in events if set(event) != EVENT_KEYS], [])
        for router in examined("router(s) that open the request span", ROUTERS):
            source = router.read_text(encoding="utf-8")
            self.assertEqual(len(REQUEST_SPAN.findall(source)), 1, router.name)


if __name__ == "__main__":
    unittest.main()
