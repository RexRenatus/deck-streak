"""The formal obligations ledger's checker and its census (SPEC-496, ADR-514).

`formal/obligations.json` names every engine property the formal tools carry: the artifacts that
carry it, the ties that bind each artifact to the shipped code, and the gaps no tool closes. `check`
reads a ledger against a tree and lists every refusal by its arm; `census` reads the ledger's text
for a private core name. CI runs no formal tool, so the `@phx` headers and the test definitions are
parsed here. Both read files only and start no process.
"""

import ast
import hashlib
import re
from pathlib import Path
from typing import NamedTuple

SCHEMA = "formal-obligations.v1"
LEDGER = Path("formal") / "obligations.json"

TOP_KEYS = ("schema", "phases", "obligations", "entries")
PHASE_KEYS = ("1", "2")
OBLIGATION_KEYS = ("id", "property", "layers", "artifacts", "ties", "gaps", "status")
STATUSES = ("planned", "artifact", "tied", "complete")
# The statuses that claim every artifact exists, and those that claim every artifact is tied.
PAST_PLANNED = STATUSES[1:]
TIED = STATUSES[2:]
KINDS = ("tla", "lean", "kani")
STATES = ("planned", "present")
COMPANION = "companion"
METHODS = ("vectors", "trace", "kani-identity", "mbt", "census", "cross-target-hash")
# A census or a cross-target hash tests an absence or an equality and proves no property.
COUNTING = ("vectors", "trace", "kani-identity", "mbt")
TIE_KEYS = ("method", "for", "test", "mutant")
ROW_KEYS = ("entry", "disposition", "act", "obligations")
DISPOSITIONS = ("keep", "re-stamp", "re-point", "retire")

ID = re.compile(r"^O[1-9][0-9]*a?$")
ENTRY = re.compile(r"^(tla|lean)/([A-Z][A-Za-z0-9]*)$")
NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_']*$")
HARNESS = re.compile(r"^K-[a-z0-9]+(?:-[a-z0-9]+)*$")
TEST = re.compile(
    r"^(?P<path>[A-Za-z0-9_][A-Za-z0-9_./-]*\.(?P<ext>rs|py))"
    r"::(?P<name>[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)?)$"
)

# The header lines that declare a TLA+ property and a Lean theorem, in the entries' own grammar.
TLA_DECLARES = re.compile(r"^\s*\\\*\s*@phx\s+property\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s|$)")
LEAN_DECLARES = re.compile(r"^\s*--\s*@phx\s+theorem\s+([A-Za-z_][A-Za-z0-9_']*)(?:\s|$)")
# Rust comments are blanked before a test is looked for: a commented-out test defines nothing.
RUST_COMMENT = re.compile(r"//[^\n]*|/\*.*?\*/", re.DOTALL)
RUST_DEFINITION = re.compile(
    r"#\[\s*(?P<kind>test|tokio::test|kani::proof)\s*(?:\([^\]]*\))?\s*\]"
    r"(?:\s*#\[[^\]]*\])*\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?"
    r"fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
)
TEST_KINDS = frozenset({"test", "tokio::test", "kani::proof"})
HARNESS_KINDS = frozenset({"kani::proof"})

# The census. The brand is assembled from parts, so this file holds no token derived from it.
CENSUS_SALT = "formal-ledger-census:"
BRAND = "".join(("r", "e", "x"))
BRAND_AT_A_WORD_START = re.compile(f"(?<![A-Za-z0-9]){BRAND}", re.IGNORECASE)
BRAND_AT_A_HUMP = re.compile(f"(?<=[a-z0-9])R(?:{BRAND[1:]}|{BRAND[1:].upper()})")
TOKEN = re.compile(r"[A-Za-z0-9_]+(?:[-.:][A-Za-z0-9_]+)*")
PART = re.compile(r"[-.:]")
# The private core names, each as the SHA-256 of the salt and the name in lower case.
PRIVATE_NAME_DIGESTS = frozenset(
    (
        "ef4b75756228b6a72d10811c10d3aa97fab4c1fb284ddfdd025841e48e833de1",
        "a59175f3edf5cffe4589038941ad7a6f47c651b3e1a85718f6fd3340d370c522",
        "611242a514f71c7955c1bf54729aca37c47be77d6aff46b261a2b68b6a11d9ba",
        "a0a587d54ce9147f901dcc5847791b755508b2542ed4191f95c9e00f07574419",
        "ff4701a06bc65fa26145e1a9599876919a015ecf5b14615ad94109d7ad94c20c",
        "5488b9a96d0ec6345956df83d201da3deed45a61a1814550954063cb4f3ba3f7",
        "29966b1c88d9bd1e7da215ed596b890ef4cc1303836c39cd76816f4605f27dd3",
        "7955ac0c98f54ad131059ea8d245adc8c1a11e4d573fdedbad6c13d81193df30",
        "3a69966521d9931e6737d50a84ee5d9ab0de316c70b45ed62252e2810318f682",
        "56d1f6affd34e94a251a1772d25b7a58bb699505611f9955d1b74682037ba0f7",
        "f368c9dff628c60191d4207445d05db8e714cbc01a380a10daf93aa41fae870b",
        "302c5b15e60e0baae5270e37cfeeec2a0dd64776959dde81a8d670b1b9c0793e",
        "ba7cbf7b7796b8c11d1fc598abaa3c02f120986c925b56af873ef51c5ff14919",
        "948d8c10a2bf9f76c37f911d9f36e4bfa0a8a444d613f529504a2dd71d6ecfbc",
        "6beca2fbfc460bef29dc3227d6c5594008382af4c4956ead1be7c13e5a568c7a",
        "b2b3432f3e1eea90944159dac7382b960373802fea595d4425bc3bf6c5853e4b",
        "7977bbd769b4620120085bcb7f011ab651306daf81c512c38b2b38392710031f",
        "ff144dc18423e3aca17872694c2285a1963dd444f102fdef98b781406eee9c7a",
        "f36ea2cfaa2b2e525b80a601dff266bd3e2475739b7535b8a18ab5580ba7cae2",
        "a7216f28f7ca8be9a6bee85579949a79c9a46fa4b3cc4350d838a62216bbbb32",
        "9687787c387352ff678828be06b24396fccbb7788a367f4ba6d4898334b23724",
        "9abe6f50b3793e78ff1d86c263fd977f6b9fc34cfedd5b5081e0a05383e5d16a",
        "9129d5b2dc452c9e7f5946841b12ad67cca60e1518ee6827327df8eb28d01db3",
        "1407c797b90d2291502d86283a81987e374e54b2220df06d127b69a0360f6aca",
        "f810059a67fcc729ec02253e70a7be7ccac17d3701aad489a55cc6a470e22e4b",
        "0c60c70d7d8a36f06bdccda2251ae0deb587ab089732bc40ad1f16dd53b087bf",
        "641bd31fb1521dca104263c835c24f2c9feea532ff2e6ef8d4d7924a2d8322ff",
        "2198fe69013cc84b07e2c2243a4e1a654024f3cff7c9aa630b3b2d8713297fde",
        "bb5c608a1d6864bc98cb65347b8e84c31bcb730f509e7edd4898b41a9d63d486",
        "6cf8c39272927d9d8b8297e41e0aa64b580f54b2ea17987c147f6fbd7711b2f0",
        "99b3cdb3717f78609c2e3855198d58b2b70d75e688df626a66e000b103dcd9b7",
        "73dcab871e5233e0a93fe0565251df623ae5ea19b42907e92175f459f352c227",
        "3cca1ad745ed71a47458d78400cc3717fa5034b810522b12d50245976639031b",
        "f85464f24ea5e500ed4be1a56f7923903a75725d1de12d9e72462307c316ab78",
        "c60e9729fcc28fd9bae7b32ef8a03c67cf25d9e0e410f6960fa52b87f95d16e8",
        "b1360ff56114b3cd77aca30be26881eb85821479c824c7c825995b6326f1a6ea",
        "1747fb1718b5bd84670d85112cfad7444c10899c82e5b32a9d262db810820fe7",
        "c234602672f5c72291872ac54d491a5f74caa61d10bab53a697c47b859e81eb0",
        "ada29d91ffba3cf2944d67659c8580b10d5c3cb701fea229c2305c22c2186377",
        "e2cf48866a4d4d11542d6ae27c5f14cb2c39fa32c464d70c8edc5e90b9a61bc7",
        "58037bc882f8620a42cfed4980b19a4e2625aca983e03910e1e86c228b0bedfe",
        "7bbd376428080be95813d5d8f2509c2e998a14c346dbf77f923ef25c9c2eeca0",
        "fe6694e2e1b6e461c66fee0770d6b87f63fe12373916bca0c59568bd043a4f89",
        "b65983d9e7923417e0e0a9082c3ccf936fa7af895b4fbeb010296745b9720563",
        "01a90ca995188038685d643cdbf39b9dd94610564b194ebb80c164fad5e3b05e",
        "41775e8196c1093790929c91924c35ad06a448e694a1b2af6cb699268b96f5b7",
        "172b9d17a4218f385f5442c9693a3e94a3e1fb2906d0640df7ca9d62ddca4879",
        "ef5a39e5b90528f49749243ecd400e7be9a4223e8985acb33ffc953b26eb428b",
        "033eedac7886944d02962ec6a1c33ca55bd1ad3fbcc9d4918eeaaca4c1dd8a62",
        "dc2cc651b4a16a7a50a945f6ea48953e9c84dd65ef077b5a4927a67f4e6e40c3",
    )
)


class Refusal(NamedTuple):
    """One refusal: the arm that refused, where in the ledger, and why."""

    arm: str
    where: str
    why: str


def names(path, pattern):
    """The names the header lines of the file at `path` declare by `pattern`."""
    found = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        match = pattern.match(line)
        if match:
            found.add(match.group(1))
    return frozenset(found)


def plain_file(path):
    return path.is_file() and not path.is_symlink()


def declared(root):
    """Every formal entry of the tree at `root`, by its id, with the names its `@phx` header
    declares. A TLA+ entry is a folder `formal/tla/<Name>` holding `<Name>.tla`; a Lean entry is a
    module `formal/lean/Formal/<Name>.lean` whose header declares at least one theorem."""
    root = Path(root)
    found = {}
    tla = root / "formal" / "tla"
    if tla.is_dir() and not tla.is_symlink():
        for folder in sorted(tla.iterdir()):
            module = folder / f"{folder.name}.tla"
            if folder.is_dir() and not folder.is_symlink() and plain_file(module):
                found[f"tla/{folder.name}"] = names(module, TLA_DECLARES)
    lean = root / "formal" / "lean" / "Formal"
    if lean.is_dir() and not lean.is_symlink():
        for module in sorted(lean.glob("*.lean")):
            if plain_file(module):
                theorems = names(module, LEAN_DECLARES)
                if theorems:
                    found[f"lean/{module.stem}"] = theorems
    return found


def tree_file(root, relative):
    """The regular file `relative` names under `root`, or None: a path that is absolute, steps up,
    passes a link or names no regular file names nothing."""
    path = Path(relative)
    if path.is_absolute() or any(part in ("..", ".") for part in path.parts):
        return None
    walked = Path(root)
    for part in path.parts:
        walked = walked / part
        if walked.is_symlink():
            return None
    return walked if walked.is_file() else None


def rust_definitions(text):
    """Each function of a Rust source marked as a test or a Kani proof, with its marks' kinds."""
    found = {}
    for match in RUST_DEFINITION.finditer(RUST_COMMENT.sub(" ", text)):
        found.setdefault(match.group("name"), set()).add(match.group("kind"))
    return found


def python_defines(text, name):
    """Whether a Python source defines `name` as `<Class>.<method>`: a test method of a top-level
    class. That is the one form a Python tie names; unittest runs no top-level function."""
    try:
        module = ast.parse(text)
    except SyntaxError:
        return False
    functions = (ast.FunctionDef, ast.AsyncFunctionDef)
    if "." in name:
        owner, method = name.split(".", 1)
        return method.startswith("test") and any(
            isinstance(node, ast.ClassDef)
            and node.name == owner
            and any(isinstance(item, functions) and item.name == method for item in node.body)
            for node in module.body
        )
    # A top-level function is no tie, even when its name starts with `test`: no unittest run
    # would ever fail it, so it could count a property that nothing checks.
    return False


class Tree:
    """The checked tree: its formal entries with their declared names, and its test definitions."""

    def __init__(self, root):
        self.root = Path(root)
        self.entries = declared(self.root)
        self._texts = {}

    def _text(self, path):
        if path not in self._texts:
            self._texts[path] = path.read_text(encoding="utf-8")
        return self._texts[path]

    def defines(self, reference, kinds=TEST_KINDS):
        """Whether `reference`, `<path>.rs::<fn>` or `<path>.py::<Class>.<method>`, names a test
        the tree defines; a Rust function counts only under a mark of `kinds`, and a Python test
        only when any test mark counts."""
        match = TEST.match(reference) if isinstance(reference, str) else None
        if not match:
            return False
        path = tree_file(self.root, match.group("path"))
        if path is None:
            return False
        name = match.group("name")
        if match.group("ext") == "rs":
            marks = rust_definitions(self._text(path)).get(name, set())
            return "." not in name and bool(marks & kinds)
        return kinds == TEST_KINDS and python_defines(self._text(path), name)


def check(document, root):
    """Every refusal of the ledger `document` against the tree at `root`, in reading order."""
    tree = Tree(root)
    refusals = []

    def refuse(arm, where, why):
        refusals.append(Refusal(arm, where, why))

    if not isinstance(document, dict):
        refuse("shape", "ledger", "the ledger is not an object")
        return refusals
    closed(document, TOP_KEYS, (), "ledger", refuse)
    if document.get("schema") != SCHEMA:
        refuse("schema", "schema", f"the schema is not {SCHEMA}")
    ids = set()
    linked = {}
    for index, obligation in enumerate(listed(document.get("obligations"), "obligations", refuse)):
        where = f"obligations[{index}]"
        if not isinstance(obligation, dict):
            refuse("shape", where, "an obligation is not an object")
            continue
        identity = obligation.get("id")
        if not (isinstance(identity, str) and ID.match(identity)):
            refuse("shape", where, "its id is not O<n> or O<n>a")
            continue
        if identity in ids:
            refuse("duplicate-id", where, f"{identity} is named twice")
            continue
        ids.add(identity)
        for entry in judge_obligation(obligation, identity, tree, refuse):
            linked.setdefault(entry, set()).add(identity)
    judge_phases(document.get("phases"), ids, refuse)
    judge_rows(document.get("entries"), linked, tree, refuse)
    return refusals


def closed(value, required, optional, where, refuse):
    """Refuses an object that lacks a required key or holds a key no reader judges."""
    missing = [key for key in required if key not in value]
    unknown = sorted(set(value) - set(required) - set(optional))
    if missing or unknown:
        refuse("shape", where, f"missing {missing}, unknown {unknown}")


def listed(value, where, refuse):
    """`value` when it is a list; otherwise a refusal and no member."""
    if isinstance(value, list):
        return value
    refuse("shape", where, "not a list")
    return []


def blank(value):
    return not (isinstance(value, str) and value.strip())


def judge_obligation(obligation, identity, tree, refuse):
    """Judges one obligation, and gives the entries its artifacts name."""
    closed(obligation, OBLIGATION_KEYS, (), identity, refuse)
    if blank(obligation.get("property")):
        refuse("shape", f"{identity}.property", "the property is not named")
    layers = obligation.get("layers")
    if (
        not isinstance(layers, list)
        or not layers
        or any(blank(layer) for layer in layers)
        or len(set(layers)) != len(layers)
    ):
        refuse("shape", f"{identity}.layers", "the layers are not a list of distinct names")
    status = obligation.get("status")
    if status not in STATUSES:
        refuse("status", f"{identity}.status", f"the status is not one of {', '.join(STATUSES)}")
    artifacts = listed(obligation.get("artifacts"), f"{identity}.artifacts", refuse)
    if not artifacts:
        refuse("no-artifact", identity, "the obligation names no artifact")
    handles = []
    entries = set()
    anonymous = False
    planned = False
    for index, artifact in enumerate(artifacts):
        where = f"{identity}.artifacts[{index}]"
        handle, entry, state = judge_artifact(artifact, where, tree, refuse)
        if handle is None:
            anonymous = True
        else:
            handles.append(handle)
        if entry is not None:
            entries.add(entry)
        planned = planned or state == "planned"
    gaps = listed(obligation.get("gaps"), f"{identity}.gaps", refuse)
    if not gaps or any(blank(gap) for gap in gaps):
        refuse("no-gap-line", identity, "the obligation has no gap line, or a blank one")
    counted = set()
    for index, tie in enumerate(listed(obligation.get("ties"), f"{identity}.ties", refuse)):
        bound = judge_tie(tie, f"{identity}.ties[{index}]", handles, tree, refuse)
        if bound is not None:
            counted.add(bound)
    if status in PAST_PLANNED and planned:
        refuse(
            "planned-past-planned", identity, f"a planned artifact under an obligation at {status}"
        )
    if status in TIED:
        untied = sorted(set(handles) - counted)
        if untied or anonymous:
            refuse("untied-artifact", identity, f"at {status}, no counting tie binds {untied}")
    return entries


def judge_artifact(artifact, where, tree, refuse):
    """Judges one artifact. Gives (handle, entry, state): the name a tie binds it by, the formal
    entry it names, and its state; a companion artifact has no handle and names no entry."""
    if not isinstance(artifact, dict):
        refuse("shape", where, "an artifact is not an object")
        return None, None, None
    kind = artifact.get("kind")
    state = artifact.get("state")
    if kind not in KINDS or state not in STATES:
        refuse("shape", where, "its kind or its state is not one the ledger knows")
        return None, None, None
    if "tree" in artifact:
        closed(artifact, ("kind", "tree", "about", "state"), (), where, refuse)
        if artifact.get("tree") != COMPANION or blank(artifact.get("about")):
            refuse("shape", where, "an artifact of another tree is the companion's, with an about")
        return None, None, state
    if kind == "kani":
        closed(artifact, ("kind", "harness", "state"), ("at",), where, refuse)
        harness = artifact.get("harness")
        if not (isinstance(harness, str) and HARNESS.match(harness)):
            refuse("shape", where, "its harness is not named K-<name>")
            return None, None, state
        if state == "present" and not tree.defines(artifact.get("at"), HARNESS_KINDS):
            refuse("undeclared-harness", where, f"{harness} is at no #[kani::proof] of the tree")
        elif "at" in artifact and not isinstance(artifact["at"], str):
            refuse("shape", where, "its at is not a path")
        return harness, None, state
    closed(artifact, ("kind", "entry", "state"), ("property",), where, refuse)
    entry = artifact.get("entry")
    named = ENTRY.match(entry) if isinstance(entry, str) else None
    if not named or named.group(1) != kind:
        refuse("shape", where, "its entry is not <kind>/<Name>")
        return None, None, state
    prop = artifact.get("property")
    if prop is not None and not (isinstance(prop, str) and NAME.match(prop)):
        refuse("shape", where, "its property is not a name")
        return entry, entry, state
    if state == "present":
        if entry not in tree.entries:
            refuse("undeclared-entry", where, f"{entry} is no entry of the tree")
        elif prop is None:
            refuse("shape", where, "a present artifact names no property")
        elif prop not in tree.entries[entry]:
            refuse("undeclared-property", where, f"{entry}'s header declares no {prop}")
    return entry, entry, state


def judge_tie(tie, where, handles, tree, refuse):
    """Judges one tie. Gives the handle it binds when it counts, else None."""
    if not isinstance(tie, dict):
        refuse("shape", where, "a tie is not an object")
        return None
    closed(tie, (), TIE_KEYS, where, refuse)
    method = tie.get("method")
    if method not in METHODS:
        refuse("tie-method", where, f"its method is not one of {', '.join(METHODS)}")
    bound = tie.get("for")
    if bound is None or (isinstance(bound, str) and not bound.strip()):
        refuse("tie-without-for", where, "the tie names no artifact in for")
        bound = None
    elif not isinstance(bound, str) or bound not in handles:
        refuse("tie-for-unknown", where, "its for names no artifact of its obligation")
        bound = None
    defined = tree.defines(tie.get("test"))
    if not defined:
        refuse("undefined-test", where, "its test is not defined in the tree")
    mutant = tie.get("mutant")
    if mutant is not None and blank(mutant):
        refuse("shape", where, "its mutant is not named")
    counts = method in COUNTING and defined and (method != "mbt" or not blank(mutant))
    return bound if counts else None


def judge_phases(phases, ids, refuse):
    """Judges the phase sets: each a list of distinct ids of the ledger's obligations."""
    if not isinstance(phases, dict):
        refuse("shape", "phases", "the phase sets are not an object")
        return
    closed(phases, PHASE_KEYS, (), "phases", refuse)
    for key in PHASE_KEYS:
        members = phases.get(key)
        if (
            not isinstance(members, list)
            or not members
            or any(not isinstance(member, str) for member in members)
        ):
            refuse("shape", f"phases.{key}", "a phase set is not a list of ids")
            continue
        if len(set(members)) != len(members):
            refuse("phase", f"phases.{key}", "an id is named twice")
        for member in members:
            if member not in ids:
                refuse("phase", f"phases.{key}", f"{member} is no obligation of the ledger")


def judge_rows(rows, linked, tree, refuse):
    """Judges the rows: one per formal entry of the tree, each linked to the obligations that
    name it."""
    seen = set()
    for index, row in enumerate(listed(rows, "entries", refuse)):
        where = f"entries[{index}]"
        if not isinstance(row, dict):
            refuse("shape", where, "a row is not an object")
            continue
        closed(row, ROW_KEYS, (), where, refuse)
        entry = row.get("entry")
        if not (isinstance(entry, str) and ENTRY.match(entry)):
            refuse("shape", where, "its entry is not <kind>/<Name>")
            continue
        if entry in seen:
            refuse("duplicate-row", entry, "the entry has two rows")
            continue
        seen.add(entry)
        if row.get("disposition") not in DISPOSITIONS or blank(row.get("act")):
            refuse("shape", entry, "its disposition or its act is not one the ledger knows")
        obligations = row.get("obligations")
        if (
            not isinstance(obligations, list)
            or any(not isinstance(item, str) for item in obligations)
            or len(set(obligations)) != len(obligations)
        ):
            refuse("shape", entry, "its obligations are not a list of distinct ids")
        elif set(obligations) != linked.get(entry, set()):
            refuse("row-obligations", entry, "its obligations are not those that name it")
        if entry not in tree.entries:
            refuse("row-without-entry", entry, "the tree holds no such entry")
    for entry in sorted(set(tree.entries) - seen):
        refuse("entry-without-row", entry, "the tree's entry has no row")


def digest(name):
    """The census digest of `name`: the SHA-256 of the salt and the name in lower case."""
    return hashlib.sha256((CENSUS_SALT + name.lower()).encode("utf-8")).hexdigest()


def census(text, digests=PRIVATE_NAME_DIGESTS):
    """(tokens examined, refusals) over the ledger's `text`: a line holding a token derived from
    the scheduler's brand, and a token, or a part of it between `-`, `.` and `:`, whose digest is
    in `digests`. A refusal names its line, never the token."""
    examined = 0
    refusals = []
    for number, line in enumerate(text.splitlines(), 1):
        where = f"line {number}"
        if BRAND_AT_A_WORD_START.search(line) or BRAND_AT_A_HUMP.search(line):
            refusals.append(Refusal("private-brand", where, "a token derived from the brand"))
        for match in TOKEN.finditer(line):
            examined += 1
            token = match.group(0)
            if any(digest(part) in digests for part in {token, *PART.split(token)}):
                refusals.append(Refusal("private-name", where, "a name on the census list"))
    return examined, refusals
