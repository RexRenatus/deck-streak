# SPEC-002: the skeleton and the local gate

- **Wave:** W0 (phase 1, the architect's delivery). **Issue:** {{issue:DS-W0-13}} (epic {{issue:EPIC-W0}}). **Context(s):** every crate (structure only), `repo`.
- **Decided by:** ADR-002, ADR-004, ADR-012, ADR-013, ADR-016, ADR-017.
- **Status:** judged: delivered with its tests in phase 1.

## 1. The problem, measured

Before any feature can be built red first, the repository needs the structure the gate reads and a
gate that runs every stage the same way locally and in CI:

- 24 bounded contexts must exist as crates whose manifests equal the context map, or the ddd probe
  refuses every later delivery (`python3 scripts/ddd-probe.py --root . check declared-edges-match-imports`).
- 25 packs' standard-library probes must run in public CI without the private phoenix-v2 source
  (ADR-004); 11 more (the ten built into phxd, and the proxy client scan, whose probe names a
  private secret) run only on the maintainer's box.
- A pack whose subject is not built yet reads VOID, which must never pass as green, and must never
  be forgotten either (`.packs/wiring.json`).
- The repository will be public: a private value that reaches it cannot be recalled.

## 2. Requirements

R1. The workspace holds exactly one crate per Rust context of docs/CONTEXT-MAP.md, each packaged
    `deck-streak-<context>`, each inheriting the workspace lint table (unsafe code forbidden,
    clippy pedantic on).
R2. The crate graph equals the context map in both directions.
R3. The vendored packs are byte-identical to the phoenix-v2 commit `.packs/VENDORED.json` names.
R4. Every vendored pack has a wiring state; every pending or deferred pack or row names the open
    issue that builds its subject.
R5. `scripts/check.sh` runs every stage (toolchain, fmt, clippy, test, doctest, web, python, packs,
    scrub, audit, secrets), fails a stage whose tool is missing, and is what CI runs.
R6. The parity oracle's generator writes strict goldens with the predecessor's commit and its own
    digest, and refuses a non-JSON number.
R7. A planned SPEC's number never collides with any other SPEC's.
R8. The public scrub refuses private shapes and literals by rule name, never echoing the value, and
    passes loopback, documentation addresses and version strings.
R9. The Mini App builds as a SvelteKit SPA with runes forced, svelte-check failing on warnings, and
    one Vitest and one Playwright smoke test.
R10. Every workflow runs on GitHub-hosted runners with a read-only token and actions pinned by full
    commit SHA, and the aggregate `ci` check needs every job.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every crate is named for its context directory | `cargo test -p deck-streak-daemon --test workspace -- --exact every_crate_is_named_for_its_context_directory` |
| A2 | every crate inherits the workspace lints | `cargo test -p deck-streak-daemon --test workspace -- --exact every_crate_inherits_the_workspace_lints` |
| A3 | the vendored packs match their recorded digests | `python3 -m unittest discover -s scripts/tests -p test_vendored_packs.py -k every_listed_file_has_its_recorded_digest` |
| A4 | every waiting pack or row names an open issue by number | `python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k every_waiting_pack_or_row_names_an_open_issue_by_number` |
| A5 | the oracle writes a golden with its provenance and Python's rounding | `python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k a_golden_records_its_provenance_and_rounds_half_to_even` |
| A6 | no SPEC number is held twice across judged and planned | `python3 -m unittest discover -s scripts/tests -p test_planned_specs.py -k no_number_is_held_twice_across_judged_and_planned` |
| A7 | an internal address is refused by rule name | `python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k an_internal_address_is_refused_by_rule_name` |
| A8 | the Mini App's smoke test observes its heading | `pnpm exec vitest run web/app/src/lib/smoke.test.ts -t "renders the DeckStreak heading"` |
| A9 | CI runs every stage of the local gate | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k ci_runs_every_stage_of_the_local_gate` |
| A10 | the pack runner refuses a wiring that forgets a pack | `python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k the_runner_refuses_a_wiring_that_forgets_a_pack` |

```acceptance
A1: cargo test -p deck-streak-daemon --test workspace -- --exact every_crate_is_named_for_its_context_directory
A2: cargo test -p deck-streak-daemon --test workspace -- --exact every_crate_inherits_the_workspace_lints
A3: python3 -m unittest discover -s scripts/tests -p test_vendored_packs.py -k every_listed_file_has_its_recorded_digest
A4: python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k every_waiting_pack_or_row_names_an_open_issue_by_number
A5: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k a_golden_records_its_provenance_and_rounds_half_to_even
A6: python3 -m unittest discover -s scripts/tests -p test_planned_specs.py -k no_number_is_held_twice_across_judged_and_planned
A7: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k an_internal_address_is_refused_by_rule_name
A8: pnpm exec vitest run web/app/src/lib/smoke.test.ts -t "renders the DeckStreak heading"
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k ci_runs_every_stage_of_the_local_gate
A10: python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k the_runner_refuses_a_wiring_that_forgets_a_pack
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `clippy.toml`, `deny.toml` | workspace | added |
| `crates/*/Cargo.toml`, `crates/*/src/lib.rs` | every Rust context | added: structure only, no feature code |
| `crates/daemon/tests/workspace.rs` | `deck-streak-daemon` | added |
| `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`, `web/app/**` | `miniapp` | added: the SvelteKit skeleton and its smoke tests |
| `methodology.json`, `stack.json`, `economy.json`, `notifications-policy.json` | repo | added |
| `.packs/**` | repo | added: the vendored packs (ADR-004) |
| `scripts/*.py`, `scripts/*.sh`, `scripts/tests/*.py` | repo | added: the gate, the pack runner, the scrub, the guards |
| `tools/parity-oracle/generate.py`, `tools/parity-oracle/test_generate.py` | repo | added |
| `.github/workflows/*.yml`, `.github/rulesets/*.json`, `.github/dependabot.yml` | repo | added |

## 5. What this does NOT do

- It writes no feature code: the kernel's types, the database base and every behaviour are W0's
  deliveries ({{issue:DS-W0-01}}).
- It runs no phxd-built pack in CI: those run on the maintainer's box until the open-source pack
  runner exists ({{issue:DS-W7-04}}).
- It creates no landing page: the `landing` context stays planned until W7 ({{issue:DS-W7-03}}).

## 6. Risks

- **The gate passes locally and fails in CI (or the reverse).** Detected because CI runs the same
  `scripts/check.sh` stages, and `test_ci_workflows.py` holds the workflow to every stage.
- **A vendored probe drifts from its source.** Detected by `test_vendored_packs.py`'s digests.
- **A pack stays pending forever.** Detected by `test_pack_wiring.py`, which requires an open issue
  for every waiting pack, and by W7's issue that enforces them all.
