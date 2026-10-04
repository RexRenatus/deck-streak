"""The API's SLO, its error budget policy and its burn-rate alerts, as deploy/slo.json declares them
(SPEC-031 A2, R2; ADR-031).

An alert's burn rate is the SRE workbook's: the fraction of the error budget it spends, times the
SLO's window, over the alert's long window. An alert whose burn rate times the error budget exceeds
one can never fire, since no more than every request can fail. The arithmetic is restated here in
exact fractions rather than borrowed from the observability pack's probe, so this test does not lean
on the check it stands beside.
"""

import copy
import json
import unittest
from fractions import Fraction

from _support import REPO, examined

SLO = REPO / "deploy" / "slo.json"
SYSTEMD = REPO / "deploy" / "systemd"
API_UNIT = "deck-streak-api.service"
HOURS = {"m": Fraction(1, 60), "h": Fraction(1), "d": Fraction(24), "w": Fraction(168)}
# A declared burn rate may round the exact quotient to four decimal places (0.9333 for 14/15).
ROUNDING = Fraction(1, 10_000)

# R2's declaration, as ADR-031 decided it for one owner's traffic.
OBJECTIVE = 0.99
WINDOW_DAYS = 28
EVENTS_PER_HOUR = 20
# By severity: (long window, short window, budget consumed, burn rate).
ALERTS = {
    "page": ("6h", "30m", 0.05, 5.6),
    "ticket": ("3d", "6h", 0.1, 0.9333),
}


def declaration():
    return json.loads(SLO.read_text(encoding="utf-8"))


def hours(window):
    """A window such as `30m`, `6h` or `3d`, in hours."""
    number, unit = window[:-1], window[-1]
    if unit not in HOURS or not number.isdigit():
        raise AssertionError(f"{window!r} is not a window like 30m, 6h or 3d")
    return int(number) * HOURS[unit]


def exact(number):
    """A declared number as the exact decimal it is written as."""
    return Fraction(str(number))


def burn_refusals(slo):
    """Why any alert of `slo` spends other than the budget it declares, or can never fire."""
    refusals = []
    budget = 1 - exact(slo["objective"])
    window = slo["window_days"] * 24
    for alert in slo["alerts"]:
        name = f"{slo['id']} {alert['severity']} {alert['long_window']}/{alert['short_window']}"
        long_hours, short_hours = hours(alert["long_window"]), hours(alert["short_window"])
        burn = exact(alert["burn_rate"])
        spends = exact(alert["budget_consumed"]) * window / long_hours
        if abs(burn - spends) > ROUNDING:
            refusals.append(
                f"{name}: burn rate {alert['burn_rate']} does not spend "
                f"{alert['budget_consumed']} of the budget in {alert['long_window']} "
                f"(that is {float(spends):.4f})"
            )
        if short_hours >= long_hours:
            refusals.append(f"{name}: the short window is not shorter than the long one")
        if burn * budget > 1:
            refusals.append(
                f"{name}: burn rate {alert['burn_rate']} over a budget of {float(budget):g} needs "
                "more than every request to fail, so it can never fire"
            )
    return refusals


def api_slo(body):
    """The one SLO over the API's unit."""
    slos = examined("SLO(s) declared", body["slos"])
    found = [slo for slo in slos if slo["unit"] == API_UNIT]
    if len(found) != 1:
        raise AssertionError(f"{len(found)} SLO(s) over {API_UNIT}, not one")
    return found[0]


class TheApiSloBurnsAsDeclared(unittest.TestCase):
    def test_the_api_slo_burn_rates_equal_budget_times_window_over_long_window(self):
        slo = api_slo(declaration())
        alerts = examined("alert(s) of the API's SLO", slo["alerts"])
        # Each alert spends the budget it says it spends, and each can fire.
        self.assertEqual(burn_refusals(slo), [])
        # They are R2's numbers: 99% over 28 days, a page at 5% in 6h and a ticket at 10% in 3d.
        self.assertEqual((slo["objective"], slo["window_days"]), (OBJECTIVE, WINDOW_DAYS))
        declared = {
            alert["severity"]: (
                alert["long_window"],
                alert["short_window"],
                alert["budget_consumed"],
                alert["burn_rate"],
            )
            for alert in alerts
        }
        self.assertEqual(declared, ALERTS)
        # The arithmetic refuses a page given the 30-day figure, and a burn no traffic can reach.
        thirty_day = copy.deepcopy(slo)
        for alert in thirty_day["alerts"]:
            if alert["severity"] == "page":
                alert["burn_rate"] = 6.0
        self.assertEqual(
            burn_refusals(thirty_day),
            [
                "api-availability page 6h/30m: burn rate 6.0 does not spend 0.05 of the budget "
                "in 6h (that is 5.6000)"
            ],
        )
        unreachable = copy.deepcopy(slo)
        unreachable["objective"] = 0.5
        refused = burn_refusals(unreachable)
        self.assertIn(
            "api-availability page 6h/30m: burn rate 5.6 over a budget of 0.5 needs more than "
            "every request to fail, so it can never fire",
            refused,
        )

    def test_the_api_slo_counts_response_events_for_one_owners_traffic(self):
        slo = api_slo(declaration())
        self.assertEqual(slo["id"], "api-availability")
        self.assertEqual(
            (slo["sli"]["kind"], slo["sli"]["source"]), ("availability", "journal"), slo["sli"]
        )
        self.assertIn("below 500", slo["sli"]["good"])
        self.assertIn("every response event", slo["sli"]["total"])
        # One owner's traffic lengthens the window instead of paging on two failed requests.
        self.assertEqual(slo["low_traffic"], "longer-window")
        self.assertEqual(slo["expected_events_per_hour"], EVENTS_PER_HOUR)
        self.assertTrue(slo["rationale"].strip())

    def test_every_alert_routes_to_the_one_alert_path(self):
        body = declaration()
        self.assertEqual(body["schema"], "phx.slo.v1")
        self.assertEqual(
            body["alerting"], {"unit": "deck-streak-alert@.service", "channel": "telegram"}
        )
        routes = examined(
            "alert route(s)", [alert["route"] for slo in body["slos"] for alert in slo["alerts"]]
        )
        self.assertEqual(set(routes), {"telegram"})
        # The alert unit, the evaluator and its timer, and the memory watch and its timer ship.
        shipped = {path.name for path in SYSTEMD.iterdir()}
        named = [
            body["alerting"]["unit"],
            body["evaluator"]["unit"],
            body["evaluator"]["timer"],
            body["memory_watch"]["unit"],
            body["memory_watch"]["timer"],
        ]
        self.assertEqual([name for name in named if name not in shipped], [])
        self.assertEqual(body["evaluator"]["unit"], "deck-streak-slo.service")
        self.assertEqual(body["memory_watch"]["unit"], "deck-streak-memory-watch.service")
        self.assertEqual(body["memory_watch"]["units"], "all")

    def test_the_error_budget_policy_freezes_releases_and_the_owner_arbitrates(self):
        policy = declaration()["error_budget_policy"]
        actions = examined("action(s) on exhaustion", policy["on_exhaustion"])
        self.assertTrue(any("freeze feature releases" in action for action in actions), actions)
        self.assertIn("owner", policy["escalation"])


MCP_UNIT = "deck-streak-mcp.service"


def mcp_slo(body):
    """The one SLO over the MCP server's unit."""
    slos = examined("SLO(s) declared", body["slos"])
    found = [slo for slo in slos if slo["unit"] == MCP_UNIT]
    if len(found) != 1:
        raise AssertionError(f"{len(found)} SLO(s) over {MCP_UNIT}, not one")
    return found[0]


class TheMcpSloBurnsAsDeclared(unittest.TestCase):
    def test_the_mcp_slo_counts_the_same_response_events_as_the_api(self):
        body = declaration()
        slo = mcp_slo(body)
        self.assertEqual(slo["id"], "mcp-availability")
        self.assertEqual(slo["sli"], api_slo(body)["sli"])
        self.assertEqual((slo["objective"], slo["window_days"]), (0.95, 28))
        self.assertEqual(slo["expected_events_per_hour"], 1)
        self.assertTrue(slo["rationale"].strip())

    def test_the_mcp_slo_pages_and_tickets_to_telegram_with_exact_burn_rates(self):
        slo = mcp_slo(declaration())
        alerts = examined("alert(s) of the MCP SLO", slo["alerts"])
        self.assertEqual(burn_refusals(slo), [])
        declared = {
            alert["severity"]: (
                alert["long_window"],
                alert["short_window"],
                alert["budget_consumed"],
                alert["burn_rate"],
                alert["route"],
            )
            for alert in alerts
        }
        self.assertEqual(
            declared,
            {
                "page": ("6h", "30m", 0.05, 5.6, "telegram"),
                "ticket": ("3d", "6h", 0.1, 0.9333, "telegram"),
            },
        )

    def test_one_failed_call_cannot_page_at_the_expected_traffic(self):
        slo = mcp_slo(declaration())
        page = next(alert for alert in slo["alerts"] if alert["severity"] == "page")
        budget = 1 - exact(slo["objective"])
        # Events the page's long window holds, against the count below which one failure pages.
        held = exact(slo["expected_events_per_hour"]) * hours(page["long_window"])
        needed = 1 / (exact(page["burn_rate"]) * budget)
        self.assertGreaterEqual(held, needed, f"{held} events held, {float(needed):.2f} needed")


class EverySloIsDeclaredAgainstAShippedUnit(unittest.TestCase):
    def test_every_slo_names_a_shipped_unit_and_burns_as_declared(self):
        shipped = {path.name for path in SYSTEMD.iterdir()}
        slos = examined("SLO(s) declared", declaration()["slos"])
        self.assertEqual([slo["unit"] for slo in slos if slo["unit"] not in shipped], [])
        for slo in slos:
            examined(f"alert(s) of {slo['id']}", slo["alerts"])
            self.assertEqual(burn_refusals(slo), [], slo["id"])


if __name__ == "__main__":
    unittest.main()
