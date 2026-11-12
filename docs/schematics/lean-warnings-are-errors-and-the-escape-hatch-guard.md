# The Lean build's warnings are errors, and a guard refuses the escape hatches

- **Kind:** data flow, before and after.
- **Read at:** the development branch's commit `ac7ebd75`; every `path:line` below is at that
  commit.
- **Decided by:** ADR-440 (SPEC-426). It amends nothing: the settings flow keeps its own schematic,
  `docs/schematics/formal-check-settings.md`.

## Before: a warning passes the build

`formal/lean/lakefile.toml:1-8` declares one library, `Formal`, built by the glob `Formal.*` as the
package's one default target, with no option set. Two builders run `lake build` in `formal/lean`:
the formal checker, outside this repository, whose Lean budget `config/formal.json` names
(`budgets.lean_seconds`), and a person by hand. CI runs neither: no file names `lake build`, and
no workflow names Lean. A warning is printed and the exit code stays 0.

```mermaid
flowchart TD
    LAKE0["formal/lean/lakefile.toml<br/>lean_lib Formal, globs Formal.*<br/>no option"]
    MODS0["Formal/*.lean<br/>every module under Formal/"]
    BUILD0["lake build in formal/lean<br/>by the formal checker or by hand"]
    WARN0["a warning, printed"]
    EXIT0["exit 0<br/>the build passes"]
    CHECK0["the formal check reads a built package"]
    CI0["CI's python stage<br/>reads no Lean file"]

    LAKE0 --> BUILD0
    MODS0 --> BUILD0
    BUILD0 --> WARN0 --> EXIT0 --> CHECK0
    MODS0 -.->|no reader| CI0
```

## After: a warning fails the build, and the python stage reads every Lean file

The library's own `leanOptions` sets `warningAsError = true`, so every warning a module of the
library raises is an error and the build exits non-zero. `scripts/tests/test_formal_config.py`'s
class `LakefileWarnings` pins the lakefile, and `scripts/tests/test_lean_escape_hatches.py` reads
every tracked `.lean` file under `formal/lean`, both in the python stage that
`scripts/check.sh:185-186` runs in the `hygiene` job (`.github/workflows/ci.yml:298` and `:351`).

```mermaid
flowchart TD
    LAKE1["formal/lean/lakefile.toml<br/>lean_lib Formal<br/>leanOptions warningAsError = true"]
    MODS1["Formal/*.lean"]
    BUILD1["lake build in formal/lean"]
    ERR1["the warning, raised to an error"]
    FAIL1["non-zero exit<br/>the build fails"]
    CLEAN1["no warning<br/>exit 0"]

    subgraph PY["CI's python stage, hygiene job"]
        PIN["LakefileWarnings<br/>the document equals LAKEFILE<br/>warnings_are_errors answers no problem"]
        POP["git ls-files formal/lean/*.lean<br/>examined n, refused at 0<br/>Formal.lean among them"]
        LEX["lean_code<br/>comments and string text removed<br/>interpolations kept as code<br/>unterminated refused"]
        TOK["tokens<br/>names, guillemet names, numerals"]
        ARMS["ten arms<br/>sorry, admit, axiom, native_decide,<br/>implemented_by, extern, unsafe,<br/>linter-off, heartbeats-zero, nolint"]
        KEYOFF["key-off<br/>a module that names warningAsError"]
        ALLOW["ALLOWED<br/>0 entries: path, arm, count, reason"]
        RED["the stage fails<br/>path:line: arm"]
        GREEN["0 hits outside ALLOWED"]
    end

    LAKE1 --> BUILD1
    MODS1 --> BUILD1
    BUILD1 -->|a warning| ERR1 --> FAIL1
    BUILD1 -->|none| CLEAN1
    LAKE1 --> PIN
    PIN -.->|key absent, off or in args| RED
    MODS1 --> POP --> LEX --> TOK
    TOK --> ARMS --> ALLOW
    TOK --> KEYOFF --> RED
    ALLOW -->|a hit no entry admits| RED
    ALLOW -->|every hit admitted| GREEN
    LEX -.->|unterminated| RED
```

## The red-first proof and the oracle, read locally

CI builds no Lean, so the build's rule is read where Lean runs, with the Lean
`formal/lean/lean-toolchain` names, in exports of each commit's `formal/lean` outside the work
tree. The matcher's lexing is held to Lean's own elaboration of a table of planted files, whose
verdicts the guard module records as data.

```mermaid
flowchart LR
    BASE["base export<br/>plus Formal/PlantedWarning.lean"]
    HEAD["head export<br/>plus the same plant"]
    HEADC["head export<br/>no plant"]
    CACHE["base export, built<br/>given the head's lakefile"]
    OVER["head export<br/>plant under set_option warningAsError false"]
    B0["exit 0, the warning printed"]
    H1["non-zero exit, the diagnostic as an error"]
    HC["exit 0, no warning, no error"]
    C1["non-zero exit"]
    O1["exit and output as read"]
    PLANTS["ORACLE_PLANTS<br/>sorry in each lexical position"]
    LEAN["lean, each plant alone"]
    VERD["LEAN_WARNS<br/>declaration uses sorry, per plant"]
    AGREE["A7: the sorry arm refuses<br/>exactly the plants Lean warned for"]
    REC["docs/red-first/SPEC-426.md"]

    BASE --> B0 --> REC
    HEAD --> H1 --> REC
    HEADC --> HC --> REC
    CACHE --> C1 --> REC
    OVER --> O1 --> REC
    PLANTS --> LEAN --> VERD --> AGREE
    VERD --> REC
```

## Where each step lives

| step | where | read at `ac7ebd75` |
|---|---|---|
| the library and its glob | `formal/lean/lakefile.toml:6-8` | one `[[lean_lib]]`, `name = "Formal"`, `globs = ["Formal.*"]` |
| the default target | `formal/lean/lakefile.toml:2` | `defaultTargets = ["Formal"]` |
| the key | `formal/lean/lakefile.toml`, after line 8 | added by SPEC-426 R1 |
| the package's dependencies | `formal/lean/lake-manifest.json:3` | `"packages": []` |
| the pinned Lean | `formal/lean/lean-toolchain:1` | the one line the toolchain reads |
| the population | `git ls-files` under `formal/lean/` | 28 `.lean` files, 6164 lines |
| the stage | `scripts/check.sh:185-186` | `python3 -m unittest discover -s "$suite" -p 'test_*.py'` |
| the job | `.github/workflows/ci.yml:298`, `:351` | `hygiene`, `bash scripts/check.sh python scrub secrets` |
| the presence harness's population | `scripts/tests/test_formal_config_presence.py:177` | `FormalConfig`'s tests alone |

The formal checker's Lean check builds the package too. Its readings at the base and the head are
recorded in `docs/red-first/SPEC-426.md`, beside the build's own exit codes and diagnostic lines.
