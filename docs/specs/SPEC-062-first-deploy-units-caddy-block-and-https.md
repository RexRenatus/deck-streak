# SPEC-062: DeckStreak's first tagged release is installed on the host by one command, served behind its own Caddy site block over HTTPS, and reversed by one command

- **Wave:** W2. **Issue:** #42 (epic #3). **Context(s):** `deploy` (`deploy/deploy.sh`,
  `deploy/rollback.sh`, `deploy/scripts/`), `repo` (`.github/workflows/release.yml`,
  `RELEASING.md`); the box-run packs' private wiring (ADR-069).
- **Decided by:** ADR-007 (a Caddy site block fronts the Mini App and its API, the API on
  loopback), ADR-010 (units per role, the deploy from a tag with a rollback), ADR-011 (side by side
  until the owner's go at cutover), ADR-017 and ADR-034 (releases from `main` tags only, built by
  CI, never deployed by it), ADR-055 (only a push to `dev` or `main` saves a cache), ADR-032 (the
  templates and the host budget), ADR-038 (credentials at each start), ADR-054 (the first deploy
  needs no AI route), ADR-061 (host values as drop-ins, the Caddy block rendered), and this SPEC's
  ADR-062 (a deploy installs only a release whose provenance and digests verify).
- **Waits for:** every host finding tracked privately under gate 8 (#167) closed first; then owner
  gate 2 (#161) for the units, the host-wide journald drop-in, the Caddy change and the first start;
  gate 6 (#165) for the owner's user id; SPEC-061's rail. The Mini App's registration with BotFather
  is the owner's act after HTTPS works (#171).
- **Status:** built (moved from `docs/specs/planned/` by the delivery, with its tests and
  `docs/red-first/SPEC-062.md`, ADR-016).

## 1. The problem, measured

- **A tag builds nothing yet.** RELEASING.md's step 3 says the release workflow arrives with the
  first deploy (#42), and "Until that workflow exists, a tag marks the release and carries no
  build"; SPEC-034 routed that workflow here. The release-ops pack, judged in the box run
  (ADR-069), also asks for a deploy workflow, which by ADR-010 and ADR-017 this repository will
  never have: the deploy runs from the maintainer's machine.
- **No deploy or rollback script exists** (SPEC-032 left both to this issue), and the box-run
  packs' wiring holds the release pack's rollback row until they do (ADR-069).
- **The Mini App is served by one Caddy site block (ADR-007).** DeckStreak adds that block to the
  host's Caddy configuration and changes nothing else in it: each change is validated on a copy
  first, and reversed by removing exactly what DeckStreak added. The Mini App's host name is
  private configuration, preferably a subdomain of a domain the owner owns (#168).
- **Memory, in the budget's terms.** DeckStreak's share is ADR-032's (`deploy/host-budget.json`):
  640 MiB, with a ceiling for each unit. Gate 2 checks, privately, that the host holds that share
  with every DeckStreak ceiling reached at once, beside everything else it runs; the memory watch's
  first day measures the real use (R10), and a shortfall is the owner's resize decision (ADR-011).
- **Disk, as DeckStreak's own need.** What this deploy puts on the disk is DeckStreak's: its
  collection copy and, during a full download, as much again beside it (SPEC-022's risk); three
  releases side by side, whose size the release workflow reports (R1); the database; and the
  journal, capped by SPEC-021's drop-in. Gate 2 measures that need against the host's free space,
  recorded privately.
- **The first deploy has no AI route** (ADR-054): no device key, no tunnel, no proxy, and every
  readings night records `ai_route_absent` without an alert.

## 2. Requirements

R1. `.github/workflows/release.yml` runs on a push of a SemVer tag (`v1.2.3` admitted, `v1.2`,
    `v1.2.3.4` and `latest` not). One job on the pinned `ubuntu-24.04` image, with a full-history
    checkout, proves the tag's commit is on `main` with `git merge-base --is-ancestor`, builds
    `deckstreakd` for x86-64 Linux and the Mini App's static build once, creates the release
    as a draft, attaches one tarball, its `SHA256SUMS` and a build-provenance attestation, reports
    the unpacked size, and publishes the release after the last upload. The workflow's top-level
    permissions are `contents: read`; only that job adds `contents: write`, `id-token: write` and
    `attestations: write`; every action is pinned by a full commit SHA; no secret but the job's own
    token is read. It may restore the caches that pushes to `main` saved, saves none, and uses no
    action that saves a cache by itself (SPEC-038 R2, ADR-055).
R2. The tarball holds what the host runs and nothing private: `deckstreakd`, the Mini App's build,
    `deploy/`, `agent/`, `ai-safety.json` once SPEC-043 has added it (the workflow copies it when
    present), and a manifest listing every file with its SHA-256. It
    holds no pack probe: the output gate's probes reach the host through the private rail, at the
    path the environment file names (SPEC-043 R11, ADR-069).
R3. `deploy/deploy.sh <tag>` runs on the maintainer's machine. It refuses unless `<tag>` is an
    annotated SemVer tag whose commit is an ancestor of `origin/main` after a fetch. It downloads the
    release's assets, verifies the tarball's attestation (`gh attestation verify --repo` this
    repository, `--signer-workflow` its release workflow) and every digest in `SHA256SUMS`, and only
    then reaches the host through the private rail's host command (SPEC-061). On the host it unpacks
    into `releases/<tag>.partial/`, renames that to `releases/<tag>/`, links `current` under a
    temporary name and moves it over `current` with `mv -T`, installs the release's unit templates
    byte for byte, reloads systemd, runs `effective-check.py` over every unit (SPEC-061 R7), restarts
    the long-running units, and waits for the API's readiness on its loopback listener and the bot's
    `READY=1`, each within the units' own start timeout (`TimeoutStartSec=180`, SPEC-032 R1). Those
    switch commands are written in `deploy/deploy.sh` itself, where the box-run packs read them
    (ADR-069).
R4. When readiness does not arrive in time, `deploy/deploy.sh` switches `current` back to the
    release it replaced, reinstalls that release's unit templates, restarts, and exits non-zero
    naming the unit that did not become ready. A restart that succeeds is not readiness: a
    release whose readiness never arrives is switched back from all the same. The deploy shows every
    unit's effective configuration to the check; a unit that cannot be shown refuses the deploy and is
    named, and the drop-in directories it removes are the rail's own units' alone.
R5. `deploy/rollback.sh <tag>` makes `<tag>` current again with R3's switch and unit install when
    `releases/<tag>/` is on the host, and otherwise deploys it anew through R3's verification.
R6. The host keeps the current release and the two before it. The deploy removes an older release
    directory only after a switch whose readiness succeeded, and it never removes the release it
    replaced or the one it made current.
R7. `deploy/scripts/render-caddy.py` renders `deploy/caddy/deck-streak.caddy` (SPEC-032 R6) for the
    host by replacing its `{$DECKSTREAK_HOST}`, `{$DECKSTREAK_WEB_ROOT}` and
    `{$DECKSTREAK_API_UPSTREAM}` with values from the private configuration, and refuses output that
    still holds `{$` or an upstream that is not a loopback address (ADR-061). The install adds the
    rendered file (rendered from the tag's own `deploy/caddy/deck-streak.caddy`) and one `import` line
    for it to the host's Caddyfile, and changes nothing else in
    it, only after `caddy validate` and `caddy adapt --validate`, each with the Caddyfile adapter, pass on a copy of the whole
    configuration; the change is applied by Caddy's graceful reload, which keeps the running
    configuration when the new one fails. The rollback removes the `import` line first and the file
    second, validates and reloads.
R8. HTTPS comes from Caddy's automatic certificate for the block's host name, and the change opens
    nothing new to the outside (CHARTER 10). The host name is a subdomain of a domain the owner owns
    (#168), which is preferred; a name that carries the host's address is never used while any host
    finding under gate 8 (#167) is open. After HTTPS works, the owner registers the Mini App with
    BotFather (#171).
R9. The first deploy runs with the AI route absent (ADR-054), the readings' vault archive switch off
    (SPEC-053 R8) and no vault setting required. It installs SPEC-021's journald drop-in only with
    the owner's go, because it is host-wide.
R10. Every check of the private rail's health-check list is read before the deploy and after it.
    The memory watch's first day (SPEC-031) records each DeckStreak unit's peak and the host's
    available memory. A unit that lives at its `MemoryHigh` in normal use, or available memory below
    DeckStreak's share per ADR-032 (`deploy/host-budget.json`), is an owner decision on a resize
    (ADR-011), with the numbers recorded privately.
R11. The box-run packs' private wiring moves release-ops to `enforced`, ends its rollback row's
    wait on this issue, and excludes its two deploy-workflow rows with the reason that the deploy
    runs from the maintainer's machine by design (ADR-010, ADR-017), so no deploy workflow exists
    for them to judge. The delivery hands that change back to the maintainer, and the box run on its
    head is its evidence (ADR-069).
R12. The first week's request counts of the API and the runs of the bot and the jobs are recorded in
    the maintainer's private notes, as the evidence the bot's and the jobs' SLOs will be sized from
    (SPEC-031 left them to a measurement on the host).
R13. When SPEC-048 has landed, the install gives every unit that writes readings one common lock
    directory (SPEC-048 R2): a directory inside the service's state directory, owned by the service
    user, named by its setting in the environment file, and present before those units start, since
    each of them refuses to start without it (SPEC-048's start check). Until then no unit reads it,
    gate E1 creates it, and this delivery builds no part of it.
R14. The sync login is loaded by the sync job alone. `deck-streak-job@.service` carries no
    `LoadCredential=` for it; a drop-in in the `.service.d` directory of the `sync` instance, beside the templates in
    `deploy/systemd/`, carries the two lines, so the liveness and maintenance instances request no sync credential (ADR-061,
    amended). `deploy/scripts/credential-pairs.py` lists the drop-in's pairs under the instance,
    and `deploy/scripts/effective-check.py` accepts a shipped drop-in beside the rail's own. The
    deploy installs the drop-in directories byte for byte with the unit templates.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the deploy refuses a tag whose commit is not on `main`, and a lightweight tag | `test_deploy_scripts.py` |
| A2 | the deploy refuses an asset whose digest differs from `SHA256SUMS`, and a tarball whose attestation does not verify, before it reaches the host | `test_deploy_scripts.py` |
| A3 | the deploy unpacks beside the previous releases, switches `current` by a temporary link and `mv -T`, and installs the release's unit templates byte for byte | `test_deploy_scripts.py` |
| A4 | the deploy switches back and names the unit when readiness does not arrive within its bound | `test_deploy_scripts.py` |
| A5 | the rollback makes a kept release current again, with its own unit templates | `test_deploy_scripts.py` |
| A6 | the deploy keeps the current and two previous releases and removes older ones only after a ready switch | `test_deploy_scripts.py` |
| A7 | the Caddy render refuses an unfilled placeholder and a non-loopback upstream, and its output keeps every header SPEC-032 R6 requires | `test_caddy_render.py` |
| A8 | the release workflow runs only on SemVer tags, proves the tag is on `main` on a full-history checkout, creates a draft, attests provenance and publishes after the last upload | `test_release_workflow.py` |
| A9 | the release workflow's top-level permissions are read-only, only its release job may write, it runs on the pinned image, every action is pinned by a full commit SHA, and no step saves a cache (examined count reported) | `test_release_workflow.py` |
| A10 | no deploy script names a private value, and a planted one is refused by the public scrub | `test_deploy_scripts.py`; `scripts/public-scrub.py` |
| A11 | the liveness and maintenance instances request no sync credential, the sync instance still requests the sync login, and the pair list and the effective check accept the drop-in | `test_deploy_templates.py` |
| A12 | a restart that succeeds and readiness that never arrives is switched back from, naming the unit | `test_deploy_scripts.py` |
| A13 | the release a deploy replaced is never pruned, including after a rollback | `test_deploy_scripts.py` |
| A14 | another unit's drop-ins survive a deploy, a rollback and a switch back byte for byte | `test_deploy_scripts.py` |
| A15 | a unit whose effective configuration cannot be shown refuses the deploy, names the unit and leaves `current` unchanged | `test_deploy_scripts.py` |
| A16 | the effective check admits a shipped drop-in name only in the directory beside the rail's own | `test_deploy_templates.py` |
| A17 | the Caddy install and removal validate the candidate copy with the Caddyfile adapter | `test_deploy_scripts.py` |
| A18 | the release workflow's tag guard, run against a synthetic origin, admits an annotated tag on `main` and refuses a lightweight tag and a tag off `main` | `test_release_workflow.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_deploy_refuses_a_tag_off_main_and_a_lightweight_tag
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_deploy_refuses_a_bad_digest_or_attestation_before_the_host
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_deploy_installs_beside_and_switches_current_atomically
A4: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_deploy_switches_back_when_readiness_does_not_arrive
A5: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_rollback_makes_a_kept_release_current_again
A6: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_deploy_keeps_three_releases_and_prunes_after_a_ready_switch
A7: python3 -m unittest discover -s scripts/tests -p test_caddy_render.py -k test_the_render_refuses_placeholders_and_keeps_the_headers
A8: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_release_runs_on_semver_tags_and_publishes_last
A9: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_release_workflow_is_read_only_and_pinned
A10: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_no_deploy_script_names_a_private_value
A11: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_login_is_loaded_by_the_sync_job_alone
A12: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_a_restart_that_succeeds_and_readiness_that_never_arrives_switches_back
A13: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_release_a_deploy_replaced_is_never_pruned
A14: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_a_deploy_a_rollback_and_a_switch_back_leave_them_byte_for_byte
A15: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_a_unit_that_cannot_be_shown_refuses_the_deploy_and_names_it
A16: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_a_shipped_drop_in_name_is_admitted_beside_the_rails_own_alone
A17: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_caddy_calls_name_the_caddyfile_adapter_for_the_candidate_copy
A18: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard
```

A1 to A6 and A12 to A15 and A17 run the scripts against a synthetic repository with its own tags, a synthetic release
(tarball, `SHA256SUMS`), a stub `gh` whose attestation verdict each test chooses, and a stub host
command that applies the host side inside a `TemporaryDirectory` with stub `systemctl` and readiness
probes. None of them reaches a network or a host. The box-run packs judge the release workflow and
the deploy scripts as well (ADR-069); that verdict is the delivery's evidence, not a criterion here.

## 4. The owner's gates and the evidence they record

The exact commands, the host name and the host's paths are in the maintainer's private gate packet;
this SPEC names each step only.

| step | gate | what is approved | evidence recorded (privately) | rollback |
|---|---|---|---|---|
| E0 | 8 (#167) | every host finding tracked privately under gate 8, closed | the close-out of each finding, recorded privately | none: the deploy waits |
| E1 | 2 (#161) | the service user, the release root, the readings lock directory and the units | the health-check list read before; `systemd-analyze verify` and `systemd-analyze security --threshold` over the installed units; `effective-check.py` | stop and disable the units, remove them, reload |
| E2 | 2 (#161) | the journald drop-in, host-wide | its effective `journalctl` limits | remove the drop-in, restart journald |
| E3 | 2 (#161) | the first `deploy/deploy.sh <tag>` and one `deploy/rollback.sh` | both logs, readiness each time (#42's first criterion) | `deploy/rollback.sh` to the previous tag |
| E4 | 2 (#161) | the Caddy block | the whole configuration validated and adapted on a copy; the reload; each site on the health-check list answering before and after | R7's rollback |
| E5 | 2 (#161) | HTTPS | the certificate issued; the Mini App's page and headers over HTTPS; the health routes refused from outside; cyber-pipeline's black-box rows after the owner authorizes the target | R7's rollback |
| E6 | owner (#171) | the Mini App registered with BotFather | the bot's profile shows Launch app, and the Mini App opens inside Telegram | unset the menu button |
| E7 | 2 (#161) | the memory watch's first day | each unit's peak and the host's available memory (R10) | a resize or a faster cutover, the owner's decision |

## 5. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/release.yml` | repo | added: the tag-triggered release |
| `deploy/deploy.sh` | deploy | added |
| `deploy/rollback.sh` | deploy | added |
| `deploy/scripts/render-caddy.py` | deploy | added: the Caddy block's render |
| `deploy/README.md` | deploy | changed: the deploy, the rollback and the Caddy install |
| `RELEASING.md` | repo | changed: steps 3, 4 and 7 as built |
| the box-run packs' private wiring (ADR-069) | the maintainer's | changed: release-ops `enforced`, its rollback row's wait ended, two rows excluded with their reason |
| `scripts/tests/test_deploy_scripts.py` | repo | added: A1 to A6, A10, A12 to A15, A17 |
| `scripts/tests/test_caddy_render.py` | repo | added: A7 |
| `scripts/tests/test_release_workflow.py` | repo | added: A8, A9 and A18 |
| `docs/specs/SPEC-062-first-deploy-units-caddy-block-and-https.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-062-a-deploy-installs-only-a-release-whose-provenance-and-digests-verify.md` | docs | changed: status accepted |
| `docs/decisions/ADR-061-host-values-reach-units-as-drop-ins-and-caddy-as-a-rendered-file.md` | docs | changed: status accepted, if SPEC-061 has not accepted it first |
| `docs/red-first/SPEC-062.md` | docs | added |
| `deploy/systemd/deck-streak-job@.service` | deploy | changed: R14, the sync login lines removed |
| `deploy/systemd/` drop-in `20-sync-login.conf` in the `sync` instance's `.service.d` directory | deploy | added: R14, the sync login |
| `deploy/scripts/credential-pairs.py`, `deploy/scripts/effective-check.py` | deploy | changed: R14, instance drop-ins; the shipped-name rule (A16) |
| `scripts/tests/test_deploy_templates.py` | repo | changed: A11, A16 |
| `docs/decisions/ADR-061-...md` | docs | one dated Amendment section (R14) |
| the private rail's map, rendered drop-ins and tests (`rail/`) | the maintainer's | changed: R14, committed privately |
| `scripts/mutation-rows.d/S06200-S06299.json` | repo | added: the mutation rows S06201 to S06222 |
| `changelog.d/` fragment | repo | added |

## 6. What this does NOT do

- It enables no AI route, opens no tunnel and installs no device key (#43, #162).
- It creates no folder in the vault and switches the readings' vault archive on for no one (#45).
- It builds no backup, replica or restore drill (#44).
- It declares no SLO for the bot or the jobs; it records their first week, and the declarations
  follow with every pack enforced (#60).
- It buys or points no domain, and serves no landing page (#168, #59).
- It closes no host finding; the deploy waits until the owner has closed every one (#167).
- It changes nothing of the host's network configuration or DNS records, and resizes nothing (#161).
- It performs no step of the cutover (#164).
- It makes a failed Caddy reload after the file swap restore the previous Caddyfile (#321).

## 7. Risks

- **A reload of Caddy touches every site it serves.** DeckStreak's change is validated on a copy of
  the whole configuration first, a reload that fails leaves the running configuration in place, and
  each site on the health-check list is read before and after; the rollback removes one line and
  one file (R7).
- **A hardening option breaks a role on its first start.** The box-run packs' unit rows and
  `systemd-analyze security` pass before the start; a denied call shows as the unit's failure,
  paged through the alert unit, and the deploy switches back (R4).
- **A release whose migration changed data cannot be rolled back by the binary.** Schema changes
  ship as expand, then contract (RELEASING.md), and the first deploy migrates an empty database.
- **The certificate is not issued.** Caddy retries on its own schedule, nothing else it serves
  depends on the block's certificate, and E5 records the issue before the owner registers the
  Mini App.
- **The collection's first full download fills the disk.** The inventory (SPEC-060) measures the
  headroom first, and a failed write leaves the old copy intact (SPEC-022).
- **The memory estimate is wrong.** The memory watch measures the first day (R10), and the owner
  decides any resize with the numbers.
- **The maintainer's machine is where deploys run.** A deploy needs that machine, the rail and the
  owner's go; the release itself stays downloadable, so a rollback needs only a kept tag.
