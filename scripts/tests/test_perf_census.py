"""The bench census (SPEC-630, ADR-652): A1 to A6.

Each test drives `scripts/perf-census.py` as a child process. `derive` reads the callgrind_annotate
listings of `fixtures/perf-census/`, each copied to a temporary folder with at most one edit.
`check` reads a tree planted in a temporary folder: the two committed files of
`fixtures/perf-census/` with one change, the bench files `fixtures/perf-census/tree.json` holds,
and a throwaway repository whose release merges each take a dev commit as their second parent.
Nothing here reads this repository's history, reaches a network, runs Valgrind or builds.
"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "perf-census.py"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "perf-census"
LISTINGS = ("hot-set.txt", "reach-day-queue.txt", "reach-answer.txt", "untoggled.txt")
REQUIRED = (
    "--listing",
    "--share",
    "--source-root",
    "--workload",
    "--workload-sha256",
    "--derived-at",
    "--release",
    "--toolchain",
    "--valgrind",
    "--hot-set",
    "--bench-reach",
)
RENDER = "#[divan::bench]\nfn face(bencher: Bencher) {\n    bencher.bench(|| face_measured());\n}\n"
SHA256 = "0123456789" * 6 + "0123"
DERIVED = "1234567890" * 4
FACTS = (
    "--share",
    "90",
    "--source-root",
    "/build/deck-streak",
    "--workload",
    "perf/workload.json",
    "--workload-sha256",
    SHA256,
    "--derived-at",
    DERIVED,
    "--release",
    "v0.4.0",
    "--toolchain",
    "1.99.1",
    "--valgrind",
    "3.22.0",
)
GIT = (
    "-c",
    "user.name=fixture",
    "-c",
    "user.email=fixture@example.invalid",
    "-c",
    "commit.gpgsign=false",
    "-c",
    "tag.gpgsign=false",
)
DAY_QUEUE = "deck_streak_engine_core::scheduler::day_queue"
SHOWN = "deck_streak_engine_core::answer::shown_states"
MEMCPY = "__memcpy_avx_unaligned_erms"
MEMCPY_FILE = "./string/../sysdeps/x86_64/multiarch/memmove-vec-unaligned-erms.S"
NEXT = "<deck_streak_fsrs7::replay::Replay as core::iter::traits::iterator::Iterator>::next"
DRIFTSORT = "core::slice::sort::stable::driftsort_main"
SIBLING = "deck_streak_engine_core::scheduler::sibling_rule"
DEFINED = ("grading::answer", "replay::replay", "review::step", "scheduling::day_queue")
UNREACHED = "member %s (%s) is reached by no bench the tree defines"
NEWEST = "OK, derived_at is the dev parent of v0.%d.0, the newest release tag"
NO_EDGE = "OK, no member ends within half a point of the cut"


def run(*args):
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True,
        text=True,
        check=False,
        timeout=120,
    )


def fixture(name):
    return (FIXTURES / name).read_text(encoding="utf-8")


def member(fn, file, self_ir, share):
    return {"fn": fn, "file": file, "self_ir": self_ir, "share": share}


HOT = {
    "schema": "deckstreak.perf-hot-set.v1",
    "share": 90.0,
    "workload": {"path": "perf/workload.json", "sha256": SHA256},
    "derived_at": DERIVED,
    "release": "v0.4.0",
    "toolchain": "1.99.1",
    "valgrind": "3.22.0",
    "total_ir": 3000000,
    "members": [
        member(DAY_QUEUE, "crates/engine-core/src/scheduler.rs", 1200000, 40.0),
        member(SHOWN, "crates/engine-core/src/answer.rs", 800000, 26.6667),
        member(MEMCPY, MEMCPY_FILE, 450000, 15.0),
        member(NEXT, "crates/fsrs7/src/replay.rs", 250000, 8.3333),
    ],
}
REACH = {
    "schema": "deckstreak.perf-bench-reach.v1",
    "derived_at": DERIVED,
    "benches": {
        "grading::answer": {
            "total_ir": 500000,
            "functions": [MEMCPY, "deck_streak_engine_core::answer::Grade::pick", SHOWN],
        },
        "scheduling::day_queue": {"total_ir": 400000, "functions": [DRIFTSORT, DAY_QUEUE]},
    },
    "files": [
        "crates/engine-core/src/scheduler.rs",
        "crates/ffi/src/engine.rs",
        "crates/ingest/tests/support/synthetic.rs",
    ],
}


def derive(edits=(), omit=None):
    """Run `derive` over the four listings, each edit an (listing, old, new) replacement made once,
    or without the option `omit` names; returns the run and the texts of the two documents, None
    for one not written."""
    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp)
        for name in LISTINGS:
            text = fixture(name)
            for where, old, new in edits:
                if where == name and old not in text:
                    raise AssertionError(f"{name} holds no {old!r}")
                text = text.replace(old, new) if where == name else text
            (folder / name).write_text(text, encoding="utf-8")
        args = [
            "derive",
            "--listing",
            str(folder / "hot-set.txt"),
            *FACTS,
            "--hot-set",
            str(folder / "out-hot-set.json"),
            "--bench-reach",
            str(folder / "out-bench-reach.json"),
            "--reach",
            f"scheduling::day_queue={folder / 'reach-day-queue.txt'}",
            "--reach",
            f"grading::answer={folder / 'reach-answer.txt'}",
            "--untoggled",
            str(folder / "untoggled.txt"),
        ]
        if omit:
            place = args.index(omit)
            args = args[:place] + args[place + 2 :]
        done = run(*args)
        written = [folder / "out-hot-set.json", folder / "out-bench-reach.json"]
        texts = [path.read_text(encoding="utf-8") if path.exists() else None for path in written]
        return done, texts[0], texts[1]


def git(root, *args):
    done = subprocess.run(
        ["git", "-C", str(root), *GIT, *args],
        capture_output=True,
        text=True,
        check=True,
        timeout=120,
    )
    return done.stdout.strip()


def history(root, releases):
    """A repository at `root`: dev commits d0 to dn, release merges m1 to mn each tagged v0.k.0 with
    d_k as its second parent, a tag that is no release on d0, and dev's tip one commit after dn.
    Returns the dev commits and the merges, m0 being d0."""
    git(root, "init", "-q", "-b", "dev")
    tree = git(root, "write-tree")
    dev = [git(root, "commit-tree", tree, "-m", "d0")]
    git(root, "tag", "fixture-base", dev[0])
    merges = [dev[0]]
    for k in range(1, releases + 1):
        dev.append(git(root, "commit-tree", tree, "-p", dev[-1], "-m", f"d{k}"))
        merges.append(
            git(root, "commit-tree", tree, "-p", merges[-1], "-p", dev[-1], "-m", f"release {k}")
        )
        git(root, "tag", "-a", f"v0.{k}.0", "-m", f"v0.{k}.0", merges[-1])
    head = git(root, "commit-tree", tree, "-p", dev[-1], "-m", "head")
    git(root, "update-ref", "refs/heads/dev", head)
    return dev, merges, head


def changed(doc, path, value):
    """`doc` with the value at `path`, a sequence of keys and indices, set to `value`."""
    place = doc
    for step in path[:-1]:
        place = place[step]
    place[path[-1]] = value
    return doc


def dropped(doc, path):
    place = doc
    for step in path[:-1]:
        place = place[step]
    del place[path[-1]]
    return doc


def planted(doc):
    """`doc` with a member no bench reaches added last."""
    doc["members"].append(member(SIBLING, "crates/engine-core/src/scheduler.rs", 10000, 1.0))
    return doc


def census(hot=None, reach=None, releases=3, at=None, head_tag=False, tree=None, enforce=True):
    """Plant a tree and run `check` over it. `hot` and `reach` each change the committed file's
    object (derived_at set) into what is written: an object, a text, or None for no file. `at`
    picks derived_at from the history; the default is the newest release's dev parent. Returns the
    run, derived_at and the root."""
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        dev, merges, head = history(root, releases)
        if head_tag:
            git(root, "tag", "-a", f"v0.{releases + 1}.0", "-m", "no merge", head)
        derived = at(root, dev, merges) if at else dev[-1]
        (root / "perf").mkdir()
        for name, change in (("hot-set.json", hot), ("bench-reach.json", reach)):
            doc = changed(json.loads(fixture(name)), ("derived_at",), derived)
            value = change(doc) if change else doc
            if value is not None:
                text = value if isinstance(value, str) else json.dumps(value, indent=2)
                (root / "perf" / name).write_text(text, encoding="utf-8")
        files = json.loads(fixture("tree.json")) if tree is None else tree
        for path, text in files.items():
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_text(text, encoding="utf-8")
        done = run("check", "--root", str(root), *(["--enforce"] if enforce else []))
        return done, derived, str(root)


def lines(*said):
    return "".join(f"perf-census: {line}\n" for line in said)


def report(
    unreached=(),
    stale=NEWEST % 3,
    void=(),
    edge=(),
    members=5,
    defined=DEFINED,
    tags=3,
    refused=0,
):
    """What `check --enforce` prints over a tree whose shape it admits."""
    reached = f"OK, {members} members, each reached by a bench the tree defines"
    return lines(
        "shape: OK",
        f"benches: the tree defines {len(defined)}: {', '.join(defined)}",
        *[f"unreached: {line}" for line in unreached or [reached]],
        f"stale: {stale}",
        *[f"void: {line}" for line in void or ["OK"]],
        *[f"edge: {line}" for line in edge or [NO_EDGE]],
        f"check examined {members} members, {len(defined)} defined benches, {tags} release tags; "
        f"{refused} refused (enforced)",
    )


def shape_refused(*problems):
    return lines(
        *[f"shape: {problem}" for problem in problems],
        "check examined 0 members, 0 defined benches, 0 release tags; "
        f"{len(problems)} refused (enforced)",
    )


class Derive(unittest.TestCase):
    def assert_refused(self, cases):
        for edit, problem in examined("listings derive refuses", cases):
            done, hot, reach = derive(edits=[edit])
            said = done.stdout + done.stderr
            self.assertEqual(done.returncode, 1, said)
            self.assertEqual(done.stdout, f"perf-census: derive: {problem}\n")
            self.assertEqual(done.stderr, "", said)
            self.assertEqual((hot, reach), (None, None), "a refused derive writes nothing")

    def test_derive_writes_both_documents_with_normalized_names(self):
        self.assert_refused(
            [
                (
                    ("hot-set.txt", "callgrind-3.22.0", "callgrind-3.21.0"),
                    "hot-set.txt was not made by callgrind-3.22.0",
                ),
                (
                    ("reach-answer.txt", "Events shown:     Ir", "Events shown:     Ir Dr"),
                    "reach-answer.txt shows events other than Ir",
                ),
                (
                    ("hot-set.txt", "3,000,000 (100.0%)  PROGRAM TOTALS\n", ""),
                    "hot-set.txt has no thresholds line or no program totals line",
                ),
                (
                    ("reach-day-queue.txt", "Ir                 file:function\n", ""),
                    "reach-day-queue.txt has no file:function header",
                ),
                (
                    ("untoggled.txt", "      1 ( 0.00%)  ???:???", "  calls ( 0.00%)  ???:???"),
                    "untoggled.txt: '  calls ( 0.00%)  ???:???' is no function line",
                ),
            ]
        )
        for args in examined("usage errors", [(), ("check",), ("derive", "--share", "90")]):
            done = run(*args)
            self.assertEqual((done.returncode, done.stdout), (2, ""), done.stderr)
        for option in examined("required derive options", REQUIRED):
            done, hot, reach = derive(omit=option)
            self.assertEqual((done.returncode, hot, reach), (2, None, None), option)
        done, hot, reach = derive()
        said = done.stdout + done.stderr
        self.assertEqual(done.returncode, 0, said)
        self.assertEqual(
            done.stdout, "perf-census: derive wrote 4 members at 90%, 2 benches, 3 files\n"
        )
        self.assertEqual(hot, json.dumps(HOT, indent=2) + "\n")
        self.assertEqual(reach, json.dumps(REACH, indent=2) + "\n")

    def test_derive_refuses_a_listing_that_does_not_stop_at_the_share(self):
        last = (
            "  100,000 ( 3.33%)  /build/deck-streak/crates/engine-core/src/answer.rs:"
            f"{SHOWN}::h3333333333333333 [/build/deck-streak/target/release/examples/study_day]\n"
        )
        past = "   10,000 ( 0.33%)  /build/deck-streak/crates/ffi/src/engine.rs:face\n"
        self.assert_refused(
            [
                (
                    ("hot-set.txt", "Thresholds:       90", "Thresholds:       99"),
                    "hot-set.txt was printed at the threshold 99%, not at the share 90%",
                ),
                (("hot-set.txt", last, last + past), "hot-set.txt goes on past the cut at 90%"),
                (("hot-set.txt", last, ""), "hot-set.txt sums short of the share 90%"),
                (
                    ("hot-set.txt", "3,000,000 (100.0%)  PROGRAM TOTALS", "0  PROGRAM TOTALS"),
                    "hot-set.txt counted 0 instructions",
                ),
                (
                    ("reach-day-queue.txt", "Thresholds:       100", "Thresholds:       99"),
                    "reach-day-queue.txt was printed at the threshold 99%, not at 100% "
                    "(every function)",
                ),
                (
                    ("untoggled.txt", "Thresholds:       100", "Thresholds:       99.5"),
                    "untoggled.txt was printed at the threshold 99.5%, not at 100% "
                    "(every function)",
                ),
            ]
        )


class Check(unittest.TestCase):
    def assert_judged(self, done, expected, code):
        said = done.stdout + done.stderr
        self.assertEqual(done.stdout, expected, said)
        self.assertEqual(done.returncode, code, said)

    def test_shape_refuses_an_unknown_or_missing_key_and_a_value_outside_the_schema(self):
        cases = [
            (lambda d: changed(d, ("extra",), 1), None, "hot-set.json has the unknown key 'extra'"),
            (lambda d: dropped(d, ("release",)), None, "hot-set.json has no key 'release'"),
            (None, lambda d: dropped(d, ("files",)), "bench-reach.json has no key 'files'"),
            (
                lambda d: changed(d, ("members", 0, "calls"), 3),
                None,
                "hot-set.json/members/0 has the unknown key 'calls'",
            ),
            (
                None,
                lambda d: dropped(d, ("benches", "replay::replay", "functions")),
                "bench-reach.json/benches/replay::replay has no key 'functions'",
            ),
            (
                lambda d: changed(d, ("schema",), "deckstreak.perf-hot-set.v2"),
                None,
                "hot-set.json/schema 'deckstreak.perf-hot-set.v2' is outside the schema",
            ),
            (
                None,
                lambda d: changed(d, ("schema",), "deckstreak.perf-bench-reach.v2"),
                "bench-reach.json/schema 'deckstreak.perf-bench-reach.v2' is outside the schema",
            ),
            (
                lambda d: changed(d, ("derived_at",), "abc"),
                None,
                "hot-set.json/derived_at 'abc' is outside the schema",
            ),
            (
                lambda d: changed(d, ("workload", "sha256"), "abc"),
                None,
                "hot-set.json/workload/sha256 'abc' is outside the schema",
            ),
            (
                lambda d: changed(d, ("release",), "0.4.0"),
                None,
                "hot-set.json/release '0.4.0' is outside the schema",
            ),
            (
                lambda d: changed(d, ("share",), 0),
                None,
                "hot-set.json/share 0 is outside the schema",
            ),
            (
                lambda d: changed(d, ("share",), 101),
                None,
                "hot-set.json/share 101 is outside the schema",
            ),
            (
                lambda d: changed(d, ("share",), True),
                None,
                "hot-set.json/share True is outside the schema",
            ),
            (
                lambda d: changed(d, ("share",), "90"),
                None,
                "hot-set.json/share '90' is outside the schema",
            ),
            (
                lambda d: changed(d, ("total_ir",), -1),
                None,
                "hot-set.json/total_ir -1 is outside the schema",
            ),
            (
                lambda d: changed(d, ("total_ir",), True),
                None,
                "hot-set.json/total_ir True is outside the schema",
            ),
            (
                lambda d: changed(d, ("members", 0, "self_ir"), 1.5),
                None,
                "hot-set.json/members/0/self_ir 1.5 is outside the schema",
            ),
            (
                lambda d: changed(d, ("toolchain",), ""),
                None,
                "hot-set.json/toolchain '' is outside the schema",
            ),
            (
                lambda d: changed(d, ("valgrind",), 3),
                None,
                "hot-set.json/valgrind 3 is outside the schema",
            ),
            (
                None,
                lambda d: changed(d, ("benches", "grading::answer", "functions"), "x"),
                "bench-reach.json/benches/grading::answer/functions 'x' is outside the schema",
            ),
            (
                None,
                lambda d: changed(d, ("files",), [""]),
                "bench-reach.json/files [''] is outside the schema",
            ),
            (
                None,
                lambda d: changed(d, ("derived_at",), "1" * 40),
                "hot-set.json and bench-reach.json name different derived_at commits",
            ),
            (lambda d: "{", None, "hot-set.json is not JSON"),
            (None, lambda d: None, "bench-reach.json is absent"),
            (lambda d: [], None, "hot-set.json is not an object"),
            (
                lambda d: changed(d, ("workload",), "perf/workload.json"),
                None,
                "hot-set.json/workload is not an object",
            ),
            (lambda d: changed(d, ("members",), {}), None, "hot-set.json/members is not a list"),
            (
                lambda d: changed(d, ("members",), [5]),
                None,
                "hot-set.json/members/0 is not an object",
            ),
            (
                None,
                lambda d: changed(d, ("benches",), []),
                "bench-reach.json/benches is not an object",
            ),
            (
                None,
                lambda d: changed(d, ("benches",), {"x": 5}),
                "bench-reach.json/benches/x is not an object",
            ),
        ]
        for hot, reach, problem in examined("planted shapes", cases):
            done, _derived, _root = census(hot=hot, reach=reach, releases=0)
            self.assert_judged(done, shape_refused(problem), 1)
        done, _derived, _root = census(hot=lambda d: changed(d, ("share",), 1))
        self.assert_judged(done, report(), 0)

    def test_unreached_refuses_a_member_no_bench_reaches(self):
        done, _derived, _root = census(hot=planted)
        unreached = UNREACHED % (SIBLING, "crates/engine-core/src/scheduler.rs")
        self.assert_judged(done, report(unreached=[unreached], members=6, refused=1), 1)
        done, _derived, _root = census()
        self.assert_judged(done, report(), 0)

    def test_unreached_refuses_a_member_whose_bench_was_deleted(self):
        tree = json.loads(fixture("tree.json"))
        del tree["crates/fsrs7/benches/replay.rs"]
        done, _derived, _root = census(tree=tree)
        unreached = UNREACHED % (NEXT, "crates/fsrs7/src/replay.rs") + (
            "; replay::replay reached it, and the tree no longer defines that bench"
        )
        defined = ("grading::answer", "review::step", "scheduling::day_queue")
        expected = report(unreached=[unreached], defined=defined, refused=1)
        self.assert_judged(done, expected, 1)

    def test_stale_refuses_a_derived_at_two_releases_behind(self):
        done, derived, _root = census(at=lambda root, dev, merges: dev[1])
        stale = f"derived_at {derived[:8]} is the dev parent of neither v0.3.0 nor v0.2.0"
        self.assert_judged(done, report(stale=stale, refused=1), 1)
        done, derived, _root = census(at=lambda root, dev, merges: dev[2])
        stale = "OK, derived_at is the dev parent of v0.2.0, the release tag before the newest"
        self.assert_judged(done, report(stale=stale), 0)

    def test_stale_refuses_a_merge_ref_derived_at(self):
        def merge_ref(root, dev, merges):
            tree = git(root, "write-tree")
            return git(root, "commit-tree", tree, "-p", merges[2], "-p", dev[3], "-m", "merge ref")

        done, derived, _root = census(at=merge_ref)
        stale = f"derived_at {derived[:8]} is not an ancestor of HEAD"
        self.assert_judged(done, report(stale=stale, refused=1), 1)
        done, derived, _root = census(at=lambda root, dev, merges: "1" * 40)
        stale = f"derived_at {derived[:8]} names no commit of the repository"
        self.assert_judged(done, report(stale=stale, refused=1), 1)

    def test_stale_reads_void_on_a_tag_with_no_second_parent_and_judges_one_tag_alone(self):
        done, _derived, _root = census(head_tag=True)
        stale = "VOID v0.4.0 has no second parent, and every release merges with a merge commit"
        self.assert_judged(done, report(stale=stale, tags=4, refused=1), 1)
        done, derived, _root = census(releases=1, at=lambda root, dev, merges: dev[0])
        stale = f"derived_at {derived[:8]} is not the dev parent of v0.1.0, the one release tag"
        self.assert_judged(done, report(stale=stale, tags=1, refused=1), 1)
        done, _derived, _root = census(releases=0)
        stale = "VOID the repository holds no SemVer release tag"
        self.assert_judged(done, report(stale=stale, tags=0, refused=1), 1)
        done, _derived, _root = census(releases=1)
        self.assert_judged(done, report(stale=NEWEST % 1, tags=1), 0)

    def test_void_refuses_an_empty_hot_set(self):
        done, _derived, _root = census(hot=lambda d: changed(d, ("members",), []))
        expected = report(void=["the hot set has no member"], members=0, refused=1)
        self.assert_judged(done, expected, 1)
        done, _derived, _root = census(hot=lambda d: changed(d, ("total_ir",), 0))
        expected = report(void=["the hot set counted 0 instructions (total_ir 0)"], refused=1)
        self.assert_judged(done, expected, 1)

    def test_void_refuses_a_bench_with_no_instructions_inside_its_toggle(self):
        def emptied(doc):
            changed(doc, ("benches", "render::face"), {"total_ir": 0, "functions": []})
            return changed(doc, ("benches", "scheduling::day_queue", "total_ir"), 0)

        tree = json.loads(fixture("tree.json"))
        tree["crates/ffi/benches/render.rs"] = RENDER
        done, _derived, _root = census(reach=emptied, tree=tree)
        void = [
            "bench render::face counted 0 instructions inside its toggle",
            "bench scheduling::day_queue counted 0 instructions inside its toggle",
        ]
        defined = (DEFINED[0], "render::face", *DEFINED[1:])
        self.assert_judged(done, report(void=void, defined=defined, refused=2), 1)

    def test_edge_prints_a_member_near_the_cut_and_never_refuses(self):
        def near(doc):
            changed(doc, ("members", 3, "self_ir"), 96000)
            return changed(doc, ("members", 4, "self_ir"), 6000)

        done, _derived, _root = census(hot=near)
        edge = [
            f"member 4 of 5, {NEXT}, ends at 89.60% of the instructions, 0.40 from the cut at 90%",
            f"member 5 of 5, {DRIFTSORT}, ends at 90.20% of the instructions, 0.20 from the cut "
            "at 90%",
        ]
        self.assert_judged(done, report(edge=edge), 0)

    def test_stale_admits_a_derived_at_one_release_behind(self):
        done, _derived, _root = census(at=lambda root, dev, merges: dev[2])
        said = done.stdout + done.stderr
        self.assertEqual(done.returncode, 0, said)
        self.assertTrue(done.stdout.splitlines()[-1].endswith("; 0 refused (enforced)"), said)

    def test_report_only_prints_and_exits_0(self):
        done, _derived, _root = census(
            hot=planted,
            reach=lambda d: changed(d, ("benches", "replay::replay", "total_ir"), 0),
            at=lambda root, dev, merges: dev[1],
            enforce=False,
        )
        said = done.stdout + done.stderr
        self.assertEqual(done.returncode, 0, said)
        last = done.stdout.splitlines()[-1]
        self.assertTrue(last.startswith("perf-census: check examined "), said)
        self.assertTrue(last.endswith(" refused (report-only)"), said)


if __name__ == "__main__":
    unittest.main()
