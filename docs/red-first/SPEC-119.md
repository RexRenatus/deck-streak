# Red-first record: SPEC-119

This record is the first part's, #158's pull request: the bearer guard as a library of the new
crate `deck-streak-mcp`. It records the criteria this pull request delivers, A2 to A6, A9 to A12,
A15 to A21 and A40 (SPEC-119 sections 3 and 11); #157 and the drill tools add the lines of the
criteria they deliver when they move them back into the acceptance fence (section 3c).

The order of work: the SPEC moved out of `docs/specs/planned/` with its amendments and ADR-320;
the registry and the three guard goldens, generated at the predecessor's `27ee2bc`; the
BearerGuard model with its witnesses, checked before any guard code; the crate's tests beside
stubs that compile and answer nothing; then their implementation.

Each criterion is run at its red commit, selecting its own test, and fails by assertion, not by a
compile error, a missing fixture or an empty selection. At the stubs' commit the grants load no
grant and refuse nothing, the guard refuses every request with an empty bucket and its refusal
answers 200 with an empty body, the limiter's constants are 0 and its buckets are empty strings,
and no `ct_eq` exists. All 17 tests of the four files were red there, and all 17 are green at the
implementation, where clippy with `-D warnings` was clean.

A20's first assertion is the core token's admission, so at the stubs, which admit no token, it is
red before its ten scope failures can run; the scope failures, the second admission and the
bucket's five failures are each asserted after it at the green commit. A16's census was committed
with the stubs, before any code: its planted control was green there, and its first positive
artifact, the two `ct_eq` callers, was red. A15's failure text below writes the core token as
`<core token>`; the test builds it from parts at run time.

```red-first
A2: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: the_core_credential_is_required panicked at crates/mcp/tests/settings.rs:78: a missing core credential: Ok(Grants { scopes: [], .. })
A2: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A3: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: a_missing_scope_credential_grants_nothing panicked at crates/mcp/tests/settings.rs:114: assertion `left == right` failed, left [] right [["core"]]
A3: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A4: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: a_broken_scope_credential_refuses_start panicked at crates/mcp/tests/settings.rs:146: a empty law-track credential: Ok(Grants { scopes: [], .. })
A4: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A5: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: a_short_credential_refuses_start panicked at crates/mcp/tests/settings.rs:154: a 31-character core token: Ok(Grants { scopes: [], .. })
A5: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A6: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: two_credentials_with_one_value_refuse_start panicked at crates/mcp/tests/settings.rs:211: two credentials with one value: Ok(Grants { scopes: [], .. })
A6: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A9: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: a_request_with_no_bearer_is_refused panicked at crates/mcp/tests/guard.rs:139: initialize with no Authorization header, left 200 right 401
A9: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A10: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: a_malformed_bearer_is_refused panicked at crates/mcp/tests/guard.rs:139: an empty header, left 200 right 401
A10: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A11: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: a_wrong_bearer_is_refused panicked at crates/mcp/tests/guard.rs:139: a token differing in its last character, left 200 right 401
A11: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A12: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: every_request_refusal_is_the_same_response panicked at crates/mcp/tests/guard.rs:139: absent, left 200 right 401
A12: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A15: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: a_token_in_the_query_string_is_ignored panicked at crates/mcp/tests/guard.rs:139: /mcp?token=<core token>, left 200 right 401
A15: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A16: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: the_guard_compares_only_digests_in_constant_time panicked at crates/mcp/tests/guard_census.rs:260: assertion `left == right` failed, left [] right ["grants.rs", "guard.rs"]
A16: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A17: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: the_limiter_matches_its_golden panicked at crates/mcp/tests/limiter.rs:175: the outcomes of Some("sixth-failure"), six "denied" where the golden's sixth is "rate_limited"
A17: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A18: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: the_bucket_matches_its_golden panicked at crates/mcp/tests/limiter.rs:210: token "synthetic-bucket-token", left "" right "3f2b438df4401cf2"
A18: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A19: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: the_guard_constants_match_the_golden panicked at crates/mcp/tests/limiter.rs:242: assertion `left == right` failed, left "0" right "60000"
A19: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A20: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: a_granted_token_never_limits_itself panicked at crates/mcp/tests/limiter.rs:272: the core token is admitted: Err(Refusal { outcome: Denied, bucket: Bucket("") })
A20: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A21: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: a_refusal_logs_the_bucket_and_never_the_token panicked at crates/mcp/tests/limiter.rs:423: assertion `left == right` failed, left [] right nine `WARN deck_streak_mcp::guard outcome=.. bucket=.. scope=..` lines
A21: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
A40: red at 3b7ab54213d2d89cf514cfbb384d8a6263990435: each_grant_holds_its_scopes_and_no_other panicked at crates/mcp/tests/guard.rs:340: the core token, left [] right [Some(["core"])]
A40: green at 0d677be17905ee568f2e3e5e70f6149663d9dc30
```

## Mutation round 0: the tests that kill the guard's surviving mutants

Each test lives in `crates/mcp/tests/scopes_and_service.rs` and was run against its own hand-applied
plant before it was committed; the plant was restored byte-equal.

```
M1: plant crates/mcp/src/grants.rs:54 `self.0 | scope.bit()` -> `self.0 ^ scope.bit()`: adding_a_scope_already_held_keeps_it_held panicked at crates/mcp/tests/scopes_and_service.rs:75:5: assertion `left == right` failed: core added twice, left Scopes(0) right Scopes(1)
M3: plant crates/mcp/src/grants.rs `<impl fmt::Debug for Grants>::fmt` -> `Ok(Default::default())`: the_grants_debug_names_its_type_and_never_a_token panicked at crates/mcp/tests/scopes_and_service.rs:87:5
M5: plant crates/mcp/src/guard.rs `GuardService::poll_ready` -> `Poll::from(Ok(()))`: the_guard_service_is_pending_while_its_inner_service_is panicked at crates/mcp/tests/scopes_and_service.rs:95:5
S11930: row on crates/mcp/src/settings.rs `MIN_CREDENTIAL_CHARS` 32 -> 31, killer settings::a_short_credential_refuses_start
```

## #157, part a: the server's first slice

This section records the criteria the server's first slice delivers (SPEC-119 sections 13 and 14):
A1, A13, A14, A22 to A25, A33, A38, A39 and A41 to A45. The order of work: the roster golden; the
amendments and ADR-329; the tests beside stubs that compile and serve nothing, in one test-only
commit; then the implementation, the mutation rows, and the BearerGuard re-read with its two new
covers.

Each red line below is the reading at the red commit itself. Every test of the five files was run
there, in a scratch worktree of that commit, and only the criteria's own tests failed in each
file: 2 of `guard`'s 8, all 7 of `server`'s, 2 of `settings`'s 7, all 3 of `tools`'s, and 1 of the
daemon's `roles`' 18. Earlier red logs, taken on a draft of the tests before the red commit, are
superseded: their panic lines do not match the committed tests, so none of them is quoted here.

A25 and A44 are red at the red commit on their own precondition, not on their own assertion:
nothing is served there, so no granted call reaches the law track and neither test reaches its
slot assertion. Their own assertions were read after the green commit, in a scratch copy of the
crate with only the layer the criterion names removed, and are disclosed as that copy's reading:
- A25, the in-flight limit's line deleted: `the_ninth_request_in_flight_is_shed` panicked at
  `crates/mcp/tests/server.rs:156`: the ninth request, left 200 right 503.
- A44, the guard's layer moved inside the bound: `a_refused_request_holds_no_slot` panicked at
  `crates/mcp/tests/server.rs:211`: refusal 0 with every slot held, left 503 right 401.

A1's failure text writes the address as "the unspecified address"; A24's writes the test's
ephemeral port as `<port>`. A38's failure is the exit status of an unknown role; its usage line
named the roles `api, bot, job, data`.

```red-first
A1: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: the_listen_address_must_be_loopback panicked at crates/mcp/tests/settings.rs:268: the unspecified address was not refused as NotLoopback
A1: green at 8b96d52f8eea54800eb75b460610508075810714
A13: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: each_grant_reaches_its_scopes_and_no_other panicked at crates/mcp/tests/guard.rs:376: a granted call is answered, left 404 right 200
A13: green at 8b96d52f8eea54800eb75b460610508075810714
A14: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: a_tool_outside_the_scope_answers_unauthorized panicked at crates/mcp/tests/guard.rs:376: a granted call is answered, left 404 right 200
A14: green at 8b96d52f8eea54800eb75b460610508075810714
A22: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: only_the_mcp_path_is_served panicked at crates/mcp/tests/server.rs:77: /mcp, left 404 right 200
A22: green at 8b96d52f8eea54800eb75b460610508075810714
A23: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: a_body_over_64_kib_is_refused panicked at crates/mcp/tests/server.rs:94: a body of 64 KiB and one byte, left 404 right 413
A23: green at 8b96d52f8eea54800eb75b460610508075810714
A24: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: a_foreign_host_is_refused panicked at crates/mcp/tests/server.rs:123: Host example.invalid:<port>, left 404 right 403
A24: green at 8b96d52f8eea54800eb75b460610508075810714
A25: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: the_ninth_request_in_flight_is_shed panicked at crates/mcp/tests/server.rs:146: only 0 of 8 calls reached the law track within 5s
A25: green at 8b96d52f8eea54800eb75b460610508075810714
A33: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: the_law_track_answers_numbers_only panicked at crates/mcp/tests/tools.rs:62: get_law_track, left 404 right 200
A33: green at 8b96d52f8eea54800eb75b460610508075810714
A38: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: the_mcp_role_is_a_known_role panicked at crates/daemon/tests/roles.rs:937: the exit status, left Some(2) right Some(1)
A38: green at 8b96d52f8eea54800eb75b460610508075810714
A39: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: the_role_reads_its_tokens_only_through_the_loader panicked at crates/mcp/tests/settings.rs:372: the mcp role's source is missing: crates/daemon/src/role_mcp.rs
A39: green at 8b96d52f8eea54800eb75b460610508075810714
A41: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: each_served_tool_matches_its_roster_golden_entry panicked at crates/mcp/tests/tools.rs:136: tools/list, left 404 right 200
A41: green at 8b96d52f8eea54800eb75b460610508075810714
A42: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: the_pending_law_numbers_answer_null_never_zero panicked at crates/mcp/tests/tools.rs:62: get_law_track, left 404 right 200
A42: green at 8b96d52f8eea54800eb75b460610508075810714
A43: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: the_served_stack_refuses_initialize_without_a_bearer panicked at crates/mcp/tests/server.rs:173: initialize without a bearer, left 404 right 401
A43: green at 8b96d52f8eea54800eb75b460610508075810714
A44: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: a_refused_request_holds_no_slot panicked at crates/mcp/tests/server.rs:202: only 0 of 8 calls reached the law track within 5s
A44: green at 8b96d52f8eea54800eb75b460610508075810714
A45: red at 43f6d989d35769b4e11f3d8436a660c9795398ce: initialize_answers_json_and_no_session_id panicked at crates/mcp/tests/server.rs:229: initialize, left 404 right 200
A45: green at 8b96d52f8eea54800eb75b460610508075810714
```

### An addition with no criterion number

The green commit put `#[schemars(required)]` on each pending number's `Option` field, and schemars
then declares the inner type alone, so the served output schema said `"type": "integer"` for
`dues`, `leech_total` and `mastery`, which answer null while pending. SPEC-119 section 13 says the
schema declares them nullable and required, and A41's test reads only the required list, so it
stayed green over the defect. A new test,
`tools::the_output_schema_admits_each_pending_number_as_null`, was committed red first: at
598ae89ff206cb37e72a96b1b339e5b0dcb58d7f, selecting it alone, it panicked at
`crates/mcp/tests/tools.rs:268`: the output schema's dues (an integer schema) refuses the answered
null. It is green at 57f2fafe71bdedf5b41c4412312b59397bc8d9e5, where each field also declares
null as a type. It adds no criterion and no acceptance line.

### Test and code changes between the red and green commits

- `crates/daemon/tests/roles.rs:104`: the usage line's expected roles grow from
  `api, bot, job, data;` to `api, bot, job, data, mcp;`, because the role set grows by `mcp`. It is
  an amendment of the assertion's subject, not a weakening; nothing else in that file changed.
- `crates/mcp/tests/support/mod.rs:165`: one `push_str(&format!(..))` became three `push_str`
  calls, for a lint. The bytes the helper builds are the same, and no test or member changed.
- The daemon's logging census (`logging::every_role_logs_json_with_its_priority_from_its_first_line`)
  reaches the new `mcp` role at green, and it refused the listen refusal's first wording. The
  refusal's `Display` for an unset address now reads "it is required and is not set"; the change
  is in `crates/mcp/src/settings.rs`, and the census is unchanged.

### A test change after the green commit

- `crates/mcp/tests/settings.rs`, A39's test: its source census printed its count by hand, and the
  tdd probe's examined-counts class refused the file, which walks a directory without the house
  `examined` contract. The census now passes its sources through an `examined` helper appended at
  the end of the file, which prints the count and refuses an empty population; its own
  `more than one source` assertion is kept. The new body was run at the red commit, in a scratch
  copy of that commit with only this file replaced: only A1's and A39's tests failed of the file's
  7, and A39 panicked at `crates/mcp/tests/settings.rs:372`, the mcp role's source is missing,
  the failure its fence line above quotes.
