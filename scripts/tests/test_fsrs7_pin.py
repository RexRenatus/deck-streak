"""The engine and the FSRS-7 crate each resolve their own scheduler (SPEC-342 A1; ADR-338, ADR-353
D1 and D5).

The lockfile holds exactly two packages of the upstream scheduler crate, `fsrs`:

- the released one, from the registry, which the engine depends on, and which `deck-streak-fsrs7`
  reaches only through its coexistence test's dev-dependency (`fsrs6`);
- the pinned one, from fsrs-rs at the root manifest's full revision, which only `deck-streak-fsrs7`
  depends on (`fsrs7`).

The two share a name and a version string, so cargo writes a dependency on either as
`name version (source)`. A dependency on a package whose name is unique is written `name`, and one
whose name is shared by versions `name version`. All three forms are read.
"""

import re
import tomllib
import unittest

from _support import REPO, examined

MANIFEST = REPO / "Cargo.toml"
LOCKFILE = REPO / "Cargo.lock"
CRATE_MANIFEST = REPO / "crates" / "fsrs7" / "Cargo.toml"
#: The upstream scheduler's repository, the one source of FSRS-7 (ADR-338).
FSRS_RS = "https://github.com/open-spaced-repetition/fsrs-rs.git"
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
SCHEDULER = "fsrs"
CRATE = "deck-streak-fsrs7"
#: Who may depend on each package: the engine and the coexistence test on the released one, the
#: FSRS-7 crate alone on the pinned one.
RELEASED_USERS = ["anki", CRATE]
PINNED_USERS = [CRATE]
COMMIT = re.compile(r"[0-9a-f]{40}")
DEPENDENCY = re.compile(r"^(?P<name>\S+)(?: (?P<version>[^\s(]+))?(?: \((?P<source>[^)]+)\))?$")


def manifest_findings(manifest):
    """Why the root manifest's `fsrs7` entry is not fsrs-rs's `fsrs` by a full revision; returns
    the findings and the revision, which is None when there is none to judge the lock by."""
    entry = tomllib.loads(manifest).get("workspace", {}).get("dependencies", {}).get("fsrs7")
    if not isinstance(entry, dict):
        return ["the root manifest's [workspace.dependencies] has no fsrs7 entry"], None
    found = []
    if entry.get("package") != SCHEDULER or entry.get("git") != FSRS_RS:
        found.append(
            f"the fsrs7 entry takes {entry.get('package')} from {entry.get('git')}, "
            "not fsrs from fsrs-rs"
        )
    pins = sorted(key for key in entry if key not in ("package", "git"))
    if pins != ["rev"]:
        found.append(f"the fsrs7 entry pins by {', '.join(pins) or 'nothing'}, not by rev alone")
        return found, None
    if COMMIT.fullmatch(entry["rev"]) is None:
        found.append(f"the fsrs7 entry's rev {entry['rev']!r} is not a full commit id")
        return found, None
    return found, entry["rev"]


def resolved(dependency, packages):
    """The one package a lockfile dependency string names, or None."""
    spelled = DEPENDENCY.match(dependency)
    if spelled is None:
        return None
    matches = [
        package
        for package in packages
        if package["name"] == spelled["name"]
        and spelled["version"] in (None, package.get("version"))
        and spelled["source"] in (None, package.get("source"))
    ]
    return matches[0] if len(matches) == 1 else None


def lock_findings(lock, rev):
    """Why the lockfile does not hold one released and one pinned scheduler, each used only by its
    own consumers; empty when it does."""
    packages = tomllib.loads(lock).get("package", [])
    schedulers = [package for package in packages if package["name"] == SCHEDULER]
    released = [package for package in schedulers if package.get("source") == REGISTRY]
    pinned = [
        package for package in schedulers if package.get("source", "").startswith(f"git+{FSRS_RS}")
    ]
    if (len(schedulers), len(released), len(pinned)) != (2, 1, 1):
        return [
            f"the lockfile holds {len(schedulers)} scheduler package(s), {len(released)} from the "
            f"registry and {len(pinned)} from fsrs-rs, not one of each"
        ]
    found = []
    expected = f"git+{FSRS_RS}?rev={rev}#{rev}"
    if rev is not None and pinned[0]["source"] != expected:
        found.append(f"the git scheduler comes from {pinned[0]['source']}, not {expected}")
    users = {released[0]["source"]: [], pinned[0]["source"]: []}
    for package in packages:
        for dependency in package.get("dependencies", []):
            if not dependency.startswith(f"{SCHEDULER} ") and dependency != SCHEDULER:
                continue
            target = resolved(dependency, schedulers)
            if target is None:
                found.append(f"{package['name']}'s {dependency!r} names no one scheduler package")
            else:
                users[target["source"]].append(package["name"])
    for package, allowed, which in (
        (released[0], RELEASED_USERS, "registry's"),
        (pinned[0], PINNED_USERS, "git"),
    ):
        if sorted(users[package["source"]]) != allowed:
            found.append(
                f"the {which} scheduler is used by {sorted(users[package['source']])}, "
                f"not {allowed}"
            )
    return found


def crate_findings(crate):
    """Why `crates/fsrs7/Cargo.toml` does not take the pinned scheduler alone, with the released
    one for its tests alone; empty when it does."""
    if crate is None:
        return [f"{CRATE} has no manifest"]
    manifest = tomllib.loads(crate)
    found = []
    for table, allowed in (("dependencies", ["fsrs7"]), ("dev-dependencies", ["fsrs6"])):
        names = sorted(manifest.get(table, {}))
        if names != allowed:
            found.append(f"{CRATE}'s [{table}] are {names}, not {allowed}")
    return found


def scheduler_findings(manifest, lock, crate):
    """Every reason the engine and the FSRS-7 crate do not each resolve their own scheduler."""
    found, rev = manifest_findings(manifest)
    return found + lock_findings(lock, rev) + crate_findings(crate)


# ------------------------------------------------------------------------------------- planted

PLANTED_REV = "0123456789abcdef0123456789abcdef01234567"
PLANTED_OTHER = "89abcdef0123456789abcdef0123456789abcdef"
ELSEWHERE = "https://github.com/someone/fsrs-rs.git"
RELEASED = f"fsrs 6.6.2 ({REGISTRY})"


def pinned_source(rev=PLANTED_REV):
    return f"git+{FSRS_RS}?rev={rev}#{rev}"


PINNED = f"fsrs 6.6.2 ({pinned_source()})"


def planted_manifest(entry=None):
    entry = entry or f'{{ package = "fsrs", git = "{FSRS_RS}", rev = "{PLANTED_REV}" }}'
    return f"[workspace.dependencies]\nfsrs7 = {entry}\n"


def planted_lock(users=None, pinned=pinned_source()):
    """A planted lockfile: each user in `users` with its dependency strings, the released
    scheduler, and the pinned one from `pinned` unless it is None."""
    users = users or {"anki": [RELEASED], CRATE: [PINNED, RELEASED]}
    body = "version = 4\n"
    for name, dependencies in users.items():
        listed = ", ".join(f'"{dependency}"' for dependency in dependencies)
        body += f'\n[[package]]\nname = "{name}"\nversion = "0.0.0"\ndependencies = [{listed}]\n'
    body += f'\n[[package]]\nname = "fsrs"\nversion = "6.6.2"\nsource = "{REGISTRY}"\n'
    if pinned is not None:
        body += f'\n[[package]]\nname = "fsrs"\nversion = "6.6.2"\nsource = "{pinned}"\n'
    return body


def planted_crate(dependencies="fsrs7", dev_dependencies="fsrs6"):
    def table(names):
        return "".join(f"{name}.workspace = true\n" for name in names.split())

    return (
        f'[package]\nname = "{CRATE}"\n\n[dependencies]\n{table(dependencies)}\n'
        f"[dev-dependencies]\n{table(dev_dependencies)}"
    )


class EachResolvesItsOwnScheduler(unittest.TestCase):
    def test_the_engine_and_the_fsrs7_crate_each_resolve_their_own_scheduler(self):
        self.assertEqual(
            scheduler_findings(planted_manifest(), planted_lock(), planted_crate()), []
        )
        refusals = {
            "one scheduler package": (
                planted_manifest(),
                planted_lock({"anki": ["fsrs"], CRATE: ["fsrs"]}, pinned=None),
                planted_crate(),
                [
                    "the lockfile holds 1 scheduler package(s), 1 from the registry and 0 from "
                    "fsrs-rs, not one of each"
                ],
            ),
            "the engine on the git scheduler": (
                planted_manifest(),
                planted_lock({"anki": [PINNED], CRATE: [PINNED, RELEASED]}),
                planted_crate(),
                [
                    f"the registry's scheduler is used by [{CRATE!r}], not {RELEASED_USERS}",
                    f"the git scheduler is used by ['anki', {CRATE!r}], not {PINNED_USERS}",
                ],
            ),
            "another package on the git scheduler": (
                planted_manifest(),
                planted_lock(
                    {"anki": [RELEASED], CRATE: [PINNED, RELEASED], "deck-streak-ffi": [PINNED]}
                ),
                planted_crate(),
                [
                    f"the git scheduler is used by ['deck-streak-ffi', {CRATE!r}], "
                    f"not {PINNED_USERS}"
                ],
            ),
            "a dependency naming no one package": (
                planted_manifest(),
                planted_lock({"anki": ["fsrs 6.6.2"], CRATE: [PINNED, RELEASED]}),
                planted_crate(),
                [
                    "anki's 'fsrs 6.6.2' names no one scheduler package",
                    f"the registry's scheduler is used by [{CRATE!r}], not {RELEASED_USERS}",
                ],
            ),
            "the git scheduler at another revision": (
                planted_manifest(),
                planted_lock(
                    {
                        "anki": [RELEASED],
                        CRATE: [f"fsrs 6.6.2 ({pinned_source(PLANTED_OTHER)})", RELEASED],
                    },
                    pinned=pinned_source(PLANTED_OTHER),
                ),
                planted_crate(),
                [
                    f"the git scheduler comes from {pinned_source(PLANTED_OTHER)}, "
                    f"not {pinned_source()}"
                ],
            ),
            "a pin by branch": (
                planted_manifest(f'{{ package = "fsrs", git = "{FSRS_RS}", branch = "main" }}'),
                planted_lock(),
                planted_crate(),
                ["the fsrs7 entry pins by branch, not by rev alone"],
            ),
            "a short rev": (
                planted_manifest(
                    f'{{ package = "fsrs", git = "{FSRS_RS}", rev = "{PLANTED_REV[:7]}" }}'
                ),
                planted_lock(),
                planted_crate(),
                [f"the fsrs7 entry's rev {PLANTED_REV[:7]!r} is not a full commit id"],
            ),
            "another repository": (
                planted_manifest(
                    f'{{ package = "fsrs", git = "{ELSEWHERE}", rev = "{PLANTED_REV}" }}'
                ),
                planted_lock(),
                planted_crate(),
                [f"the fsrs7 entry takes fsrs from {ELSEWHERE}, not fsrs from fsrs-rs"],
            ),
            "no entry": (
                "[workspace.dependencies]\n",
                planted_lock(),
                planted_crate(),
                ["the root manifest's [workspace.dependencies] has no fsrs7 entry"],
            ),
            "the crate on the engine": (
                planted_manifest(),
                planted_lock(),
                planted_crate(dependencies="anki fsrs7"),
                [f"{CRATE}'s [dependencies] are ['anki', 'fsrs7'], not ['fsrs7']"],
            ),
            "a second dev-dependency": (
                planted_manifest(),
                planted_lock(),
                planted_crate(dev_dependencies="fsrs6 tempfile"),
                [f"{CRATE}'s [dev-dependencies] are ['fsrs6', 'tempfile'], not ['fsrs6']"],
            ),
            "no crate manifest": (
                planted_manifest(),
                planted_lock(),
                None,
                [f"{CRATE} has no manifest"],
            ),
        }
        for name, (manifest, lock, crate, refusal) in examined(
            "planted defect(s)", refusals.items()
        ):
            with self.subTest(name):
                self.assertEqual(scheduler_findings(manifest, lock, crate), refusal)
        manifest, lock = (path.read_text(encoding="utf-8") for path in (MANIFEST, LOCKFILE))
        crate = CRATE_MANIFEST.read_text(encoding="utf-8") if CRATE_MANIFEST.is_file() else None
        packages = tomllib.loads(lock).get("package", [])
        examined(
            "scheduler package(s) in Cargo.lock",
            [package for package in packages if package["name"] == SCHEDULER],
        )
        self.assertEqual(scheduler_findings(manifest, lock, crate), [])


if __name__ == "__main__":
    unittest.main()
