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
