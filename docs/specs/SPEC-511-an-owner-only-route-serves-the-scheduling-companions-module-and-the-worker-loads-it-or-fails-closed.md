# SPEC-511: An owner-only route serves the scheduling companion's module, and the Worker loads it or fails closed

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | #998 (the web engine carrying frames in the Worker beside a separately built module) |
| decided by | ADR-529, under ADR-348, ADR-349, ADR-336 and ADR-519 |
| measured at | `92755ef0` (every file this SPEC cites is byte-equal there and at the delivery's base) |
| schematic | `docs/schematics/the-owner-route-and-the-companion-loader.md` (new); `docs/schematics/context-map.md` (changed: one arrow) |
| mutation band | S51100-S51199 (no hand row: section 9) |

## 1. The problem, measured

SPEC-501 gave the scheduling companion its documented interface: integer-only frames, their codec, the decision that
fails closed and the public `Absent` slot. It left the web half to this change (SPEC-501 section 5, #998): where the
owner's companion module comes from, how the web client's Worker loads it, and what the Worker does when it cannot.
What was read at `92755ef0`:

- **No route serves a companion.** `crates/api/src/router.rs:222-283` merges 18 route groups, 17 of them under the
  owner's access; none names a companion. The owner's session extractor is identity's `OwnerSession`
  (`crates/identity/src/session.rs:369-396`), which answers 401 `no_session` to no session and to a `link` session.
- **The site is public.** The SvelteKit build is served from the web root by Caddy's last `handle`
  (`deploy/caddy/deck-streak.caddy:89-93`, `try_files {path} {path}.html /index.html`); every file under it is
  readable by anyone. Only `/api/*` reaches the API (`:46-51`). So a file that must reach the owner alone cannot sit
  in the site, and nothing checks today that none does.
- **The Worker loads one module.** `web/app/src/lib/engine/worker.ts:14-18` names the public engine's module and
  bindings under `/engine/`, and `browserDeps` imports the bindings' script from there (`:121-135`). Importing a
  script runs it with the Worker's whole scope: its storage, its network and its clock.
- **The web engine cannot decide.** `crates/web-engine/Cargo.toml` names 9 dependencies, every one under
  `[target.'cfg(target_arch = "wasm32")'.dependencies]`, and none is `deck-streak-companion`; no file under
  `crates/web-engine/src` names the companion (`git grep -c companion 92755ef0 -- crates/web-engine` prints nothing).
- **The size gate takes two files.** `scripts/web-engine-size.py` requires exactly `--module` and `--bindings` and sums
  their `gzip -9 -n` sizes against ADR-336's 8,000,000 bytes (`:20-21`, `:97-106`); a page that also loads a companion
  module and its weights downloads bytes the gate never counts.

## 2. Requirements

R1. **The route serves three fixed names.** `GET /api/companion/v1/manifest.json`, `/api/companion/v1/module.wasm` and
`/api/companion/v1/weights.bin` are three routes with no path parameter (`crates/api/src/companion_routes.rs`). Each
reads the file of its own name from the configured directory, and no part of a request reaches a file system path.
Any other path under `/api/companion/` answers 404, traversal forms included.

R2. **Owner session first, then the directory.** Each route's first extractor is identity's `OwnerSession`: no session,
an unknown session and a `link` session answer 401 `no_session`. With an owner session, a service with no directory,
a missing file and a file that is not a regular file answer 404 `companion_absent`, which is what every public
deployment answers.

R3. **What a served file carries.** A present file answers 200 with its exact bytes, its content type
(`application/json`, `application/wasm`, `application/octet-stream`) and `X-Content-Type-Options: nosniff`. Every
answer of the three routes, a refusal included, carries `Cache-Control: private, no-store`. A file larger than
`COMPANION_FILE_CAP`, 16,777,216 bytes (16 MiB), is refused 404 `companion_over_cap`, and no read takes more than the
cap plus one byte, so each answer's buffer is bounded.

R4. **The directory setting.** `DECKSTREAK_COMPANION_DIR` names the directory. Unset or empty, the routes answer 404
(the public default). Set to a path that is not absolute, the `api` role refuses start, naming the setting and never
its value. The daemon's `api` role reads it with its other settings, before it binds.

R5. **The Worker's loader fails closed.** `web/app/src/lib/engine/companion.ts` fetches the manifest from the route; a
manifest that does not answer 200, or is not exactly `{"module": <64 hex>, "weights": <64 hex>}`, reads `Absent`. It
fetches `module.wasm` and `weights.bin` and checks each one's SHA-256 with Web Crypto against the manifest; any other
answer, a file over 16 MiB, or one byte changed reads `Absent`. It imports no script from the route: it fetches only
the three names, and its source holds no `import(` and no `importScripts(`.

R6. **A fixed public import object.** The loader instantiates the module with `IMPORTS`, a frozen empty object defined
in public code, and with nothing else. A module that asks for any import, such as `env.x`, fails to link and reads
`Absent`; so does one that does not export `memory`, `alloc`, `free`, `open` and `call`. The module crosses a raw
byte-frame ABI: `alloc(len) -> ptr`, `free(ptr, len)`, `open(ptr, len) -> u64` and `call(ptr, len) -> u64`, the `u64`
packing the reply's pointer in its high 32 bits and its length in its low 32 bits. The loader copies the weights in
through `alloc`, calls `open`, copies the HelloReply frame out and frees both buffers; every later frame takes the
same path through `call`. The Worker moves bytes only.

R7. **`Absent` answers the canonical refusal.** An `Absent` companion's hello is the `Refused { Absent }` frame at epoch
0, and it answers every request with the `Refused { Absent }` frame at the request's epoch: byte-equal, at epoch 7, to
`formal/vectors/companion-frames.jsonl`'s `reply-refused`. A present companion whose `call` traps answers the same.
The web engine refuses every press over those frames (R10).

R8. **The checkpoint is stored verbatim, and purge removes it.** The Worker stores the companion's checkpoint frame
verbatim as `state.bin` in the origin private file system's directory `deck-streak-companion`, and reads it back
byte-equal. Purge sends the companion the `Purge` frame the web engine builds and removes the whole directory.

R9. **The web load check.** Before a stored checkpoint is kept, the Worker sends the companion the `Restore` frame the
web engine builds from the stored checkpoint (through `Restore::from(CheckpointReply)`, never from raw bytes), and the
web engine judges the companion's reply with `companion::load_check`. It keeps the checkpoint only when, in this
order: the hello negotiates; the stored frame decodes as a `Checkpoint` with status `Ok`; its model id is the
negotiated one and its adapter mode the hello's; the reply decodes as an `Ack` with status `Ok`; unless the `Ack`'s
`folded` is 0, its `head_t_ms` names a review in the store; and a fold index is held whose count and head equal the
`Ack`'s, or `folded` is 0. Anything else is one named discard, and the Worker purges and folds from genesis.

R10. **The web engine decides as the native build does.** `crates/web-engine/src/companion.rs` is compiled on every
target. Its `decide(hello, epoch, preview, reply)` takes the hello frame, the session's epoch, the `Preview` request
frame and the reply frame, runs the interface crate's `Session::new` and `decide_frame`, and answers 17 bytes for an
admitted due date (`0`, then `card` i64, `interval_days` u32 and `due_day` i32, little-endian) or 3 bytes for a refusal
(`1`, the refusal's code in `Refusal`'s declared order from 1, and the status for `Status`, else `0xff`). Its answers
over every decision vector of `formal/vectors/companion-decide.jsonl` are byte-equal to an answer built from the
interface crate's own `decide`, and agree with the vectors' outcomes. `wasm.rs` exports `companion_decide`,
`companion_restore`, `companion_load_check` and `companion_purge`, each a call of the module's function and nothing
else.

R11. **The size gate sums every file it is given.** `scripts/web-engine-size.py` takes `--owner-file PATH`, any number
of times, beside `--module` and `--bindings`, and sums every file's `gzip -9 -n` size against 8,000,000: the owner
lane's bundle is the public module and bindings plus the companion's module and weights. A missing or empty owner
file reads VOID, as the other two do.

R12. **The built site holds no file the route serves.** `scripts/companion-site-census.py --root DIR` reads every file
of a built site and refuses, by path: a file named `module.wasm` or `weights.bin`; any file under `api/`; a
`manifest.json` that names `module.wasm` or `weights.bin`; and a WebAssembly module outside `engine/`, where the
public engine is staged. It prints how many files it examined, exits 1 on a finding, and reads VOID (exit 2) for a
missing or empty site. CI's `web-engine` job runs it over the staged build.

R13. **The map carries the new edge.** The fence reads `deck-streak-web-engine ... depends on: companion, engine-core`;
the schematic draws `web_engine --> companion`.

R14. **Unchanged.** The interface crate and the slot, the engine core, every other route and client, the privacy
inventory and policy (R16), every migration and every mutation row.

R15. **The web-security rows of the route.** No traversal (R1: three literal routes, no request-derived path), no
cache (R3: `private, no-store` on every answer, refusals included), owner session (R2: 401 for no session and for a
`link` session). Each row has its own test in C1 to C6.

R16. **No device category yet.** Nothing in this change calls `CheckpointStore.save` on a product path: the press path
that asks the companion for a checkpoint is #997's, and it declares the `companion-state` category with its erase in the
same change (SPEC-501 R10). `privacy.json` and `PRIVACY.md` are unchanged.

## 3. Acceptance criteria of SPEC-511

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| C1 | R2, R15: no session, an unknown session and a `link` session get 401 on all three names, 9 of 9; an owner with no directory, and with a directory missing the file, gets 404 on each | the stub serves no route, so the refusals are 404 | ADR-529 D1 |
| C2 | R1, R3: each of the three names answers 200 with its exact bytes, its type, `nosniff` and `private, no-store` | the stub serves no route | ADR-529 D1 |
| C3 | R1, R15: 24 planted names, traversal and case forms among them, each answer 404 while a file of every name sits in the directory and its parent, and exactly the three names are served | the stub serves no route, so 0 names are served | ADR-529 D1 |
| C4 | R1: a route that serves a fourth name is refused by the same census | not red: the control holds over the stub as over the route | ADR-529 D1 |
| C5 | R3: a file of the cap is served, and one byte over it is refused `companion_over_cap` | the stub serves no route | ADR-529 D1 |
| C6 | R3, R15: every refusal of the three routes carries `private, no-store` | the stub serves no route | ADR-529 D1 |
| C7 | R4: the setting is off unset and empty, refuses start when relative without its value, and names an absolute directory | the stub reads every value as off | ADR-529 D1 |
| C8 | R5, R7: a manifest that answers 404 reads `Absent`, and `Absent` answers the committed refusal frame to all 9 committed requests | the stub loader admits every manifest | ADR-529 D2 |
| C9 | R5: a module or weights changed by one byte read `Absent`; a loader that skips the digest admits it, and the census refuses that loader | the stub loader checks no digest | ADR-529 D2 |
| C10 | R6: a module that imports `env.x` reads `Absent` under `IMPORTS`; a loader that passes a non-empty import object links it, and the census refuses that loader | the stub loader passes an import object with `env.x` | ADR-529 D2 |
| C11 | R5, R6: a present module is opened with the weights through `alloc`, answers through `call`, is given `IMPORTS` and nothing else, and only the three names are fetched; the source holds no script import | the stub loader passes another import object | ADR-529 D2 |
| C12 | R8: a checkpoint is stored and read back verbatim, and purge sends `Purge` and removes the directory | the stub store keeps nothing | ADR-529 D3 |
| C13 | R9: the Worker keeps a checkpoint only when the engine's load check keeps it, and otherwise purges and folds from genesis | the stub keeps every checkpoint | ADR-529 D3 |
| C14 | R9: `load_check` keeps one valid case and refuses each of its 8 discards, 9 of 9, and a check that skips the head clause is refused by the table | the stub check keeps everything | ADR-529 D3 |
| C15 | R9: the `Restore` frame the web engine builds carries the stored checkpoint's bytes verbatim, and an unreadable checkpoint builds none | the stub builds no frame | ADR-529 D3 |
| C16 | R10, R7: the web engine's `decide` answers all 191 decision vectors byte-equal with the native decision, and the `Absent` frames are refused | the stub answers one constant | ADR-529 D4 |
| C17 | R11: four files summing to 8,000,001 bytes `gzip -9 -n` fail the gate, the same set at 8,000,000 passes, and the module and bindings alone are under the budget | the gate refuses `--owner-file` | ADR-529 D5 |
| C18 | R12: a planted site with no served file passes and counts its files; each of 5 planted findings fails by path; an empty or missing site is VOID | the census does not exist | ADR-529 D5 |
| C19 | R13: the context map's tests are green with the new edge | the manifest names the edge and the fence and the drawing do not | ADR-529 D4 |

```acceptance
C1: cargo test -p deck-streak-api --test companion_route -- --exact no_session_gives_401_and_an_owner_with_no_directory_or_no_file_gives_404
C2: cargo test -p deck-streak-api --test companion_route -- --exact each_of_the_three_names_answers_its_exact_bytes_its_type_and_private_no_store
C3: cargo test -p deck-streak-api --test companion_route -- --exact every_other_name_and_every_traversal_form_gives_404_and_three_names_are_served
C4: cargo test -p deck-streak-api --test companion_route -- --exact a_route_that_serves_a_fourth_name_is_refused_by_the_census
C5: cargo test -p deck-streak-api --test companion_route -- --exact a_file_over_the_cap_is_refused_and_one_at_the_cap_is_served
C6: cargo test -p deck-streak-api --test companion_route -- --exact every_refusal_of_the_route_carries_private_no_store
C7: cargo test -p deck-streak-api --test companion_route -- --exact the_directory_setting_is_off_unset_and_refuses_start_when_relative
C8: pnpm exec vitest run web/app/src/lib/engine/companion.test.ts -t "a manifest that answers 404 reads Absent"
C9: pnpm exec vitest run web/app/src/lib/engine/companion.test.ts -t "a module or weights changed by one byte read Absent"
C10: pnpm exec vitest run web/app/src/lib/engine/companion.test.ts -t "a module that imports env.x reads Absent"
C11: pnpm exec vitest run web/app/src/lib/engine/companion.test.ts -t "a present module is opened with the weights"
C12: pnpm exec vitest run web/app/src/lib/engine/companion.test.ts -t "a checkpoint is stored and read back verbatim"
C13: pnpm exec vitest run web/app/src/lib/engine/companion.test.ts -t "the worker keeps a checkpoint only when the load check keeps it"
C14: cargo test -p deck-streak-web-engine --test companion_load -- --exact the_load_check_keeps_one_case_and_refuses_each_discard_by_name
C14: cargo test -p deck-streak-web-engine --test companion_load -- --exact a_load_check_that_skips_the_head_clause_is_refused_by_the_table
C15: cargo test -p deck-streak-web-engine --test companion_load -- --exact the_restore_frame_carries_the_stored_checkpoint_verbatim
C16: cargo test -p deck-streak-web-engine --test companion_decide -- --exact the_web_engines_decide_answers_every_vector_byte_equal_with_the_native_decision
C16: cargo test -p deck-streak-web-engine --test companion_decide -- --exact the_absent_frames_are_refused_by_the_web_engines_decide
C17: python3 -m unittest discover -s scripts/tests -p test_owner_bundle_size.py -k test_a_set_one_byte_over_fails_and_the_same_set_at_the_budget_passes
C18: python3 -m unittest discover -s scripts/tests -p test_companion_site_census.py
C19: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py
```

Beside the fence: `cargo test -p deck-streak-web-engine --test boundary` holds the four exports' bodies (R10), and
`python3 scripts/companion-site-census.py --root web/app/build` runs over the app's own build after
`pnpm --dir web/app run build` (R12). The rest of C19 is the hub's ddd probe, `check context-map-parses` and
`check declared-edges-match-imports`, run from its checkout with `--root` at this tree.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/api/Cargo.toml` | api | changed: tokio's `fs` and `io-util` features, for the bounded read |
| `crates/api/src/companion_routes.rs` | api | new: the three routes, the setting and the bounded read (R1 to R4) |
| `crates/api/src/router.rs` | api | changed: the state's directory, `with_companion`, and the routes merged under the owner's access |
| `crates/api/src/lib.rs` | api | changed: the module and its line in the crate's doc |
| `crates/api/tests/companion_route.rs` | api tests | new: C1 to C7 |
| `crates/daemon/src/role_api.rs` | daemon | changed: the `api` role reads the setting before it binds and hands the directory to the state |
| `deploy/deck-streak.env.example` | deploy | changed: the setting, empty, with its comment |
| `crates/web-engine/Cargo.toml` | web-engine | changed: the interface crate on every target; `serde_json` as the vectors reader's dev-dependency |
| `crates/web-engine/src/lib.rs` | web-engine | changed: the module and the crate doc's sentence on its native dependency |
| `crates/web-engine/src/companion.rs` | web-engine | new: `decide`, `restore`, `load_check`, `purge` (R9, R10) |
| `crates/web-engine/src/wasm.rs` | web-engine | changed: the four exports (R10) |
| `crates/web-engine/tests/companion_decide.rs` | web-engine tests | new: C16 |
| `crates/web-engine/tests/companion_load.rs` | web-engine tests | new: C14, C15 |
| `crates/web-engine/tests/boundary.rs` | web-engine tests | changed: the four exports' owed statements |
| `Cargo.lock` | workspace | changed: the web engine's dependency list names the interface crate |
| `web/app/src/lib/engine/companion.ts` | web | new: the loader, `Absent`, the checkpoint store and the load sequence (R5 to R9) |
| `web/app/src/lib/engine/companion.test.ts` | web tests | new: C8 to C13 |
| `web/app/src/lib/engine/companion-modules.test.support.ts` | web tests | new: the hand-assembled test modules |
| `scripts/web-engine-size.py` | scripts | changed: `--owner-file` (R11) |
| `scripts/tests/test_owner_bundle_size.py` | scripts tests | new: C17 |
| `scripts/companion-site-census.py` | scripts | new: the census of a built site (R12) |
| `scripts/tests/test_companion_site_census.py` | scripts tests | new: C18 |
| `.github/workflows/ci.yml` | ci | changed: the `web-engine` job runs the census over the staged build |
| `docs/CONTEXT-MAP.md` | docs | changed: the web engine's fence row and its prose |
| `docs/schematics/context-map.md` | docs | changed: the arrow and its prose |
| `docs/schematics/the-owner-route-and-the-companion-loader.md` | docs | new: the loader's sequence and the data flow |
| `docs/specs/SPEC-511-an-owner-only-route-serves-the-scheduling-companions-module-and-the-worker-loads-it-or-fails-closed.md` | docs | new: this file |
| `docs/decisions/ADR-529-the-owners-companion-module-is-served-by-an-owner-only-route-and-instantiated-with-a-fixed-public-import-object.md` | docs | new: the decisions |
| `docs/red-first/SPEC-511.md` | docs | new: the red-first record |
| `changelog.d/owner-route-serves-scheduling-companion-511.md` | changelog | new: the fragment |

## 5. What this does NOT cover

- The press path asking the companion: the preview, write and commit steps, the fold and the catch-up, the checkpoint
  written on a product path, the fold index beside it, the engine's answer whether a review id names a review, and
  the `companion-state` privacy category with its erase (#997). Until then the Worker's load sequence is called by no
  product path, and with no fold index held `load_check` discards every checkpoint with `folded` above 0.
- The device erase routine that runs purge on the device's own erase and on a pull that answers `Reset` (#997).
- The owner's module, its weights and its manifest: built, deployed and held outside the web root by the owner's own
  lane, never by this repository (#998).
- The wasm32 build of the four exports run in a browser over the vectors: UNMEASURED here (section 7).

## 6. Risks

- **A substituted module.** The manifest comes from the same route as the module, so its digest detects corruption,
  not substitution. Substitution is bounded by `IMPORTS`: a module that asks for any import fails to link (C10), so a
  substituted module reads no storage, network or clock; its wrong replies meet `decide`'s range checks (SPEC-501 R4).
- **A route that serves more than it names.** Held by C3's census of 24 names with a file of every name planted, and
  by C4's control.
- **A cached copy.** Held by C2 and C6: every answer carries `private, no-store`.
- **Memory.** Each read is capped at 16 MiB plus one byte; the shell's 64 requests in flight bound the total, and only
  the owner's session reaches a read.
- **A torn checkpoint write.** A checkpoint cut short no longer decodes, and `load_check` discards it (C14's
  `unreadable`), so the Worker folds from genesis rather than restoring it.
- **The companion's files in the public site.** Held by C18's census and its run over the build in CI.

## 7. What only CI proves

The workspace's clippy and tests (`rust`), the release profile's tests (`release`), the web engine's wasm32 build of
the four exports with its size gate (`web-engine`), the census over the staged build (`web-engine`), the Mini App's
check, unit tests and build (`web`), and the mutation jobs, each read by name at the head. The engine and study
browser suites (`test:engine`, `test:study`) are not changed by this delivery. The four exports are not called in a
browser by any suite: their answers on wasm32 are UNMEASURED until a suite drives them over the vectors.

## 8. Formal model

- **Lean and TLA+: NOT APPLICABLE here.** The decision this change reaches is SPEC-501's, whose Lean entry and vectors
  already cover it; C16 runs those vectors through the web engine. The loader and the route hold no interleaving the
  existing models do not.
- **Kani: NOT APPLICABLE here.** The new Rust holds no arithmetic beyond the interface's; the codec's harnesses stay
  planned under SPEC-501.

## 9. Mutation rows

None. The new code has no hand-held property beyond its tests: cargo-mutants and Stryker generate its mutants, and C1
to C18 are their killers. The band S51100-S51199 holds no row.
