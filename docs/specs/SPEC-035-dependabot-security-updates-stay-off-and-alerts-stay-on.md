# SPEC-035: Dependabot's security updates stay off, its alerts stay on, and a fixable advisory becomes a pull request into dev

- **Wave:** W0. **Issue:** #23 (epic #1). **Context(s):** `repo` (`scripts/github-setup.sh`,
  `docs/OWNER-SETUP.md`, `SECURITY.md`).
- **Decided by:** ADR-036, this SPEC's own (security fixes are triaged into dev, never opened
  against main). Also ADR-017 (the branch model), ADR-034 (the release flow) and ADR-035 (only this
  repository's dev reaches main).
- **Status:** judged: delivered with its tests.

## 1. The problem, measured

- **Security updates open against main.**
  - The maintainer measured a Dependabot security-update run
    (`npm_and_yarn in /. for cookie`, whose job's command was `security`) on head branch `main` at
    20:58:52Z. That is the moment `scripts/github-setup.sh security` enabled automated security
    fixes (`gh api repos/RexRenatus/deck-streak/automated-security-fixes`: `{"enabled": true}`).
  - `.github/dependabot.yml`'s `target-branch: dev` governs version updates only.
- **Such a pull request can never merge.**
  - That run failed only because pnpm could not move a pinned transitive dependency.
  - The next fixable advisory would open a pull request into `main` from a branch that is not
    `dev`. `base-is-dev` fails it, the required `ci` fails with it, and it can never merge (ADR-035,
    probe #187). It would sit there as noise, and the fix would reach `dev` only by hand.
- **The setup script re-enables it.** `security()` in `scripts/github-setup.sh` puts
  `automated-security-fixes` on every run, so turning the setting off by hand would not last.

## 2. Requirements

R1. `scripts/github-setup.sh security` turns Dependabot's automated security fixes OFF
    (`DELETE repos/{repo}/automated-security-fixes`) and never turns them on. A re-run leaves them
    off.
R2. It keeps Dependabot alerts on (`PUT repos/{repo}/vulnerability-alerts`), with secret scanning,
    push protection and private vulnerability reporting.
R3. `SECURITY.md` and `docs/OWNER-SETUP.md` say how an alert is handled. The maintainer triages it,
    and a fixable one becomes a pull request into `dev` that ships with the next release, or at once
    as a patch release. Version updates keep flowing into `dev` (`.github/dependabot.yml`).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the security setup turns automated security fixes off and never on | `test_github_setup.py` |
| A2 | the security setup keeps Dependabot alerts, secret scanning and private reporting on | `test_github_setup.py` |
| A3 | the owner and security documents say an alert becomes a pull request into dev | `test_github_setup.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_github_setup.py -k the_security_setup_turns_automated_security_fixes_off_and_never_on
A2: python3 -m unittest discover -s scripts/tests -p test_github_setup.py -k the_security_setup_keeps_alerts_scanning_and_private_reporting_on
A3: python3 -m unittest discover -s scripts/tests -p test_github_setup.py -k the_documents_say_an_alert_becomes_a_pull_request_into_dev
```

A1 and A2 run `scripts/github-setup.sh security` with a fake `gh` first on `PATH`. The fake records
every call and answers success, so the test judges the calls the script makes and never touches
GitHub.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/github-setup.sh` | `repo` | changed: security fixes deleted, never put |
| `scripts/tests/test_github_setup.py` | `repo` | added: A1 to A3 |
| `SECURITY.md` | `repo` | changed: how an advisory is handled |
| `docs/OWNER-SETUP.md` | `repo` | changed: what the security step sets |
| `docs/decisions/ADR-036-security-fixes-are-triaged-into-dev-not-automated.md` | `repo` | added |
| `docs/specs/SPEC-035-dependabot-security-updates-stay-off-and-alerts-stay-on.md` | `repo` | added |
| `docs/red-first/SPEC-035.md` | `repo` | added |
| `changelog.d/fix-dependabot-security-updates.md` | `repo` | added |

## 5. What this does NOT do

- It changes no version update: cargo, npm and the actions still move into `dev` weekly, and majors
  wait for the house radar (#23).
- It adds no scanner of its own. The maintainer's daily scan will read this repository's lockfiles
  as well, and its findings for this public repository are filed scrubbed (#23).
- It does not change the default branch, which stays `main`, the branch visitors see (#160).

## 6. Risks

- **An advisory waits for a person.** Alerts stay on and notify, and the maintainer's daily scan
  reads the lockfiles. A fix is one pull request into `dev`, plus a patch release when it cannot
  wait.
- **The setting is flipped back by hand.** A re-run of `github-setup.sh security` turns it off
  again, and A1 holds the script.
