"""SPEC-025's registrations: the watchdog's constants.

`watchdog.constants` reads the two constants the predecessor arms its systemd watchdog heartbeat
with (`watchdog.py:create_heartbeat`): the fewest seconds of `WatchdogSec=` for which it arms the
heartbeat at all, and the divisor of `WatchdogSec=` that gives the heartbeat's interval. The
daemon's lifecycle ports both, and nothing here computes the rule.
"""

FUNCTIONS = {
    "watchdog.constants": {
        "kind": "constants",
        "names": [
            "watchdog._MIN_WATCHDOG_SEC",
            "watchdog._HEARTBEAT_DIVISOR",
        ],
    },
}
