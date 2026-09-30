# SPEC-298: the deploy tests' host stand-in runs only the commands the tests use

- **Wave:** W4. **Issue:** #502. **Context(s):** `repo` (`scripts/tests/` and `docs/`).
- **Decided by:** ADR-298 (this SPEC's own: the stand-in is default-deny, and what it was chosen
  against).
- **Status:** built. It holds `docs/red-first/SPEC-298.md`.

## 1. The problem, measured

- **The stand-in executes whatever it is given.** `HOST` in `scripts/tests/test_deploy_scripts.py`
  is the script the deploy tests put where the deploy host command goes. Its last line was
  `exec "$@"`, so any first argument ran. The deploy script prefixes its host step with the command
  named by `DECKSTREAK_DEPLOY_ELEVATE`, falling back to a privilege command when the variable is
  unset (`${DECKSTREAK_DEPLOY_ELEVATE-sudo}` in `deploy/deploy.sh`). The shared test environment
  sets it empty, so the fallback is never taken today; a test environment built without the
  variable, or a change of the fallback's form, would hand the stand-in a privilege command and the
  stand-in would run it (#502).
- **Nothing refused an environment without the setting.** `World.run` and two direct calls in the
  Caddy test started a deploy script with whatever environment they built, and no test read the
  environment's names (#502).

## 2. Requirements

- **R1.** The stand-in runs an `argv[0]` only if it is one of the values in `HOST_ALLOWED`, the one
  list. Any other `argv[0]`, including the empty one and every spelling of a listed name that is not
  the name, exits non-zero, prints one line naming it, and runs nothing. No list of refused commands
  exists.
- **R2.** `HOST_ALLOWED` equals the set of `argv[0]` values the deploy tests' calls make. The set is
  derived twice and compared: from the deploy script's host call and the elevation values the test
  module gives, and from the stand-in's own log of the `argv[0]` it received over a deploy, a second
  deploy and a rollback. A new shape fails the test until it is listed.
- **R3.** Every call that starts a deploy script goes through `launch`, which raises an
  `AssertionError` naming `DECKSTREAK_DEPLOY_ELEVATE` when the environment it is given does not set
  it. The names are read from a mapping's keys, from a list's `NAME=value` entries, and from the
  file a call sources before the script, so the settings-refusal tests keep their meaning.
- **R4.** A census over the test module's syntax tree finds no call that starts a program outside
  `launch`, the git wrapper of `World` and the public-scrub test.
- **R5.** A test that gives the stand-in a privilege-named `argv[0]` puts a recording shim of that
  name first on its own `PATH`, so a stand-in that ran it would reach the shim and fail the test on
  its record.
- **R6.** `deploy/deploy.sh` is unchanged.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | `HOST_ALLOWED` is one tuple without repeats, equals the set derived from the deploy script and the test module, and equals the set of `argv[0]` the stand-in logged over a deploy, a second deploy and a rollback | `test_deploy_standin.py` `the_list_lives_in_one_place` |
| A2 | the stand-in refuses each member of a generated population (privilege names, arbitrary names, spellings of a listed name, the empty name and option-shaped names) with a non-zero status, nothing on stdout, exactly one line naming it on stderr, and no program run | `test_deploy_standin.py` `a_command_outside_the_list_is_refused_by_name_and_run_by_nothing` |
| A3 | the stand-in runs each listed shape | `test_deploy_standin.py` `a_listed_command_is_run` |
| A4 | a deploy with the setting empty reaches no privilege command and gives the stand-in only the listed shape | `test_deploy_standin.py` `an_empty_elevation_setting_runs_the_host_step` |
| A5 | `launch` refuses each generated environment that leaves the setting unset, naming it, and admits each that names it | `test_deploy_standin.py` `a_call_whose_environment_leaves_the_setting_unset` |
| A6 | the shared environment names the setting | `test_deploy_standin.py` `the_shared_environment_names_the_setting` |
| A7 | only `launch`, the git wrapper and the scrub test start a program | `test_deploy_standin.py` `only_the_helper_the_git_wrapper_and_the_scrub_call_start_a_program` |
| A8 | `launch` reads the environment's names and refuses before it starts a program | `test_deploy_standin.py` `the_helper_names_the_setting_before_it_starts_anything` |
| A9 | the only stub in the module that runs its first argument is the host stand-in | `test_deploy_standin.py` `the_only_stub_that_runs_its_first_argument` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k the_list_lives_in_one_place
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k a_command_outside_the_list_is_refused_by_name_and_run_by_nothing
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k a_listed_command_is_run
A4: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k an_empty_elevation_setting_runs_the_host_step
A5: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k a_call_whose_environment_leaves_the_setting_unset
A6: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k the_shared_environment_names_the_setting
A7: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k only_the_helper_the_git_wrapper_and_the_scrub_call_start_a_program
A8: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k the_helper_names_the_setting_before_it_starts_anything
A9: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k the_only_stub_that_runs_its_first_argument
```

The population of A2 is generated from the derived listed set, never from a list of refused
commands: each listed name is varied by path prefix, case, suffix, padding, a glob or class
character and a leading dash. A2 and A5 print how many members they examined and refuse zero.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/tests/test_deploy_scripts.py` | `repo` | the stand-in, `HOST_ALLOWED`, `launch`, and three call sites routed through it |
| `scripts/tests/test_deploy_standin.py` | `repo` | added |
| `scripts/mutation-rows.d/S29800-S29899.json` | `repo` | added: nine rows |
| `docs/specs/SPEC-298-the-deploy-tests-host-stand-in-runs-only-the-commands-the-tests-use.md` | `repo` | added |
| `docs/decisions/ADR-298-the-host-stand-in-is-default-deny-not-a-deny-list.md` | `repo` | added |
| `docs/red-first/SPEC-298.md` | `repo` | added |
| `changelog.d/test-deploy-standin-502.md` | `repo` | added |

## 5. What this does NOT do

- It does not change `deploy/deploy.sh`, including the fallback of the elevation setting; that is
  production behaviour and this delivery changes tests only (#502).
- It does not change the dispatch-shard stand-in, which has the same rule under its own issue (#497).
- It does not add a refusal list of commands anywhere: the stand-in's only knowledge is the shapes
  it runs (#502).

## 6. Risks

- **A new legitimate host shape fails the suite until it is listed.** That is the test working: A1
  names the shape the stand-in received and the list it was compared with.
- **The suite that measures this runs only in CI.** The deploy tests are never run on the
  maintainer's machine, so the evidence is the pull request's checks and the mutation job's rows
  S29800 to S29808.
