"""No pull request adds a file, or grows one, past the large-file bound unless the file is listed
with its recorded size and its reason (SPEC-673 R1 to R11; ADR-696).

The census reads every entry git tracks at HEAD by `git ls-tree -r -z --full-tree`, in a format
that prints each entry's mode, type, blob size and path and no object name. In a pull request's CI
run HEAD is the merge of the pull request into its base, so a size it reads is the size that merge
would keep. It reads sizes only and opens no tracked file, so every tracked file stays in its
population whatever its name or kind.

A file past the bound with no entry in LISTED prints a `WARN` line, and so does a listed file grown
past the size its entry records. An entry whose path holds no tracked file, or whose file is at or
under the bound again, prints a `STALE` line and fails at both levels. An entry with no blob size
(a submodule's commit) prints an `UNDECIDED` line and fails at `fail` only. The planted listings
under `fixtures/large-files/` are text, one entry per row as the format prints it, and no planted
file is large, so the fixtures add no site of their own.
"""

import subprocess
import unittest

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "large-files"
# R1, R3: the listing's format names no object, so a planted row carries no object name.
FORMAT = "--format=%(objectmode) %(objecttype) %(objectsize)%x09%(path)"
# R2: the bound in bytes (1 MiB); a file is past it when its blob is strictly larger.
BOUND = 1048576
# R9: the census starts at warn; moving it to fail is its own delivery, under ADR-450 D6.
LEVEL = "warn"
# R8: today's sites, each with the size its blob had when it was listed and its reason.
GOLDEN = "a parity golden: its driver writes it and a Rust test reads it (SPEC-498)"
LISTED = {
    "tools/parity-oracle/goldens/anki.read.collection_view.json": (1067666, GOLDEN),
    "tools/parity-oracle/goldens/anki.render.card.json": (1685445, GOLDEN),
    "tools/parity-oracle/goldens/anki.render.compare_answer.json": (1452309, GOLDEN),
    "tools/parity-oracle/goldens/anki.render.latex_images.json": (1237599, GOLDEN),
    "tools/parity-oracle/goldens/anki.render.occlusion_render.json": (1407645, GOLDEN),
}


def records(text):
    """Each entry of a NUL-terminated listing as (mode, type, size, path); a size that is not a
    whole number of bytes, the `-` git prints for a submodule's commit, reads as None (R3)."""
    entries = []
    for record in text.split("\0"):
        if record:
            meta, _, path = record.partition("\t")
            mode, kind, size = meta.split(" ")
            entries.append((mode, kind, int(size) if size.isdigit() else None, path))
    return entries


def tracked(root=REPO):
    """Every entry git tracks at HEAD, read by the format above; no tracked file is opened (R1)."""
    done = subprocess.run(
        ["git", "-C", str(root), "ls-tree", "-r", "-z", "--full-tree", FORMAT, "HEAD"],
        capture_output=True,
        check=True,
    )
    return records(done.stdout.decode("utf-8"))


def planted(name):
    """A planted listing's entries: each row that is not blank and does not open with `#` is one
    entry, ended by NUL as git ends them (R11)."""
    rows = (FIXTURES / name).read_text(encoding="utf-8").splitlines()
    return records("".join(row + "\0" for row in rows if row and not row.startswith("#")))


def judge(entries, bound, listed):
    """The sites, the stale entries and the undecided entries of one population, each as the line
    it prints (R4 to R7)."""
    sites, undecided, present = [], [], {}
    for mode, kind, size, path in entries:
        if size is None:
            undecided.append(
                f"UNDECIDED large-file {path} ({kind} entry, mode {mode}: no blob size)"
            )
            continue
        present[path] = size
        if path in listed:
            if size > listed[path][0]:
                sites.append(
                    f"WARN large-file {path} {size} (listed at {listed[path][0]}, grown past it)"
                )
        elif size > bound:
            sites.append(f"WARN large-file {path} {size} (unlisted, past the bound {bound})")
    gone = sorted(path for path in listed if path not in present)
    back = sorted(path for path in listed if path in present and present[path] <= bound)
    stale = [
        f"STALE large-file {path} {listed[path][0]} (no tracked file at this path)" for path in gone
    ]
    stale += [f"STALE large-file {path} {listed[path][0]} (at or under the bound)" for path in back]
    return sites, stale, undecided


def verdict(entries, bound=BOUND, listed=LISTED, level=LEVEL):
    """Print the examined count and each line, and return the lines that fail at `level` (R9,
    R10): a stale entry fails at both levels, a site and an undecided entry at `fail` only."""
    entries = examined("file(s)", entries)
    sites, stale, undecided = judge(entries, bound, listed)
    for line in sites + stale + undecided:
        print(line)
    return (sites + undecided if level == "fail" else []) + stale


class PlantedListings(unittest.TestCase):
    """The census's arms over planted listings: text only, so the box runs these (R11)."""

    def test_a_file_past_the_bound_with_no_entry_is_a_site(self):
        entries = examined("planted file(s)", planted("past-the-bound.fixture"))
        site = "WARN large-file assets/past.bin 1048577 (unlisted, past the bound 1048576)"
        self.assertEqual(judge(entries, BOUND, {}), ([site], [], []))

    def test_a_file_at_the_bound_is_not_a_site(self):
        entries = examined("planted file(s)", planted("at-the-bound.fixture"))
        self.assertEqual(
            entries,
            [
                ("100644", "blob", 1048576, "assets/at the bound.bin"),
                ("100644", "blob", 0, "docs/empty.md"),
                ("100755", "blob", 2048, "scripts/run.sh"),
            ],
        )
        self.assertEqual(judge(entries, BOUND, {}), ([], [], []))

    def test_a_listed_file_at_or_under_its_recorded_size_is_not_a_site(self):
        entries = examined("planted file(s)", planted("listed-at-its-size.fixture"))
        listing = {
            "assets/listed.bin": (2000000, "planted"),
            "assets/shrunk.bin": (2000000, "planted"),
        }
        self.assertEqual([entry[2] for entry in entries], [2000000, 1999999])
        self.assertEqual(judge(entries, BOUND, listing), ([], [], []))

    def test_a_listed_file_grown_past_its_recorded_size_is_a_site(self):
        entries = examined("planted file(s)", planted("listed-and-grown.fixture"))
        listing = {"assets/listed.bin": (2000000, "planted")}
        site = "WARN large-file assets/listed.bin 2000001 (listed at 2000000, grown past it)"
        self.assertEqual(judge(entries, BOUND, listing), ([site], [], []))

    def test_a_renamed_listed_file_leaves_a_stale_entry_and_an_unlisted_site(self):
        entries = examined("planted file(s)", planted("listed-and-renamed.fixture"))
        listing = {"assets/listed.bin": (2000000, "planted")}
        site = "WARN large-file assets/moved/listed.bin 2000000 (unlisted, past the bound 1048576)"
        stale = "STALE large-file assets/listed.bin 2000000 (no tracked file at this path)"
        self.assertEqual(judge(entries, BOUND, listing), ([site], [stale], []))

    def test_a_listed_file_back_at_the_bound_leaves_a_stale_entry(self):
        entries = examined("planted file(s)", planted("listed-and-shrunk.fixture"))
        listing = {"assets/listed.bin": (2000000, "planted")}
        stale = "STALE large-file assets/listed.bin 2000000 (at or under the bound)"
        self.assertEqual(judge(entries, BOUND, listing), ([], [stale], []))

    def test_an_entry_with_no_blob_size_is_undecided_and_a_link_is_judged_by_its_size(self):
        entries = examined("planted file(s)", planted("no-blob-size.fixture"))
        self.assertEqual(
            entries,
            [
                ("160000", "commit", None, "vendor/engine"),
                ("120000", "blob", 12, "assets/link.bin"),
            ],
        )
        undecided = "UNDECIDED large-file vendor/engine (commit entry, mode 160000: no blob size)"
        self.assertEqual(judge(entries, BOUND, {}), ([], [], [undecided]))

    def test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does(self):
        entries = planted("listed-and-renamed.fixture") + planted("no-blob-size.fixture")
        listing = {"assets/listed.bin": (2000000, "planted")}
        site = "WARN large-file assets/moved/listed.bin 2000000 (unlisted, past the bound 1048576)"
        stale = "STALE large-file assets/listed.bin 2000000 (no tracked file at this path)"
        undecided = "UNDECIDED large-file vendor/engine (commit entry, mode 160000: no blob size)"
        self.assertEqual(verdict(entries, BOUND, listing, "warn"), [stale])
        self.assertEqual(verdict(entries, BOUND, listing, "fail"), [site, undecided, stale])

    def test_an_empty_population_is_void(self):
        entries = planted("empty.fixture")
        self.assertEqual(entries, [])
        with self.assertRaises(AssertionError) as caught:
            verdict(entries, BOUND, {}, "warn")
        self.assertIn("examined 0 file(s)", str(caught.exception))

    def test_the_planted_listings_are_the_eight_this_file_reads(self):
        names = examined("planted listing(s)", sorted(path.name for path in FIXTURES.iterdir()))
        self.assertEqual(
            names,
            [
                "at-the-bound.fixture",
                "empty.fixture",
                "listed-and-grown.fixture",
                "listed-and-renamed.fixture",
                "listed-and-shrunk.fixture",
                "listed-at-its-size.fixture",
                "no-blob-size.fixture",
                "past-the-bound.fixture",
            ],
        )


class TheListing(unittest.TestCase):
    """The level and the listing as committed, read as data, so the box runs these too (R8, R9)."""

    def test_the_census_is_at_warn_and_each_entry_records_a_site_and_its_reason(self):
        self.assertEqual(LEVEL, "warn")
        print(f"listed {len(LISTED)} entry(ies)")
        for path, (recorded, reason) in LISTED.items():
            self.assertGreater(recorded, BOUND, path)
            self.assertTrue(reason, path)


class TheTrackedTree(unittest.TestCase):
    """The census over every tracked file: it reads the repository, so it runs in CI (R1, R8)."""

    def test_no_tracked_file_fails_the_census_at_its_level(self):
        entries = tracked()
        self.assertIn("scripts/tests/test_large_files.py", [entry[3] for entry in entries])
        self.assertEqual(verdict(entries), [])


if __name__ == "__main__":
    unittest.main()
