"""The backup window is seven days and the policy says so (SPEC-584 A1, A4 to A6; ADR-603).

Every backup copy is disclosed with that window, every record that stated an older period carries
its SPEC-584 amendment, and no page claims crypto-erase before the per-learner keys exist, nor
lacks the claim once they do. Each census prints what it examined, refuses an empty population,
and is paired with a planted text it must refuse. Files are read; nothing runs.
"""

import json
import re
import unittest

from _support import REPO, examined

WINDOW = "P7D"
POLICY = REPO / "PRIVACY.md"
INVENTORY = REPO / "privacy.json"
SPECS = REPO / "docs" / "specs"
DECISIONS = REPO / "docs" / "decisions"
MIGRATIONS = REPO / "migrations"
UNREACHED = "## What an erase does not reach"
# The parked surface's names: a record or a migration so named is counted and never opened.
PARKED = re.compile(r"drill|readings_tree|_standin_checks|test_deploy_standin", re.IGNORECASE)
# The older periods a record may state only beside its SPEC-584 amendment.
OLD_PERIOD = re.compile(r"\bP(?:3|30)D\b")
# The records that stated an older period before SPEC-584, each amended by it.
AMENDED = ("ADR-064", "ADR-351", "SPEC-021", "SPEC-340")
# The records that decide the change, which name the older periods by design.
DECIDING = ("ADR-603", "SPEC-584")
AMENDMENT = re.compile(r"^## (?:\d+\. )?Amendments? \(SPEC-584\)", re.MULTILINE)
# The backup copies the policy discloses beside the replica and the daily copies.
COPIES = {
    "the disk snapshots": "disk snapshot",
    "the second-region copy": "second-region copy",
}
# The words that claim crypto-erase, and the table whose rows make the claim true.
CLAIM = re.compile(r"unreadable from that moment|crypto-?erase|destroys the key", re.IGNORECASE)
KEY_TABLE = re.compile(
    r"create\s+table\s+(?:if\s+not\s+exists\s+)?[\"`]?user_data_keys\b", re.IGNORECASE
)
# SPEC-584 R7's sentence: it lands in the delivery that creates the key table.
SENTENCE = (
    "An erase destroys the key your data is encrypted with before it removes anything else, so "
    "every backup copy of your data is unreadable from that moment, and each copy is gone within "
    "7 days (`P7D`)."
)


def flat(text):
    """Text with its whitespace runs made single spaces, casefolded."""
    return " ".join(text.split()).casefold()


def unreached_bullets(policy):
    """Each bullet of the policy's unreached section, flattened."""
    _, found, rest = policy.partition(UNREACHED)
    section = rest.split("\n## ", 1)[0] if found else ""
    return [flat(bullet) for bullet in re.split(r"\n(?=- )", section) if bullet.startswith("- ")]


def states_window(bullet):
    """Whether a flattened bullet states the window as the period and in words."""
    return f"`{WINDOW}`".casefold() in bullet and "7 days" in bullet


def window_findings(policy, data):
    """Every place the declared window is not seven days, by name."""
    found = []
    if data.get("backups", {}).get("retention") != WINDOW:
        found.append("privacy.json backups retention")
    bullets = unreached_bullets(policy)
    named = (
        ("the backup replica", "backup replica"),
        ("the daily copies", "daily copies of the database"),
    )
    for name, words in named:
        hits = [bullet for bullet in bullets if words in bullet]
        if len(hits) != 1 or not states_window(hits[0]):
            found.append(name)
    return found


def copies_findings(policy):
    """Every extra backup copy no single bullet discloses with the window, by name."""
    bullets = unreached_bullets(policy)
    found = []
    for name, words in COPIES.items():
        hits = [bullet for bullet in bullets if words in bullet]
        if len(hits) != 1 or not states_window(hits[0]):
            found.append(name)
    return found


def records():
    """Every SPEC and ADR file outside the parked names, and the parked names counted apart."""
    paths = sorted([*SPECS.rglob("*.md"), *DECISIONS.glob("*.md")])
    parked = [path for path in paths if PARKED.search(str(path.relative_to(REPO)))]
    kept = [path for path in paths if not PARKED.search(str(path.relative_to(REPO)))]
    return kept, parked


def record_id(name):
    """The `SPEC-<n>` or `ADR-<n>` a record's file name opens with, or None."""
    match = re.match(r"((?:SPEC|ADR)-\d+)-", name)
    return match.group(1) if match else None


def amendment_findings(texts):
    """`texts` maps a record's file name to its text. Every record that states an older period
    and is not one of the amended four, and every one of the four whose last section is not its
    SPEC-584 amendment stating the window, by file name."""
    found = []
    for name, text in sorted(texts.items()):
        rid = record_id(name)
        if rid in DECIDING:
            continue
        if rid not in AMENDED:
            if OLD_PERIOD.search(text):
                found.append(f"{name}: states an older period and carries no SPEC-584 amendment")
            continue
        last = text[text.rfind("\n## ") + 1 :] if "\n## " in text else ""
        if not AMENDMENT.match(last) or f"`{WINDOW}`" not in last:
            found.append(f"{name}: its last section is not its SPEC-584 amendment with the window")
    return found


def claim_findings(policy, keys):
    """The page claims crypto-erase exactly when the key table exists (SPEC-584 R7)."""
    if CLAIM.search(policy) and not keys:
        return ["PRIVACY.md claims crypto-erase before any migration creates user_data_keys"]
    if keys and SENTENCE not in " ".join(policy.split()):
        return ["user_data_keys exists and PRIVACY.md lacks SPEC-584 R7's sentence"]
    return []


class TheBackupWindow(unittest.TestCase):
    def test_the_declared_backup_window_is_seven_days(self):
        """A1 (R1, R2): privacy.json declares `P7D`, and the replica's and the daily copies'
        bullets each state it, as the period and in words."""
        data = json.loads(INVENTORY.read_text(encoding="utf-8"))
        policy = POLICY.read_text(encoding="utf-8")
        examined("unreached bullet(s)", unreached_bullets(policy))
        self.assertEqual(data["backups"]["retention"], WINDOW)
        self.assertEqual(window_findings(policy, data), [])
        # Planted: the old window in the inventory, and in both bullets.
        planted = json.loads(json.dumps(data))
        planted["backups"]["retention"] = "P3D"
        self.assertEqual(window_findings(policy, planted), ["privacy.json backups retention"])
        old = policy.replace(f"(`{WINDOW}`), the backup", "(`P3D`), the backup")
        self.assertEqual(window_findings(old, data), ["the backup replica", "the daily copies"])

    def test_the_policy_discloses_the_disk_snapshots_and_the_second_region_copy(self):
        """A4 (R3, R4): one bullet each discloses the disk snapshots and the second-region copy,
        each with the window as the period and in words."""
        policy = POLICY.read_text(encoding="utf-8")
        examined("extra backup copies", list(COPIES))
        self.assertEqual(copies_findings(policy), [])
        # Planted: a policy without each bullet's first line refuses that copy by name.
        for name, words in COPIES.items():
            without = "\n".join(
                line for line in policy.splitlines() if words not in line.casefold()
            )
            self.assertEqual(copies_findings(without), [name])

    def test_every_record_that_states_an_older_period_carries_its_amendment(self):
        """A5 (R6): every SPEC and ADR outside the parked names that states `P3D` or `P30D` is
        one of the amended four, and each of the four ends with its SPEC-584 amendment."""
        kept, parked = records()
        print(f"counted {len(parked)} parked record name(s), never opened")
        texts = {path.name: path.read_text(encoding="utf-8") for path in examined("records", kept)}
        for rid in AMENDED:
            self.assertEqual(len([n for n in texts if n.startswith(f"{rid}-")]), 1, rid)
        stating = [n for n, text in texts.items() if OLD_PERIOD.search(text)]
        stating = [n for n in stating if record_id(n) not in DECIDING]
        examined("records stating an older period", stating)
        self.assertEqual(sorted(record_id(n) for n in stating), sorted(AMENDED))
        self.assertEqual(amendment_findings(texts), [])
        # Planted: an unamended record that states an older period is refused by name.
        planted = dict(texts)
        planted["SPEC-9999-a-planted-record.md"] = "## 2. Requirements\n\n- R1. It keeps `P3D`.\n"
        self.assertEqual(
            [finding.split(":")[0] for finding in amendment_findings(planted)],
            ["SPEC-9999-a-planted-record.md"],
        )
        # Planted: an amendment heading renamed is refused by its record's name.
        name = next(n for n in texts if n.startswith("ADR-351-"))
        renamed = dict(texts)
        renamed[name] = texts[name].replace("## Amendment (SPEC-584)", "## Note (SPEC-584)")
        self.assertEqual([f.split(":")[0] for f in amendment_findings(renamed)], [name])

    def test_the_policy_claims_crypto_erase_exactly_when_the_keys_exist(self):
        """A6 (R7): no page claims crypto-erase before a migration creates `user_data_keys`, and
        once one does the page carries R7's sentence."""
        policy = POLICY.read_text(encoding="utf-8")
        names = sorted(MIGRATIONS.glob("*.sql"))
        parked = [path for path in names if PARKED.search(path.name)]
        print(f"counted {len(parked)} parked migration name(s), never opened")
        migrations = examined("migrations", [p for p in names if not PARKED.search(p.name)])
        keys = any(KEY_TABLE.search(path.read_text(encoding="utf-8")) for path in migrations)
        self.assertEqual(claim_findings(policy, keys), [])
        # Planted: the sentence without the table, and the table without the sentence.
        self.assertEqual(len(claim_findings(policy + "\n" + SENTENCE + "\n", False)), 1)
        flattened = " ".join(policy.split()).replace(SENTENCE, "")
        self.assertEqual(len(claim_findings(flattened, True)), 1)
        self.assertIsNotNone(KEY_TABLE.search("CREATE TABLE user_data_keys (user_id TEXT);"))


if __name__ == "__main__":
    unittest.main()
