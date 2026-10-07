"""The per-review XP crate does no I/O, takes serde_json alone, and is drawn in the map (SPEC-360
A3 to A7; ADR-371 D1, D4 and D6).

`deck-streak-xp` holds progression's per-review XP rule, so a client can link it without the
kernel, the engine or any server context. This census reads, by name:

- the crate's manifest, by `tomllib`: serde_json alone, with `float_roundtrip`, and no build
  script, feature, target table or build dependency (A3);
- `Cargo.lock`: every package the crate reaches is on the allow-list (A4);
- the crate's source: `economy.json` embedded once, and no file, socket, clock, environment,
  process or thread API named (A5);
- the context map's fence and every member's manifest: the crate depends on nothing, and
  progression alone depends on it (A6);
- the rule's two homes: the rounding lives in the crate once, and progression keeps no copy (A7).

Each test judges the real tree first, then says how much it examined, then refuses each planted
defect by name, so a census that went blind fails on its own controls. A lockfile dependency is
written `name`, `name version` or `name version (source)`; all three are read, as
`test_fsrs7_pin.py` reads them.
"""

import re
import tomllib
import unittest

from _support import REPO, examined

CRATE = "deck-streak-xp"
CRATE_DIR = REPO / "crates" / "xp"
CRATE_MANIFEST = CRATE_DIR / "Cargo.toml"
CRATE_SOURCE = CRATE_DIR / "src"
CRATE_RULE = CRATE_SOURCE / "review_xp.rs"
LOCKFILE = REPO / "Cargo.lock"
MAP = REPO / "docs" / "CONTEXT-MAP.md"
MEMBERS = REPO / "crates"
PROGRESSION_RULE = REPO / "crates" / "progression" / "src" / "review_xp.rs"
PROGRESSION_TABLE = REPO / "crates" / "progression" / "src" / "economy_config.rs"
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"

#: Every package the crate's locked closure may hold: itself, serde_json, and what serde_json and
#: serde reach (SPEC-360 R5). A serde_json upgrade that adds a package is reviewed here.
ALLOWED = {
    CRATE,
    "serde_json",
    "serde",
    "serde_core",
    "serde_derive",
    "proc-macro2",
    "quote",
    "syn",
    "unicode-ident",
    "itoa",
    "memchr",
    "zmij",
}
#: The packages that do I/O, each refused by its own name beside the kernel.
IO_PACKAGES = ("sqlx", "tokio")
#: The members that depend on the crate, as the map draws them (ADR-371 D3); #639 extends it under
#: its own ADR.
EXPECTED_DEPENDENTS = ["deck-streak-progression"]
#: The one file the crate embeds, as its source spells it.
EMBED = 'include_str!("../../../economy.json")'
#: The names of a file, socket, clock, environment, process or thread API, and of any other way to
#: reach outside the crate's own source; each is refused where an identifier could not continue it.
REFUSED = (
    "std::fs",
    "std::net",
    "std::io",
    "std::time",
    "std::env",
    "std::process",
    "std::thread",
    "SystemTime",
    "Instant::",
    "env!(",
    "option_env!(",
    "include_bytes!(",
    "#[path",
    "extern crate",
)
REFUSED_PATTERNS = tuple(
    (token, re.compile(r"(?<![A-Za-z0-9_])" + re.escape(token))) for token in REFUSED
)
#: What progression's translation may never name again: the rule's rounding, its table and its
#: maturity boundary.
RULE_NAMES = ("round_ties_even", "economy_config", "mature_interval_days")
#: The per-review keys of `economy.json` that only the crate parses now.
TABLE_KEYS = (
    '"base"',
    '"ease_multipliers"',
    '"maturity"',
    '"type_multipliers"',
    '"multipliers"',
    '"untagged"',
)
DEPENDENCY = re.compile(r"^(?P<name>\S+)(?: (?P<version>[^\s(]+))?(?: \((?P<source>[^)]+)\))?$")
FENCE = re.compile(r"```context-map\n(.*?)```", re.S)


# ------------------------------------------------------------------------------------ A3: manifest


def manifest_findings(text, build_script=False):
    """Why the crate's manifest does not take serde_json alone, with exact floats, and declare no
    build script, feature, target table or build dependency; empty when it does."""
    if text is None:
        return [f"{CRATE} has no manifest"]
    manifest = tomllib.loads(text)
    found = []
    dependencies = manifest.get("dependencies", {})
    if sorted(dependencies) != ["serde_json"]:
        found.append(f"{CRATE}'s [dependencies] are {sorted(dependencies)}, not ['serde_json']")
    serde_json = dependencies.get("serde_json")
    if serde_json is not None and not (
        isinstance(serde_json, dict)
        and serde_json.get("workspace") is True
        and "float_roundtrip" in serde_json.get("features", [])
    ):
        found.append(f"{CRATE}'s serde_json is not the workspace's with float_roundtrip")
    beyond = sorted(set(manifest.get("dev-dependencies", {})) - {"serde", "serde_json"})
    if beyond:
        found.append(f"{CRATE}'s [dev-dependencies] reach {beyond}, beyond serde and serde_json")
    if "build" in manifest.get("package", {}):
        found.append(f"{CRATE}'s [package] names a build script")
    if build_script:
        found.append(f"{CRATE} has a build.rs")
    for table in ("features", "target", "build-dependencies"):
        if table in manifest:
            found.append(f"{CRATE} declares a [{table}] table")
    return found


# ---------------------------------------------------------------------------------------- A4: lock


def referenced(source):
    """A package's source as a dependency string writes it: a git source loses its `#<commit>`."""
    return source.split("#", 1)[0] if source is not None else None


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
        and spelled["source"] in (None, referenced(package.get("source")))
    ]
    return matches[0] if len(matches) == 1 else None


def refusal(name):
    """The finding for a package outside the allow-list, named by what it is."""
    if name == "deck-streak-kernel":
        return f"the closure of {CRATE} reaches the kernel, deck-streak-kernel"
    if name in IO_PACKAGES:
        return f"the closure of {CRATE} reaches {name}, which does I/O"
    if name.startswith("deck-streak-"):
        return f"the closure of {CRATE} reaches another workspace crate, {name}"
    return f"the closure of {CRATE} reaches {name}, which the allow-list does not hold"


def lock_findings(lock):
    """Why the crate's locked closure leaves the allow-list, and the names it reached. A member's
    entry lists its dev-dependencies too, so the walk judges them."""
    packages = tomllib.loads(lock).get("package", [])
    queue = [package for package in packages if package["name"] == CRATE]
    if not queue:
        return [f"Cargo.lock holds no {CRATE} package"], []
    found = []
    reached = {}
    while queue:
        package = queue.pop()
        key = (package["name"], package.get("version"), package.get("source"))
        if key in reached:
            continue
        reached[key] = package["name"]
        for dependency in package.get("dependencies", []):
            target = resolved(dependency, packages)
            if target is None:
                found.append(f"{package['name']}'s dependency {dependency!r} names no one package")
            else:
                queue.append(target)
    names = sorted(reached.values())
    found.extend(refusal(name) for name in sorted(set(names) - ALLOWED))
    return found, names


# -------------------------------------------------------------------------------------- A5: source


def source_findings(sources):
    """Why the crate's source, `{path: text}`, does not embed `economy.json` once or names an I/O
    API; empty when it does neither."""
    found = []
    embeds = sum(text.count(EMBED) for text in sources.values())
    if embeds != 1:
        found.append(f"crates/xp/src embeds economy.json {embeds} times, not once")
    for path, text in sorted(sources.items()):
        if text.count("include_str!(") > text.count(EMBED):
            found.append(f"{path} names include_str!( beyond economy.json")
        found.extend(
            f"{path} names {token}" for token, pattern in REFUSED_PATTERNS if pattern.search(text)
        )
    return found


def crate_sources():
    """Every Rust file under `crates/xp/src`, by its path from the root; empty before the crate."""
    if not CRATE_SOURCE.is_dir():
        return {}
    return {
        path.relative_to(REPO).as_posix(): path.read_text(encoding="utf-8")
        for path in sorted(CRATE_SOURCE.rglob("*.rs"))
    }


# ----------------------------------------------------------------------------------------- A6: map


def fence_lines(map_text):
    """Each context the map's fence draws, `{name: [the contexts it depends on]}`."""
    fence = FENCE.search(map_text)
    contexts = {}
    for line in fence.group(1).splitlines() if fence else []:
        if not line.strip():
            continue
        depends = line.rsplit("depends on:", 1)[-1].strip()
        listed = [] if depends in ("nothing", "nothing internal") else depends.split(", ")
        contexts[line.split()[0]] = listed
    return contexts


def manifest_names_crate(text):
    """Whether a member's manifest depends on the crate, under `[dependencies]` or any
    `[target.*.dependencies]`, by its key or by a `package` rename."""
    manifest = tomllib.loads(text)
    tables = [manifest.get("dependencies", {})]
    tables += [target.get("dependencies", {}) for target in manifest.get("target", {}).values()]
    return any(
        key == CRATE or (isinstance(entry, dict) and entry.get("package") == CRATE)
        for table in tables
        for key, entry in table.items()
    )


def map_findings(map_text, manifests):
    """Why the map's fence and the members' manifests, `{package name: text}`, do not agree that the
    crate depends on nothing and that progression alone depends on it; empty when they do."""
    contexts = fence_lines(map_text)
    if CRATE not in contexts:
        return [f"the map's fence holds no line for {CRATE}"]
    found = []
    if contexts[CRATE]:
        found.append(f"the map's {CRATE} line depends on {contexts[CRATE]}, not nothing")
    drawn = sorted(name for name, listed in contexts.items() if "xp" in listed)
    named = sorted(name for name, text in manifests.items() if manifest_names_crate(text))
    found.extend(
        f"{name}'s manifest depends on {CRATE} and its map line does not draw xp"
        for name in named
        if name not in drawn
    )
    found.extend(
        f"{name}'s map line draws xp and its manifest does not depend on {CRATE}"
        for name in drawn
        if name not in named
    )
    if drawn != EXPECTED_DEPENDENTS:
        found.append(f"the map draws xp for {drawn}, not {EXPECTED_DEPENDENTS}")
    return found


def member_manifests():
    """Every member's manifest, `{package name: text}`."""
    manifests = {}
    for path in sorted(MEMBERS.glob("*/Cargo.toml")):
        text = path.read_text(encoding="utf-8")
        manifests[tomllib.loads(text)["package"]["name"]] = text
    return manifests


# ---------------------------------------------------------------------------------------- A7: rule


def rule_findings(crate_rule, progression_rule, progression_table):
    """Why the rule does not live in the crate alone: its rounding once in the crate's
    `src/review_xp.rs`, and no copy of the rule or of its table in progression."""
    found = []
    rounds = (crate_rule or "").count(".round_ties_even()")
    if rounds != 1:
        found.append(f"crates/xp/src/review_xp.rs rounds half to even {rounds} times, not once")
    found.extend(
        f"crates/progression/src/review_xp.rs names {name}"
        for name in RULE_NAMES
        if name in progression_rule
    )
    found.extend(
        f"crates/progression/src/economy_config.rs names the key {key}"
        for key in TABLE_KEYS
        if key in progression_table
    )
    return found


# ------------------------------------------------------------------------------------- planted

GOOD_MANIFEST = f"""[package]
name = "{CRATE}"
version.workspace = true

[lints]
workspace = true

[dependencies]
serde_json = {{ workspace = true, features = ["float_roundtrip"] }}

[dev-dependencies]
serde.workspace = true
"""


def planted_manifest(dependencies="", dev_dependencies="", serde_json=None, tail=""):
    """The good manifest, with extra lines under its two tables and after it."""
    text = GOOD_MANIFEST.replace("[dev-dependencies]\n", f"[dev-dependencies]\n{dev_dependencies}")
    text = text.replace("[dependencies]\n", f"[dependencies]\n{dependencies}")
    if serde_json is not None:
        text = text.replace(
            'serde_json = { workspace = true, features = ["float_roundtrip"] }', serde_json
        )
    return text + tail


SERDE_CLOSURE = [
    ("serde_json", "1.0.151", ["itoa", "memchr", "serde", "serde_core", "zmij"]),
    ("serde", "1.0.229", ["serde_core", "serde_derive"]),
    ("serde_core", "1.0.229", ["serde_derive"]),
    ("serde_derive", "1.0.229", ["proc-macro2", "quote", "syn 3.0.6"]),
    ("proc-macro2", "1.0.107", ["unicode-ident"]),
    ("quote", "1.0.47", ["proc-macro2"]),
    ("syn", "2.0.119", ["proc-macro2", "quote", "unicode-ident"]),
    ("syn", "3.0.6", ["proc-macro2", "quote", "unicode-ident"]),
    ("unicode-ident", "1.0.26", []),
    ("itoa", "1.0.18", []),
    ("memchr", "2.8.3", []),
    ("zmij", "1.0.23", []),
]
GOOD_DEPENDENCIES = ["serde", f"serde_json 1.0.151 ({REGISTRY})"]


def planted_lock(crate=GOOD_DEPENDENCIES, extra=()):
    """A planted lockfile: the crate with the dependency strings `crate` (none when it is None),
    serde_json's closure from the registry, and each `(name, dependencies)` of `extra`."""
    body = "version = 4\n"
    if crate is not None:
        listed = ", ".join(f'"{dependency}"' for dependency in crate)
        body += f'\n[[package]]\nname = "{CRATE}"\nversion = "0.0.0"\ndependencies = [{listed}]\n'
    for name, version, dependencies in SERDE_CLOSURE:
        listed = ", ".join(f'"{dependency}"' for dependency in dependencies)
        body += (
            f'\n[[package]]\nname = "{name}"\nversion = "{version}"\nsource = "{REGISTRY}"\n'
            f"dependencies = [{listed}]\n"
        )
    for name, dependencies in extra:
        listed = ", ".join(f'"{dependency}"' for dependency in dependencies)
        body += f'\n[[package]]\nname = "{name}"\nversion = "0.1.0"\ndependencies = [{listed}]\n'
    return body


GOOD_TABLE = (
    "use std::sync::LazyLock;\n\n"
    f"const ECONOMY_FILE: &str = {EMBED};\n\n"
    "// The table reads no file, socket or clock: the file is the build's own.\n"
)
#: One planted line per refused name, each holding that name and no other.
PLANTED_LINES = {
    "std::fs": "use std::fs;",
    "std::net": "use std::net::TcpStream;",
    "std::io": "use std::io::Read;",
    "std::time": "use std::time::Duration;",
    "std::env": "use std::env;",
    "std::process": "use std::process::Command;",
    "std::thread": "use std::thread;",
    "SystemTime": "let now = SystemTime::now();",
    "Instant::": "let then = Instant::now();",
    "env!(": 'const HOME: &str = env!("HOME");',
    "option_env!(": 'const HOME: Option<&str> = option_env!("HOME");',
    "include_bytes!(": 'const BYTES: &[u8] = include_bytes!("planted.bin");',
    "#[path": '#[path = "planted.rs"]\nmod planted;',
    "extern crate": "extern crate alloc;",
}

GOOD_MAP = """# Context map

```context-map
deck-streak-kernel        (shared kernel)  depends on: nothing
deck-streak-ingest        (anti-corruption layer for Anki)  depends on: kernel
deck-streak-progression   (XP ledger and grant port, levels, tiers, badges, records, seasons)  depends on: kernel, ingest, xp
deck-streak-coordination  (cross-context steps)  depends on: kernel, ingest, progression
deck-streak-ffi           (the umbrella FFI crate)  depends on: nothing
deck-streak-xp            (the per-review XP rule, with no I/O)  depends on: nothing
```
"""


def planted_member(name, dependencies=()):
    """A member's manifest that depends on each of `dependencies`."""
    listed = "".join(f"{dependency}.workspace = true\n" for dependency in dependencies)
    return f'[package]\nname = "{name}"\n\n[dependencies]\n{listed}'


def planted_members(**overrides):
    """The planted members, progression alone on the crate, with `overrides` by short name."""
    members = {
        "kernel": planted_member("deck-streak-kernel"),
        "ingest": planted_member("deck-streak-ingest", ["deck-streak-kernel"]),
        "progression": planted_member(
            "deck-streak-progression", ["deck-streak-kernel", "deck-streak-ingest", CRATE]
        ),
        "coordination": planted_member(
            "deck-streak-coordination", ["deck-streak-kernel", "deck-streak-progression"]
        ),
        "ffi": planted_member("deck-streak-ffi"),
        "xp": planted_member(CRATE, ["serde_json"]),
    }
    members.update(overrides)
    return {f"deck-streak-{short}": text for short, text in members.items()}


GOOD_CRATE_RULE = "    (economy.base * ease * maturity * kind * tier).round_ties_even() as u32\n"
GOOD_TRANSLATION = (
    "//! A review's XP, translated at progression's edge into the XP crate's facts.\n"
    "use deck_streak_xp::review_xp::{ReviewFacts, Tier as XpTier};\n"
)
GOOD_PROGRESSION_TABLE = (
    'let daily = &xp["daily_bonuses"];\nlet excluded = &xp["day_base_excludes"];\n'
)
KEPT_RULE = (
    "use crate::economy_config::xp;\n"
    "let maturity = if review.interval >= economy.mature_interval_days {\n"
    "(economy.base * ease * maturity * kind * tier).round_ties_even() as u32\n"
)


class TheXpCrateDoesNoIo(unittest.TestCase):
    def test_the_xp_crate_takes_serde_json_alone_with_exact_floats(self):
        text = CRATE_MANIFEST.read_text(encoding="utf-8") if CRATE_MANIFEST.is_file() else None
        self.assertEqual(manifest_findings(text, (CRATE_DIR / "build.rs").exists()), [])
        manifest = tomllib.loads(text)
        examined(
            "dependency line(s) in crates/xp/Cargo.toml",
            [*manifest.get("dependencies", {}), *manifest.get("dev-dependencies", {})],
        )
        self.assertEqual(manifest_findings(GOOD_MANIFEST), [])
        refusals = {
            "a kernel edge": (
                planted_manifest(dependencies="deck-streak-kernel.workspace = true\n"),
                [
                    f"{CRATE}'s [dependencies] are ['deck-streak-kernel', 'serde_json'], not ['serde_json']"
                ],
            ),
            "sqlx": (
                planted_manifest(dependencies="sqlx.workspace = true\n"),
                [f"{CRATE}'s [dependencies] are ['serde_json', 'sqlx'], not ['serde_json']"],
            ),
            "tokio": (
                planted_manifest(dependencies="tokio.workspace = true\n"),
                [f"{CRATE}'s [dependencies] are ['serde_json', 'tokio'], not ['serde_json']"],
            ),
            "serde_json without float_roundtrip": (
                planted_manifest(serde_json="serde_json.workspace = true"),
                [f"{CRATE}'s serde_json is not the workspace's with float_roundtrip"],
            ),
            "a build key": (
                GOOD_MANIFEST.replace("version.workspace = true\n", 'build = "build.rs"\n'),
                [f"{CRATE}'s [package] names a build script"],
            ),
            "a [features] table": (
                planted_manifest(tail="\n[features]\nfast = []\n"),
                [f"{CRATE} declares a [features] table"],
            ),
            "a target table": (
                planted_manifest(tail="\n[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n"),
                [f"{CRATE} declares a [target] table"],
            ),
            "a [build-dependencies] table": (
                planted_manifest(tail='\n[build-dependencies]\ncc = "1"\n'),
                [f"{CRATE} declares a [build-dependencies] table"],
            ),
            "a tempfile dev-dependency": (
                planted_manifest(dev_dependencies="tempfile.workspace = true\n"),
                [f"{CRATE}'s [dev-dependencies] reach ['tempfile'], beyond serde and serde_json"],
            ),
            "no manifest": (None, [f"{CRATE} has no manifest"]),
        }
        for name, (planted, refused) in examined("planted manifest defect(s)", refusals.items()):
            with self.subTest(name):
                self.assertEqual(manifest_findings(planted), refused)
        with self.subTest("a build.rs beside the manifest"):
            self.assertEqual(
                manifest_findings(GOOD_MANIFEST, build_script=True), [f"{CRATE} has a build.rs"]
            )

    def test_the_xp_crates_locked_closure_is_the_allow_list(self):
        found, reached = lock_findings(LOCKFILE.read_text(encoding="utf-8"))
        self.assertEqual(found, [])
        examined("package(s) in the closure of deck-streak-xp", reached)
        self.assertEqual(lock_findings(planted_lock()), ([], sorted(ALLOWED)))
        refusals = {
            "a kernel edge": (
                planted_lock(
                    [*GOOD_DEPENDENCIES, "deck-streak-kernel"], [("deck-streak-kernel", [])]
                ),
                [f"the closure of {CRATE} reaches the kernel, deck-streak-kernel"],
            ),
            "sqlx through another package": (
                planted_lock([*GOOD_DEPENDENCIES, "helper"], [("helper", ["sqlx"]), ("sqlx", [])]),
                [
                    f"the closure of {CRATE} reaches helper, which the allow-list does not hold",
                    f"the closure of {CRATE} reaches sqlx, which does I/O",
                ],
            ),
            "tokio": (
                planted_lock([*GOOD_DEPENDENCIES, "tokio"], [("tokio", [])]),
                [f"the closure of {CRATE} reaches tokio, which does I/O"],
            ),
            "another workspace crate": (
                planted_lock(
                    [*GOOD_DEPENDENCIES, "deck-streak-ingest"], [("deck-streak-ingest", [])]
                ),
                [f"the closure of {CRATE} reaches another workspace crate, deck-streak-ingest"],
            ),
            "a dependency naming no one package": (
                planted_lock([*GOOD_DEPENDENCIES, "syn"]),
                [f"{CRATE}'s dependency 'syn' names no one package"],
            ),
            "no deck-streak-xp package": (None, [f"Cargo.lock holds no {CRATE} package"]),
        }
        for name, (planted, refused) in examined("planted lockfile defect(s)", refusals.items()):
            with self.subTest(name):
                lock = planted if planted is not None else planted_lock(crate=None)
                self.assertEqual(lock_findings(lock)[0], refused)

    def test_the_xp_crates_source_embeds_economy_json_once_and_names_no_io(self):
        sources = crate_sources()
        self.assertEqual(source_findings(sources), [])
        examined("Rust file(s) under crates/xp/src", sources)
        good = {"crates/xp/src/table.rs": GOOD_TABLE}
        self.assertEqual(source_findings(good), [])
        refusals = {
            f"names {token}": (
                {**good, "crates/xp/src/planted.rs": f"{line}\n"},
                [f"crates/xp/src/planted.rs names {token}"],
            )
            for token, line in PLANTED_LINES.items()
        }
        refusals["another include_str!"] = (
            {
                **good,
                "crates/xp/src/planted.rs": 'const OTHER: &str = include_str!("other.json");\n',
            },
            ["crates/xp/src/planted.rs names include_str!( beyond economy.json"],
        )
        refusals["economy.json embedded twice"] = (
            {**good, "crates/xp/src/planted.rs": GOOD_TABLE},
            ["crates/xp/src embeds economy.json 2 times, not once"],
        )
        refusals["economy.json not embedded"] = (
            {"crates/xp/src/table.rs": "use std::sync::LazyLock;\n"},
            ["crates/xp/src embeds economy.json 0 times, not once"],
        )
        self.assertEqual(sorted(PLANTED_LINES), sorted(REFUSED))
        for name, (planted, refused) in examined("planted source defect(s)", refusals.items()):
            with self.subTest(name):
                self.assertEqual(source_findings(planted), refused)

    def test_the_map_and_the_manifests_agree_that_progression_alone_depends_on_the_xp_crate(self):
        manifests = member_manifests()
        map_text = MAP.read_text(encoding="utf-8")
        self.assertEqual(map_findings(map_text, manifests), [])
        examined("member manifest(s)", manifests)
        examined("context-map fence line(s)", fence_lines(map_text))
        self.assertEqual(map_findings(GOOD_MAP, planted_members()), [])
        progression = (
            "depends on: kernel, ingest, xp\n",
            "depends on: kernel, ingest\n",
        )
        refusals = {
            "coordination's manifest naming the crate": (
                GOOD_MAP,
                planted_members(
                    coordination=planted_member(
                        "deck-streak-coordination", ["deck-streak-progression", CRATE]
                    )
                ),
                [
                    f"deck-streak-coordination's manifest depends on {CRATE} and its map line "
                    "does not draw xp"
                ],
            ),
            "progression's map line without xp": (
                GOOD_MAP.replace(*progression),
                planted_members(),
                [
                    f"deck-streak-progression's manifest depends on {CRATE} and its map line "
                    "does not draw xp",
                    f"the map draws xp for [], not {EXPECTED_DEPENDENTS}",
                ],
            ),
            "the umbrella FFI crate's map line naming xp with no manifest edge": (
                GOOD_MAP.replace(
                    "(the umbrella FFI crate)  depends on: nothing",
                    "(the umbrella FFI crate)  depends on: xp",
                ),
                planted_members(),
                [
                    f"deck-streak-ffi's map line draws xp and its manifest does not depend on {CRATE}",
                    "the map draws xp for ['deck-streak-ffi', 'deck-streak-progression'], "
                    f"not {EXPECTED_DEPENDENTS}",
                ],
            ),
            "the crate's own line depending on the kernel": (
                GOOD_MAP.replace(
                    "(the per-review XP rule, with no I/O)  depends on: nothing",
                    "(the per-review XP rule, with no I/O)  depends on: kernel",
                ),
                planted_members(),
                [f"the map's {CRATE} line depends on ['kernel'], not nothing"],
            ),
            "a target-only edge the map does not draw": (
                GOOD_MAP,
                planted_members(
                    ffi='[package]\nname = "deck-streak-ffi"\n\n'
                    f"[target.'cfg(target_os = \"ios\")'.dependencies]\n{CRATE}.workspace = true\n"
                ),
                [
                    f"deck-streak-ffi's manifest depends on {CRATE} and its map line does not draw xp"
                ],
            ),
            "no line for the crate": (
                GOOD_MAP.replace(
                    "deck-streak-xp            (the per-review XP rule, with no I/O)  depends on: "
                    "nothing\n",
                    "",
                ),
                planted_members(),
                [f"the map's fence holds no line for {CRATE}"],
            ),
        }
        for name, (planted_map, planted, refused) in examined(
            "planted map defect(s)", refusals.items()
        ):
            with self.subTest(name):
                self.assertEqual(map_findings(planted_map, planted), refused)

    def test_the_rule_lives_in_the_xp_crate_alone(self):
        crate_rule = CRATE_RULE.read_text(encoding="utf-8") if CRATE_RULE.is_file() else None
        progression_rule = PROGRESSION_RULE.read_text(encoding="utf-8")
        progression_table = PROGRESSION_TABLE.read_text(encoding="utf-8")
        self.assertEqual(rule_findings(crate_rule, progression_rule, progression_table), [])
        examined(
            "rule file(s) read",
            [path for path in (CRATE_RULE, PROGRESSION_RULE, PROGRESSION_TABLE) if path.is_file()],
        )
        self.assertEqual(
            rule_findings(GOOD_CRATE_RULE, GOOD_TRANSLATION, GOOD_PROGRESSION_TABLE), []
        )
        refusals = {
            "the rule kept in progression": (
                GOOD_CRATE_RULE,
                GOOD_TRANSLATION + KEPT_RULE,
                GOOD_PROGRESSION_TABLE,
                [f"crates/progression/src/review_xp.rs names {name}" for name in RULE_NAMES],
            ),
            "no rounding in the crate": (
                "    (economy.base * ease * maturity * kind * tier).round() as u32\n",
                GOOD_TRANSLATION,
                GOOD_PROGRESSION_TABLE,
                ["crates/xp/src/review_xp.rs rounds half to even 0 times, not once"],
            ),
            "the rounding twice in the crate": (
                GOOD_CRATE_RULE * 2,
                GOOD_TRANSLATION,
                GOOD_PROGRESSION_TABLE,
                ["crates/xp/src/review_xp.rs rounds half to even 2 times, not once"],
            ),
            "the per-review keys kept in progression": (
                GOOD_CRATE_RULE,
                GOOD_TRANSLATION,
                GOOD_PROGRESSION_TABLE + "".join(f"&xp[{key}]\n" for key in TABLE_KEYS),
                [
                    f"crates/progression/src/economy_config.rs names the key {key}"
                    for key in TABLE_KEYS
                ],
            ),
        }
        for name, (crate, progression, table, refused) in examined(
            "planted rule defect(s)", refusals.items()
        ):
            with self.subTest(name):
                self.assertEqual(rule_findings(crate, progression, table), refused)


if __name__ == "__main__":
    unittest.main()
