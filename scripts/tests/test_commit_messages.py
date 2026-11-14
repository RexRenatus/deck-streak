"""Commit subjects and trailers are read in CI, at warn (SPEC-675 R1 to R16, ADR-698).

The census reads every commit of its run's range (R1 to R3). On a pull request the checkout is the
merge ref, so the range is the merge ref's own commit and every commit of the pull request; on a
push it is the pushed tip and every commit its first parent lacks. A subject a person or a bot
wrote is a Conventional Commit, `type(scope)!: description`, with a type of TYPES and at most LIMIT
characters (R4 to R6). A merge subject git or GitHub wrote on a merge commit, and git's revert
subject beside its own body line, are counted and not judged (R7, R8). The trailer block is read by
git's own rule (R9): a key that attributes the commit is a site, and every other key is counted on
an INFO line (R10). Each unlisted site prints `WARN commit-message <id> <form> <key>`, which passes
at warn and fails at fail; a listed entry that names no current site fails as `STALE` at both
levels (R11 to R13). The planted populations are text under FIXTURES, read and never run (R15).
"""

import re
import subprocess
import unittest

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "commit-messages"
# R14: the census starts at warn; moving it to fail is its own delivery, under ADR-450 D6's rule.
LEVEL = "warn"
# R13: the sites of commits reachable from dev's tip, each (id, form, key, reason).
LISTED = ()
# R4: the specification's two types, the conventional configuration's other nine, and the release
# pull request's own.
TYPES = frozenset(
    {
        "build",
        "chore",
        "ci",
        "docs",
        "feat",
        "fix",
        "perf",
        "refactor",
        "release",
        "revert",
        "style",
        "test",
    }
)
SUBJECT = re.compile(r"(?P<type>[A-Za-z]+)(?:\((?P<scope>[^()\s][^()]*)\))?!?: \S")
# R6: the longest subject a person or a bot writes.
LIMIT = 72
# R7: the merge subjects GitHub and git write, a tool's only on a commit of two parents or more.
TOOL_MERGES = (
    re.compile(r"Merge pull request #[0-9]+ from \S+"),
    re.compile(r"Merge [0-9a-f]{40} into [0-9a-f]{40}"),
    re.compile(r"Merge (?:remote-tracking )?branch '[^']+'(?: of \S+)?(?: into \S+)?"),
)
# R8: git's revert subject, a tool's only beside the body line git writes with it.
REVERT = re.compile(r'(?:Revert|Reapply) ".+"')
REVERTS = re.compile(r"^This reverts commit [0-9a-f]{40}\.", re.M)
# R9: a trailer line as git reads one, and the two prefixes git writes by itself.
TRAILER = re.compile(r"(?P<key>[A-Za-z0-9-]+)[ \t]*:")
GIT_WRITTEN = ("signed-off-by:", "(cherry picked from commit ")
# R10: the keys that attribute a commit.
ATTRIBUTION = frozenset({"co-authored-by", "signed-off-by"})
# R2: one record per commit, its id, its parents, its author's name and its raw message.
FORMAT = "%x1e%H%x1f%P%x1f%an%x1f%B"


def git(*args):
    """git's answer in the repository: its exit code, its standard output and its standard error."""
    done = subprocess.run(
        ["git", "-C", str(REPO), *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
        timeout=120,
    )
    return done.returncode, done.stdout, done.stderr


def is_bot(author):
    """Whether an author's name is a bot account's (R2)."""
    return author.endswith("[bot]")


def parse_log(text):
    """The commits a `git log --format=FORMAT` answer holds, each (id, parents, bot, message): the
    parents are counted, and the message keeps its lines with its trailing newlines dropped (R2)."""
    commits = []
    for record in text.split("\x1e")[1:]:
        ident, parents, author, message = record.split("\x1f", 3)
        commits.append((ident, len(parents.split()), is_bot(author), message.rstrip("\n")))
    return commits


def planted(name):
    """The planted population a fixture spells (R15): each commit opens with a line
    `== <id> <parents> <author>`, and its message is every line after it up to the next such line,
    its trailing blank lines dropped. Text before the first commit, but a blank line, is refused."""
    commits = []
    for line in (FIXTURES / name).read_text(encoding="utf-8").split("\n"):
        if line.startswith("== "):
            ident, parents, author = line[3:].split(" ")
            commits.append((ident, int(parents), is_bot(author), []))
        elif commits:
            commits[-1][3].append(line)
        elif line:
            raise AssertionError(f"{name}: text before the first planted commit: {line}")
    return [
        (ident, parents, bot, "\n".join(lines).rstrip("\n"))
        for ident, parents, bot, lines in commits
    ]


def range_void(shallow, first_parent):
    """Why a run's range cannot be read, or None (R3): a shallow checkout can cut the range, and a
    head with no first parent has no base to read it from."""
    if shallow.strip() != "false":
        return f"VOID the checkout's shallow flag reads {shallow.strip()!r}, so its range is cut"
    if first_parent != 0:
        return "VOID HEAD has no first parent, so the range has no base"
    return None


def run_range():
    """The run's population: every commit `HEAD^1..HEAD` lists (R2, R3)."""
    _code, shallow, _error = git("rev-parse", "--is-shallow-repository")
    first_parent, _out, _error = git("rev-parse", "--verify", "--quiet", "HEAD^1")
    void = range_void(shallow, first_parent)
    if void:
        raise AssertionError(void)
    code, out, error = git("log", f"--format={FORMAT}", "HEAD^1..HEAD", "--")
    if code:
        raise AssertionError(f"git log HEAD^1..HEAD exited {code}: {error.strip()}")
    return parse_log(out)


def listed_commits(listed):
    """Each listed entry's commit, read by its id wherever it sits (R12); an id the checkout does
    not hold reads no commit, so its entry is stale."""
    commits = []
    for ident in sorted({entry[0] for entry in listed}):
        code, out, _error = git("log", "-1", f"--format={FORMAT}", ident, "--")
        if code == 0:
            commits.extend(parse_log(out))
    return commits


def trailer_keys(lines):
    """The keys of the trailer block git's rule reads (R9). The message ends before a line that is
    `---` or opens `--- `. Its last paragraph, when a blank line comes before it, is the block when
    every line is a trailer, or when a line opens with a prefix git writes by itself and at least a
    quarter of its lines are trailers. A line that opens with a space or a tab, after a trailer,
    continues that trailer."""
    cut = [at for at, line in enumerate(lines) if line == "---" or line.startswith("--- ")]
    kept = lines[: cut[0]] if cut else lines
    filled = [at for at, line in enumerate(kept) if line.strip()]
    kept = kept[: filled[-1] + 1] if filled else []
    gaps = [at for at, line in enumerate(kept) if not line.strip()]
    if not gaps:
        return []
    trailers, others, written, folding, keys = 0, 0, False, False, []
    for line in kept[gaps[-1] + 1 :]:
        if line[:1] in (" ", "\t") and folding:
            continue
        found = TRAILER.match(line)
        generated = line.lower().startswith(GIT_WRITTEN)
        written = written or generated
        if found:
            keys.append(found.group("key"))
        if found or generated:
            trailers += 1
            folding = True
        else:
            others += 1
            folding = False
    if trailers and (others == 0 or (written and 4 * trailers >= trailers + others)):
        return keys
    return []


def classify(commit):
    """One commit's kind, its sites and the trailer keys it counts (R4 to R10). The kind is `merge`
    or `revert` for a subject a tool wrote, else `judged`; each site is (form, key)."""
    _ident, parents, _bot, message = commit
    lines = message.split("\n")
    subject = lines[0]
    sites, counted = [], []
    if parents >= 2 and any(form.fullmatch(subject) for form in TOOL_MERGES):
        kind = "merge"
    elif REVERT.fullmatch(subject) and REVERTS.search(message):
        kind = "revert"
    else:
        kind = "judged"
        found = SUBJECT.match(subject)
        if not found:
            sites.append(("no-type", "-"))
        elif found.group("type").lower() not in TYPES:
            sites.append(("other-type", found.group("type").lower()))
        if len(subject) > LIMIT:
            sites.append(("long-subject", str(len(subject))))
    for key in trailer_keys(lines):
        if key.lower() in ATTRIBUTION:
            sites.append(("attribution-trailer", key.lower()))
        else:
            counted.append(key)
    return kind, sites, counted


def summary(population):
    """The population by kind on one `read` line, then each trailer key counted and not refused on
    its own INFO line (R2, R10)."""
    kinds = {"judged": 0, "merge": 0, "revert": 0}
    bots, info = 0, []
    for commit in population:
        kind, _sites, counted = classify(commit)
        kinds[kind] += 1
        bots += commit[2]
        info.extend(f"INFO commit-message {commit[0][:12]} trailer {key}" for key in counted)
    head = (
        f"read {len(population)} commit(s): {kinds['judged']} judged, {kinds['merge']} merge(s) "
        f"and {kinds['revert']} revert(s) a tool wrote, {bots} by a bot"
    )
    return [head, *info]


def judge(population, lookups, listed, level):
    """The verdict over a run's population, the listed commits read by their ids, and the listing:
    (lines, refused) (R1, R11, R12). An unlisted site of the population prints
    `WARN commit-message <id> <form> <key>` and is refused only at fail; a listed entry that no site
    of the population or of the listed commits matches prints `STALE commit-message <id> <form>
    <key>` and is refused at both levels. An empty population is VOID."""
    if level not in ("warn", "fail"):
        raise AssertionError(f"the level is warn or fail, not {level}")
    population = examined("commit(s)", population)
    found, known = [], set()
    for commit in population:
        found.extend((commit[0][:12], form, key) for form, key in classify(commit)[1])
    for commit in [*population, *lookups]:
        known |= {(commit[0][:12], form, key) for form, key in classify(commit)[1]}
    names = {(ident, form, key) for ident, form, key, _reason in listed}
    lines, refused = [], []
    for site in found:
        if site not in names:
            line = "WARN commit-message " + " ".join(site)
            lines.append(line)
            if level == "fail":
                refused.append(line)
    for ident, form, key, _reason in listed:
        if (ident, form, key) not in known:
            stale = f"STALE commit-message {ident} {form} {key}"
            lines.append(stale)
            refused.append(stale)
    return lines, refused


def sites_of(name):
    """Each planted commit's id with its kind and its sites, in the fixture's order."""
    return [(commit[0], *classify(commit)[:2]) for commit in planted(name)]


class PlantedSubjects(unittest.TestCase):
    def test_a_house_type_subject_is_clean_and_every_other_subject_is_a_site(self):
        self.assertEqual(
            sites_of("subjects.fixture"),
            [
                ("clean-scope", "judged", []),
                ("clean-bare", "judged", []),
                ("clean-bang", "judged", []),
                ("clean-caps", "judged", []),
                ("clean-rel", "judged", []),
                ("clean-72", "judged", []),
                ("squash", "judged", []),
                ("no-type", "judged", [("no-type", "-")]),
                ("no-desc", "judged", [("no-type", "-")]),
                ("empty-scope", "judged", [("no-type", "-")]),
                ("other-type", "judged", [("other-type", "wip")]),
                ("over-limit", "judged", [("long-subject", "73")]),
            ],
        )


class PlantedTrailers(unittest.TestCase):
    def test_an_attribution_key_in_gits_block_is_a_site_and_every_other_key_is_counted(self):
        commits = planted("trailers.fixture")
        self.assertEqual(
            [(commit[0], *classify(commit)[1:]) for commit in commits],
            [
                ("co-author", [("attribution-trailer", "co-authored-by")], []),
                ("sign-off", [("attribution-trailer", "signed-off-by")], []),
                ("caps-key", [("attribution-trailer", "co-authored-by")], []),
                ("spaced-key", [("attribution-trailer", "signed-off-by")], []),
                ("folded", [("attribution-trailer", "co-authored-by")], []),
                ("quarter", [("attribution-trailer", "signed-off-by")], []),
                ("refs", [], ["Refs"]),
                ("break-dash", [], ["BREAKING-CHANGE"]),
                ("break-space", [], []),
                ("prose", [], []),
                ("middle", [], []),
                ("divider", [], []),
            ],
        )


class PlantedToolSubjects(unittest.TestCase):
    def test_merges_and_reverts_a_tool_wrote_are_counted_and_every_other_subject_is_judged(self):
        self.assertEqual(
            sites_of("tools.fixture"),
            [
                ("gh-merge", "merge", []),
                ("gh-long", "merge", []),
                ("merge-ref", "merge", []),
                ("git-merge", "merge", []),
                ("git-remote", "merge", []),
                ("merge-credit", "merge", [("attribution-trailer", "co-authored-by")]),
                ("git-revert", "revert", []),
                ("git-reapply", "revert", []),
                ("one-parent", "judged", [("no-type", "-")]),
                ("own-merge", "judged", [("no-type", "-")]),
                ("bare-revert", "judged", [("no-type", "-")]),
            ],
        )

    def test_the_read_line_counts_each_kind_and_each_counted_key(self):
        self.assertEqual(
            summary(planted("tools.fixture")),
            [
                "read 11 commit(s): 3 judged, 6 merge(s) and 2 revert(s) a tool wrote, 2 by a bot",
                "INFO commit-message gh-merge trailer ci",
            ],
        )


class PlantedBots(unittest.TestCase):
    def test_a_bots_commit_is_read_by_the_same_forms(self):
        commits = planted("bots.fixture")
        self.assertEqual(
            [(commit[0], commit[2]) for commit in commits],
            [
                ("bot-bump", True),
                ("bot-build", True),
                ("person", False),
            ],
        )
        self.assertEqual(
            sites_of("bots.fixture"),
            [
                (
                    "bot-bump",
                    "judged",
                    [("no-type", "-"), ("attribution-trailer", "signed-off-by")],
                ),
                ("bot-build", "judged", []),
                ("person", "judged", []),
            ],
        )


class PlantedVerdict(unittest.TestCase):
    def test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does(self):
        population = planted("subjects.fixture")
        tools = planted("tools.fixture")
        lookups = [commit for commit in tools if commit[0] in ("own-merge", "git-revert")]
        listed = (
            ("no-type", "no-type", "-", "planted: a listed site of the population"),
            ("own-merge", "no-type", "-", "planted: a listed site read by its id"),
            ("git-revert", "no-type", "-", "planted: a revert git wrote is no site"),
            ("gone", "no-type", "-", "planted: an id the checkout does not hold"),
        )
        warn = [
            "WARN commit-message no-desc no-type -",
            "WARN commit-message empty-scope no-type -",
            "WARN commit-message other-type other-type wip",
            "WARN commit-message over-limit long-subject 73",
        ]
        stale = [
            "STALE commit-message git-revert no-type -",
            "STALE commit-message gone no-type -",
        ]
        self.assertEqual(judge(population, lookups, listed, "warn"), (warn + stale, stale))
        self.assertEqual(judge(population, lookups, listed, "fail"), (warn + stale, warn + stale))
        self.assertEqual(LEVEL, "warn")
        self.assertEqual(LISTED, ())

    def test_an_empty_population_is_void(self):
        with self.assertRaises(AssertionError) as raised:
            judge(planted("empty.fixture"), [], (), "warn")
        self.assertEqual(
            str(raised.exception),
            "examined 0 commit(s): the population is empty, so nothing was judged",
        )
        with self.assertRaises(AssertionError) as raised:
            judge(planted("subjects.fixture"), [], (), "error")
        self.assertEqual(str(raised.exception), "the level is warn or fail, not error")


class PlantedRange(unittest.TestCase):
    def test_the_log_reader_splits_each_record_and_an_unreadable_range_is_void(self):
        text = (
            "\x1eplanted-one\x1fparent-a parent-b\x1fplanted[bot]\x1f"
            "Merge pull request #1 from planted/x\n\nci: a title\n\n"
            "\x1eplanted-two\x1fparent-a\x1fPlanted Person\x1fdocs: name the types\n\n"
        )
        self.assertEqual(
            parse_log(text),
            [
                ("planted-one", 2, True, "Merge pull request #1 from planted/x\n\nci: a title"),
                ("planted-two", 1, False, "docs: name the types"),
            ],
        )
        self.assertEqual(
            range_void("true\n", 0),
            "VOID the checkout's shallow flag reads 'true', so its range is cut",
        )
        self.assertEqual(
            range_void("false\n", 1), "VOID HEAD has no first parent, so the range has no base"
        )
        self.assertIsNone(range_void("false\n", 0))


class TheRunsRange(unittest.TestCase):
    def test_every_commit_of_the_runs_range_is_read_at_the_level_against_the_listing(self):
        _code, head, _error = git("rev-parse", "HEAD")
        population = run_range()
        self.assertIn(head.strip(), [commit[0] for commit in population])
        lines, refused = judge(population, listed_commits(LISTED), LISTED, LEVEL)
        for line in [*summary(population), *lines]:
            print(line)
        self.assertEqual(refused, [])
