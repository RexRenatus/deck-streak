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
