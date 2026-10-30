"""SPEC-496: the formal ledger names every engine property, its artifact, its tie and its gaps.

`formal/obligations.json` is the ledger, and `_formal_ledger` its checker (ADR-514). A1 judges the
committed ledger: its schema, its obligations and planned entries, and its phase sets. A2 to A5
plant one fault at a time in a ledger the checker accepts, over a small tree the class builds in a
temporary folder, and read the one arm that refuses it; each then judges the committed ledger over
the repository's own tree. A6 runs the census, with planted names. Every enumerating test prints
`examined N` and refuses zero. The tests read files only and start no process.
"""

import copy
import hashlib
import json
import re
import tempfile
import unittest
from pathlib import Path

import _formal_ledger as ledger
from _support import REPO, examined

LEDGER = REPO / "formal" / "obligations.json"
IDS = ["O1a"] + [f"O{n}" for n in range(1, 18)]
PHASES = {
    "1": ["O1a", "O4", "O5", "O6"],
    "2": ["O1", "O2", "O3", "O4", "O5", "O6", "O7", "O8", "O9", "O11"],
}
# The planned entries, each with the obligations that name it.
PLANNED_ENTRIES = {
    "tla/ScheduledAnswer": ("O1a", "O6"),
    "tla/CollectionStore": ("O12",),
    "tla/MediaSync": ("O13",),
    "tla/RelayEnvelope": ("O7",),
    "tla/AnkiCutover": ("O8",),
    "lean/ScheduledState": ("O6",),
    "lean/EffectiveLog": ("O12",),
    "lean/QueueRule": ("O12",),
    "lean/AnkiDay": ("O12",),
    "lean/SearchCompile": ("O14",),
    "lean/ClozeFront": ("O15",),
    "lean/Allowlist": ("O15",),
    "lean/EditUndo": ("O15",),
    "lean/ImportMerge": ("O8",),
    "lean/ImportPlan": ("O8",),
    "lean/AnkiRoundTrip": ("O9",),
    "lean/EnvelopeLayout": ("O7",),
}
# The sync entries that land ahead of the sync core (SPEC-504), each with the obligations that name
# it; each such obligation lists every name the entry's header declares, present.
SYNC_ENTRIES = {
    "tla/OpLog": ("O1", "O2", "O3", "O11"),
    "lean/SyncClock": ("O1", "O16"),
    "lean/ReviewKey": ("O1", "O16"),
    "lean/FieldMerge": ("O1", "O16"),
    "lean/PushJudge": ("O1", "O16"),
}
# The planned Kani harnesses of this repository's own code, each in the obligation of its layer.
HARNESSES = {
    "K-decide": "O6",
    "K-companion-codec": "O6",
    "K-envelope": "O7",
    "K-import-key": "O8",
    "K-plan-merge": "O8",
    "K-wire-walk": "O8",
    "K-field-split": "O8",
    "K-due-conversion": "O8",
    "K-member-names": "O8",
    "K-next-id": "O12",
    "K-effective-log": "O12",
    "K-queue": "O12",
    "K-anki-day": "O12",
    "K-judges": "O12",
    "K-blob-path": "O13",
    "K-bind": "O13",
    "K-sweep": "O13",
    "K-evictable": "O13",
    "K-scan": "O13",
    "K-put": "O13",
    "K-compile-binds": "O14",
    "K-operator-profile": "O14",
    "K-template": "O15",
    "K-render-total": "O15",
    "K-cloze-front": "O15",
    "K-allowlist": "O15",
    "K-sync-codec": "O16",
    "K-key-order": "O16",
    "K-hlc": "O16",
}
# The artifacts of the scheduling companion's own tree, counted per obligation.
COMPANIONS = {"O4": 2, "O5": 2, "O10": 2}
SYNC_MODELS = ("tla/FullSyncChoice", "tla/MinimumClientHandshake", "tla/SyncCredential")
ENGINE_TIED = SYNC_MODELS + (
    "tla/SyncSnapshotWindow",
    "tla/UndoOwnAnswer",
    "tla/SkipDayWrite",
    "tla/AwardOnce",
    "tla/FoldSettlesOnce",
    "tla/HeldFlush",
    "tla/LandmarkOnce",
    "tla/RelightOrder",
    "tla/SensitiveDeckGate",
    "tla/SkipDayOnce",
    "lean/ReplayHistory",
    "lean/SensitiveDeck",
)
REPLAY_TIE = {
    "method": "vectors",
    "for": "lean/ReplayHistory",
    "test": "crates/fsrs7/tests/formal_vectors_replay_history.rs"
    "::the_history_selection_matches_the_lean_vectors",
}
# The release gates' entry (SPEC-502): present, and tied by the release lane's Python vectors test.
RELEASE_GATES = "lean/ReleaseGates"
RELEASE_GATES_TIE = {
    "method": "vectors",
    "for": RELEASE_GATES,
    "test": "scripts/tests/test_release_gates.py"
    "::TheLeanVectors.test_every_vector_is_the_scripts_verdict",
}
# SPEC-501: the scheduling companion's decision, the first planned entry built, present under O6
# with its vectors tie.
COMPANION_ENTRIES = ("lean/CompanionDecide",)
# SPEC-514: the skip wrap's holds, present under O14 beside its still planned entry and harnesses.
SEARCH_ENTRIES = ("lean/SearchHolds",)
COMPANION_TIE = {
    "method": "vectors",
    "for": "lean/CompanionDecide",
    "test": "crates/companion/tests/formal_vectors_decide.rs"
    "::decide_and_negotiate_answer_every_lean_vector",
}
# SPEC-513: the media core's bind machine and sweep decision, present under O13 beside the media
# sync model and the store's two harnesses, still planned; each with its vectors tie.
MEDIA_ENTRIES = ("lean/MediaBind", "lean/MediaGc")
MEDIA_TIES = [
    {
        "method": "vectors",
        "for": "lean/MediaBind",
        "test": "crates/media/tests/formal_vectors_bind.rs"
        "::step_and_release_step_answer_every_lean_vector",
    },
    {
        "method": "vectors",
        "for": "lean/MediaGc",
        "test": "crates/media/tests/formal_vectors_gc.rs::fate_answers_every_lean_vector",
    },
]
# SPEC-509: the sync core's id allocator, present under O12 with its vectors tie, and the three
# sync entries it stamps, each tied under O16 by the crate's test that reads its vectors.
ID_SLOT_ENTRIES = ("lean/IdSlot",)
ID_SLOT_TIE = {
    "method": "vectors",
    "for": "lean/IdSlot",
    "test": "crates/sync/tests/formal_vectors_id_slot.rs"
    "::next_id_answers_every_lean_id_slot_vector_byte_equal",
}
SYNC_CORE_TIES = [
    {
        "method": "vectors",
        "for": "lean/SyncClock",
        "test": "crates/sync/tests/formal_vectors_sync_clock.rs"
        "::tick_and_recv_answer_every_lean_clock_vector_byte_equal",
    },
    {
        "method": "vectors",
        "for": "lean/ReviewKey",
        "test": "crates/sync/tests/formal_vectors_review_key.rs"
        "::sort_keys_orders_every_lean_review_key_vector_byte_equal",
    },
    {
        "method": "vectors",
        "for": "lean/FieldMerge",
        "test": "crates/sync/tests/formal_vectors_field_merge.rs"
        "::apply_merges_every_lean_field_merge_vector_byte_equal",
    },
]
# The entries built after the 44 were dispositioned, each with its disposition and the
# obligations that name it.
NEW_ENTRIES = {
    "lean/CompanionDecide": ("keep", {"O6"}),
    "lean/SearchHolds": ("keep", {"O14"}),
    "lean/MediaBind": ("keep", {"O13"}),
    "lean/MediaGc": ("keep", {"O13"}),
    "lean/IdSlot": ("keep", {"O12"}),
}
# The 49 dispositioned entries: the 43 that predate the sync entries, those five, and the
# release gates' entry.
DISPOSITIONS = {
    "retire": ("tla/FullSyncChoice", "tla/SyncSnapshotWindow"),
    "re-point": (
        "tla/MinimumClientHandshake",
        "tla/SyncCredential",
        "tla/UndoOwnAnswer",
        "tla/SkipDayWrite",
    ),
    "re-stamp": (
        "tla/AwardOnce",
        "tla/FoldSettlesOnce",
        "tla/HeldFlush",
        "tla/LandmarkOnce",
        "tla/RelightOrder",
        "tla/SensitiveDeckGate",
        "tla/SkipDayOnce",
        "lean/SensitiveDeck",
    )
    + tuple(SYNC_ENTRIES),
    "keep": tuple(
        f"tla/{name}"
        for name in (
            "BandUpOnce",
            "BearerGuard",
            "CaptureOnce",
            "ChestOpenedOnce",
            "ChestRolledOnce",
            "EveryLegCounted",
            "HabitXpFollowsItsLog",
            "MintReadsTheFinalBase",
            "PasskeyOnce",
            "SecondRoute",
            "StagedNoJournal",
            "TokenWindow",
            "TracedSetCovers",
            "WalletFloor",
        )
    )
    + tuple(
        f"lean/{name}"
        for name in (
            "Chest",
            "Exchange",
            "HabitWriting",
            "NextMilestone",
            "OpenLapse",
            "RecordClassBinding",
            "ReleaseCarry",
            "ReleaseGates",
            "ReplayHistory",
            "RoadToC2",
            "SkipTariff",
            "TableCensusReach",
            "TokenBonus",
            "TokenProgress",
            "Wallet",
            "XpReconciliation",
        )
    ),
}

# The census, read independently of the checker: the brand is assembled from parts, so this file
# holds no token derived from it, and a planted name's digest is computed here from the salt.
BRAND = "".join(("r", "e", "x"))
SALT = "formal-ledger-census:"
PLANTED_NAME = "PlantedCoreName"
HEX64 = re.compile(r"^[0-9a-f]{64}$")

# The small tree every planted fault is judged over. Two TLA+ entries, the first bound by a covers
# stamp; a Lean entry beside a vectors module that declares nothing; a Rust test file with one test,
# one commented-out test, one test inside a block comment and one plain function; a Rust source
# with one Kani harness and one plain function; and a Python test module with a test method, a
# helper method and a top-level function named as a test. Every text is synthetic.
FIXTURE = {
    "formal/tla/Alpha/Alpha.tla": (
        "---- MODULE Alpha ----\n"
        "\\* @phx covers crates/demo/src/lib.rs anchor=run digest=sha256:" + "0" * 64 + "\n"
        "\\* @phx property Holds ramp=report\n"
        "\\* @phx witness witness/a-run-that-breaks-it.cfg kills=Holds\n"
        "====\n"
    ),
    "formal/tla/Beta/Beta.tla": (
        "---- MODULE Beta ----\n\\* @phx property Keeps ramp=report\n====\n"
    ),
    "formal/lean/Formal/Gamma.lean": (
        "-- @phx vectors formal/vectors/gamma.jsonl\n"
        "-- @phx theorem gamma_holds ramp=report\n"
        "theorem gamma_holds : True := trivial\n"
    ),
    "formal/lean/Formal/GammaVectors.lean": "def main : IO Unit := pure ()\n",
    "crates/demo/tests/formal_vectors_gamma.rs": (
        "#[test]\nfn the_gamma_vectors_hold() {}\n\n"
        "// #[test]\n// fn a_commented_out_test() {}\n\n"
        "/* #[test]\nfn a_test_in_a_block_comment() {} */\n\n"
        "fn a_plain_function() {}\n"
    ),
    "crates/demo/src/lib.rs": (
        "pub fn run() {}\n\n"
        "#[cfg(kani)]\n#[kani::proof]\nfn run_never_panics() {\n    run();\n}\n\n"
        "fn a_plain_helper() {}\n"
    ),
    "scripts/tests/test_demo.py": (
        "import unittest\n\n\nclass Demo(unittest.TestCase):\n"
        "    def test_runs(self):\n        pass\n\n    def helper(self):\n        pass\n\n\n"
        "def test_at_module_level():\n    pass\n"
    ),
}
GOOD_RS = "crates/demo/tests/formal_vectors_gamma.rs::the_gamma_vectors_hold"


def accepted():
    """A ledger the checker accepts over the fixture: a planned obligation with a planned entry, a
    planned harness and a companion artifact; and an obligation at `tied` whose every artifact the
    tree declares and a counting tie binds, with one census tie that counts for nothing."""
    return {
        "schema": "formal-obligations.v1",
        "phases": {"1": ["O1"], "2": ["O1", "O2"]},
        "obligations": [
            {
                "id": "O1",
                "property": "a planned property",
                "layers": ["sync"],
                "artifacts": [
                    {"kind": "tla", "entry": "tla/Planned", "state": "planned"},
                    {"kind": "kani", "harness": "K-planned", "state": "planned"},
                    {
                        "kind": "lean",
                        "tree": "companion",
                        "about": "a proof in the companion's own tree",
                        "state": "planned",
                    },
                ],
                "ties": [],
                "gaps": ["holds at the model's bounds"],
                "status": "planned",
            },
            {
                "id": "O2",
                "property": "a tied property",
                "layers": ["storage"],
                "artifacts": [
                    {"kind": "tla", "entry": "tla/Alpha", "property": "Holds", "state": "present"},
                    {
                        "kind": "lean",
                        "entry": "lean/Gamma",
                        "property": "gamma_holds",
                        "state": "present",
                    },
                    {
                        "kind": "kani",
                        "harness": "K-run",
                        "at": "crates/demo/src/lib.rs::run_never_panics",
                        "state": "present",
                    },
                ],
                "ties": [
                    {
                        "method": "mbt",
                        "for": "tla/Alpha",
                        "test": "scripts/tests/test_demo.py::Demo.test_runs",
                        "mutant": "a run that skips its check",
                    },
                    {"method": "vectors", "for": "lean/Gamma", "test": GOOD_RS},
                    {
                        "method": "kani-identity",
                        "for": "K-run",
                        "test": "crates/demo/src/lib.rs::run_never_panics",
                    },
                    {"method": "census", "for": "tla/Alpha", "test": GOOD_RS},
                ],
                "gaps": ["the filesystem is trusted"],
                "status": "tied",
            },
        ],
        "entries": [
            {"entry": "tla/Alpha", "disposition": "keep", "act": "kept", "obligations": ["O2"]},
            {"entry": "tla/Beta", "disposition": "retire", "act": "retired", "obligations": []},
            {
                "entry": "lean/Gamma",
                "disposition": "re-stamp",
                "act": "re-stamped when its span moves",
                "obligations": ["O2"],
            },
        ],
    }


def obligation(document, identity):
    return next(item for item in document["obligations"] if item["id"] == identity)


def row(document, entry):
    return next(item for item in document["entries"] if item["entry"] == entry)


def tie_to(document, identity, test):
    """Appends to obligation `identity` a vectors tie for its planned entry, naming `test`."""
    obligation(document, identity)["ties"].append(
        {"method": "vectors", "for": "tla/Planned", "test": test}
    )


def drop_ties(document, bound, methods):
    """Drops the ties of O2 for `bound` whose method is one of `methods`."""
    ties = obligation(document, "O2")["ties"]
    ties[:] = [t for t in ties if not (t["for"] == bound and t["method"] in methods)]


def add_artifact(document, identity, artifact):
    obligation(document, identity)["artifacts"].append(artifact)


def present(entry, prop):
    kind = entry.split("/")[0]
    return {"kind": kind, "entry": entry, "property": prop, "state": "present"}


def kani_at(harness, at=None):
    artifact = {"kind": "kani", "harness": harness, "state": "present"}
    if at is not None:
        artifact["at"] = at
    return artifact


class Instead:
    """What a planted change gives when its fault is the ledger's own type: the whole ledger."""

    def __init__(self, document):
        self.document = document


def planted(change):
    """An accepted ledger with `change` planted in it, or the ledger the change gives instead."""
    document = accepted()
    given = change(document)
    return given.document if isinstance(given, Instead) else document


# Each planted fault: its label, the one arm that must refuse it, and the change to an accepted
# ledger that plants it.
SCHEMA_PLANTS = (
    ("another schema", "schema", lambda d: d.update(schema="formal-obligations.v2")),
    ("an unknown top-level key", "shape", lambda d: d.update(notes=[])),
    ("no phase sets", "shape", lambda d: d.pop("phases")),
    ("a third phase set", "shape", lambda d: d["phases"].update({"3": ["O1"]})),
    ("an unknown status", "status", lambda d: obligation(d, "O1").update(status="done")),
    (
        "an id that is no obligation id",
        "shape",
        lambda d: d["obligations"].append(dict(copy.deepcopy(obligation(d, "O1")), id="X1")),
    ),
    (
        "an obligation named twice",
        "duplicate-id",
        lambda d: d["obligations"].append(copy.deepcopy(obligation(d, "O1"))),
    ),
    ("an unknown obligation key", "shape", lambda d: obligation(d, "O1").update(owner="x")),
    ("an unnamed property", "shape", lambda d: obligation(d, "O1").update(property=" ")),
    ("no layer", "shape", lambda d: obligation(d, "O1").update(layers=[])),
    ("a layer named twice", "shape", lambda d: obligation(d, "O1").update(layers=["a", "a"])),
    ("ties that are no list", "shape", lambda d: obligation(d, "O1").update(ties={})),
    (
        "an unknown artifact kind",
        "shape",
        lambda d: obligation(d, "O1")["artifacts"][0].update(kind="coq"),
    ),
    (
        "an unknown artifact state",
        "shape",
        lambda d: obligation(d, "O1")["artifacts"][0].update(state="built"),
    ),
    (
        "an entry of another kind",
        "shape",
        lambda d: obligation(d, "O1")["artifacts"][0].update(entry="lean/Planned"),
    ),
    (
        "a harness with no K- name",
        "shape",
        lambda d: obligation(d, "O1")["artifacts"][1].update(harness="decide"),
    ),
    (
        "a companion artifact naming an entry",
        "shape",
        lambda d: obligation(d, "O1")["artifacts"][2].update(entry="tla/Planned"),
    ),
    (
        "an artifact of another tree",
        "shape",
        lambda d: obligation(d, "O1")["artifacts"][2].update(tree="elsewhere"),
    ),
    ("an unknown tie key", "shape", lambda d: obligation(d, "O2")["ties"][1].update(weight=1)),
    (
        "an unknown row disposition",
        "shape",
        lambda d: row(d, "tla/Beta").update(disposition="archive"),
    ),
    # Each shape below has one refusal of its own in the checker, and its plant reaches only that.
    ("a ledger that is not an object", "shape", lambda d: Instead([d])),
    ("an obligation that is not an object", "shape", lambda d: d["obligations"].append("O3")),
    (
        "an artifact that is not an object",
        "shape",
        lambda d: obligation(d, "O1")["artifacts"].append("tla/Planned"),
    ),
    (
        "a planned harness whose at is not a path",
        "shape",
        lambda d: obligation(d, "O1")["artifacts"][1].update(at=7),
    ),
    (
        "a present artifact whose property is not a name",
        "shape",
        lambda d: obligation(d, "O2")["artifacts"][0].update(property="Holds always"),
    ),
    (
        "a present artifact that names no property",
        "shape",
        lambda d: obligation(d, "O2")["artifacts"][0].pop("property"),
    ),
    (
        "a tie that is not an object",
        "shape",
        lambda d: obligation(d, "O1")["ties"].append("vectors"),
    ),
    (
        "a tie whose mutant is blank",
        "shape",
        lambda d: obligation(d, "O2")["ties"][1].update(mutant=" "),
    ),
    (
        "phase sets that are not an object",
        "shape",
        lambda d: d.update(phases=[["O1"], ["O1", "O2"]]),
    ),
    (
        "a phase set that is not a list of ids",
        "shape",
        lambda d: d["phases"].update({"1": "O1"}),
    ),
    ("a row that is not an object", "shape", lambda d: d["entries"].append("tla/Alpha")),
    (
        "a row whose entry is not <kind>/<Name>",
        "shape",
        lambda d: d["entries"].append(
            {"entry": "Alpha", "disposition": "keep", "act": "kept", "obligations": []}
        ),
    ),
    (
        "a row naming an obligation twice",
        "shape",
        lambda d: row(d, "tla/Alpha").update(obligations=["O2", "O2"]),
    ),
)
PHASE_PLANTS = (
    ("a phase set naming no obligation", "phase", lambda d: d["phases"]["2"].append("O9")),
    ("a phase set naming an id twice", "phase", lambda d: d["phases"]["1"].append("O1")),
)
NO_ARTIFACT_PLANTS = (
    (
        "a planned obligation with no artifact",
        "no-artifact",
        lambda d: obligation(d, "O1").update(artifacts=[]),
    ),
)
NO_GAP_PLANTS = (
    ("no gap line", "no-gap-line", lambda d: obligation(d, "O1").update(gaps=[])),
    ("a blank gap line", "no-gap-line", lambda d: obligation(d, "O2").update(gaps=["   "])),
    (
        "an empty gap line beside a real one",
        "no-gap-line",
        lambda d: obligation(d, "O1").update(gaps=["holds at its bounds", ""]),
    ),
    ("a gap that is no text", "no-gap-line", lambda d: obligation(d, "O2").update(gaps=[3])),
)
UNDECLARED_PLANTS = (
    (
        "a property another entry declares",
        "undeclared-property",
        lambda d: obligation(d, "O2")["artifacts"][0].update(property="Keeps"),
    ),
    (
        "a theorem the entry does not declare",
        "undeclared-property",
        lambda d: obligation(d, "O2")["artifacts"][1].update(property="gamma_fails"),
    ),
    (
        "an entry the tree lacks",
        "undeclared-entry",
        lambda d: add_artifact(d, "O1", present("tla/Missing", "Holds")),
    ),
    (
        "a Lean module that declares no theorem",
        "undeclared-entry",
        lambda d: add_artifact(d, "O1", present("lean/GammaVectors", "gamma_holds")),
    ),
    (
        "a harness at a plain function",
        "undeclared-harness",
        lambda d: add_artifact(
            d, "O1", kani_at("K-helper", "crates/demo/src/lib.rs::a_plain_helper")
        ),
    ),
    (
        "a harness at no function",
        "undeclared-harness",
        lambda d: add_artifact(d, "O1", kani_at("K-missing", "crates/demo/src/lib.rs::no_harness")),
    ),
    (
        "a harness at a test, not a proof",
        "undeclared-harness",
        lambda d: add_artifact(d, "O1", kani_at("K-test", GOOD_RS)),
    ),
    (
        "a present harness with no at",
        "undeclared-harness",
        lambda d: add_artifact(d, "O1", kani_at("K-nowhere")),
    ),
    (
        "a planned artifact under an obligation at artifact",
        "planned-past-planned",
        lambda d: (
            obligation(d, "O2").update(status="artifact"),
            add_artifact(d, "O2", {"kind": "lean", "entry": "lean/Later", "state": "planned"}),
        ),
    ),
)
UNDEFINED_TESTS = (
    "crates/demo/tests/formal_vectors_gamma.rs::no_such_test",
    "crates/demo/tests/formal_vectors_gamma.rs::a_commented_out_test",
    "crates/demo/tests/formal_vectors_gamma.rs::a_test_in_a_block_comment",
    "crates/demo/tests/formal_vectors_gamma.rs::a_plain_function",
    "crates/demo/tests/no_such_file.rs::the_gamma_vectors_hold",
    "crates/demo/../demo/tests/formal_vectors_gamma.rs::the_gamma_vectors_hold",
    "/crates/demo/tests/formal_vectors_gamma.rs::the_gamma_vectors_hold",
    "crates/demo/tests/formal_vectors_gamma.rs",
    "scripts/tests/test_demo.py::Demo.helper",
    "scripts/tests/test_demo.py::Missing.test_runs",
    "scripts/tests/test_demo.py::test_runs",
)
TIE_PLANTS = tuple(
    (f"a tie naming {test}", "undefined-test", lambda d, test=test: tie_to(d, "O1", test))
    for test in UNDEFINED_TESTS
) + (
    (
        "a tie naming a top-level test function, not <Class>.<method>",
        "undefined-test",
        lambda d: tie_to(d, "O1", "scripts/tests/test_demo.py::test_at_module_level"),
    ),
    (
        "a tie whose method is a covers stamp",
        "tie-method",
        lambda d: obligation(d, "O1")["ties"].append(
            {"method": "covers", "for": "tla/Planned", "test": GOOD_RS}
        ),
    ),
    (
        "a tie with no method",
        "tie-method",
        lambda d: obligation(d, "O1")["ties"].append({"for": "tla/Planned", "test": GOOD_RS}),
    ),
    (
        "a tie with no for",
        "tie-without-for",
        lambda d: obligation(d, "O1")["ties"].append({"method": "vectors", "test": GOOD_RS}),
    ),
    (
        "a tie with a blank for",
        "tie-without-for",
        lambda d: obligation(d, "O1")["ties"].append(
            {"method": "vectors", "for": "", "test": GOOD_RS}
        ),
    ),
    (
        "a tie for another obligation's entry",
        "tie-for-unknown",
        lambda d: obligation(d, "O1")["ties"].append(
            {"method": "vectors", "for": "tla/Alpha", "test": GOOD_RS}
        ),
    ),
    (
        "a tie for another obligation's harness",
        "tie-for-unknown",
        lambda d: obligation(d, "O1")["ties"].append(
            {"method": "kani-identity", "for": "K-run", "test": GOOD_RS}
        ),
    ),
    (
        "an obligation tied only by a covers stamp",
        "untied-artifact",
        lambda d: drop_ties(d, "tla/Alpha", ("mbt", "census")),
    ),
    (
        "an artifact tied only by a census",
        "untied-artifact",
        lambda d: drop_ties(d, "tla/Alpha", ("mbt",)),
    ),
    (
        "an mbt tie that names no mutant",
        "untied-artifact",
        lambda d: obligation(d, "O2")["ties"][0].pop("mutant"),
    ),
    (
        "an artifact tied only by a cross-target hash",
        "untied-artifact",
        lambda d: obligation(d, "O2")["ties"][0].update(method="cross-target-hash"),
    ),
    (
        "a complete obligation tied only by a covers stamp",
        "untied-artifact",
        lambda d: (
            obligation(d, "O2").update(status="complete"),
            drop_ties(d, "tla/Alpha", ("mbt", "census")),
        ),
    ),
    (
        "a companion artifact under a tied obligation",
        "untied-artifact",
        lambda d: add_artifact(
            d, "O2", dict(copy.deepcopy(obligation(d, "O1")["artifacts"][2]), state="present")
        ),
    ),
)
ROW_PLANTS = (
    (
        "an entry of the tree with no row",
        "entry-without-row",
        lambda d: d["entries"].remove(row(d, "tla/Beta")),
    ),
    (
        "a row for an entry the tree lacks",
        "row-without-entry",
        lambda d: d["entries"].append(
            {"entry": "tla/Missing", "disposition": "keep", "act": "kept", "obligations": []}
        ),
    ),
    (
        "an entry with two rows",
        "duplicate-row",
        lambda d: d["entries"].append(copy.deepcopy(row(d, "tla/Alpha"))),
    ),
    (
        "a row missing an obligation that names it",
        "row-obligations",
        lambda d: row(d, "tla/Alpha").update(obligations=[]),
    ),
    (
        "a row naming an obligation that does not name it",
        "row-obligations",
        lambda d: row(d, "tla/Beta").update(obligations=["O1"]),
    ),
    (
        "a row naming one obligation too many",
        "row-obligations",
        lambda d: row(d, "lean/Gamma").update(obligations=["O2", "O1"]),
    ),
)


def planted_text(text_change):
    """The accepted ledger's text after `text_change` edits one of its strings."""
    document = accepted()
    text_change(document)
    return json.dumps(document, indent=2) + "\n"


CENSUS_PLANTS = (
    (
        "the brand in a gap line",
        "private-brand",
        lambda d: obligation(d, "O1")["gaps"].append(f"the {BRAND} core is trusted"),
    ),
    (
        "the brand in capitals",
        "private-brand",
        lambda d: obligation(d, "O1").update(property=f"{BRAND.upper()} schedules"),
    ),
    (
        "the brand at a camel hump",
        "private-brand",
        lambda d: obligation(d, "O1")["artifacts"][2].update(
            about=f"Planted{BRAND.capitalize()}Name, in its own tree"
        ),
    ),
    (
        "a token the brand starts",
        "private-brand",
        lambda d: obligation(d, "O1")["gaps"].append(f"{BRAND}-core is trusted"),
    ),
    (
        "a listed name in a gap line",
        "private-name",
        lambda d: obligation(d, "O1")["gaps"].append(f"the {PLANTED_NAME} is trusted"),
    ),
    (
        "a listed name as a part of a token",
        "private-name",
        lambda d: obligation(d, "O2").update(property=f"x-{PLANTED_NAME}:y holds"),
    ),
    (
        "a listed name in lower case",
        "private-name",
        lambda d: row(d, "tla/Beta").update(act=f"retired with {PLANTED_NAME.lower()}"),
    ),
)


def load():
    """The committed ledger, or an assertion that names the missing or unreadable file."""
    if not LEDGER.is_file():
        raise AssertionError("formal/obligations.json is absent: the ledger names no obligation")
    try:
        return json.loads(LEDGER.read_text(encoding="utf-8"))
    except ValueError as error:
        raise AssertionError(f"formal/obligations.json is not JSON: {error}") from None


def header_names(root):
    """Every formal entry of the tree with its declared names, read here apart from the checker."""
    found = {}
    for module in sorted((root / "formal" / "tla").glob("*/*.tla")):
        if module.stem == module.parent.name:
            text = module.read_text(encoding="utf-8")
            found[f"tla/{module.stem}"] = set(re.findall(r"(?m)^\\\* @phx property (\S+)", text))
    for module in sorted((root / "formal" / "lean" / "Formal").glob("*.lean")):
        text = module.read_text(encoding="utf-8")
        theorems = set(re.findall(r"(?m)^-- @phx theorem (\S+)", text))
        if theorems:
            found[f"lean/{module.stem}"] = theorems
    return found


class TheFormalLedger(unittest.TestCase):
    @staticmethod
    def setUpClass():
        # The fixture tree, built once for the class. unittest calls this on the class, and a
        # static method is a name the workflow files' read census places; a class method is not.
        cls = TheFormalLedger
        scratch = tempfile.TemporaryDirectory()
        cls.addClassCleanup(scratch.cleanup)
        cls.tree = Path(scratch.name)
        for relative, text in FIXTURE.items():
            path = cls.tree / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")

    def assert_accepted(self, document):
        self.assertEqual(ledger.check(document, self.tree), [], "the accepted ledger is refused")

    def assert_planted(self, plants):
        """Each planted fault is refused by its one arm, and by no other."""
        for label, arm, change in examined("planted faults", plants):
            document = planted(change)
            refusals = ledger.check(document, self.tree)
            with self.subTest(plant=label):
                self.assertEqual({r.arm for r in refusals}, {arm}, f"{label}: {refusals}")

    def test_the_ledger_exists_in_schema_v1(self):
        document = load()
        self.assertEqual(document.get("schema"), "formal-obligations.v1")
        self.assertEqual(list(document), ["schema", "phases", "obligations", "entries"])
        refusals = ledger.check(document, REPO)
        self.assertEqual(refusals, [], "the checker refuses the committed ledger")
        examined("obligations in the committed ledger", document["obligations"])
        self.assert_accepted(accepted())
        self.assert_planted(SCHEMA_PLANTS)

    def test_the_rows_hold_o1_to_o17_and_the_planned_entries(self):
        document = load()
        obligations = document["obligations"]
        self.assertEqual([item["id"] for item in obligations], IDS)
        self.assertEqual(
            {item["id"]: item["status"] for item in obligations},
            dict.fromkeys(IDS, "planned") | {"O17": "tied"},
        )
        planned, harnesses, companions, present_names = {}, {}, {}, {}
        for item in obligations:
            for artifact in item["artifacts"]:
                if artifact.get("tree") == "companion":
                    companions[item["id"]] = companions.get(item["id"], 0) + 1
                elif artifact["kind"] == "kani":
                    harnesses.setdefault(artifact["harness"], set()).add(item["id"])
                elif artifact["state"] == "planned":
                    planned.setdefault(artifact["entry"], set()).add(item["id"])
                else:
                    present_names.setdefault(item["id"], []).append(
                        (artifact["entry"], artifact["property"])
                    )
        self.assertEqual(planned, {entry: set(ids) for entry, ids in PLANNED_ENTRIES.items()})
        self.assertEqual(harnesses, {harness: {item} for harness, item in HARNESSES.items()})
        self.assertEqual(companions, COMPANIONS)
        headers = header_names(REPO)
        sync_named = {}
        for entry, ids in SYNC_ENTRIES.items():
            for identity in ids:
                sync_named.setdefault(identity, []).append(entry)
        expected_present = {
            "O1a": SYNC_MODELS,
            "O6": COMPANION_ENTRIES,
            "O11": ENGINE_TIED + ("tla/OpLog",),
            "O12": ID_SLOT_ENTRIES,
            "O13": MEDIA_ENTRIES,
            "O14": SEARCH_ENTRIES,
            "O17": (RELEASE_GATES,),
        }
        for identity, entries in sync_named.items():
            if identity != "O11":
                expected_present[identity] = tuple(entries)
        self.assertEqual(set(present_names), set(expected_present))
        for identity, entries in expected_present.items():
            names = present_names[identity]
            self.assertEqual(len(names), len(set(names)), f"{identity} names an artifact twice")
            self.assertEqual(set(names), {(e, p) for e in entries for p in headers[e]})
        self.assertEqual(
            [tie for item in obligations for tie in item["ties"]],
            [
                COMPANION_TIE,
                REPLAY_TIE,
                ID_SLOT_TIE,
                *MEDIA_TIES,
                *SYNC_CORE_TIES,
                RELEASE_GATES_TIE,
            ],
        )
        examined("planned entries", planned)
        examined("planned harnesses", harnesses)
        examined("present artifacts", [n for names in present_names.values() for n in names])

    def test_the_phase_sets_list_o1a_and_o1_to_o11(self):
        document = load()
        self.assertEqual(document["phases"], PHASES)
        ids = {item["id"] for item in document["obligations"]}
        members = examined("phase-set members", [m for ms in PHASES.values() for m in ms])
        self.assertTrue(set(members) <= ids, "a phase set names an id the ledger lacks")
        self.assert_accepted(accepted())
        self.assert_planted(PHASE_PLANTS)

    def test_an_obligation_with_no_artifact_is_refused(self):
        self.assert_accepted(accepted())
        self.assert_planted(NO_ARTIFACT_PLANTS)
        document = load()
        bare = [item["id"] for item in document["obligations"] if not item["artifacts"]]
        self.assertEqual(bare, [], "a committed obligation names no artifact")
        refusals = [r for r in ledger.check(document, REPO) if r.arm == "no-artifact"]
        self.assertEqual(refusals, [])
        examined("committed obligations with an artifact", document["obligations"])

    def test_an_obligation_with_no_gap_line_is_refused(self):
        self.assert_accepted(accepted())
        self.assert_planted(NO_GAP_PLANTS)
        document = load()
        gapless = [
            item["id"]
            for item in document["obligations"]
            if not item["gaps"] or any(not str(gap).strip() for gap in item["gaps"])
        ]
        self.assertEqual(gapless, [], "a committed obligation has no gap line")
        refusals = [r for r in ledger.check(document, REPO) if r.arm == "no-gap-line"]
        self.assertEqual(refusals, [])
        examined(
            "committed gap lines", [g for item in document["obligations"] for g in item["gaps"]]
        )

    def test_an_artifact_naming_an_undeclared_entry_or_property_is_refused(self):
        self.assert_accepted(accepted())
        self.assert_planted(UNDECLARED_PLANTS)
        document = load()
        headers = header_names(REPO)
        present_artifacts = [
            artifact
            for item in document["obligations"]
            for artifact in item["artifacts"]
            if artifact.get("state") == "present"
        ]
        for artifact in examined("committed present artifacts", present_artifacts):
            self.assertIn(artifact["property"], headers.get(artifact["entry"], set()), artifact)
        arms = {"undeclared-entry", "undeclared-property", "undeclared-harness"}
        refusals = [r for r in ledger.check(document, REPO) if r.arm in arms]
        self.assertEqual(refusals, [])

    def test_a_tie_the_tree_does_not_define_or_that_cannot_count_is_refused(self):
        self.assert_accepted(accepted())
        completed = accepted()
        obligation(completed, "O2").update(status="complete")
        self.assert_accepted(completed)
        self.assert_planted(TIE_PLANTS)
        document = load()
        ties = [tie for item in document["obligations"] for tie in item["ties"]]
        for tie in examined("committed ties", ties):
            path, name = tie["test"].split("::")
            text = (REPO / path).read_text(encoding="utf-8")
            if path.endswith(".py"):
                # A Python tie names `<Class>.<method>`: a test method of a top-level class.
                owner, method = name.split(".")
                self.assertTrue(method.startswith("test"), tie)
                body = re.search(rf"(?ms)^class {owner}\(.*?(?=^\S|\Z)", text)
                self.assertIsNotNone(body, tie)
                self.assertRegex(body.group(0), rf"(?m)^    def {method}\(self", tie)
            else:
                self.assertRegex(text, rf"#\[test\]\s*fn {name}\(", tie)
        arms = {"tie-method", "tie-without-for", "tie-for-unknown", "undefined-test"}
        refusals = [r for r in ledger.check(document, REPO) if r.arm in arms | {"untied-artifact"}]
        self.assertEqual(refusals, [])

    def test_every_existing_entry_has_its_disposition_row(self):
        self.assert_accepted(accepted())
        self.assert_planted(ROW_PLANTS)
        document = load()
        tree = header_names(REPO)
        self.assertEqual(sorted(ledger.declared(REPO)), sorted(tree))
        expected = {
            entry: disposition for disposition, entries in DISPOSITIONS.items() for entry in entries
        }
        self.assertEqual(len(expected), 49)
        self.assertFalse(set(expected) & set(NEW_ENTRIES), "a new entry was among the 49")
        expected.update({entry: disposition for entry, (disposition, _) in NEW_ENTRIES.items()})
        self.assertEqual(
            set(tree), set(expected), "the tree's entries are not the 49 designed and the new ones"
        )
        rows = {item["entry"]: item for item in examined("rows", document["entries"])}
        self.assertEqual(len(rows), len(document["entries"]), "an entry has two rows")
        self.assertEqual({entry: item["disposition"] for entry, item in rows.items()}, expected)
        for entry, item in rows.items():
            if entry in NEW_ENTRIES:
                linked = NEW_ENTRIES[entry][1]
            elif entry in SYNC_MODELS:
                linked = {"O1a", "O11"}
            elif entry in ENGINE_TIED:
                linked = {"O11"}
            elif entry == RELEASE_GATES:
                linked = {"O17"}
            else:
                linked = set(SYNC_ENTRIES.get(entry, ()))
            self.assertEqual(set(item["obligations"]), linked, entry)
            self.assertTrue(item["act"].strip(), entry)
        arms = {"entry-without-row", "row-without-entry", "duplicate-row", "row-obligations"}
        self.assertEqual([r for r in ledger.check(document, REPO) if r.arm in arms], [])

    def test_a_census_finds_no_private_core_name_in_the_ledger(self):
        digests = set(ledger.PRIVATE_NAME_DIGESTS)
        planted_digest = hashlib.sha256((SALT + PLANTED_NAME.lower()).encode("utf-8")).hexdigest()
        widened = frozenset(digests | {planted_digest})
        examined_clean, clean = ledger.census(planted_text(lambda d: None), widened)
        self.assertEqual(clean, [], "the accepted ledger's text is refused")
        self.assertGreater(examined_clean, 0)
        for label, arm, change in examined("planted names", CENSUS_PLANTS):
            _, refusals = ledger.census(planted_text(change), widened)
            with self.subTest(plant=label):
                self.assertEqual({r.arm for r in refusals}, {arm}, f"{label}: {refusals}")
                spoken = " ".join(f"{r.where} {r.why}" for r in refusals).lower()
                self.assertNotIn(PLANTED_NAME.lower(), spoken, "a refusal names the token")
                self.assertNotIn(BRAND, spoken, "a refusal names the token")
        longer = planted_text(lambda d: obligation(d, "O1")["gaps"].append(PLANTED_NAME + "s"))
        self.assertEqual(ledger.census(longer, widened)[1], [], "a longer token is no listed name")
        listed = examined("listed private-name digests", sorted(digests))
        self.assertTrue(all(HEX64.match(item) for item in listed), "a listed digest is not hex")
        text = LEDGER.read_text(encoding="utf-8") if LEDGER.is_file() else ""
        self.assertTrue(text, "formal/obligations.json is absent: the census reads nothing")
        tokens, refusals = ledger.census(text)
        print(f"examined {tokens} tokens of the committed ledger")
        self.assertGreater(tokens, 0)
        self.assertEqual(refusals, [], "the committed ledger holds a private core name")


if __name__ == "__main__":
    unittest.main()
