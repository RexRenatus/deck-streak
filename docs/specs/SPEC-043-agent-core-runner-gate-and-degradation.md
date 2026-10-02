# SPEC-043: the agent checks its optional AI route first, runs each duty capped through the proxy when that route is configured, fences what it reads, and delivers only what the packs pass

- **Wave:** W1. **Issue:** #29 (epic #2). **Context(s):** `deck-streak-agent`; the public `agent/` directory; `ai-safety.json`.
- **Decided by:** ADR-069 (every pack, the proxy client rows among them, judged on the box), ADR-010
  (units), ADR-015 (headless Claude Code through the subscription proxy, fail closed), ADR-038 (the
  device key as a credential from the credential socket), ADR-054 (the AI route is optional, and
  no-AI mode is the default and a first-class path), and ADR-043 (a shell runner, the gate as the
  packs' own probes, the duty caps).
- **Status:** built (`docs/red-first/SPEC-043.md`, ADR-016).

## 1. The problem, measured

- **No agent exists.** `crates/agent/src/` holds only `lib.rs`; there is no `agent/` directory and no
  `ai-safety.json`. The box run reads the apiKeyHelper scan as pending on this issue until the
  agent's settings template lands (SPEC-056 R9), and the ai-content-safety pack waits on this issue
  (the box-run packs' wiring, ADR-069).
- **The reference client cannot be copied.** The subscription-proxy pack's reference runner and its
  scanner name the maintainer's private secret, so neither may be copied into this public tree
  (ADR-059). Its scanner reads shell and reads a runner written in Rust or Python as no launch,
  which is VOID. DeckStreak therefore writes its own generic shell runner under `agent/`.
- **What must not be repeated.** An AI pass that fails night after night with a message that names
  no cause, or stops mid-request when its provider's credit runs out, tells the owner nothing. The
  charter asks the opposite: each run capped, a failure that says why, and nothing delivered or
  written when a step fails (constraints 16 and 17).
- **The route is optional (ADR-054).** At gate 6 the device key became conditional on the owner's
  confirmation, with an API key as the alternative, and the first deploy must not depend on it. An
  unconfigured host has no AI route; every duty must then be whole and quiet, recording that the
  route is absent rather than failing and paging.
- **Prerequisites.** SPEC-020 (configuration, credentials, the offload rail, the clock), SPEC-044
  (the persona templates, the instantiated persona and the golden outputs the gate is proved on),
  SPEC-041 (the one router, for the failure alert) and SPEC-031 (the alert path). The live key and
  the reverse tunnel are W2's (#43); every test here uses fakes, and nothing here needs them: with no
  route configured, the crate runs every duty in no-AI mode.

## 2. Requirements

R1. `agent/run-headless.sh PROMPT_FILE` is the runner, a shell script. The prompt reaches `claude`
    on stdin. It exits 0 (success; the result on stdout), 1 (a tool, the input or the key is
    missing), 2 (a refused shape), 3 (the proxy rejected the key), 4 (the roster is exhausted; the
    retry instant is printed), 5 (the proxy or its tunnel is unreachable, or answered no documented
    word) or 6 (Claude Code failed, ran past its wall clock, or returned an error result). Every
    refusal is one `REFUSE:` line on stderr.
R2. The device key is a systemd credential (ADR-038): the runner reads it at launch from
    `$CREDENTIALS_DIRECTORY/agent-device-key` into its own variable, and it reaches `claude` only in
    its environment as `CLAUDE_CODE_OAUTH_TOKEN` (ADR-015): never on an argv, in a file, a log line
    or a row. `agent-device-key` is DeckStreak's own credential id; the private rail's socket maps it
    to its secret (#41), and no secret name or project is in the repository or the unit. The runner
    unsets `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_CUSTOM_HEADERS`,
    `CLAUDE_CONFIG_DIR` and the Bedrock, Vertex and Foundry switches, and sets
    `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1`.
R3. The base URL comes from the unit's environment and must be a loopback URL: `http://` and then
    `localhost`, the IPv4 loopback address or `[::1]`, then at most a port, and nothing after it;
    any other, whatever it carries after the scheme, refuses with exit 2. No file in the
    repository sets an `apiKeyHelper`, `--bare`, `bypassPermissions` or
    `--dangerously-skip-permissions`, and none sets `ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN`.
R4. Before a launch the runner asks the proxy's capacity endpoint (a configured path, which must be
    an absolute path: any other refuses with exit 2), with the key on curl's stdin, and reads the status word: `ready` launches, `exhausted` exits 4, an HTTP 401
    exits 3, and anything else exits 5.
R5. Every launch passes `--max-turns`, `--max-budget-usd`, `--output-format json`,
    `--permission-mode dontAsk`, `--strict-mcp-config` and `--settings agent/settings.json`, under a
    `timeout` wall clock. The runner reads `subtype` and `is_error` before `result`, and maps a
    non-zero exit, 124 (the wall clock) and 143 (a SIGTERM) to exit 6.
R6. Each duty declares its caps; the defaults are 30 turns, 5 USD and 1800 seconds (the
    subscription-proxy pack's reference defaults), and the daily reading's wall clock is 620 seconds
    (the predecessor's per-topic deadline, `preread.py:PREREAD_TOPIC_DEADLINE_S`). A run that
    reaches a cap is stopped, delivers nothing, and its verdict names the cap. The caps are a safety
    bound per run, never a daily cap on readings.
R7. `agent/settings.json` sets `defaultMode: dontAsk` and `disableBypassPermissionsMode: disable`;
    denies reads of `.env`, `*.pem` and `*.key` and the commands `gcloud secrets`, `printenv` and
    `env`; sets `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` and `DISABLE_AUTOUPDATER` in `env`; carries a
    `PreToolUse` guard that refuses any command naming the credential, `/environ`, Secret Manager or
    the capacity path, and a `StopFailure` hook that logs the ending to the journal. It holds no
    private value.
R8. The daily-reading task holds no tool: its allow list is empty under `dontAsk`, so a tool call
    is denied, and the run's only product is its reply.
R9. The prompt is composed in persona-core's order: the shared rules (`agent/prompts/system/rules.md`,
    persona-core's `rules.md`) and the untrusted-content policy (`agent/prompts/system/untrusted-policy.md`)
    as the system files; then the instantiated persona (SPEC-044); then the duty's instructions
    (`agent/duties/daily-reading.duty.md`, study-duties' template); then the subject's memory as
    data; then the untrusted inputs. Each untrusted input (sources `cards` and `memory`) stands
    alone between `<untrusted source="...">` and `</untrusted>` on lines of their own, JSON-encoded,
    with `<` and `>` written as the JSON escapes `\u003c` and `\u003e`. No untrusted text enters a system file.
R10. `ai-safety.json` at the repository root declares the tasks `daily-reading-law` and
    `daily-reading-language` (format `persona`, kinds `law` and `language`) with their system files,
    prompt, inputs and sources (`template`, `duty`, `memory`, `cards`), an empty tool list, SPEC-044's
    golden outputs, `on_invalid: withhold`, and a gate naming every class the ai-content-safety pack
    requires for the kind (its own `output-links`, `output-invisible` and `output-marked`;
    persona-core's `output-contract`, `no-dates`, `scrubber`, `no-human-claim` and `memory-scope`;
    for law, law-professors' `rule-cites-corpus`, `citations-resolve`, `quotes-grounded` and
    `authority-grounded`). `links.allow` names only an example host of DeckStreak's own,
    `disclosure` names the bot's `/start` reply and the Mini App's first screen, `redteam` names
    `agent/redteam/` and the test that runs it, and `agent.settings` names `agent/settings.json`.
R11. The output gate runs every class a task's gate names, on each output, before anything is
    delivered, by running the box-run packs' probes as subprocesses with `--subject` naming the
    output and its template, outside the model. The public tree holds no probe (ADR-069), so the
    deploy supplies their path. It also runs `output-invisible` on each untrusted input before the
    input is fenced. Only an output every blocking class passes is delivered.
R12. A run's verdict is `#[must_use]` and is one of: delivered (the output and its telemetry);
    withheld (the failing class and its finding lines, never the output's text); unavailable,
    with a closed cause: `key_missing`, `refused_shape`, `key_rejected`, `capacity_exhausted`,
    `proxy_unreachable`, `run_failed`, `turn_cap`, `time_cap` or `budget_cap`; or `ai_route_absent`
    (R16). A withheld or unavailable verdict delivers nothing to the owner or the vault, logs its
    cause, and raises one alert through the router; `ai_route_absent` raises none.
R13. The agent owns the table `agent_runs` (duty, persona template id, subject, verdict, cause or
    class, turns, input and output tokens, the CLI's cost estimate, duration, `created_at`),
    created `STRICT` by `migrations/004301_agent_runs.sql`, registered in the context map's
    ownership register, declared in `privacy.json` with a retention of 90 days (the predecessor's
    telemetry window), and exported and erased by the agent's data-rights port.
R14. `agent/redteam/` holds cases in the ai-content-safety template's shape, one per untrusted
    source, including an instruction override, a fence breakout and an exfiltration link. A test
    feeds each case through the engine with a fake runner that obeys the attack, and the gate
    withholds every one.

The AI route (ADR-054)

R15. The agent context exposes the AI route port, `AiRoute`: `Absent`; `Proxy`, this SPEC's runner
    (R1 to R5), unchanged; and, only after an amendment of ADR-015, `ApiKey`. The route comes from
    configuration, `DECKSTREAK_AI_ROUTE` (`proxy` selects the proxy runner); unset or empty, the route
    is `Absent`, and an unknown value refuses start by name (SPEC-020's settings rule). `.env.example`
    names the setting, unset.
R16. Every duty checks the route before its caps and its runner. With `Absent`, the run's verdict is
    `ai_route_absent`: nothing is launched or delivered, nothing is alerted, and nothing is retried;
    the verdict is recorded in `agent_runs` with no turns, tokens or cost. A configured route that
    fails keeps R12's unavailable verdict, its closed cause and its one alert.
R17. None of R1 to R14's proxy-runner criteria is a precondition for the crate's use: with the route
    `Absent`, the crate builds, its tests pass and every duty completes on a host with no device key,
    no proxy, no tunnel and no `claude`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the runner hands the key to `claude` only in its environment: a fake `claude` records an argv without it and an environment with it, and no file the run leaves holds it | subscription-proxy client rows (on the box, ADR-004); `test_the_runner_keeps_the_key_off_argv_and_disk` |
| A2 | a base URL that is not exactly a loopback host with at most a port, whatever it carries after the scheme, a capacity path that is not absolute, `--bare` and a bypass flag each refuse with exit 2 and one `REFUSE:` line, and neither `curl` nor `claude` is called | subscription-proxy `launch-base-url-loopback`, `launch-no-bare`, `launch-no-bypass`; `test_the_runner_refuses_a_remote_url_bare_and_bypass`, `test_the_runner_accepts_only_an_exact_loopback_url_and_an_absolute_path`; rows S04321 and S04322 |
| A3 | the preflight reads the status word: `exhausted` exits 4 with the retry instant, a 401 exits 3, an unknown answer exits 5, and the key never reaches curl's argv | subscription-proxy `launch-preflight-reads-status`; `test_the_preflight_reads_the_status_word` |
| A4 | no settings file in the tree names an `apiKeyHelper` (the scan examines `agent/settings.json`, which declares the settings `$schema` so the scan finds it) | the box run's apiKeyHelper scan (SPEC-056 R9); `test_no_settings_file_names_an_api_key_helper` |
| A5 | every ai-content-safety row is green over `ai-safety.json`, none VOID | ai-content-safety, every row; `test_every_ai_content_safety_row_is_green` |
| A6 | each red-team case (instruction override, fence breakout, exfiltration link) is withheld by the gate | ai-content-safety `redteam-present`; `every_redteam_case_is_withheld_by_the_gate` |
| A7 | a run that reaches its turn cap is stopped and delivers nothing, and its verdict names `turn_cap` | `a_run_past_its_turn_cap_delivers_nothing` |
| A8 | a run that reaches its wall clock is stopped and delivers nothing, and its verdict names `time_cap` | `a_run_past_its_wall_clock_delivers_nothing` |
| A9 | an output failing a blocking class is withheld, the class is recorded in `agent_runs`, the fake vault receives nothing, and the router receives ONE alert carrying only the class, never the content | `an_output_failing_a_blocking_class_is_withheld_and_recorded` |
| A10 | every untrusted input is fenced alone, JSON-encoded, with `<` and `>` escaped, and no system file holds untrusted text | `every_untrusted_input_is_fenced_alone_and_encoded` |
| A11 | the prompt is composed in persona-core's order | `the_prompt_is_composed_in_the_persona_order` |
| A12 | an unreachable proxy yields an unavailable verdict with `proxy_unreachable`, one alert, and nothing delivered | `an_unreachable_proxy_is_unavailable_with_its_cause` |
| A13 | the agent's data-rights port exports and erases `agent_runs` | `the_agent_runs_are_exported_and_erased` |
| A14 | an unset route is `Absent`, and with it a duty launches nothing (the fake runner records no call), records `ai_route_absent` in `agent_runs`, raises no alert and is not retried | `an_absent_route_records_ai_route_absent_and_alerts_nothing` |
| A15 | the runner's credential comes only from the credential socket: with a token in the environment (every name the script unsets, and `CLAUDE_CODE_OAUTH_TOKEN`) and no credentials directory it refuses and never runs `claude`; a token file at any path but `$CREDENTIALS_DIRECTORY/agent-device-key` is refused; the token never reaches an argv and no fallback chain reads it; and no committed file under `agent/` (nor under `deploy/` where it names the agent) carries a token literal or a path to a token on persistent disk. This substitutes for the proxy client scan's `credential-from-secret-manager` row (see section 6; issues #341 and #342) | `CredentialComesOnlyFromTheSocket` (five tests in `agent/tests/test_run_headless.py`); rows S04318 to S04320 |

```acceptance
A1: python3 -m unittest discover -s agent/tests -p test_run_headless.py -k test_the_runner_keeps_the_key_off_argv_and_disk
A2: python3 -m unittest discover -s agent/tests -p test_run_headless.py -k test_the_runner_refuses_a_remote_url_bare_and_bypass -k test_the_runner_accepts_only_an_exact_loopback_url_and_an_absolute_path
A3: python3 -m unittest discover -s agent/tests -p test_run_headless.py -k test_the_preflight_reads_the_status_word
A4: python3 -m unittest discover -s scripts/tests -p test_ai_safety_rows.py -k test_no_settings_file_names_an_api_key_helper
A5: python3 -m unittest discover -s scripts/tests -p test_ai_safety_rows.py -k test_every_ai_content_safety_row_is_green
A6: cargo test -p deck-streak-agent --test redteam -- --exact every_redteam_case_is_withheld_by_the_gate
A7: cargo test -p deck-streak-agent --test runner -- --exact a_run_past_its_turn_cap_delivers_nothing
A8: cargo test -p deck-streak-agent --test runner -- --exact a_run_past_its_wall_clock_delivers_nothing
A9: cargo test -p deck-streak-agent --test duty -- --exact an_output_failing_a_blocking_class_is_withheld_and_recorded
A10: cargo test -p deck-streak-agent --test compose -- --exact every_untrusted_input_is_fenced_alone_and_encoded
A11: cargo test -p deck-streak-agent --test compose -- --exact the_prompt_is_composed_in_the_persona_order
A12: cargo test -p deck-streak-agent --test runner -- --exact an_unreachable_proxy_is_unavailable_with_its_cause
A13: cargo test -p deck-streak-agent --test data_rights -- --exact the_agent_runs_are_exported_and_erased
A14: cargo test -p deck-streak-agent --test duty -- --exact an_absent_route_records_ai_route_absent_and_alerts_nothing
A15: python3 -m unittest discover -s agent/tests -p test_run_headless.py -k CredentialComesOnlyFromTheSocket
```

## 3a. What the box run judges

The ai-content-safety probes are box-only (ADR-069). The tree's own test checks the structure of
`ai-safety.json` and the red-team cases on every run, and runs the real probes only when the
packs' scripts are present; the box run is where each row is judged over `ai-safety.json`, none VOID.

| id | what it judges | population it must examine |
|---|---|---|
| B1 | the pack's `redteam-present` row: every case in `agent/redteam/` is present and is a case the gate withholds | the 5 red-team cases under `agent/redteam/` |
| B2 | the pack's `disclosure-first-contact` row: the learner's first contact says the coach is an AI (the bot's `/start` reply; no web change here) | the 1 disclosure surface `ai-safety.json` names |
| B3 | every other ai-content-safety row the pack lists, including the blocking output classes the gate runs before any delivery | the 1 duty (the daily reading) and its 2 prompt templates in `ai-safety.json` |
| B4 | the subscription-proxy client rows and the apiKeyHelper scan | the 1 runner `agent/run-headless.sh` and the 1 settings template `agent/settings.json` |

The private wiring change that enforces them: the ai-content-safety pack becomes `enforced`, and the
apiKeyHelper scan's waiting entry is lifted; the JSON diff is handed back with this delivery.

## 4. File manifest

| file | context | change |
|---|---|---|
| `agent/run-headless.sh` | agent (public) | added: the runner |
| `agent/settings.json` | agent (public) | added: the Claude Code settings template |
| `agent/prompts/system/rules.md` | agent (public) | added: persona-core's shared rules, copied |
| `agent/prompts/system/untrusted-policy.md` | agent (public) | added: from the ai-content-safety template |
| `agent/prompts/daily-reading.prompt.md` | agent (public) | added: the task prompt with its fenced slots |
| `agent/duties/daily-reading.duty.md` | agent (public) | added: study-duties' duty template, copied |
| `agent/redteam/` | agent (public) | added: the red-team cases |
| `agent/tests/test_run_headless.py` | agent (public) | added |
| `docs/specs/SPEC-123-the-box-proxy-scan-admits-an-expected-red-that-names-an-open-issue.md` | repo | amended: the VOID case's wording, SPEC-123 R5 and A4 |
| `agent/tests/fakes/` | agent (public) | added: fake `claude` and `curl`, and a temporary credentials directory, for the runner's tests |
| `ai-safety.json` | repo | added |
| `crates/agent/Cargo.toml` | `deck-streak-agent` | changed: the workspace dependencies it uses |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the modules below |
| `crates/agent/src/duty.rs` | `deck-streak-agent` | added: the duty declaration and its caps |
| `crates/agent/src/route.rs` | `deck-streak-agent` | added: the AI route port, `Absent` by default, and the route check every duty makes first |
| `crates/agent/src/compose.rs` | `deck-streak-agent` | added: prompt composition |
| `crates/agent/src/fence.rs` | `deck-streak-agent` | added: the untrusted fence and its encoding |
| `crates/agent/src/runner.rs` | `deck-streak-agent` | added: the runner port and the process runner |
| `crates/agent/src/gate.rs` | `deck-streak-agent` | added: the output gate over the box-run packs' probes |
| `crates/agent/src/verdict.rs` | `deck-streak-agent` | added: the verdict and its closed causes |
| `crates/agent/src/runs.rs` | `deck-streak-agent` | added: the `agent_runs` repository |
| `crates/agent/src/data_rights.rs` | `deck-streak-agent` | added: the data-rights port |
| `migrations/004301_agent_runs.sql` | `deck-streak-agent` | added |
| `crates/agent/tests/runner.rs` | `deck-streak-agent` | added |
| `crates/agent/tests/compose.rs` | `deck-streak-agent` | added |
| `crates/agent/tests/gate.rs` | `deck-streak-agent` | added |
| `crates/agent/tests/duty.rs` | `deck-streak-agent` | added: the duty engine's order, A9 and A14 |
| `crates/coordination/src/maintenance.rs` | `deck-streak-coordination` | changed: the daily upkeep prunes `agent_runs` past its 90 days |
| `crates/coordination/tests/maintenance.rs` | `deck-streak-coordination` | changed: the prune of `agent_runs` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the agent's data-rights port joins the ports an export and an erase run |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded `agent_runs` block of 101 rows, so the symmetry test covers the new table |
| `scripts/tests/test_check_gate.py` | repo | changed: the gate's python-stage test also plants the `agent/tests` suite it now discovers |
| `crates/agent/tests/constants.rs` | `deck-streak-agent` | added: the configured literals, each asserted written out |
| `crates/agent/tests/support/mod.rs` | `deck-streak-agent` | added: the recorded alerts, vault, runner and gate fakes |
| `crates/agent/tests/redteam.rs` | `deck-streak-agent` | added |
| `crates/agent/tests/data_rights.rs` | `deck-streak-agent` | added |
| `crates/agent/tests/fixtures/` | `deck-streak-agent` | added: the fake runner and its canned replies |
| `scripts/tests/test_ai_safety_rows.py` | repo | added |
| `scripts/check.sh` | repo | changed: the python stage also discovers `agent/tests` |
| the box-run packs' private wiring (ADR-069) | the maintainer's | changed: ai-content-safety becomes `enforced` |
| `docs/CONTEXT-MAP.md` | docs | changed: the ownership register gains `agent_runs` |
| `privacy.json` | repo | changed: the agent-run category |
| `.env.example` | repo | changed: the AI route setting, by name, unset |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/schematics/agent-duty-run.md` | docs | added |
| `docs/specs/SPEC-043-agent-core-runner-gate-and-degradation.md` | docs | moved from `docs/specs/planned/` |
| `changelog.d/feat-agent-core-043.md` | docs | added |
| `scripts/mutation-rows.d/S04300-S04399.json` | repo | added: the constants' rows, the three credential script rows S04318 to S04320 (A15), and the two runner-check rows S04321 and S04322 (A2) |
| `scripts/mutation_rows.py` | repo | changed: `agent/tests` joins the unittest roots a script row's killer resolves in |
| `PRIVACY.md` | docs | changed: the agent-runs row |
| `docs/decisions/ADR-043-shell-runner-pack-gate-and-duty-caps.md` | docs | added |
| `docs/red-first/SPEC-043.md` | docs | added |

## 5. What this does NOT do

- It opens no tunnel, adds no device key to the proxy and runs nothing against the live proxy: the
  agent's path is W2's and an owner gate (#43, #162).
- It installs none of the owner's private guards on the host; the private rail does (#41).
- It runs no duty but the daily reading; every other duty adds its task to `ai-safety.json` in its
  own delivery (#46, #53).
- It ports no vault-ops skill and offers no on-demand vault run (#54).
- It builds no DeckStreak-owned skill pack under `skills/`, so the pack-authoring rows have nothing
  to judge here (#60).
- The output gate does not refuse an empty input-class list, and the verdict's `#[must_use]` has no
  test that pins it (#362).
- It adds no index on `agent_runs.created_at` for the daily prune (#363).
- It builds no `ApiKey` adapter: that waits for an amendment of ADR-015 and the owner's choice of a
  route at gate 3 (#162).

## 6. Risks

- **The box scanner judges the runner differently from the CI tests** (the proxy client rows run
  only on the maintainer's box). Detected by `scripts/box-packs.sh` before the merge into `dev`
  (ADR-004); the CI tests cover the same exits against fakes.
- **The gate's subprocesses are slow on the small host.** Measured by the run's duration in
  `agent_runs`; a reading's gate is a few dozen standard-library Python processes.
- **The host lacks `python3`.** The gate needs it; the host budget and the deploy templates
  (SPEC-032) name it as a runtime dependency.
- **An attack the red-team cases do not cover.** Mitigated by the tool-less task (nothing can carry
  data out) and the output-links class; new cases are added with each new untrusted source.
- **A cap too tight for a long reading.** Visible as `time_cap` verdicts in `agent_runs` and in the
  readings health (SPEC-050); the cap is a duty declaration, changed by a SPEC amendment.
- **The box scanner refuses the runner's credential read, and the row is substituted, not
  waived.** The subscription-proxy client scan's `credential-from-secret-manager` row accepts only
  a secret-manager call inside the client, and R2 reads a systemd credential (ADR-038, which
  rejects a secret-manager read in the application). Until the scan learns the socket (issue #341)
  the row is substituted by A15, which fails if the runner takes its token from anywhere but
  `$CREDENTIALS_DIRECTORY/agent-device-key` or a committed file carries a token or a token path;
  the box's deferral mechanism (issue #342) records the scan's red as expected against #341, so the
  box run reads `BOX PACKS OK` without hiding it. The unit's `LoadCredential=agent-device-key:`
  line in the socket form (the third leg of #341) is not this SPEC's: it ships no unit, and
  SPEC-063's A2 (issue #43) pins it where the unit ships.
- **A mistyped route would silently turn the readings off.** An unknown value refuses start by name
  (R15), so only an unset route is `Absent`, and the surfaces then say readings are not enabled,
  never that something failed (ADR-054).

## 7. Amendment, 2026-09-29: the gate refuses a missing input class, the verdict's must_use is pinned, and agent_runs is indexed for its prune

Issues #362 and #363, both follow-ups of the review of this SPEC's build. Sections 1 to 6 stand as
written; the two bullets of section 5 that name #362 and #363 record what the first delivery left
out, and this amendment delivers them.

**R11a. The gate refuses to be built unchecked.** `ProbeGate::new` returns a `Result` and takes one
more argument, `reads_inputs`, which says whether the duty the gate serves reads untrusted inputs.
It returns `GateBuildError::NoOutputClass` for an empty list of blocking output classes, and
`GateBuildError::NoInputClass` when `reads_inputs` holds and no input class was given. A gate that
names no blocking class would pass every output, and a gate with no input class for a duty that
reads inputs would pass every input unchecked, so both refusals happen at construction, with a named
error, and neither is a panic. A gate for a duty that reads no input still needs no input class.

`reads_inputs` is the caller's statement, and nothing checks it against a duty: `DutySpec` has no
such field, a `DutyEngine` holds one gate for every duty it runs, and `DutyEngine::decide` runs
`check_input` on each run's memory and cards whatever the duty. A gate built with `reads_inputs =
false` and no input class answers `Passed` for every input, so the gate a `DutyEngine` holds is
always built with `reads_inputs = true`, as R11 requires of both daily-reading tasks, whose sources
name `memory` and `cards` (R10); `false` is only for a gate that no engine hands untrusted input.
Rejected: a `reads_inputs` field on `DutySpec`, because the engine's gate is built before any duty
is known and serves them all, so a duty's field could not decide it; and refusing a missing input
class always, because a gate for a duty with no untrusted input would then name a class it never
runs.

The constructor was a `const fn` and is one no longer: it drops its arguments on a refusal, which a
`const fn` cannot do. Its callers today are this crate's own tests, each changed to build the gate
with `reads_inputs` and to expect the `Result`. The daemon wires the gate in a later delivery (the
agent's path is W2's, section 5); that wiring propagates the refusal, so a daemon that cannot build
its gate refuses to start, loudly, and never falls back to a gate that passes.

**R12a. The verdict's `#[must_use]` is pinned.** `Verdict` stays `#[must_use]`, and a test reads the
attributes above `pub enum Verdict` in `crates/agent/src/verdict.rs` and asserts the attribute is
there, beside a positive fact: the enum was found and its `derive` was read. The alternative
rejected is a `compile_fail` doctest under `#![deny(unused_must_use)]`: it would observe the
behaviour and not the text, but the mutation-row runner selects a killer as an integration-test
target or a unittest id, so no row could name a doctest, and an attribute pinned by no row is the
gap this issue reports.

**R16a. `agent_runs` is indexed on `created_at`.** Migration `004302_agent_runs_created_at_index.sql`
creates `agent_runs_by_created_at`, so the daily prune's `DELETE FROM agent_runs WHERE created_at <
?1` seeks the old rows and never scans the table. It adds no table and no column, so the
six-file rule of a new table does not apply, and it changes no query, so `.sqlx/` is unchanged.
The premise of #363, that the other run tables carry an index on the column their prune reads, was
measured and does not hold for any of them:

| table | prune | the column's index |
|---|---|---|
| `cron_fires` | `DELETE FROM cron_fires WHERE fire_date < ?1` in `crates/coordination/src/maintenance.rs` | none: `fire_date` is the second column of the primary key `(job_id, fire_date)`, so the range cannot seek it |
| `reading_runs` | none (only the erase deletes every row) | none needed |
| `sync_runs` | none (only the erase deletes every row) | none needed |

The index on `agent_runs` is wanted either way, since a daily range delete over an unindexed column
scans the table. `cron_fires` has the same shape and is not changed here: it is another context's
table, and it holds about a row per job per day.

**Files.**

| file | context | change |
|---|---|---|
| `crates/agent/src/gate.rs` | `deck-streak-agent` | changed: `GateBuildError`, and `ProbeGate::new` returns a `Result` |
| `crates/agent/tests/gate.rs` | `deck-streak-agent` | changed: the callers, and the two refusals |
| `crates/agent/tests/redteam.rs` | `deck-streak-agent` | changed: its gate is built with the new argument |
| `crates/agent/tests/verdict.rs` | `deck-streak-agent` | added: the verdict's attribute pin |
| `crates/agent/tests/runs.rs` | `deck-streak-agent` | added: the index exists and the prune uses it |
| `migrations/004302_agent_runs_created_at_index.sql` | `deck-streak-agent` | added |
| `scripts/mutation-rows.d/S04300-S04399.json` | repo | changed: rows S04323 to S04327 |
| `docs/red-first/SPEC-043.md` | docs | changed: the amendment's red and green lines |
| `changelog.d/fix-agent-gate-362.md` | docs | added |

**Rows.** S04323 skips the empty-list refusal and S04324 skips the reads-inputs refusal (both in
`gate.rs`, killed by `gate::an_empty_class_list_is_refused_at_construction` and
`gate::a_duty_that_reads_inputs_needs_an_input_class`); S04325 removes `#[must_use]` from `Verdict`
(killed by `verdict::the_verdict_type_is_must_use`); S04326 replaces the migration's `CREATE INDEX`
with a no-op and S04327 moves the index to another column (killed by
`runs::agent_runs_is_indexed_on_created_at` and
`runs::the_prune_reads_agent_runs_through_the_created_at_index`).

This amendment does not change any other requirement: #362 and #363 own what it delivers, and the
gate's daemon wiring stays with the agent's W2 path (#162).

## 8. Acceptance criteria of the 2026-09-29 amendment

| id | criterion | decided by |
|---|---|---|
| A16 | building the gate with an empty list of blocking output classes is refused with `NoOutputClass`, and the same call with one class is built | `an_empty_class_list_is_refused_at_construction` |
| A17 | building the gate without an input class for a duty that reads inputs is refused with `NoInputClass`, while a duty that reads none, and a duty with the class, are built | `a_duty_that_reads_inputs_needs_an_input_class` |
| A18 | `pub enum Verdict` carries `#[must_use]`, read from its source beside its `derive` | `the_verdict_type_is_must_use` |
| A19 | after the migrations, `agent_runs_by_created_at` exists on `agent_runs` and covers `created_at` alone | `agent_runs_is_indexed_on_created_at` |
| A20 | the plan of the prune's exact statement names `agent_runs_by_created_at` and holds no table scan | `the_prune_reads_agent_runs_through_the_created_at_index` |

```acceptance
A16: cargo test -p deck-streak-agent --test gate -- --exact an_empty_class_list_is_refused_at_construction
A17: cargo test -p deck-streak-agent --test gate -- --exact a_duty_that_reads_inputs_needs_an_input_class
A18: cargo test -p deck-streak-agent --test verdict -- --exact the_verdict_type_is_must_use
A19: cargo test -p deck-streak-agent --test runs -- --exact agent_runs_is_indexed_on_created_at
A20: cargo test -p deck-streak-agent --test runs -- --exact the_prune_reads_agent_runs_through_the_created_at_index
```

## 9. Amendment, 2026-09-29: the prune and verdict pins refuse a quoted decoy

Issue #404, a follow-up of the review of section 7's delivery. Sections 1 to 8 stand as written and
this section only adds to them; the two pins it strengthens are R12a's (criterion A18) and R16a's
(criterion A20), and both still hold for every shape the SPEC promises. What follows refuses the
decoys named below and the spellings R16b and R12b list. Both pins read the tokens rustc's lexer
produces (ADR-293), so a decoy spelled any way rustc lexes the same is read as rustc reads it; what
stays green is what the tokens of one file do not say, and issue #444 tracks it. For the prune: a
copy of the tested statement handed to `sqlx::query!` in code that never executes it (a function
nothing calls, a query built and dropped, an item compiled out), beside a prune that runs and whose
`DELETE` keyword is split across `concat!` parts, or whose text is not in `runs.rs` (read by
`sqlx::query_file!` or `include_str!`, or held in another module). A keyword written with a string
escape such as `\x44ELETE` or `\u{44}ELETE`, or broken by a `\`-newline continuation, is no longer
in that class: the pin reads a literal's cooked text. For the verdict, the measured population holds
no such decoy: an enum the source does not declare (made by a macro from a parameter, declared in
another file, or renamed by a `use` or a type alias) is refused rather than read, whatever copy
stands beside it. Neither pin evaluates a `cfg` or expands a macro.

**What the decoys were.** A20 found the prune's statement by looking for its quoted literal in
`crates/agent/src/runs.rs`, so a comment quoting that literal, beside a prune that no longer used
the index, kept the test green. A20 also wrote the statement twice in its own file, once as `PRUNE`
and once inside its `EXPLAIN QUERY PLAN` text, so a later change to one left the plan check on the
other. A18 read a line as an attribute when it began with `#[` and ended with `]`, so an item marked
`#[rustfmt::skip]` whose line ended in a `// ]` comment read as an attribute.

**R16b. The prune is one statement, written once.** The pin reads `runs.rs` as the tokens rustc's
lexer produces: `proc-macro2`'s lexer, with each string-like literal read through `syn` as the value
rustc cooks (ADR-293). A comment is no token; a doc comment, outer or inner, becomes a `doc`
attribute whose text is prose; a literal of any kind (a string, a raw string with any number of
hashes, a byte string, a C string, raw or not) is one token whatever it holds; and whitespace is
whatever separates two tokens, so every code point rustc skips between tokens, U+200E and U+200F
included, is skipped. A source that does not lex is refused. Its code hands the tested statement to
`sqlx::query!` exactly once: each `sqlx::query!` in the token tree, at any depth and in any spelling
rustc reads as that call (whitespace around `::` and `!`, raw identifiers, any delimiter), whose
first argument is a string literal cooking to the tested statement, counts once. Its code writes the
word `delete` exactly once, in any case, as a word of its own in an identifier or in any string-like
literal's cooked text, a doc attribute's text excepted. So a comment that calls the prune a delete,
or quotes it, keeps the test green, and a copy of the call held by a literal of any kind, closing on
its own line, on the line before the code or on the code's own line, is no call. A second statement
turns the test red; so does a changed statement beside a copy of the first held by a comment, by a
literal of any kind (a constant, a static, a doc attribute, a plain literal) or by a `sqlx::query!`
that never runs; so does a changed statement whose table is spelled `main.agent_runs` or
`"agent_runs"` or with an SQL comment between its keywords; and so does a prune whose `DELETE` is
written with a string escape or a line continuation beside a copy of the call that never runs.
Refused although it would build correctly, and failing closed: a literal of any kind whose cooked
text writes the word `delete` beside the one prune, since a literal can be handed to `sqlx::query`
at run time and the pin follows no value. Not refused: the prune class named at the head of this
section, which the tokens of `runs.rs` do not show. The test file writes the statement once, as a
`macro_rules!` literal; the tested statement (`PRUNE`) and the text of its `EXPLAIN QUERY PLAN` are
both made from that literal with `concat!`, and a count of the test file's own tokens asserts that
its literals write `DELETE FROM agent_runs` once, cooked, in any case or spacing, so a changed copy
written in any literal beside the macro turns the test red. A copy assembled from the `TABLE`
constant through `format!` is not written out and is not counted. Production keeps its one
`sqlx::query!` literal and its behaviour: the derived literal feeds only the test's plain
`sqlx::query_as` call, which takes a `&str`. sqlx's own documentation says that "The query must be a
string literal or a concatenation of string literals; dynamic queries or those generated by other
macros are not supported" (Context7, `/websites/rs_sqlx`, the `query!` macro's requirements), so the
`query!` macros are not given a literal made by a macro.

**R12b. The verdict's attributes are the tokens before its declaration.** The pin reads `verdict.rs`
as the tokens rustc's lexer produces, as R16b reads `runs.rs`, and a source that does not lex
declares nothing and is refused. A declaration of the enum is the identifier `enum` followed by the
identifier `Verdict`, its raw prefix removed, at any depth of the token tree: a copy in a module, a
function or a macro's body, or compiled out by its own or an enclosing `cfg`, is a declaration, and
a copy in a comment or in any literal is none. The source declares the enum exactly once, so a copy
in code beside the enum in any spelling (`pub enum r#Verdict {`, any whitespace rustc skips or a
comment between the words, another visibility) is a second declaration and cannot lend the enum its
attributes. The attributes are the `#` and bracket-group pairs directly before the declaration at
the top of the file, after its visibility (`pub` or `pub(..)`), each written out from its tokens
with a space only between two words, so an attribute reads the same however rustc's whitespace
spaced it; a doc comment is one of them, as its `doc` attribute. A `#[must_use]` held by a string or
a comment, wherever the holder closes (the declaration's own line included), is inside one token or
none and does not count; nor does one on an item before the enum, such as `#[rustfmt::skip] pub fn
f() {} // ]`. `#[must_use]` counts bare, with a string reason, and with its raw prefix removed
(`#[r#must_use]`). An attribute of the enum whose path's last segment, raw prefix removed, is `cfg`
or `cfg_attr` is conditional and refused rather than judged, so a copy compiled out by `#[cfg(...)]`
cannot satisfy the pin with its `#[must_use]`, and a `#[must_use]` given only under a `cfg_attr`
condition does not count. Refused although it would build correctly, each failing closed: a copy of
the enum in code compiled out beside the enum, a second declaration because the pin evaluates no
`cfg`; an enum made by a macro, declared in another file, or renamed by a `use` or a type alias,
which the pin does not read as a declaration; and a `#[must_use]` whose reason is a raw string.

**Insertions.** None inside sections 1 to 8: the amendment is insert-only, so the criteria rows and
fences of A18 and A20 keep their words. Their strengthening is carried by the new criteria A21 (the
prune is one statement), A22 (the statement is written once in the test) and A23 (the verdict's
attributes are the tokens before its declaration), listed in section 10, which follow the last
A-number of section 8.

**Files.**

| file | context | change |
|---|---|---|
| `Cargo.toml` | workspace | changed: `proc-macro2` and `syn` as workspace dependencies with no default feature (ADR-293), both already in the lock |
| `Cargo.lock` | workspace | changed: two dependency edges of `deck-streak-agent`; no crate added |
| `crates/agent/Cargo.toml` | `deck-streak-agent` | changed: `proc-macro2` and `syn` as dev-dependencies |
| `crates/agent/tests/runs.rs` | `deck-streak-agent` | changed: the one macro literal, the pin read through the lexer's tokens with its run count and word count, planted decoys, prose fixtures, and generated populations of literal and comment holders, of spellings of the call, of spelled prunes beside copies that never run, of copies that do not run and of character literals |
| `crates/agent/tests/verdict.rs` | `deck-streak-agent` | changed: the declaration count and the attribute read through the lexer's tokens, the conditional-attribute refusal, planted decoys, and generated populations of spellings, of copies in code, of literal and comment holders and of whitespace |
| `scripts/mutation-rows.d/S04300-S04399.json` | repo | changed: rows S04328 to S04355 |
| `docs/red-first/SPEC-043.md` | docs | changed: an addendum with the decoys' red and green lines, and each fix round's |
| `docs/specs/SPEC-043-agent-core-runner-gate-and-degradation.md` | docs | changed: this section and section 10 |
| `docs/decisions/ADR-293-the-agent-pins-read-rust-as-tokens.md` | docs | added |
| `docs/decisions/ADR-043-shell-runner-pack-gate-and-duty-caps.md` | docs | changed: an amended-by line |
| `changelog.d/fix-agent-pins-404.md` | docs | added |

**Rows.** S04329, S04330 and S04334 were installed against the tests as they stood before the
amendment or the round that added them and SURVIVED them, and each is KILLED by the tests as
amended: S04329 puts an item line ending in a `// ]` comment between the verdict's `#[must_use]` and
its `derive` (killed by `verdict::the_verdict_type_is_must_use`); S04330 replaces the plan's derived
text with a separately typed copy of the statement (killed by
`runs::the_prune_reads_agent_runs_through_the_created_at_index`); S04334 disables the delete word
count (killed by `runs::a_prune_spelled_around_the_keyword_scan_beside_a_quoted_copy_is_refused`).
S04338 and S04339 stop seeing `cfg` and `cfg_attr` as conditional (both killed by
`verdict::a_conditional_attribute_on_the_enum_is_refused`). The fifth round deleted the hand lexer,
and with it the finds of S04331 to S04333, S04335 to S04337 and S04340 to S04343. It re-anchors
those ids on the token rule's decisions and adds S04344 to S04355; each is KILLED by the test its
row names, and each but S04353 mutates a helper the head did not have; S04353 mutates the statement
normaliser the head already had, whose mutant the head's tests kill as well. S04328, whose doc line
is prose to a token reader, is re-pointed: it adds a second statement held by a constant to
`runs.rs` (killed by `runs::the_prune_reads_agent_runs_through_the_created_at_index`). Code or not
code: S04335 and S04336 read an outer and an inner doc attribute as code, and S04351 stops
recognising one; S04341, S04344 and S04345 stop reading a string's, a byte string's and a C string's
cooked text; S04346 and S04347 stop the word count and the run count at the top of the token tree;
S04340 takes any string handed to `sqlx::query!` as the tested statement; S04348 accepts a source
that does not lex; S04352 stops the test file's count reading its own literals; S04332 stops the
declaration count at the top of the token tree; S04354 counts any enum as the verdict. Whitespace or
not: S04349 splits words only at a space, S04353 widens the gap a statement's whitespace collapses
to, and S04331 writes a space between every two tokens of an attribute. Spellings: S04350 and S04337
keep the raw prefix on the call's identifiers and on the enum's name, S04343 and S04355 stop reading
`#[r#must_use]` and a must_use with a reason, S04342 stops passing a `pub(..)` visibility, and
S04333 reads only the attribute nearest the declaration.

The fifth round closes the lexical class the second, third and fourth rounds each reopened, with one
rule for both pins, and derives each killer from the class's axes; each test asserts the count it
examined. `runs::a_literal_of_any_kind_holding_the_call_is_not_the_call` generates eighty-four
holders of a copy of the call (every one of eleven literal kinds that can hold it, raw with none to
three hashes, byte and C strings included, each closing on its own line, on the line before the code
or on the code's own line, as a statement, a constant, a static or a doc attribute) against the one
prune and a prune that runs elsewhere: 168 members.
`runs::a_comment_of_any_kind_holding_the_statement_is_not_read` generates thirty-two comment holders
(eight comment kinds, nested and doc forms included, each holding the call or the bare statement):
64 members. `runs::every_spelling_of_the_call_rustc_reads_as_the_call_is_the_call` generates 130
spellings: each of rustc's eleven whitespace code points in each of six gaps of the call, and eight
path spellings by eight literal spellings.
`runs::a_prune_spelled_by_an_escape_a_continuation_or_tokens_beside_a_dead_copy_is_refused` puts
five prunes that run a statement the index cannot seek, spelled by a string escape, a line
continuation, a raw string or `stringify!`, beside four copies of the call that never run: 20
members. `verdict::a_must_use_held_by_a_literal_or_a_comment_is_not_the_enums` and
`verdict::a_copy_of_the_enum_held_by_a_literal_or_a_comment_is_no_declaration` each generate sixty
holders (eleven literal kinds and six comment kinds, closing in each place a kind can close): 120
members each. `verdict::every_gap_rustc_reads_between_the_enums_tokens_is_whitespace` puts each of
the eleven code points in each of seven gaps of the declaration and its `#[must_use]`, and in three
gaps of an enum without one beside a commented copy, and reads three spellings of `#[must_use]`: 113
members.
`verdict::a_copy_of_the_enum_in_code_is_a_declaration_and_in_a_comment_or_a_literal_is_none`
generates twelve copies against six spellings of the enum: 72 members, a copy in code read as a
second declaration and a held copy as none. Each member these tests generate was labelled by
compiling it with rustc, the prune's through a stand-in `sqlx` that logs the statement it is handed;
ADR-293 records the population and the rules it was measured against. The earlier rounds'
populations stay: twenty-one copies that do not run against three prunes that run elsewhere,
thirteen character literals, sixteen comment forms, and sixteen spellings of the declaration and of
a conditional attribute.

This amendment changes no production code and no other requirement. The agent crate gains two
dev-dependencies, both already in the lock (ADR-293).

## 10. Acceptance criteria of the 2026-09-29 second amendment

| id | criterion | decided by |
|---|---|---|
| A21 | the pin reads `runs.rs` as rustc's lexer tokenizes it and refuses a source that does not lex; its code hands the tested statement to `sqlx::query!` once, in any spelling rustc reads as that call, and writes the word `delete` once over its identifiers and every string-like literal's cooked text, doc attributes excepted, so a second statement, a changed statement beside a copy of the first held by a comment or a literal of any kind or handed to a `sqlx::query!` that never runs, a statement spelled `main.agent_runs`, `"agent_runs"` or with an SQL comment between its keywords, a second statement hidden after a character literal, and a prune whose keyword is written with an escape or a continuation beside a copy that never runs are refused; a comment naming or quoting the delete is not, and a literal writing the word `delete` beside the one prune is refused, failing closed; a copy handed to `sqlx::query!` that never executes, beside a prune whose keyword is split across `concat!` parts or whose text is not in `runs.rs`, is not refused | `the_prune_reads_agent_runs_through_the_created_at_index`, `a_comment_quoting_the_prune_beside_a_prune_that_skips_the_index_is_refused`, `a_second_delete_statement_is_refused_however_it_is_spelled`, `a_prune_spelled_around_the_keyword_scan_beside_a_quoted_copy_is_refused`, `prose_that_names_the_delete_beside_the_prune_is_not_counted`, `a_delete_word_or_the_statement_in_any_comment_form_is_not_counted`, `a_copy_of_the_statement_that_does_not_run_is_not_the_tested_statement`, `no_character_literal_hides_a_second_statement_in_the_string_after_it`, `a_literal_of_any_kind_holding_the_call_is_not_the_call`, `a_comment_of_any_kind_holding_the_statement_is_not_read`, `every_spelling_of_the_call_rustc_reads_as_the_call_is_the_call`, `a_prune_spelled_by_an_escape_a_continuation_or_tokens_beside_a_dead_copy_is_refused`, `a_source_that_does_not_lex_is_refused` |
| A22 | the tested statement and the text of its plan are made from one literal, and a changed copy of the statement written in any literal of the test file, cooked, in any case or spacing, is refused | `the_prune_reads_agent_runs_through_the_created_at_index`, `a_changed_copy_of_the_statement_in_the_plan_string_is_refused` |
| A23 | the pin reads `verdict.rs` as rustc's lexer tokenizes it; the enum is declared once (the identifier `enum` then `Verdict`, raw prefix removed, at any depth of the token tree, none in a comment or a literal), its attributes are the `#[..]` pairs before its declaration at the top of the file, written out from their tokens, `#[must_use]` counts bare, with a string reason or with its raw prefix, and a conditional (`cfg` or `cfg_attr`) attribute on it is refused, so an item line ending in a `// ]` comment, a bracket inside a string, a `#[must_use]` held by a string or a comment wherever it closes, the declaration's line included, a copy of the enum in code beside any spelling of the enum, any whitespace rustc skips between its tokens (U+200E and U+200F included), and a compiled-out or condition-only `#[must_use]` are each read as rustc reads them; a copy of the enum in code compiled out beside it, and an enum the source does not declare, are refused, failing closed | `the_verdict_type_is_must_use`, `an_item_ending_in_a_bracket_comment_is_not_an_attribute`, `a_bracket_inside_a_string_or_a_comment_never_closes_an_attribute`, `a_commented_copy_of_the_enum_above_it_is_refused`, `a_raw_identifier_declaration_is_the_verdict_enum`, `a_conditional_attribute_on_the_enum_is_refused`, `every_spelling_of_the_declaration_and_of_a_conditional_attribute_is_read`, `a_copy_of_the_enum_in_code_is_a_declaration_and_in_a_comment_or_a_literal_is_none`, `a_must_use_held_by_a_literal_or_a_comment_is_not_the_enums`, `a_copy_of_the_enum_held_by_a_literal_or_a_comment_is_no_declaration`, `every_gap_rustc_reads_between_the_enums_tokens_is_whitespace` |

```acceptance
A21: cargo test -p deck-streak-agent --test runs -- --exact the_prune_reads_agent_runs_through_the_created_at_index a_comment_quoting_the_prune_beside_a_prune_that_skips_the_index_is_refused a_second_delete_statement_is_refused_however_it_is_spelled a_prune_spelled_around_the_keyword_scan_beside_a_quoted_copy_is_refused prose_that_names_the_delete_beside_the_prune_is_not_counted a_delete_word_or_the_statement_in_any_comment_form_is_not_counted a_copy_of_the_statement_that_does_not_run_is_not_the_tested_statement no_character_literal_hides_a_second_statement_in_the_string_after_it a_literal_of_any_kind_holding_the_call_is_not_the_call a_comment_of_any_kind_holding_the_statement_is_not_read every_spelling_of_the_call_rustc_reads_as_the_call_is_the_call a_prune_spelled_by_an_escape_a_continuation_or_tokens_beside_a_dead_copy_is_refused a_source_that_does_not_lex_is_refused
A22: cargo test -p deck-streak-agent --test runs -- --exact the_prune_reads_agent_runs_through_the_created_at_index a_changed_copy_of_the_statement_in_the_plan_string_is_refused
A23: cargo test -p deck-streak-agent --test verdict -- --exact the_verdict_type_is_must_use an_item_ending_in_a_bracket_comment_is_not_an_attribute a_bracket_inside_a_string_or_a_comment_never_closes_an_attribute a_commented_copy_of_the_enum_above_it_is_refused a_raw_identifier_declaration_is_the_verdict_enum a_conditional_attribute_on_the_enum_is_refused every_spelling_of_the_declaration_and_of_a_conditional_attribute_is_read a_copy_of_the_enum_in_code_is_a_declaration_and_in_a_comment_or_a_literal_is_none a_must_use_held_by_a_literal_or_a_comment_is_not_the_enums a_copy_of_the_enum_held_by_a_literal_or_a_comment_is_no_declaration every_gap_rustc_reads_between_the_enums_tokens_is_whitespace
```

## 11. Amendment, 2026-09-30: the pins read the source as rustc receives it

Issue #404, the sixth review of section 9's delivery. Sections 1 to 10 stand as written and this
section only adds to them. Where a sentence of section 9 claims more than the pins read, it stays
where it is, and the reading this section gives supersedes it. The pins it strengthens are R16b's
(criterion A21) and R12b's (criterion A23); its criteria, A24 and A25, are in section 12.

**What the review found.** Two members of the lexical class were still open. rustc removes a first
line that opens with `#!` and is not an inner attribute (a shebang) before it lexes, and
`proc-macro2`'s lexer has no such step, so a copy of the call, or a `#[must_use]`, written on that
line was read as code. And the run count read the tokens of a `doc` attribute that the word count
skips, so a copy of the call written as a doc attribute's value, which a macro then discards, was
counted as the prune beside a prune that runs another statement. Four mutants of the counts'
comparisons also survived every test: they accepted two runs of the prune, a source that writes the
word `delete` nowhere, a test file that writes the statement nowhere, and two declarations of the
enum.

**R16c. Both pins read the source as rustc receives it, and every count reads one token set.** Both
pins read the tokens rustc's lexer produces for the source as rustc receives it, each string-like
literal read as the value rustc cooks. rustc's input format removes one byte order mark, reads CRLF
as LF, and removes a first line that opens with `#!` when the next token, past whitespace and plain
comments, is not `[`. The pins remove one byte order mark as rustc does, and refuse every source
that then opens with `#!` and not `#![`: `proc-macro2` skips more between `#!` and `[` than rustc
does, so the pins do not re-derive rustc's lookahead, and the tokens they read are always tokens
rustc reads. A CRLF line ending moves no count: the tested statement holds no line break, so a
literal holding one is not the statement in either reading, and a word ends at any whitespace. Every
count reads that one token set and skips the same groups: a `doc` attribute, outer (`#[doc ..]`) or
inner (`#![doc ..]`), its name raw (`r#doc`) or not, whatever its value, is neither a run nor a
word. A source the pins cannot lex is refused. What rustc does after it lexes (expanding a macro,
evaluating a `cfg`, running one path and not another) is not read, and issue #444 tracks it.

**R12c. The verdict pin reads `verdict.rs` as R16c reads `runs.rs`.** A source that opens with `#!`
and not `#![`, after one byte order mark, is refused. A copy of the enum in code compiled out beside
it stays a second declaration, as R12b says, and is refused.

**Section 9, read as this section says.**

- Section 9's opening paragraph says "Both pins read the tokens rustc's lexer produces (ADR-293), so
  a decoy spelled any way rustc lexes the same is read as rustc reads it". It reads: both pins read
  the tokens rustc's lexer produces for the source as rustc receives it (R16c), a first line rustc
  removes as a shebang is refused rather than read, and both counts skip the same groups; so a decoy
  spelled any way rustc's lexer reads as the same tokens is read as those tokens, and what a macro,
  a `cfg` or a path that never runs does with those tokens is not read (issue #444).
- R16b says "The pin reads `runs.rs` as the tokens rustc's lexer produces" and "a doc comment, outer
  or inner, becomes a `doc` attribute whose text is prose". They read: the pin reads `runs.rs` as
  R16c says; a doc comment becomes a `doc` attribute, and a `doc` attribute of any spelling is
  skipped by both counts whatever its value, so a macro that captures a doc attribute's text or
  tokens and makes code of them is not followed (issue #444).
- R16b says "A second statement turns the test red". It reads: a second statement turns the test red
  when a count reads it, as a second `sqlx::query!` call of the tested statement or as a second word
  `delete` in an identifier or a literal's cooked text; a second statement whose keyword the count
  cannot read (split across `concat!` parts, or held in another file), and one a macro makes from a
  doc attribute, stay green, and issue #444 tracks them.

**Refused although it would build correctly, each failing closed.** Each refusal below makes the pin
report a problem and the test fail; none lets a source through. New with this section: a source that
opens with `#!` and not `#![`, after one byte order mark, whatever the line holds, including a first
line rustc reads as an inner attribute because whitespace or a comment stands between `#!` and `[`;
and a prune that only a macro makes from a doc attribute's value, which both counts skip, so the pin
reads no prune. Named by section 9 and measured again here: a literal whose cooked text writes the
word `delete` beside the one prune; a copy of the call beside the one prune in code that never runs
(an item compiled out by `cfg`, a `cfg_attr` whose condition is false, an attribute a macro
discards), which is a second call or a second word; the tested statement handed to `sqlx::query!`
twice, both running; `r#delete` written as an identifier beside the prune; a source that does not
lex, a string whose line continuation skips a lone carriage return (which rustc accepts and
`proc-macro2` does not lex) included; on the verdict side, a copy of the enum in code compiled out
beside it, a conditional attribute on the enum (`#[cfg_attr(all(), must_use)]` included, raw path or
not), and an enum made by a macro or renamed by a `use` or an alias. Also refused and failing
closed, and named here for the first time: a `#[must_use]` whose reason is a raw string (`#[must_use
= r"why"]`, with any number of hashes) or a macro call (`#[must_use = concat!("why")]`), and an
attribute before the enum whose path is written raw (`#[r#derive(Clone)]`).

**Measured.** Each member of each population below was labelled by compiling it with rustc 1.97.0,
edition 2024, as ADR-293 records (the prune's truth through a stand-in `sqlx` that logs each
statement it runs, the verdict's under `#![deny(unused_must_use)]`), and read by the pins as they
stood before this section and as it leaves them. An escape is a member the pin passes and rustc's
truth fails; it is in the lexical class when a reader of the same tokens, given the text rustc lexes
after its input format, refuses it. After this section, 0 in-class escapes in every population:

| population | members | rustc accepts | in-class escapes before | in-class escapes after | out-of-class escapes after | refused although correct, after |
|---|---|---|---|---|---|---|
| section 9's generated population | 7,058 | 5,518 | 0 | 0 | 12 | 101 |
| the sixth review's population | 1,369 | 1,039 | 37 (a shebang line) | 0 | 32 | 46 |
| the sixth review's doc-attribute members, first set | 40 | 24 | 12 (the counts read different groups) | 0 | 0 | 0 |
| the sixth review's doc-attribute members, second set | 24 | 24 | 12 (the counts read different groups) | 0 | 0 | 0 |
| raw `r#doc` members | 8 | 8 | 4 (the counts read different groups) | 0 | 0 | 0 |
| the fifth round's planted sources (truth by construction) | 83 | not compiled | 0 | 0 | 3 | 0 |
| new: input format, group symmetry, raw identifiers | 860 | 741 | 92 (60 a shebang line, 32 the counts read different groups) | 0 | 18 | 141 |

The new members come from the Rust Reference's lexical chapters: the input format (a byte order
mark, CRLF, a lone CR, a first line opening with `#!` with and without an attribute after it, NUL
and other control characters), every group one count might skip and the other read (`#[doc ..]`,
`#![doc ..]`, `#[doc(..)]`, `#[cfg_attr(.., doc = ..)]`, attributes inside a macro's input), and raw
identifiers. Every out-of-class escape is an issue #444 shape: a keyword the count cannot read (9),
a copy that never runs beside a prune the count cannot read (21), a macro that captures a doc
attribute's text or tokens (18), and a copy that never runs, with or without a prune beside it (17);
none is unclassified.

**Files.**

| file | context | change |
|---|---|---|
| `crates/agent/tests/runs.rs` | `deck-streak-agent` | changed: the shebang refusal, the doc-attribute skip in the run count, and three tests |
| `crates/agent/tests/verdict.rs` | `deck-streak-agent` | changed: the shebang refusal and two tests |
| `scripts/mutation-rows.d/S04300-S04399.json` | repo | changed: rows S04356 to S04365 |
| `docs/red-first/SPEC-043.md` | docs | changed: a `Round 7` section with its red and green lines |
| `docs/specs/SPEC-043-agent-core-runner-gate-and-degradation.md` | docs | changed: this section and section 12 |
| `docs/decisions/ADR-293-the-agent-pins-read-rust-as-tokens.md` | docs | changed: an amendment |

**Rows.** S04356 and S04357 stop refusing a first line rustc removes as a shebang in the prune pin
and the verdict pin (killed by `runs::a_first_line_rustc_removes_as_a_shebang_is_refused` and
`verdict::a_first_line_rustc_removes_as_a_shebang_is_refused`); S04358 refuses an inner attribute
`#![` as a shebang and S04359 stops removing the byte order mark before the check (both killed by
the prune pin's shebang test). S04360 lets the run count read a doc attribute's tokens and S04361
stops recognising a raw `r#doc` (both killed by
`runs::a_query_written_inside_a_doc_attribute_is_neither_a_run_nor_a_word`). S04362 accepts two runs
of the prune and S04364 a test file that writes the statement nowhere (both killed by
`runs::each_count_names_the_number_it_read`); S04363 accepts a source that writes the word `delete`
nowhere (killed by the doc-attribute test); S04365 accepts two declarations of the enum (killed by
`verdict::a_copy_of_the_enum_compiled_out_beside_it_is_refused_as_a_second_declaration`). S04361 to
S04365 mutate comparisons the head already had, and each survived the tests as they stood before
this section.

**The tests.** `runs::a_first_line_rustc_removes_as_a_shebang_is_refused` and
`verdict::a_first_line_rustc_removes_as_a_shebang_is_refused` each put a first line opening with
`#!`, with and without a byte order mark, before the call or the `#[must_use]`, with eight gaps
after `#!` (none, a space, a tab, an interpreter path, a plain and a doc block comment, and NBSP or
U+3000 before a bracket, which rustc does not skip): 16 members each, and each reads a source that
opens with the inner attribute `#![allow(unused)]`.
`runs::a_query_written_inside_a_doc_attribute_is_neither_a_run_nor_a_word` hands a doc attribute
holding a copy of the call to four macros that discard it (matched as an expression, as token trees,
as an inner attribute, and a raw `r#doc` as one token tree), each around the prune or beside it,
against four prunes (a changed statement through `sqlx::query`, a changed table through
`sqlx::query!`, the tested prune, and none): 32 members. `runs::each_count_names_the_number_it_read`
asserts the number each count reads.
`verdict::a_copy_of_the_enum_compiled_out_beside_it_is_refused_as_a_second_declaration` refuses a
copy of the enum compiled out beside it.

This amendment changes no production code, no dependency and no other requirement. Its criteria are
A24 (the prune pin) and A25 (the verdict pin), listed in section 12, which follow the last A-number
of section 10.

## 12. Acceptance criteria of the 2026-09-30 amendment

| id | criterion | decided by |
|---|---|---|
| A24 | the prune pin reads `runs.rs` as rustc receives it: after one byte order mark, a source that opens with `#!` and not `#![` is refused and one that opens with `#![` is read; the run count and the word count skip the same groups, so a copy of the call written as a doc attribute's value, outer or inner, raw or not, is neither a run nor a word; and each count refuses every number but one, the test file's own count of the statement included | `a_first_line_rustc_removes_as_a_shebang_is_refused`, `a_query_written_inside_a_doc_attribute_is_neither_a_run_nor_a_word`, `each_count_names_the_number_it_read` |
| A25 | the verdict pin reads `verdict.rs` as rustc receives it: after one byte order mark, a source that opens with `#!` and not `#![` is refused and one that opens with `#![` is read; a copy of the enum compiled out beside it is a second declaration and is refused | `a_first_line_rustc_removes_as_a_shebang_is_refused`, `a_copy_of_the_enum_compiled_out_beside_it_is_refused_as_a_second_declaration` |

```acceptance
A24: cargo test -p deck-streak-agent --test runs -- --exact a_first_line_rustc_removes_as_a_shebang_is_refused a_query_written_inside_a_doc_attribute_is_neither_a_run_nor_a_word each_count_names_the_number_it_read
A25: cargo test -p deck-streak-agent --test verdict -- --exact a_first_line_rustc_removes_as_a_shebang_is_refused a_copy_of_the_enum_compiled_out_beside_it_is_refused_as_a_second_declaration
```
