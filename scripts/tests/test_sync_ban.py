"""The sync route's login bound (SPEC-340 R5, A6; ADR-351 D3): a ban filter and a jail over the
edge's access log of the sync route. The filter matches a refused sync login and captures its
address, and matches nothing else. The jail reads the edge's journal and bans an address after five
refused logins within ten minutes, for one hour.

Both files are read as plain text, one `key = value` per line under its `[section]`, as the ban
service reads them. No test runs the ban service, and the rail installs both only on the owner's
go."""

import re
import unittest

from _support import REPO, examined

FAIL2BAN = REPO / "deploy" / "fail2ban"
FILTER = FAIL2BAN / "filter.d" / "deck-streak-sync.conf"
JAIL = FAIL2BAN / "jail.d" / "deck-streak-sync.conf"
# The ban service's address tag, which it expands to an IPv4 or IPv6 address and never a host name.
# The test stands a capture of the same characters in its place.
ADDR = "<ADDR>"
CAPTURE = r"(?P<host>[0-9A-Fa-f.:]+)"
# Documentation addresses (RFC 5737, RFC 3849), never a real client's.
CLIENT = "192.0.2.7"
CLIENT_V6 = "2001:db8::7"
# The sync server's login route as the edge logs it, before its prefix is stripped.
LOGIN = "/anki-sync/sync/hostKey"


def edge_line(uri, status, client=CLIENT):
    """One line of the edge's JSON access log, its fields in the order the edge writes them."""
    return (
        '{"level":"info","ts":1700000000.0,"logger":"http.log.access","msg":"handled request",'
        f'"request":{{"remote_ip":"{client}","remote_port":"50000","client_ip":"{client}",'
        f'"proto":"HTTP/2.0","method":"POST","host":"app.example.org","uri":"{uri}"}},'
        f'"bytes_read":0,"user_id":"","duration":0.01,"size":0,"status":{status}}}'
    )


def sections(path):
    """A ban service file's sections, each `[name]` with its `key = value` lines; a comment or a
    blank line is skipped, and an indented line continues the value above it. A file the
    repository does not ship has none, so the assertion on what it should hold is the one that
    fails."""
    if not path.is_file():
        return {}
    found, current, key = {}, None, None
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.lstrip().startswith(("#", ";")):
            continue
        if line[0].isspace() and key is not None:
            found[current][key] += "\n" + line.strip()
            continue
        header = re.fullmatch(r"\[([^\]]+)\]", line.strip())
        if header:
            current, key = header.group(1), None
            if current in found:
                raise AssertionError(f"{path.name}: the section [{current}] twice")
            found[current] = {}
            continue
        name, equals, value = line.partition("=")
        if not equals or current is None:
            raise AssertionError(f"{path.name}: a line the ban service cannot read: {line!r}")
        key = name.strip()
        if key in found[current]:
            raise AssertionError(f"{path.name}: the key {key} twice in [{current}]")
        found[current][key] = value.strip()
    return found


def failregexes():
    """The filter's failure patterns, one per line of its `failregex`."""
    return sections(FILTER).get("Definition", {}).get("failregex", "").splitlines()


def captured(line):
    """The address the first pattern that matches the line captures, or None. The ban service
    searches a line, so a pattern matches anywhere in it."""
    for pattern in failregexes():
        match = re.search(pattern.replace(ADDR, CAPTURE), line)
        if match:
            return match.group("host")
    return None


class TheSyncBanJail(unittest.TestCase):
    def test_the_filter_matches_a_refused_sync_login_and_nothing_else(self):
        """A6: a refused login at the sync route counts against its address, and nothing else
        does: a granted login, a refusal elsewhere, a refusal of another sync route, or another
        error at the login route."""
        self.assertEqual(captured(edge_line(LOGIN, 403)), CLIENT, "a refused sync login")
        self.assertEqual(
            captured(edge_line(LOGIN, 403, CLIENT_V6)), CLIENT_V6, "a refused login over IPv6"
        )
        for uri, status in examined(
            "line(s) that are not a refused sync login",
            (
                (LOGIN, 200),
                ("/api/x", 403),
                ("/anki-sync/sync/meta", 403),
                (LOGIN, 404),
            ),
        ):
            self.assertIsNone(captured(edge_line(uri, status)), f"{status} on {uri}")
        # One pattern, and the address it bans is the tag's one capture.
        patterns = failregexes()
        self.assertEqual(len(patterns), 1, patterns)
        self.assertEqual(patterns[0].count(ADDR), 1, patterns[0])

    def test_the_jail_bans_five_refused_logins_in_ten_minutes_for_an_hour(self):
        """A6: the jail reads the edge's journal through the filter above, and holds its numbers."""
        self.assertEqual(
            sections(JAIL),
            {
                "deck-streak-sync": {
                    "enabled": "true",
                    "filter": FILTER.stem,
                    "backend": "systemd",
                    "journalmatch": "_SYSTEMD_UNIT=caddy.service",
                    "maxretry": "5",
                    "findtime": "10m",
                    "bantime": "1h",
                    "port": "http,https",
                }
            },
        )


if __name__ == "__main__":
    unittest.main()
