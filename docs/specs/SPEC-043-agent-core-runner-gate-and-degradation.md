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
(criterion A20), and both still hold for every shape the SPEC promises. What follows closes the
remaining gap: a deliberate decoy could keep either test green.

**What the decoys were.** A20 found the prune's statement by looking for its quoted literal in
`crates/agent/src/runs.rs`, so a comment quoting that literal, beside a prune that no longer used
the index, kept the test green. A20 also wrote the statement twice in its own file, once as `PRUNE`
and once inside its `EXPLAIN QUERY PLAN` text, so a later change to one left the plan check on the
other. A18 read a line as an attribute when it began with `#[` and ended with `]`, so an item marked
`#[rustfmt::skip]` whose line ended in a `// ]` comment read as an attribute.

**R16b. The prune is one statement, written once.** The source of `runs.rs` holds exactly one
`DELETE FROM agent_runs`, counted case-insensitively with every run of whitespace collapsed, so a
second statement, and a quoted or commented copy of the first, each turn the test red. The test
file writes the statement once, as a `macro_rules!` literal; the tested statement (`PRUNE`) and the
text of its `EXPLAIN QUERY PLAN` are both made from that literal with `concat!`, and a scan of the
test file's own text asserts it holds the statement once, so a changed copy beside the macro turns
the test red. Production keeps its one `sqlx::query!` literal and its behaviour: the derived
literal feeds only the test's plain `sqlx::query_as` call, which takes a `&str`. sqlx's own
documentation says that "The query must be a string literal or a concatenation of string literals;
dynamic queries or those generated by other macros are not supported" (Context7,
`/websites/rs_sqlx`, the `query!` macro's requirements), so the `query!` macros are not given a
literal made by a macro.

**R12b. An attribute line is one whole attribute.** A line is read as an attribute only when the
bracket that closes its `#[` is the last character of the line. `#[rustfmt::skip] pub fn f() {} // ]`
is therefore an item, the scan above `pub enum Verdict` stops at it, and a `#[must_use]` above it
does not count for the verdict.

**Insertions.** None inside sections 1 to 8: the amendment is insert-only, so the criteria rows
and fences of A18 and A20 keep their words. Their strengthening is carried by the new criteria A21
(the prune is one statement), A22 (the statement is written once in the test) and A23 (an attribute
line is one whole attribute), listed in section 10, which follow the last A-number of section 8.

**Files.**

| file | context | change |
|---|---|---|
| `crates/agent/tests/runs.rs` | `deck-streak-agent` | changed: the one macro literal, the two scans, and three planted decoys |
| `crates/agent/tests/verdict.rs` | `deck-streak-agent` | changed: the whole-attribute rule, and one planted decoy |
| `scripts/mutation-rows.d/S04300-S04399.json` | repo | changed: rows S04328 to S04330 |
| `docs/red-first/SPEC-043.md` | docs | changed: an addendum with the decoys' red and green lines |
| `docs/specs/SPEC-043-agent-core-runner-gate-and-degradation.md` | docs | changed: this section and section 10 |
| `changelog.d/fix-agent-pins-404.md` | docs | added |

**Rows.** Each mutant below was installed against the tests as they stood before this amendment and
SURVIVED them, and each is KILLED by the tests as amended. S04328 adds a commented copy of the prune
statement to `runs.rs`; S04329 puts an item line ending in a `// ]` comment between the verdict's
`#[must_use]` and its `derive` (both killed by the tests named in the row); S04330 replaces the
plan's derived text with a separately typed copy of the statement. The killers are
`runs::the_prune_reads_agent_runs_through_the_created_at_index` for S04328 and S04330, and
`verdict::the_verdict_type_is_must_use` for S04329.

This amendment changes no production code, and no other requirement.

## 10. Acceptance criteria of the 2026-09-29 second amendment

| id | criterion | decided by |
|---|---|---|
| A21 | the source of `runs.rs` holds exactly one delete statement, so a second statement or a commented or quoted copy of the first is refused | `the_prune_reads_agent_runs_through_the_created_at_index`, `a_comment_quoting_the_prune_beside_a_prune_that_skips_the_index_is_refused`, `a_second_delete_statement_is_refused_however_it_is_spelled` |
| A22 | the tested statement and the text of its plan are made from one literal, and a changed copy of the statement in the test file is refused | `the_prune_reads_agent_runs_through_the_created_at_index`, `a_changed_copy_of_the_statement_in_the_plan_string_is_refused` |
| A23 | a line is read as an attribute only when the bracket closing its `#[` ends the line, so an item line ending in a `// ]` comment is not one | `the_verdict_type_is_must_use`, `an_item_ending_in_a_bracket_comment_is_not_an_attribute` |

```acceptance
A21: cargo test -p deck-streak-agent --test runs -- --exact the_prune_reads_agent_runs_through_the_created_at_index a_comment_quoting_the_prune_beside_a_prune_that_skips_the_index_is_refused a_second_delete_statement_is_refused_however_it_is_spelled
A22: cargo test -p deck-streak-agent --test runs -- --exact the_prune_reads_agent_runs_through_the_created_at_index a_changed_copy_of_the_statement_in_the_plan_string_is_refused
A23: cargo test -p deck-streak-agent --test verdict -- --exact the_verdict_type_is_must_use an_item_ending_in_a_bracket_comment_is_not_an_attribute
```
