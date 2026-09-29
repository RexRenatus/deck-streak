"""SPEC-059 A3: the path unit that starts the sync job, and who may write its request."""

import posixpath
import re
import tempfile
import unittest
from pathlib import Path

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
    """The problems a path unit has as the sync job's doorbell.

    The [Path] section is the exact watch set: one PathChanged= on the request file and no other
    key, so a level trigger (PathExists, PathExistsGlob, DirectoryNotEmpty), another edge
    (PathModified) or a redirected Unit= are each refused without being named.
    """
    problems = []
    if unit.get("Path") != [("PathChanged", REQUEST_FILE)]:
        problems.append("the [Path] section is exactly PathChanged=" + REQUEST_FILE)
    if values(unit, "Path", "Unit"):
        problems.append("Unit= must stay unset so the instance's own service starts")
    text = repr(unit)
    for forbidden in ("LoadCredential", "Environment", "ExecStart"):
        if forbidden in text:
            problems.append(f"a doorbell carries no {forbidden}")
    return problems


def writable_paths(unit):
    """Every path a unit's [Service] section makes writable: ReadWritePaths= and BindPaths=
    entries (their "-" and "+" prefixes and a bind's destination dropped), and each
    RuntimeDirectory=, which systemd creates below /run, owned by the unit's user."""
    for key in ("ReadWritePaths", "BindPaths"):
        for value in values(unit, "Service", key):
            for entry in value.split():
                yield as_systemd_reads(entry.strip("\"'").lstrip("-+").split(":")[0])
    for value in values(unit, "Service", "RuntimeDirectory"):
        for entry in value.split():
            yield as_systemd_reads("/run/" + entry.strip("\"'").split(":")[0])


def as_systemd_reads(path):
    """`path` as systemd resolves it: "." and repeated slashes dropped, and /var/run, the
    symlink every host keeps to /run, followed."""
    path = posixpath.normpath("/" + path.lstrip("/"))
    return "/run" + path[len("/var/run") :] if (path + "/").startswith("/var/run/") else path


def reaches_the_request_directory(path):
    """Whether write access to `path` is write access to the request directory: the directory
    itself, a path inside it, or any directory above it."""
    inside = (path + "/").startswith(REQUEST_DIRECTORY + "/")
    return inside or REQUEST_DIRECTORY.startswith(path.rstrip("/") + "/")


def request_directory_writers(units):
    """The names of the units that can write the request directory."""
    return sorted(
        name
        for name, unit in units.items()
        if any(reaches_the_request_directory(path) for path in writable_paths(unit))
    )


def every_unit(root=SYSTEMD):
    """Every unit file directly under `root`, parsed, by file name."""
    return {path.name: sections(read(path)) for path in sorted(root.iterdir()) if path.is_file()}


class TheSyncPath(unittest.TestCase):
    def test_a_planted_bad_path_unit_is_refused(self):
        planted = sections("[Path]\nPathExists=" + REQUEST_FILE + "\nLoadCredential=x:y\n")
        self.assertEqual(len(judge_path_unit(planted)), 2, judge_path_unit(planted))
        good = "[Path]\nPathChanged=" + REQUEST_FILE + "\n"
        self.assertEqual(judge_path_unit(sections(good)), [])
        for extra in (
            "PathModified=/run/deck-streak-sync/other",
            "PathExistsGlob=/run/deck-streak-sync/*",
            "DirectoryNotEmpty=/run/deck-streak-sync",
            "MakeDirectory=true",
        ):
            self.assertTrue(
                judge_path_unit(sections(good + extra + "\n")),
                extra + " widens the watch set and is refused",
            )

    def test_a_planted_second_writer_of_the_request_directory_is_refused(self):
        bot = sections("[Service]\nReadWritePaths=" + REQUEST_DIRECTORY + "\n")
        api = sections("[Service]\nReadWritePaths=/var/lib/deck-streak\n")
        planted = sections("[Service]\nReadWritePaths=/var/lib/x " + REQUEST_DIRECTORY + "\n")
        self.assertEqual(
            request_directory_writers({"deck-streak-bot.service": bot, "a.service": api}),
            ["deck-streak-bot.service"],
        )
        self.assertEqual(
            request_directory_writers(
                {"deck-streak-bot.service": bot, "deck-streak-api.service": planted}
            ),
            ["deck-streak-api.service", "deck-streak-bot.service"],
        )
        for line in (
            "ReadWritePaths=/run",
            "ReadWritePaths=-/run/",
            "ReadWritePaths=/",
            "ReadWritePaths=/run/deck-streak-sync/request",
            "RuntimeDirectory=deck-streak-sync",
            "BindPaths=/run/deck-streak-sync",
            'ReadWritePaths="/run/deck-streak-sync"',
            "ReadWritePaths=/run/./deck-streak-sync",
            "ReadWritePaths=//run/deck-streak-sync",
            "ReadWritePaths=/var/run/deck-streak-sync",
        ):
            writer = sections("[Service]\n" + line + "\n")
            self.assertEqual(
                request_directory_writers(
                    {"deck-streak-bot.service": bot, "deck-streak-api.service": writer}
                ),
                ["deck-streak-api.service", "deck-streak-bot.service"],
                line + " writes the request directory and is named",
            )
        sibling = sections("[Service]\nReadWritePaths=/run/deck-streak-syncer\n")
        self.assertEqual(
            request_directory_writers(
                {"deck-streak-bot.service": bot, "deck-streak-api.service": sibling}
            ),
            ["deck-streak-bot.service"],
            "a sibling directory is not the request directory",
        )

    def test_a_drop_in_that_writes_the_request_directory_is_a_writer(self):
        writes = "[Service]\nReadWritePaths=" + REQUEST_DIRECTORY + "\n"
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "deck-streak-bot.service").write_text(writes, encoding="utf-8")
            for directory in ("deck-streak-job@.service.d", "deck-streak-job@sync.service.d"):
                (root / directory).mkdir()
                (root / directory / "10-plant.conf").write_text(writes, encoding="utf-8")
            self.assertEqual(
                request_directory_writers(every_unit(root)),
                [
                    "deck-streak-bot.service",
                    "deck-streak-job@.service.d/10-plant.conf",
                    "deck-streak-job@sync.service.d/10-plant.conf",
                ],
            )

    def test_only_the_bot_unit_writes_the_request_directory(self):
        units = examined("units under deploy/systemd", list(every_unit().items()))
        self.assertEqual(
            request_directory_writers(dict(units)),
            ["deck-streak-bot.service"],
            "only deck-streak-bot.service names the request directory in ReadWritePaths=",
        )

    def test_the_path_unit_and_its_request_directory(self):
        path_unit = sections(read(SYSTEMD / ("deck-streak-job@" + "sync.path")))
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
