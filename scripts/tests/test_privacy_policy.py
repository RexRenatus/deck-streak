"""PRIVACY.md gives every category of privacy.json its lawful basis and its retention, and discloses
every copy an erase cannot reach, each with its window: the backup replica, the service journal,
the private copy of the collection, and the cron-fire ledger (SPEC-021 A9, R7; CHARTER 13).

The policy is judged against privacy.json and against the ledger's retention in the code, never
against a number written here, and each census is paired with a planted policy it must refuse.
"""

import json
import re
import unittest

from _support import REPO, examined
from test_sync_server_runbook import OFFSITE_PERIOD

POLICY = REPO / "PRIVACY.md"
INVENTORY = REPO / "privacy.json"
README = REPO / "README.md"
ABOUT = REPO / "web" / "app" / "src" / "routes" / "about" / "+page.svelte"
MAINTENANCE = REPO / "crates" / "coordination" / "src" / "maintenance.rs"
# The section of the policy that lists what an erase leaves.
UNREACHED = "## What an erase does not reach"
# The words an Article 6(1) basis and a retention event are written in, in the policy.
BASES = {
    "consent": "consent",
    "contract": "contract",
    "legal-obligation": "legal obligation",
    "vital-interests": "vital interests",
    "public-task": "public task",
    "legitimate-interests": "legitimate interests",
}
EVENTS = {
    "account-deletion": "account deletion",
    "consent-withdrawal": "consent withdrawal",
    "purpose-end": "purpose end",
    "last-activity": "last activity",
    "collection": "collection",
}
UNITS = {"Y": "year", "M": "month", "W": "week", "D": "day"}


def inventory():
    return json.loads(INVENTORY.read_text(encoding="utf-8"))


def in_words(period):
    """An ISO 8601 period of one unit, `P3D`, as the policy writes it: `3 days`."""
    match = re.fullmatch(r"P(\d+)([YMWD])", period)
    if match is None:
        raise AssertionError(f"{period!r} is not a period of one unit")
    count = int(match.group(1))
    return f"{count} {UNITS[match.group(2)]}{'' if count == 1 else 's'}"


def ledger_days():
    """How many days the cron-fire ledger keeps a row: the maintenance job's own constant."""
    source = MAINTENANCE.read_text(encoding="utf-8")
    match = re.search(r"pub const CRON_FIRES_RETENTION_DAYS: i64 = (\d+);", source)
    if match is None:
        raise AssertionError("the maintenance job names no CRON_FIRES_RETENTION_DAYS")
    return int(match.group(1))


def flat(text):
    """Text with its whitespace runs made single spaces, casefolded."""
    return " ".join(text.split()).casefold()


def undisclosed_categories(policy, categories):
    """Every category no line of the policy names with its basis and its retention."""
    lines = [flat(line) for line in policy.splitlines()]
    refused = []
    for category in categories:
        token = re.compile(rf"(?<![\w-]){re.escape(category['id'])}(?![\w-])")
        basis = BASES[category["lawful_basis"]]
        retention = category["retention"]
        kept = EVENTS[retention["until"]] if "until" in retention else in_words(retention["period"])
        if not any(token.search(line) and basis in line and kept in line for line in lines):
            refused.append(f"{category['id']} ({basis}, {kept})")
    return refused


def copies(data):
    """Each copy an erase cannot reach, with the words its disclosure must carry."""
    return {
        "the backup replica": ("replica", in_words(data["backups"]["retention"])),
        "the service journal": ("journal", in_words(data["logs"]["retention"])),
        "the private copy of the collection": ("collection", "sync", "credential"),
        "the cron-fire ledger": ("cron-fire ledger", f"{ledger_days()} days"),
        # The sync server's store is copied in its window and archived (SPEC-337 R5): no table of
        # the inventory holds it, so the policy names it here, with where its copies are kept.
        "the sync server's snapshots": ("snapshot", "sync server", "three", "offsite"),
    }


def unreached_bullets(policy):
    """Each bullet of the policy's unreached section, flattened."""
    _, found, rest = policy.partition(UNREACHED)
    section = rest.split("\n## ", 1)[0] if found else ""
    return [flat(bullet) for bullet in re.split(r"\n(?=- )", section) if bullet.startswith("- ")]


def undisclosed_copies(policy, data):
    """Every copy no bullet of the policy's unreached section discloses with its words."""
    bullets = unreached_bullets(policy)
    return [
        copy
        for copy, words in copies(data).items()
        if not any(all(word in bullet for word in words) for bullet in bullets)
    ]


class ThePolicyDisclosesWhatAnEraseLeaves(unittest.TestCase):
    def test_the_policy_discloses_every_copy_an_erase_cannot_reach(self):
        data = inventory()
        policy = POLICY.read_text(encoding="utf-8")
        categories = examined("categories of privacy.json", data["categories"])
        self.assertEqual(undisclosed_categories(policy, categories), [])
        disclosed = examined("copies an erase cannot reach", list(copies(data)))
        self.assertEqual(undisclosed_copies(policy, data), [])
        self.assertEqual(len(disclosed), 5)
        # A policy that leaves a copy out, or a category's retention, is refused by name.
        without_replica = "\n".join(
            line for line in policy.splitlines() if "replica" not in line.casefold()
        )
        self.assertEqual(undisclosed_copies(without_replica, data), ["the backup replica"])
        first = categories[0]
        without_first = "\n".join(
            line for line in policy.splitlines() if f"`{first['id']}`" not in line
        )
        self.assertEqual(
            [entry.split(" ")[0] for entry in undisclosed_categories(without_first, categories)],
            [first["id"]],
        )

    def test_the_offsite_snapshot_has_a_stated_period(self):
        """SPEC-340 A14 (R11; ADR-351 D9): the snapshots' bullet states the offsite archives'
        period, the one the runbook's bucket rule checks, and the policy discloses the edge log of
        the sync route, with the journal's window, and the ban list, with the ban's."""
        data = inventory()
        bullets = examined("unreached bullet(s)", unreached_bullets(POLICY.read_text("utf-8")))
        snapshots = [b for b in bullets if "snapshot" in b and "sync server" in b]
        self.assertEqual(len(snapshots), 1, "one bullet discloses the sync server's snapshots")
        self.assertIn(f"`{OFFSITE_PERIOD}`".casefold(), snapshots[0])
        self.assertIn(in_words(OFFSITE_PERIOD), snapshots[0])
        self.assertNotIn("as long as the bucket's own retention", snapshots[0])
        logged = ("edge", "sync route", "address", "method", "path", "status")
        window = in_words(data["logs"]["retention"])
        edge = [b for b in bullets if all(word in b for word in logged)]
        self.assertEqual(len(edge), 1, "one bullet discloses the edge log of the sync route")
        self.assertIn(window, edge[0])
        self.assertIn(f"`{data['logs']['retention']}`".casefold(), edge[0])
        self.assertTrue(
            any("ban list" in b and "address" in b and "hour" in b for b in bullets),
            "no bullet discloses the ban list with the ban's period",
        )

    def test_the_readme_and_the_about_page_link_the_policy(self):
        data = inventory()
        entries = examined("policy entry points", data["policy"]["entry"])
        self.assertEqual(data["policy"]["file"], "PRIVACY.md")
        for entry in entries:
            self.assertIn(entry["text"], (REPO / entry["file"]).read_text(encoding="utf-8"))
        named = {entry["file"] for entry in entries}
        # SPEC-026 R14: the bot's command table, whose /privacy links the policy, is an entry point.
        self.assertEqual(
            named,
            {"README.md", "web/app/src/routes/about/+page.svelte", "crates/bot/src/commands.rs"},
        )
        readme = README.read_text(encoding="utf-8")
        self.assertRegex(readme, r"(?m)^## Privacy$")
        self.assertIn("[PRIVACY.md](PRIVACY.md)", readme)
        self.assertIn("PRIVACY.md", ABOUT.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
