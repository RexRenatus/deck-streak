"""The engine is Anki's upstream release tag, patched by `rev` to a commit of the maintainer's fork
that carries the rebuild fix (SPEC-055 A1, A2, A3 and A5; ADR-058).

- A1 reads the manifest, the lockfile and `deny.toml`. The dependency keeps naming the upstream
  tag, the root manifest's `[patch]` entry on the upstream URL takes `anki` from the fork by a full
  commit id, and `anki_proto` from the same commit when it is patched too (SPEC-338 A14, ADR-348),
  every engine package in the lockfile comes from that commit, and `allow-git` names exactly the
  fork, `ankitects/rust-url` and fsrs-rs (ADR-338; SPEC-342 A2).
- A2 builds `deck-streak-ingest` twice in the workspace's own target directory and reads the second
  build's `Compiling`, `Dirty`, `Fresh` and `Finished` lines. With the fix, nothing is compiled or
  judged dirty, and cargo's own time stays inside R2's bound. Unpatched, the second build recompiles
  `anki_proto` and `anki` (#228).
- A3 runs `cargo deny` over the advisories and the sources, and reads the notes it prints at the
  info level. Every advisory exception in `deny.toml` is one it still encounters, and every allowed
  git source is one a crate of the graph comes from.
- A5 reads ADR-058's Confirmation: the pinned commit and its one-file difference from the upstream
  tag, the no-op build before and after the pin, CI's runs, and a final status. When a note
  appended to ADR-058 moved the pin (SPEC-338 A15), the Confirmation is judged at the fix's own
  commit and the note at the pinned one: its diff stat, a patch table ending at it, and the ADRs
  that decided it.

Each judge refuses a set of planted defects by name before it judges the committed tree.
"""

import json
import os
import re
import shutil
import subprocess
import time
import tomllib
import unittest
from pathlib import Path

from _support import REPO, examined

MANIFEST = REPO / "Cargo.toml"
LOCKFILE = REPO / "Cargo.lock"
DENY = REPO / "deny.toml"
ADR_058 = (
    REPO
    / "docs"
    / "decisions"
    / "ADR-058-the-engine-pins-a-patched-fork-of-26-09-3-until-upstream-carries-the-fix.md"
)
#: Anki's repository, the maintainer's fork of it, and the fork the engine's own manifest pins for
#: `percent-encoding-iri` (ADR-022, ADR-058).
UPSTREAM = "https://github.com/ankitects/anki.git"
FORK = "https://github.com/RexRenatus/anki.git"
RUST_URL = "https://github.com/ankitects/rust-url.git"
#: The upstream scheduler's repository, whose development line is the only source of FSRS-7; only
#: `deck-streak-fsrs7` takes a crate from it (ADR-338, ADR-353 D1).
FSRS_RS = "https://github.com/open-spaced-repetition/fsrs-rs.git"
#: The upstream release the engine runs (SPEC-055 R1).
TAG = "26.09.3"
#: The dependency line, in the one form its two readers parse: SPEC-022's A1 check and the report
#: of `engine-measure.yml`.
DEPENDENCY = f'anki = {{ git = "{UPSTREAM}", tag = "{TAG}", features = ["rustls"] }}'
PATCH_HEADER = f'[patch."{UPSTREAM}"]'
COMMIT = re.compile(r"[0-9a-f]{40}")
#: The package sets the patch may replace: anki alone (ADR-058), or anki with its protobuf
#: messages from the same commit, which the web engine reads (ADR-348).
PATCHED = (["anki"], ["anki", "anki_proto"])

# ------------------------------------------------------------------------------------------ A1


def comment_above(text, header):
    """The comment lines directly above `header`, joined, or "" when there are none."""
    lines = text.splitlines()
    if header not in lines:
        return ""
    comment = []
    for line in reversed(lines[: lines.index(header)]):
        if not line.startswith("#"):
            break
        comment.append(line)
    return " ".join(reversed(comment))


def engine_packages(lock):
    """The lockfile's packages whose source is the engine's repository, upstream or the fork."""
    packages = tomllib.loads(lock).get("package", [])
    sources = (f"git+{UPSTREAM}", f"git+{FORK}")
    return [package for package in packages if package.get("source", "").startswith(sources)]


def patched_rev(manifest):
    """The fork commit the root manifest's `[patch]` entry names, or None."""
    entry = tomllib.loads(manifest).get("patch", {}).get(UPSTREAM, {}).get("anki")
    return entry.get("rev") if isinstance(entry, dict) else None


def pin_findings(manifest, lock, deny):
    """Why the engine is not the upstream tag patched by rev to a commit of the fork, with exactly
    the fork, rust-url and fsrs-rs allowed; empty when it is."""
    found = []
    dependency = tomllib.loads(manifest).get("workspace", {}).get("dependencies", {}).get("anki")
    if DEPENDENCY not in manifest.splitlines():
        found.append(f"the engine's dependency is {dependency}, not upstream tag {TAG} with rustls")
    patches = tomllib.loads(manifest).get("patch", {}).get(UPSTREAM)
    rev = None
    if patches is None:
        found.append(f"the root manifest has no {PATCH_HEADER} entry")
    else:
        entry = patches.get("anki") if isinstance(patches.get("anki"), dict) else {}
        pins = sorted(key for key in entry if key != "git")
        if sorted(patches) not in PATCHED:
            found.append(
                f"the patch replaces {sorted(patches)}, not anki alone or anki with anki_proto"
            )
        if entry.get("git") != FORK:
            found.append(f"the patch takes anki from {entry.get('git')}, not the fork")
        elif pins != ["rev"]:
            found.append(f"the patch pins anki by {', '.join(pins) or 'nothing'}, not by rev alone")
        elif COMMIT.fullmatch(entry["rev"]) is None:
            found.append(f"the patch's rev {entry['rev']!r} is not a full commit id")
        else:
            rev = entry["rev"]
        # ADR-348: anki_proto, when patched, comes from anki's own commit of the fork.
        proto = patches.get("anki_proto")
        if rev is not None and proto is not None:
            proto = proto if isinstance(proto, dict) else {}
            if proto.get("git") != FORK:
                found.append(f"the patch takes anki_proto from {proto.get('git')}, not the fork")
            elif proto != {"git": FORK, "rev": rev}:
                found.append(
                    f"the patch takes anki_proto by rev {proto.get('rev')}, not anki's {rev}"
                )
        comment = comment_above(manifest, PATCH_HEADER)
        found += [
            f"the patch's comment names no {cited}"
            for cited in ("ADR-058", "#233")
            if cited not in comment
        ]
    engine = engine_packages(lock)
    pinned = f"git+{FORK}?rev={rev}#{rev}"
    found += [
        f"{package['name']} comes from {package['source']}, not the fork's commit"
        for package in engine
        if rev is None or package["source"] != pinned
    ]
    if "anki" not in [package["name"] for package in engine]:
        found.append("the lockfile holds no engine package")
    unused = tomllib.loads(lock).get("patch", {}).get("unused", [])
    if unused:
        found.append(f"the lockfile records the patch as unused: {[p['name'] for p in unused]}")
    sources = tomllib.loads(deny).get("sources", {})
    allowed = sources.get("allow-git", [])
    if sorted(allowed) != sorted([FORK, RUST_URL, FSRS_RS]):
        found.append(f"allow-git is {allowed}, not exactly the fork, rust-url and fsrs-rs")
    if sources.get("unknown-git") != "deny":
        found.append(f"unknown-git is {sources.get('unknown-git')!r}, not 'deny'")
    return found


#: A planted commit of the fork, and a planted revision of rust-url.
PLANTED_REV = "0123456789abcdef0123456789abcdef01234567"
PLANTED_URL_REV = "89abcdef0123456789abcdef0123456789abcdef"


def planted_manifest(
    entry=None, comment="# ADR-058 (#233): the fix, until upstream carries it.", others=""
):
    """A planted root manifest whose patch takes anki from `entry`, then the `others` lines."""
    entry = entry or f'{{ git = "{FORK}", rev = "{PLANTED_REV}" }}'
    return (
        f"[workspace.dependencies]\n{DEPENDENCY}\n\n{comment}\n{PATCH_HEADER}\nanki = {entry}\n"
        + others
    )


#: The patch line that takes anki_proto from the same fork commit as anki (ADR-348).
PLANTED_PROTO = f'anki_proto = {{ git = "{FORK}", rev = "{PLANTED_REV}" }}\n'


def planted_lock(sources=None, unused=""):
    """A planted lockfile of two engine packages from `sources` and rust-url's crate."""
    sources = sources or {"anki": f"git+{FORK}?rev={PLANTED_REV}#{PLANTED_REV}"}
    body = "version = 4\n"
    for name in ("anki", "anki_io"):
        source = sources.get(name, sources["anki"])
        body += f'\n[[package]]\nname = "{name}"\nversion = "0.0.0"\nsource = "{source}"\n'
    body += (
        '\n[[package]]\nname = "percent-encoding-iri"\nversion = "2.2.0"\n'
        f'source = "git+{RUST_URL}?rev={PLANTED_URL_REV}#{PLANTED_URL_REV}"\n'
    )
    return body + unused


def planted_deny(allowed=(FORK, RUST_URL, FSRS_RS)):
    listed = ", ".join(f'"{url}"' for url in allowed)
    return f'[sources]\nunknown-git = "deny"\nallow-git = [{listed}]\n'


# ------------------------------------------------------------------------------------------ A2

#: R2's bound on the second build, in cargo's own `Finished` time.
NO_OP_BOUND_S = 10.0
#: The bound on each build's wall time. A cold build of the engine in a debug profile took about a
#: minute on the maintainer's machine (SPEC-055 section 7), and cargo may first wait for another
#: build to finish. The bound is 25 minutes, inside CI's 30-minute hygiene job, so a hung build
#: fails here by name before the runner kills the job.
BUILD_BOUND_S = 1500
BUILD = ["cargo", "build", "--locked", "-v", "-p", "deck-streak-ingest"]
UNIT = re.compile(r"^\s*(Compiling|Fresh) (\S+) v\S+")
DIRTY = re.compile(r"^\s*Dirty (\S+) v\S+ \(.*?\): (.*)$")
FINISHED = re.compile(r"^\s*Finished .* in (?:(\d+)m )?(\d+(?:\.\d+)?)s\s*$")
QUOTED = re.compile(r"`([^`]*)`")


def dirty_reason(reason):
    """Cargo's reason for a dirty unit, with every quoted path cut to its file name and the
    timestamps dropped, so a finding names no local path."""
    reason = QUOTED.sub(lambda quoted: f"`{Path(quoted.group(1)).name}`", reason)
    return reason.split(" (", 1)[0]


def build_findings(output):
    """Why a second build's verbose output shows work; empty when it compiled nothing, judged
    nothing dirty and finished inside the bound. Returns the findings and the units it judged
    Fresh."""
    compiled, fresh, dirty, finished = [], [], [], []
    for line in output.splitlines():
        unit, stale, done = UNIT.match(line), DIRTY.match(line), FINISHED.match(line)
        if unit:
            (compiled if unit.group(1) == "Compiling" else fresh).append(unit.group(2))
        elif stale:
            dirty.append(f"{stale.group(1)} dirty: {dirty_reason(stale.group(2))}")
        elif done:
            finished.append(int(done.group(1) or 0) * 60 + float(done.group(2)))
    found = []
    if compiled:
        found.append(f"the second build compiled {len(compiled)} unit(s): {', '.join(compiled)}")
    found += [f"cargo judged {unit}" for unit in dirty]
    if len(finished) != 1:
        found.append(f"the second build printed {len(finished)} Finished line(s), not one")
    elif finished[0] > NO_OP_BOUND_S:
        found.append(
            f"cargo's own time for the second build was {finished[0]:.2f} s, "
            f"over R2's {NO_OP_BOUND_S:g} s bound"
        )
    return found, fresh


def need_build_tools():
    """Why the engine cannot be built here, naming the missing tool as the gate does; None when it
    can. The engine's build runs protoc from PROTOC, or from PATH (ADR-022)."""
    if shutil.which("cargo") is None:
        return "missing tool: cargo (rustup, then rustup show in this repository)"
    protoc = os.environ.get("PROTOC")
    if protoc:
        if not (Path(protoc).is_file() and os.access(protoc, os.X_OK)):
            return "PROTOC names no executable protoc"
    elif shutil.which("protoc") is None:
        return (
            "missing tool: protoc (protoc 31.1 from github.com/protocolbuffers/protobuf "
            "releases, or set PROTOC)"
        )
    return None


def build(what):
    """One build of ingest in the workspace's own target, bounded; returns it and its seconds."""
    env = dict(os.environ, CARGO_TERM_COLOR="never")
    started = time.monotonic()
    try:
        done = subprocess.run(
            BUILD,
            cwd=REPO,
            env=env,
            capture_output=True,
            text=True,
            timeout=BUILD_BOUND_S,
            check=False,
        )
    except subprocess.TimeoutExpired as expired:
        raise AssertionError(
            f"the {what} build did not finish within {BUILD_BOUND_S} s: {' '.join(BUILD)}"
        ) from expired
    return done, time.monotonic() - started


#: A planted second build: two units Fresh, the proto build script dirty, two units compiled.
PLANTED_REBUILD = """\
       Fresh unicode-ident v1.0.19
   Compiling anki_proto v0.0.0 (/somewhere/rslib/proto)
       Dirty anki_proto v0.0.0 (/somewhere/rslib/proto): the file `/t/debug/build/anki_proto-1/out/anki.cards.rs` has changed (1790000000.1s, 1s after last build at 1790000000.0s)
       Fresh prost v0.14.1
   Compiling anki v0.0.0 (/somewhere/rslib)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 02s
"""  # noqa: E501
PLANTED_NO_OP = """\
       Fresh unicode-ident v1.0.19
       Fresh anki v0.0.0 (/somewhere/rslib)
       Fresh deck-streak-ingest v0.1.0 (/somewhere/crates/ingest)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.42s
"""

# ------------------------------------------------------------------------------------------ A3

#: The audit A3 reads. At the info level cargo deny prints a note for each advisory exception it
#: encounters (`advisory-ignored`) and for each crate a source allowance admits (`allowed-source`).
AUDIT = [
    "cargo",
    "deny",
    "--locked",
    "--log-level",
    "info",
    "--format",
    "json",
    "check",
    "advisories",
    "sources",
]
#: The audit fetches the advisory database, so its bound allows for a slow network.
AUDIT_BOUND_S = 600


def diagnostics_of(stderr):
    """The JSON objects cargo deny wrote, one per line."""
    return [json.loads(line) for line in stderr.splitlines() if line.startswith("{")]


def spans(diagnostics, code, label):
    """The spans of the labels called `label` in every diagnostic whose code is `code`."""
    return {
        each.get("span")
        for diagnostic in diagnostics
        if diagnostic.get("type") == "diagnostic"
        and diagnostic.get("fields", {}).get("code") == code
        for each in diagnostic["fields"].get("labels") or []
        if each.get("message") == label
    }


def exceptions_of(deny):
    """The advisory ids `deny.toml` ignores, and its allowed git sources."""
    config = tomllib.loads(deny)
    ignore = config.get("advisories", {}).get("ignore", [])
    ids = [entry.get("id") if isinstance(entry, dict) else entry for entry in ignore]
    return ids, list(config.get("sources", {}).get("allow-git", []))


def audit_findings(deny, diagnostics):
    """Why an exception or an allowed source in `deny.toml` is stale; empty when each is live."""
    ids, urls = exceptions_of(deny)
    ignored = spans(diagnostics, "advisory-ignored", "advisory ignored here")
    allowed = spans(diagnostics, "allowed-source", "source allowance")
    found = [
        f"{advisory}: deny.toml ignores it, and cargo deny encountered no crate it names"
        for advisory in ids
        if advisory not in ignored
    ]
    found += [
        f"{url}: allow-git names it, and no crate in the graph comes from it"
        for url in urls
        if url not in allowed
    ]
    return found


def note(code, *labels):
    """A planted diagnostic line of cargo deny's JSON format."""
    fields = {
        "code": code,
        "labels": [{"message": message, "span": span} for message, span in labels],
        "severity": "note",
    }
    return {"fields": fields, "type": "diagnostic"}


PLANTED_DENY_TOML = f"""\
[advisories]
ignore = [
    {{ id = "RUSTSEC-0000-0001", reason = "planted" }},
    {{ id = "RUSTSEC-0000-0002", reason = "planted" }},
]

[sources]
allow-git = ["{FORK}", "{RUST_URL}"]
"""
PLANTED_ENCOUNTERED = [
    note("advisory-ignored", ("advisory ignored here", "RUSTSEC-0000-0001")),
    note("allowed-source", ("source", f"git+{FORK}?rev={PLANTED_REV}"), ("source allowance", FORK)),
]
PLANTED_LIVE = [
    *PLANTED_ENCOUNTERED,
    note("advisory-ignored", ("advisory ignored here", "RUSTSEC-0000-0002")),
    note("allowed-source", ("source", f"git+{RUST_URL}"), ("source allowance", RUST_URL)),
]

# ------------------------------------------------------------------------------------------ A5

STATUS = re.compile(r"(?m)^status: (\S+)$")
#: The no-op build's table in ADR-058's Confirmation.
NO_OP_HEADER = ["measure", "before the pin", "after the pin"]
#: A note appended to ADR-058 that moves the pin (SPEC-338 R10), its patch table, and a diff stat's
#: count of files.
NOTE = re.compile(r"(?ms)^## Note, appended[^\n]*\n(.*?)(?=^## |\Z)")
PATCH_TABLE_HEADER = ["patch", "commit", "reason", "removal condition"]
FILES_CHANGED = re.compile(r"\b\d+ files? changed\b")
FULL_COMMIT = re.compile(r"`([0-9a-f]{40})`")
#: A number of seconds, or a range of them, at the start of a cell: "0.42 s", "31.4 to 34.5 s".
SECONDS = re.compile(r"^(\d+(?:\.\d+)?)(?: to (\d+(?:\.\d+)?))? s\b")
CI_RUN = re.compile(r"`ci\.yml` run \d+")
MEASURE_RUN = re.compile(r"`engine-measure\.yml` run \d+")


def cells(line):
    """A markdown table row's cells, or None for a line that is not a row."""
    if not line.startswith("|"):
        return None
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def table(text, header):
    """The rows under the markdown table whose header row is `header`, or None."""
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if cells(line) == header:
            rows = []
            for row in lines[index + 2 :]:
                if cells(row) is None:
                    break
                rows.append(cells(row))
            return rows
    return None


def section(text, heading):
    """The body of the `### <heading>` section, up to the next heading."""
    match = re.search(rf"(?ms)^### {re.escape(heading)}\n(.*?)(?=^#|\Z)", text)
    return match.group(1) if match else ""


def no_op_findings(confirmation):
    """Why the Confirmation's no-op build before and after the pin does not show the fix."""
    rows = [row for row in table(confirmation, NO_OP_HEADER) or [] if row[0].startswith("a no-op")]
    if len(rows) != 1:
        return ["ADR-058's Confirmation records no one no-op build before and after the pin"]
    before, after = (SECONDS.match(cell) for cell in rows[0][1:3])
    if before is None or after is None:
        return [f"the no-op build's cells {rows[0][1:3]} are not numbers of seconds"]
    fastest_before, after_s = float(before.group(1)), float(after.group(2) or after.group(1))
    if after_s > NO_OP_BOUND_S:
        return [f"the no-op build after the pin took {after_s:g} s, over R2's bound"]
    if after_s >= fastest_before:
        return [f"the no-op build after the pin ({after_s:g} s) is no faster than before it"]
    return []


def note_findings(note, rev, tag):
    """Why the note appended to ADR-058 does not record the pin it moved to: the commit's diff stat
    against the upstream tag, a patch table ending at the commit, and the two ADRs that decided it."""
    found = []
    stat = re.search(rf"(?m)^.*`git diff --stat {re.escape(tag)} {rev[:7]}[0-9a-f]*`.*$", note)
    if stat is None or FILES_CHANGED.search(stat.group(0)) is None:
        found.append(f"ADR-058's note records no `git diff --stat {tag} {rev[:7]}` count")
    rows = table(note, PATCH_TABLE_HEADER) or []
    last = rows[-1][1] if rows else "no row"
    if last != f"`{rev}`":
        found.append(f"ADR-058's note's patch table ends at {last}, not the pinned commit `{rev}`")
    found += [
        f"ADR-058's note names no {cited}" for cited in ("ADR-336", "ADR-348") if cited not in note
    ]
    return found


def confirmation_findings(adr, rev, tag):
    """Why ADR-058 does not record the pinned commit and what it saves; empty when it does."""
    found = []
    status = STATUS.search(adr)
    status = status.group(1) if status else None
    if status not in ("accepted", "superseded"):
        found.append(f"ADR-058's status is {status!r}, not final (accepted or superseded)")
    confirmation = section(adr, "Confirmation")
    notes = [note for note in NOTE.findall(adr) if rev is not None and f"`{rev}`" in note]
    if rev is None:
        found.append("the root manifest patches the engine to no commit, so none can be recorded")
    else:
        if notes:
            # SPEC-338 R10: a note moved the pin past the fix, so the Confirmation keeps judging the
            # fix's own commit, and the note judges the pinned one.
            found += note_findings(notes[-1], rev, tag)
            fix = FULL_COMMIT.search(confirmation)
            rev = fix.group(1) if fix else "no commit"
        if f"`{rev}`" not in confirmation:
            found.append(f"ADR-058's Confirmation does not name the pinned commit `{rev}`")
        stat = re.search(
            rf"(?m)^.*`git diff --stat {re.escape(tag)} {rev[:7]}[0-9a-f]*`.*$", confirmation
        )
        if stat is None:
            found.append(f"ADR-058's Confirmation records no `git diff --stat {tag} {rev[:7]}`")
        elif "`rslib/io/src/lib.rs`" not in stat.group(0) or not re.search(
            r"\b1 file changed\b", stat.group(0)
        ):
            found.append("ADR-058's diff stat does not show one file, `rslib/io/src/lib.rs`")
    found += no_op_findings(confirmation)
    if CI_RUN.search(confirmation) is None:
        found.append("ADR-058's Confirmation names no `ci.yml` run")
    if MEASURE_RUN.search(confirmation) is None:
        found.append("ADR-058's Confirmation names no `engine-measure.yml` run")
    return found


def planted_adr(
    status="accepted",
    rev=PLANTED_REV,
    stat="`rslib/io/src/lib.rs`, 1 file changed, 47 insertions(+), 2 deletions(-)",
    after="0.42 s",
    runs="`ci.yml` run 1 and `engine-measure.yml` run 2",
):
    """A planted ADR-058 whose Confirmation records `rev`, its diff stat and its no-op build."""
    return (
        f"---\nstatus: {status}\n---\n\n# A planted ADR\n\n### Confirmation\n\n"
        f"The pinned commit is `{rev}`.\n\n"
        f"| record | measured |\n|---|---|\n| `git diff --stat 0.0 {rev[:7]}` | {stat} |\n\n"
        f"| {' | '.join(NO_OP_HEADER)} |\n|---|---|---|\n"
        f"| a no-op `cargo build -p deck-streak-ingest` | 31.4 to 34.5 s | {after} |\n\n"
        f"CI measured it in {runs}.\n\n## What would make this wrong\n"
    )


#: A planted commit at the tip of the fork's wasm32 patches (ADR-348).
PLANTED_TIP = "fedcba9876543210fedcba9876543210fedcba98"


def planted_note(
    rev=PLANTED_TIP,
    stat="22 files changed, 338 insertions(+), 69 deletions(-)",
    last=PLANTED_TIP,
    cited="ADR-336 and ADR-348",
):
    """A planted note appended to ADR-058 that moves the pin to `rev`, its patch table ending at
    `last`."""
    return (
        f"\n## Note, appended by SPEC-0: the pin moves\n\n{cited} decided it. The pinned commit"
        f" is `{rev}`.\n\n| record | measured |\n|---|---|\n"
        f"| `git diff --stat 0.0 {rev[:7]}` on the fork | {stat} |\n\n"
        f"| {' | '.join(PATCH_TABLE_HEADER)} |\n|---|---|---|---|\n"
        f"| `first` | `{PLANTED_URL_REV}` | a reason | a condition |\n"
        f"| `last` | `{last}` | a reason | a condition |\n"
    )


# ----------------------------------------------------------------------------------------- tests


class TheEngineIsPatchedByRev(unittest.TestCase):
    def test_the_engine_is_patched_by_rev_to_a_commit_of_the_fork(self):
        self.assertEqual(pin_findings(planted_manifest(), planted_lock(), planted_deny()), [])
        # ADR-348: anki_proto may join anki, from the same commit of the fork.
        self.assertEqual(
            pin_findings(planted_manifest(others=PLANTED_PROTO), planted_lock(), planted_deny()),
            [],
        )
        upstream = f"git+{UPSTREAM}?tag={TAG}#{PLANTED_URL_REV}"
        refusals = {
            "anki_proto from another commit": (
                planted_manifest(
                    others=f'anki_proto = {{ git = "{FORK}", rev = "{PLANTED_URL_REV}" }}\n'
                ),
                planted_lock(),
                planted_deny(),
                [f"the patch takes anki_proto by rev {PLANTED_URL_REV}, not anki's {PLANTED_REV}"],
            ),
            "a third package patched": (
                planted_manifest(
                    others=PLANTED_PROTO
                    + f'anki_io = {{ git = "{FORK}", rev = "{PLANTED_REV}" }}\n'
                ),
                planted_lock(),
                planted_deny(),
                [
                    "the patch replaces ['anki', 'anki_io', 'anki_proto'], "
                    "not anki alone or anki with anki_proto"
                ],
            ),
            "a patch by branch": (
                planted_manifest(f'{{ git = "{FORK}", branch = "fix" }}'),
                planted_lock(),
                planted_deny(),
                [
                    "the patch pins anki by branch, not by rev alone",
                    f"anki comes from git+{FORK}?rev={PLANTED_REV}#{PLANTED_REV}, "
                    "not the fork's commit",
                    f"anki_io comes from git+{FORK}?rev={PLANTED_REV}#{PLANTED_REV}, "
                    "not the fork's commit",
                ],
            ),
            "an abbreviated rev": (
                planted_manifest(f'{{ git = "{FORK}", rev = "{PLANTED_REV[:7]}" }}'),
                planted_lock({"anki": f"git+{FORK}?rev={PLANTED_REV[:7]}#{PLANTED_REV}"}),
                planted_deny(),
                [
                    f"the patch's rev '{PLANTED_REV[:7]}' is not a full commit id",
                    f"anki comes from git+{FORK}?rev={PLANTED_REV[:7]}#{PLANTED_REV}, "
                    "not the fork's commit",
                    f"anki_io comes from git+{FORK}?rev={PLANTED_REV[:7]}#{PLANTED_REV}, "
                    "not the fork's commit",
                ],
            ),
            "an engine package left upstream": (
                planted_manifest(),
                planted_lock(
                    {
                        "anki": f"git+{FORK}?rev={PLANTED_REV}#{PLANTED_REV}",
                        "anki_io": upstream,
                    }
                ),
                planted_deny(),
                [f"anki_io comes from {upstream}, not the fork's commit"],
            ),
            "an unused patch": (
                planted_manifest(),
                planted_lock(
                    unused=(
                        '\n[[patch.unused]]\nname = "anki"\nversion = "0.0.0"\n'
                        f'source = "git+{FORK}?rev={PLANTED_REV}#{PLANTED_REV}"\n'
                    )
                ),
                planted_deny(),
                ["the lockfile records the patch as unused: ['anki']"],
            ),
            "upstream still allowed": (
                planted_manifest(),
                planted_lock(),
                planted_deny((UPSTREAM, FORK, RUST_URL, FSRS_RS)),
                [
                    f"allow-git is {[UPSTREAM, FORK, RUST_URL, FSRS_RS]}, "
                    "not exactly the fork, rust-url and fsrs-rs"
                ],
            ),
            "fsrs-rs not allowed": (
                planted_manifest(),
                planted_lock(),
                planted_deny((FORK, RUST_URL)),
                [f"allow-git is {[FORK, RUST_URL]}, not exactly the fork, rust-url and fsrs-rs"],
            ),
            "a comment that names neither": (
                planted_manifest(comment="# the fix"),
                planted_lock(),
                planted_deny(),
                ["the patch's comment names no ADR-058", "the patch's comment names no #233"],
            ),
        }
        for name, (manifest, lock, deny, refusal) in examined(
            "planted defect(s)", refusals.items()
        ):
            with self.subTest(name):
                self.assertEqual(pin_findings(manifest, lock, deny), refusal)
        manifest, lock, deny = (
            path.read_text(encoding="utf-8") for path in (MANIFEST, LOCKFILE, DENY)
        )
        engine = examined("engine package(s) in Cargo.lock", engine_packages(lock))
        self.assertIn("anki", [package["name"] for package in engine])
        self.assertEqual(pin_findings(manifest, lock, deny), [])


class TheFixHolds(unittest.TestCase):
    def test_a_second_build_of_ingest_recompiles_nothing(self):
        # The judge refuses a second build that rebuilt the engine, and passes one that did not.
        planted, fresh = build_findings(PLANTED_REBUILD)
        self.assertEqual(
            planted,
            [
                "the second build compiled 2 unit(s): anki_proto, anki",
                "cargo judged anki_proto dirty: the file `anki.cards.rs` has changed",
                "cargo's own time for the second build was 62.00 s, over R2's 10 s bound",
            ],
        )
        self.assertEqual(fresh, ["unicode-ident", "prost"])
        self.assertEqual(
            build_findings(PLANTED_NO_OP), ([], ["unicode-ident", "anki", "deck-streak-ingest"])
        )
        missing = need_build_tools()
        if missing is not None:
            self.fail(missing)
        first, first_s = build("first")
        if first.returncode != 0:
            self.fail(f"the first build failed (exit {first.returncode}): {first.stderr[-2000:]}")
        second, second_s = build("second")
        self.assertEqual(second.returncode, 0, second.stderr[-2000:])
        found, fresh = build_findings(second.stderr)
        examined("unit(s) the second build judged Fresh", fresh)
        print(
            f"deck-streak-ingest: the first build took {first_s:.1f} s of wall time and the "
            f"second {second_s:.1f} s"
        )
        self.assertEqual(found, [])


class DenyTomlIsLive(unittest.TestCase):
    def test_every_advisory_exception_and_git_source_in_deny_toml_is_live(self):
        self.assertEqual(audit_findings(PLANTED_DENY_TOML, PLANTED_LIVE), [])
        self.assertEqual(
            audit_findings(PLANTED_DENY_TOML, PLANTED_ENCOUNTERED),
            [
                "RUSTSEC-0000-0002: deny.toml ignores it, and cargo deny encountered no crate it "
                "names",
                f"{RUST_URL}: allow-git names it, and no crate in the graph comes from it",
            ],
        )
        if shutil.which("cargo-deny") is None:
            self.fail(
                "missing tool: cargo-deny (https://github.com/EmbarkStudios/cargo-deny releases, "
                "or taiki-e/install-action)"
            )
        deny = DENY.read_text(encoding="utf-8")
        ids, urls = exceptions_of(deny)
        examined("advisory exception(s) in deny.toml", ids)
        examined("allowed git source(s) in deny.toml", urls)
        try:
            done = subprocess.run(
                AUDIT, cwd=REPO, capture_output=True, text=True, timeout=AUDIT_BOUND_S, check=False
            )
        except subprocess.TimeoutExpired as expired:
            raise AssertionError(
                f"cargo deny did not finish within {AUDIT_BOUND_S} s: {' '.join(AUDIT)}"
            ) from expired
        diagnostics = diagnostics_of(done.stderr)
        summaries = [d for d in diagnostics if d.get("type") == "summary"]
        self.assertEqual(len(summaries), 1, f"cargo deny ran no check: {done.stderr[-2000:]}")
        self.assertEqual(audit_findings(deny, diagnostics), [])


class Adr058RecordsThePin(unittest.TestCase):
    def test_adr_058_records_the_pinned_commit_and_what_it_saves(self):
        self.assertEqual(confirmation_findings(planted_adr(), PLANTED_REV, "0.0"), [])
        # SPEC-338 R10: a note appended after the Confirmation moves the pin, and the Confirmation
        # keeps recording the fix's own commit.
        self.assertEqual(
            confirmation_findings(planted_adr() + planted_note(), PLANTED_TIP, "0.0"), []
        )
        # The pin a note moved is judged at the note's commit.
        note_refusals = {
            "a note whose patch table ends at another commit": (
                planted_adr() + planted_note(last=PLANTED_URL_REV),
                [
                    f"ADR-058's note's patch table ends at `{PLANTED_URL_REV}`, "
                    f"not the pinned commit `{PLANTED_TIP}`"
                ],
            ),
            "a note with no diff stat": (
                planted_adr() + planted_note(stat="a few files"),
                [f"ADR-058's note records no `git diff --stat 0.0 {PLANTED_TIP[:7]}` count"],
            ),
            "a note that names neither ADR": (
                planted_adr() + planted_note(cited="The seat"),
                ["ADR-058's note names no ADR-336", "ADR-058's note names no ADR-348"],
            ),
            "a note over a Confirmation that lost the fix's diff stat": (
                planted_adr(stat="`rslib/io/src/lib.rs`, 2 files changed") + planted_note(),
                ["ADR-058's diff stat does not show one file, `rslib/io/src/lib.rs`"],
            ),
        }
        for name, (text, refusal) in examined("planted note defect(s)", note_refusals.items()):
            with self.subTest(name):
                self.assertEqual(confirmation_findings(text, PLANTED_TIP, "0.0"), refusal)
        refusals = {
            "a proposed status": (
                planted_adr(status="proposed"),
                ["ADR-058's status is 'proposed', not final (accepted or superseded)"],
            ),
            "another commit recorded": (
                planted_adr(rev=PLANTED_URL_REV),
                [
                    f"ADR-058's Confirmation does not name the pinned commit `{PLANTED_REV}`",
                    f"ADR-058's Confirmation records no `git diff --stat 0.0 {PLANTED_REV[:7]}`",
                ],
            ),
            "a difference of two files": (
                planted_adr(stat="`rslib/io/src/lib.rs`, 2 files changed"),
                ["ADR-058's diff stat does not show one file, `rslib/io/src/lib.rs`"],
            ),
            "a no-op past the bound": (
                planted_adr(after="12.5 s"),
                ["the no-op build after the pin took 12.5 s, over R2's bound"],
            ),
            "no run named": (
                planted_adr(runs="a run"),
                [
                    "ADR-058's Confirmation names no `ci.yml` run",
                    "ADR-058's Confirmation names no `engine-measure.yml` run",
                ],
            ),
        }
        for name, (text, refusal) in examined("planted defect(s)", refusals.items()):
            with self.subTest(name):
                self.assertEqual(confirmation_findings(text, PLANTED_REV, "0.0"), refusal)
        adr = ADR_058.read_text(encoding="utf-8")
        self.assertIn("### Confirmation", adr)
        rev = patched_rev(MANIFEST.read_text(encoding="utf-8"))
        self.assertEqual(confirmation_findings(adr, rev, TAG), [])


if __name__ == "__main__":
    unittest.main()
