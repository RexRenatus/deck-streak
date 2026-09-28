"""SPEC-059 A3: the path unit that starts the sync job, and who may write its request."""

import re
import unittest

from _support import REPO, examined

SYSTEMD = REPO / "deploy" / "systemd"
TMPFILES = REPO / "deploy" / "tmpfiles.d" / "deck-streak-sync-request.conf"
REQUEST_DIRECTORY = "/run/deck-streak-sync"
REQUEST_FILE = REQUEST_DIRECTORY + "/request"
SERVICE_USER = "deck-streak"


def sections(text):
    """The unit text as {section: [(key, value), ...]}, comments and blanks dropped."""
    found, current = {}, None
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith(("#", ";")):
            continue
        if line.startswith("[") and line.endswith("]"):
            current = found.setdefault(line[1:-1], [])
        elif current is not None and "=" in line:
            key, value = line.split("=", 1)
            current.append((key.strip(), value.strip()))
    return found


def values(unit, section, key):
    return [value for name, value in unit.get(section, []) if name == key]


def read(path):
    return path.read_text(encoding="utf-8") if path.is_file() else ""


def judge_path_unit(unit):
    """The problems a path unit has as the sync job's doorbell."""
    problems = []
    if values(unit, "Path", "PathChanged") != [REQUEST_FILE]:
        problems.append("PathChanged must be the request file alone")
    if any(
        key in ("PathExists", "PathExistsGlob", "DirectoryNotEmpty")
        for key, _ in unit.get("Path", [])
    ):
        problems.append("a level trigger restarts the job while the file remains")
    if values(unit, "Path", "Unit"):
        problems.append("Unit= must stay unset so the instance's own service starts")
    text = repr(unit)
    for forbidden in ("LoadCredential", "Environment", "ExecStart"):
        if forbidden in text:
            problems.append(f"a doorbell carries no {forbidden}")
    return problems


class TheSyncPath(unittest.TestCase):
    def test_a_planted_bad_path_unit_is_refused(self):
        planted = sections("[Path]\nPathExists=" + REQUEST_FILE + "\nLoadCredential=x:y\n")
        self.assertGreaterEqual(len(judge_path_unit(planted)), 3)

    def test_the_path_unit_and_its_request_directory(self):
        path_unit = sections(read(SYSTEMD / "deck-streak-job@sync.path"))
        bot = sections(read(SYSTEMD / "deck-streak-bot.service"))
        job = sections(read(SYSTEMD / "deck-streak-job@.service"))
        tmpfiles = [
            line.split()
            for line in read(TMPFILES).splitlines()
            if line.strip() and not line.startswith("#")
        ]
        examined("units and snippets", [path_unit, bot, job, tmpfiles])
        self.assertEqual(judge_path_unit(path_unit), [])
        self.assertEqual(values(path_unit, "Install", "WantedBy"), ["paths.target"])
        self.assertEqual(
            tmpfiles,
            [["d", REQUEST_DIRECTORY, "0700", SERVICE_USER, SERVICE_USER, "-"]],
            "the directory is the service user's alone, mode 0700",
        )
        self.assertIn(REQUEST_DIRECTORY, " ".join(values(bot, "Service", "ReadWritePaths")))
        self.assertNotIn(
            "/run",
            " ".join(values(job, "Service", "ReadWritePaths")),
            "only the bot may write the request directory",
        )
        credentials = " ".join(values(bot, "Service", "LoadCredential"))
        self.assertIsNone(re.search(r"anki-sync", credentials), "the bot holds no sync login")


if __name__ == "__main__":
    unittest.main()
