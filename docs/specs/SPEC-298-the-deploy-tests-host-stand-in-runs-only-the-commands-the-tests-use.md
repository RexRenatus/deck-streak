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
  `AssertionError` naming `DECKSTREAK_DEPLOY_ELEVATE` when the environment the started program will
  see does not set it. That is the `env` the call passes (an `env` of `None` is refused, because the
  child would inherit the test process's), the entries the caller says it received, and the file a
  call sources before the script, judged by its effect: a file that holds anything but plain
  `NAME=value` lines is refused, since a branch or an `unset` could hide the name.
- **R4.** Every check this SPEC relies on to enumerate a class finds every member of it, by any
  spelling, or refuses what it cannot read (R7). The census of start sites counts each scope and
  kind of site in the test module and in every module of `scripts/tests` that imports it, and finds
  no call that starts a program outside `launch`, the git wrapper of `World` and the public-scrub
  test, besides the two calls the stand-in tests make to run the stand-in itself.
- **R5.** A test that gives the stand-in a privilege-named `argv[0]` puts a recording shim of that
  name first on its own `PATH`, so a stand-in that ran it would reach the shim and fail the test on
  its record.
- **R6.** `deploy/deploy.sh` is unchanged.
- **R7.** The four enumerating checks are functions of the text they read, in
  `scripts/tests/_standin_checks.py`, so a killer can plant one member of the class into a copy of
  the text and run the real check on it: the stub scan (every text the module holds or builds that
  carries an argument expansion outside a reviewed list of logging shapes is refused, whichever way
  the text is built; the stubs that run their arguments are listed with a reason), the derivation
  (every line of the deploy script that mentions the host or the elevation setting is reviewed, and
  every way the tests give the setting a value is read or refused), the census, and `launch`'s
  refusal. A member of the class that a check passes fails the killer, which names it.

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
| A9 | the only stubs in the module that run their first argument are the host stand-in, the trace that hands it to `runpy` (which starts no binary), and the tests' own launch of a deploy script; every other stub text is refused by construction | `test_deploy_standin.py` `the_only_stub_that_runs_its_first_argument` |
| A10 | the stub scan refuses each member of a generated population of stub bodies and ways of building a stub's text, each planted into a copy of the module | `test_deploy_standin.py` `every_route_by_which_a_stub_runs_its_arguments_is_refused` |
| A11 | the derivation adds to its set or refuses each member of a generated population of host-call spellings and ways of giving the setting a value | `test_deploy_standin.py` `every_host_call_and_every_elevation_value_adds_to_the_set_or_is_refused` |
| A12 | the census finds or refuses each member of a generated population of ways to start a program, and every module that imports the deploy tests | `test_deploy_standin.py` `every_start_spelling_is_found_or_refused` |
| A13 | `launch` refuses each member of a generated population of environments the started program would see without the setting | `test_deploy_standin.py` `an_environment_the_program_will_see_unset_is_refused` |

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
A10: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k every_route_by_which_a_stub_runs_its_arguments_is_refused
A11: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k every_host_call_and_every_elevation_value_adds_to_the_set_or_is_refused
A12: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k every_start_spelling_is_found_or_refused
A13: python3 -m unittest discover -s scripts/tests -p test_deploy_standin.py -k an_environment_the_program_will_see_unset_is_refused
```

The population of A2 is generated from the derived listed set, never from a list of refused
commands: each listed name is varied by path prefix, case, suffix, padding, a glob or class
character and a leading dash. A2 and A5 print how many members they examined and refuse zero. A10 to A13 each print the size of
their population, drawn from the routes of the class and never from a list of what the real check
refuses, and fail naming every member the real check passed; the counts are the ones the pull
request's CI run prints.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/tests/test_deploy_scripts.py` | `repo` | the stand-in, `HOST_ALLOWED`, `launch` (which judges the environment the program will see), and three call sites routed through it |
| `scripts/tests/test_deploy_standin.py` | `repo` | added |
| `scripts/tests/_standin_checks.py` | `repo` | added: the four enumerating checks, as functions of the text they read |
| `scripts/mutation-rows.d/S29800-S29899.json` | `repo` | added: fourteen rows |
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
- It does not plant the route that hands a stub's arguments to the shell's own evaluator. That
  route is closed by construction, since the stub scan refuses any argument expansion outside the
  reviewed logging shapes, and it is not written out as a member of a population (#502).

## 6. Risks

- **A new legitimate host shape fails the suite until it is listed.** That is the test working: A1
  names the shape the stand-in received and the list it was compared with.
- **The suite that measures this runs only in CI.** The evidence is the pull request's checks and
  the mutation job's rows S29800 to S29813.
