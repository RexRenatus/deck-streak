# Red-first record: SPEC-369

SPEC-369, ADR-380, the SPEC-119 and SPEC-001 amendments and the schematic were committed first.
The red commit then added A1 to A9 over stubs that compile: `Scope::Write` with its name and bit,
`Scope::ALL` of three, `WRITE_CREDENTIAL`, a `Grants::load` that ignores the write credential, and
`WITHDRAWN` empty in `crates/mcp/tests/support/mod.rs`, which `roster()` in
`crates/mcp/tests/tools.rs` drops. Each red below is quoted from the run of its fence line at the
red commit, each reading `running 1 test`.

```red-first
A1: red at fe2591b27cb6dde1dacad30be2c0505dbbaa205d: assertion `left == right` failed left: [["core"], ["core", "law_track"]] right: [["core"], ["core", "law_track"], ["core", "write"]]
A2: not red: the stub's ignore of the write credential is the missing answer, so this test is a control that reads [["core"], ["core", "law_track"]] at the base and after
A3: red at fe2591b27cb6dde1dacad30be2c0505dbbaa205d: a empty write credential: Ok(Grants { scopes: [Scopes(1), Scopes(3)], .. })
A4: red at fe2591b27cb6dde1dacad30be2c0505dbbaa205d: a write token equal to the core token: Ok(Grants { scopes: [Scopes(1), Scopes(3)], .. })
A5: red at fe2591b27cb6dde1dacad30be2c0505dbbaa205d: assertion `left == right` failed: the write token: Answer { status: 401, headers: {"www-authenticate": "Bearer"}, body: [117, 110, 97, 117, 116, 104, 111, 114, 105, 122, 101, 100], seen: [] } left: [] right: [Some(["core", "write"])]
A6: not red: a guard; at the base it examines 7 source files, 1 tool and 0 write tools, after refusing its planted write tool by name
A7: red at fe2591b27cb6dde1dacad30be2c0505dbbaa205d: assertion `left == right` failed: the tools the portable roster leaves left: {} right: {"erase_all_data", "export_data"}
A8: not red: neither withdrawn tool is served at the base; its write leg reads 401 at the stub (the write token, tools/list: unauthorized), which is A1's reason, not this criterion's
A9: not red: a guard; the server reaches no rights use case at the base, and it examines 7 source files after refusing its planted line by name
A1: green at d0f672d27bf47334a4bb9e20fa97813ad16bb8e5
A3: green at d0f672d27bf47334a4bb9e20fa97813ad16bb8e5
A4: green at d0f672d27bf47334a4bb9e20fa97813ad16bb8e5
A5: green at d0f672d27bf47334a4bb9e20fa97813ad16bb8e5
A7: green at d0f672d27bf47334a4bb9e20fa97813ad16bb8e5
```

## Disclosures

- **Test-side edits between red and green:** `WITHDRAWN` in
  `crates/mcp/tests/support/mod.rs` gains its two names, `erase_all_data` and `export_data`, at the
  green commit. SPEC-369 section 3's base holds it empty. A6's census widened in its own commit,
  below.
- **A6 counts a tool not marked read-only as a write tool**
  (67239c71dbb135c3e8a42b1a0ae210e5a27afe24, before the green commit). MCP's `readOnlyHint`
  defaults to false, so the census now counts every
  tool whose annotations do not say `read_only_hint = true`, and SPEC-369 A6 and R7 say so. The
  fixture gained a second refused plant, `planted_unmarked`, a tool with no annotation that
  authorizes `Scope::Core`. With the plant and its assertions written and the census still reading
  `read_only_hint = false`, A6's fence line read red on the working tree over
  f6313bf047342db2e36861fc0aa9670b6917bb82: `left: ["planted_erase"] right: ["planted_erase",
  "planted_unmarked"]`. It reads green at the test commit, examining 7 source files, 1 tool and 0
  write tools: the one served tool says `read_only_hint = true`. A6's fence line above stands as
  it was measured at the red commit.
- **A shipped census moved with the red commit.** SPEC-119 A16
  (`crates/mcp/tests/guard_census.rs` `the_guard_compares_only_digests_in_constant_time`) pins
  where `ct_eq(` is called. SPEC-369 R4 and its row `S36907-WRITE-SHARED` add one constant-time
  comparison in `crates/mcp/src/grants.rs`, so the pin moved, in the red commit, from
  `["grants.rs", "guard.rs"]` to `["grants.rs", "grants.rs", "guard.rs"]`, with its module comment.
  The pin stays exact, and A16 keeps every other assertion. At the red commit it reads red for the
  write grant's missing comparison: `left: ["grants.rs", "guard.rs"] right: ["grants.rs",
  "grants.rs", "guard.rs"]`. It reads green at d0f672d27bf47334a4bb9e20fa97813ad16bb8e5.
- `crates/mcp/tests/guard.rs`'s `guard()` now builds through a new `guard_over(credentials)`, which
  A5 calls with the three credentials; `guard()` writes the same two credentials it wrote before.
- `crates/mcp/tests/support/mod.rs`'s `guard()` also writes the write credential, so the served
  stack holds three grants; at the stub the write token is refused, as A8's write leg shows.
