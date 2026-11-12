"""Each loopback-only service is allowed only the peers it uses (SPEC-395, ADR-409).

The API, the MCP server and the sync server each listen on loopback alone, and each one's callers
sit on loopback while it calls nothing. A1 reads each listener's unit through its drop-ins and
requires an allow list of `localhost` alone and a deny list of `any` alone. A2 closes the
population: the listen settings in the example configuration are exactly the table's, each a
loopback address. No refusal prints a value.
"""

import ipaddress
import re
import shutil
import tempfile
import unittest
from pathlib import Path

import _units
from _support import REPO, examined

SYSTEMD = REPO / "deploy" / "systemd"
ENVIRONMENT_FILE = REPO / "deploy" / "deck-streak.env.example"
EXAMPLE = "deploy/deck-streak.env.example"
ALLOW = ["localhost"]
DENY = ["any"]
LISTEN_SETTING = re.compile(r"DECKSTREAK_[A-Z_]+_LISTEN")

# SPEC-395 R1 to R3 and ADR-409 D1 to D3: every loopback listener, with the setting that names it.
LOOPBACK_LISTENERS = {
    "deck-streak-api.service": "DECKSTREAK_API_LISTEN",
    "deck-streak-mcp.service": "DECKSTREAK_MCP_LISTEN",
    "deck-streak-sync-server.service": "DECKSTREAK_SYNC_SERVER_LISTEN",
}


def peer_refusals(root):
    """One refusal per listener unit whose lists are not loopback alone, or that is absent."""
    units = _units.load_subject(root).units
    refusals = []
    for name in examined("loopback listener unit(s)", sorted(LOOPBACK_LISTENERS)):
        unit = units.get(name)
        if unit is None:
            refusals.append(
                f"deploy/systemd/{name}: the loopback listener's unit is absent, and is refused"
            )
            continue
        allow = unit.values("Service", "IPAddressAllow")
        deny = unit.values("Service", "IPAddressDeny")
        if allow != ALLOW:
            refusals.append(f"{unit.rel}: allows {allow!r}, not loopback alone, and is refused")
        if deny != DENY:
            refusals.append(f"{unit.rel}: denies {deny!r}, not every other address, and is refused")
    return refusals


def on_loopback(value):
    """Whether a listen value, an address with a port, names a loopback address."""
    host = value.rpartition(":")[0].strip("[]")
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


def listen_settings(path):
    """The (line, key, value) entries of an env file whose key names a listener."""
    return [
        (line, key, value)
        for line, key, value in _units.env_assignments(path)
        if LISTEN_SETTING.fullmatch(key)
    ]


def listener_refusals(settings):
    """One refusal per listen setting the table lacks or off loopback, and per table entry unset."""
    table = set(LOOPBACK_LISTENERS.values())
    refusals = []
    seen = set()
    for line, key, value in examined("listen setting(s)", settings):
        seen.add(key)
        if key not in table:
            refusals.append(
                f"{EXAMPLE}:{line}: {key} names a listener no unit in the census holds, "
                "and is refused"
            )
        if not on_loopback(value):
            refusals.append(f"{EXAMPLE}:{line}: {key} is not a loopback address, and is refused")
    for key in sorted(table - seen):
        refusals.append(f"{EXAMPLE}: {key} is absent, and is refused")
    return refusals


def plant_replace(scratch, name, find, replace):
    """Replace `find` (present exactly once) with `replace` in one scratch unit."""
    path = scratch / "deploy" / "systemd" / name
    text = path.read_text(encoding="utf-8")
    assert text.count(find) == 1, f"{name}: the planted anchor is not unique"
    path.write_text(text.replace(find, replace), encoding="utf-8")


def plant_dropin(scratch, name, text):
    folder = scratch / "deploy" / "systemd" / f"{name}.d"
    folder.mkdir(parents=True)
    (folder / "30-planted.conf").write_text(text, encoding="utf-8")


def plant_absent(scratch, name):
    (scratch / "deploy" / "systemd" / name).unlink()


class EachLoopbackListenerReachesItsPeersAlone(unittest.TestCase):
    def test_each_loopback_listener_allows_loopback_alone_and_denies_every_other_address(self):
        """SPEC-395 A1, R1 and R2; ADR-409 D1 to D3."""
        self.assertEqual(peer_refusals(REPO), [])
        units = _units.load_subject(REPO).units
        held = {
            name: (
                units[name].values("Service", "IPAddressAllow"),
                units[name].values("Service", "IPAddressDeny"),
            )
            for name in LOOPBACK_LISTENERS
        }
        self.assertEqual(held, {name: (ALLOW, DENY) for name in LOOPBACK_LISTENERS})

        api = "deck-streak-api.service"
        mcp = "deck-streak-mcp.service"
        sync = "deck-streak-sync-server.service"
        plants = {
            "the API allows every address": (
                lambda s: plant_replace(
                    s, api, "IPAddressAllow=localhost\n", "IPAddressAllow=any\n"
                ),
                [f"deploy/systemd/{api}: allows ['any'], not loopback alone, and is refused"],
            ),
            "the MCP server denies one range alone": (
                lambda s: plant_replace(
                    s, mcp, "IPAddressDeny=any\n", "IPAddressDeny=link-local\n"
                ),
                [
                    f"deploy/systemd/{mcp}: denies ['link-local'], not every other address, "
                    "and is refused"
                ],
            ),
            "the sync server's allow list is removed": (
                lambda s: plant_replace(s, sync, "IPAddressAllow=localhost\n", ""),
                [f"deploy/systemd/{sync}: allows [], not loopback alone, and is refused"],
            ),
            "a drop-in clears the API's allow list": (
                lambda s: plant_dropin(s, api, "[Service]\nIPAddressAllow=\n"),
                [f"deploy/systemd/{api}: allows [], not loopback alone, and is refused"],
            ),
            "the MCP server's unit is absent": (
                lambda s: plant_absent(s, mcp),
                [f"deploy/systemd/{mcp}: the loopback listener's unit is absent, and is refused"],
            ),
        }
        got = {}
        expected = {}
        for label in examined("planted case(s) held to the peer census", sorted(plants)):
            plant, refusals = plants[label]
            with tempfile.TemporaryDirectory() as scratch:
                scratch = Path(scratch)
                shutil.copytree(SYSTEMD, scratch / "deploy" / "systemd")
                plant(scratch)
                got[label] = peer_refusals(scratch)
            expected[label] = refusals
        self.assertEqual(got, expected)

    def test_the_listeners_are_every_listen_setting_and_each_listens_on_loopback(self):
        """SPEC-395 A2, R3; ADR-409 D1 and D3."""
        settings = listen_settings(ENVIRONMENT_FILE)
        self.assertEqual(listener_refusals(settings), [])
        self.assertEqual(sorted(key for _, key, _ in settings), sorted(LOOPBACK_LISTENERS.values()))

        api = next(entry for entry in settings if entry[1] == "DECKSTREAK_API_LISTEN")
        wildcard = f"{ipaddress.IPv4Address(0)}:{api[2].rpartition(':')[2]}"
        plants = {
            "a setting no unit holds": (
                settings + [(api[0], "DECKSTREAK_PLANTED_LISTEN", api[2])],
                [
                    f"{EXAMPLE}:{api[0]}: DECKSTREAK_PLANTED_LISTEN names a listener no unit "
                    "in the census holds, and is refused"
                ],
            ),
            "the API listens on every address": (
                [(api[0], api[1], wildcard) if entry == api else entry for entry in settings],
                [
                    f"{EXAMPLE}:{api[0]}: DECKSTREAK_API_LISTEN is not a loopback address, and is refused"
                ],
            ),
            "the MCP server's setting is removed": (
                [entry for entry in settings if entry[1] != "DECKSTREAK_MCP_LISTEN"],
                [f"{EXAMPLE}: DECKSTREAK_MCP_LISTEN is absent, and is refused"],
            ),
        }
        got = {}
        expected = {}
        for label in examined("planted case(s) held to the listen census", sorted(plants)):
            planted, refusals = plants[label]
            got[label] = listener_refusals(planted)
            expected[label] = refusals
        self.assertEqual(got, expected)


if __name__ == "__main__":
    unittest.main()
