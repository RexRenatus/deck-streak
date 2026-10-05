# SPEC-353: both Caddyfile layouts serve the site, and each Caddy step checks its candidate where the reload reads the Caddyfile

- **Issue:** #452, one of the preconditions of the sync server's first deploy (#628); it lands
  before the Caddy step of #161's sync-server move, which runs `caddy-install`. **Context(s):** none
  (the deploy scripts and their tests, not a bounded context).
- **Decided by:** ADR-364 (D1 to D3), under ADR-198 (the install undoes every write it made) and
  SPEC-127 (a failed reload restores the previous site file).
- **Status:** this pull request delivers R1 to R5 and the rows of section 7.
  **Mutation band:** S35300-S35399. **Model:** none.

## 1. The problem, measured

Two layouts are documented: the live Caddyfile in the Caddy directory, which is the default, and a
Caddyfile that `DECKSTREAK_DEPLOY_CADDYFILE` names elsewhere (`deploy/README.md`, the settings
list). Measured at dev `296963bb`:

- The install adds one line to the live Caddyfile, `IMPORT_LINE='import deck-streak.caddy'`
  (`deploy/deploy.sh:69`), and passes it to both host scripts (`:324`, `:370`). Its pattern is
  relative. Caddy resolves a relative `import` pattern against the directory of the file that holds
  the directive (Caddy's documentation of the `import` directive).
- Each step writes the site block and its candidate in the Caddy directory:
  `block=$dir/deck-streak.caddy`, and `copy=$dir/deck-streak.candidate` at `:287` and `:334`. The
  candidate is validated and adapted at `:310`-`:311` and `:355`-`:356`. The reload reads the
  Caddyfile the setting names, `caddy reload --config "$file"` at `:314` and `:360`.
- No test resolves an import. `grep -c 'import deck-streak.caddy'
  scripts/tests/test_deploy_scripts.py` prints 1: the helper of SPEC-127 A7 to A9 matches the line's
  text (`:858`). The `caddy` stand-in logs each call and counts the candidate's `import` lines; it
  resolves none.
- `grep -c '"apart"' scripts/tests/test_deploy_scripts.py` prints 5. SPEC-127 A38 and A39 run both
  steps with the Caddyfile set apart and judge the paths written and the settings read, not the
  import.
- The module holds 56 tests (`grep -c '    def test_'`). SPEC-127's band holds 46 rows.
- SPEC-127's last exclusion leaves the site import unchecked when the Caddyfile is set apart
  (#452).

## 2. Requirements

R1. In both layouts, the line the install adds to the live Caddyfile imports the site block the
    install wrote in the Caddy directory, by Caddy's rule for an import. The removal takes the line
    and the block out again, and leaves the Caddyfile byte for byte as it was before the install.
R2. Each Caddy step writes the candidate it validates and adapts in the live Caddyfile's own
    directory, the directory the reload reads the Caddyfile from. Every relative import the
    Caddyfile holds, the operator's own included, then resolves at the check to the file it
    resolves to at the reload, in both layouts.
R3. Each Caddy step refuses a Caddy directory that is not an absolute path of letters, digits and
    the characters `.`, `_`, `@`, `+`, `/` and `-`. It names the setting and exits 1 before it
    reads the tag or reaches the host, with the tree unchanged. A Caddy directory of those
    characters is admitted.
R4. SPEC-127, ADR-198 and `deploy/README.md` say that both layouts serve the site.
R5. Every criterion of SPEC-127 holds on the cured scripts and is judged as before. The helper of
    its A7 to A9 resolves each import as Caddy resolves it, and A38's stale copies are the names
    each step writes in each directory.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | R1: in both layouts, the import the install adds resolves to the site block in the Caddy directory, which holds the site; the removal leaves the Caddyfile byte for byte as it was and the block gone | `test_deploy_scripts.py` TheCaddyInstall |
| A2 | R2: in both layouts, every candidate the install and the removal validate and adapt resolves the operator's own relative import to the file the reload resolves it to; the steps check four times and reload the live Caddyfile twice | `test_deploy_scripts.py` TheCaddyInstall |
| A3 | R3: a relative Caddy directory, and absolute ones holding a space, a placeholder or a wildcard, are each refused by both steps by name, with exit 1, the tree unchanged and the host never reached; a Caddy directory holding each admitted punctuation installs, and its import names its block | `test_deploy_scripts.py` TheCaddyInstall |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_site_import_resolves_to_the_site_block_in_both_layouts
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_the_candidate_is_validated_where_the_reload_reads_the_caddyfile
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_a_caddy_directory_the_import_line_cannot_carry_is_refused_before_the_host
```

CI decides all three: the module runs the install script against stand-ins, and no builder runs
it (ruling 189). R4 and R5 have no test of their own. R4 is prose, and the diff shows it in the
three documents. R5 is decided by SPEC-127's own acceptance lines, which run in the same module.

## 4. File manifest

| file | context | change |
|---|---|---|
| `deploy/deploy.sh` | the deploy scripts | changed: the import line names the block by its absolute path; a check of the Caddy directory, called first by each Caddy step; each step's candidate in the live Caddyfile's own directory |
| `deploy/README.md` | the deploy runbook | changed: one sentence in the `caddy-install` paragraph |
| `scripts/tests/test_deploy_scripts.py` | the deploy scripts' tests | changed: Caddy's import rule as a helper, A1 to A3, the helper of SPEC-127 A7 to A9, and the stale copies of SPEC-127 A38 |
| `scripts/mutation-rows.d/S35300-S35399.json` | mutation rows | added |
| `docs/specs/SPEC-353-both-caddyfile-layouts-serve-the-site.md` | this SPEC | added |
| `docs/decisions/ADR-364-both-caddyfile-layouts-serve-the-site.md` | its decisions | added |
| `docs/schematics/both-caddyfile-layouts-serve-the-site.md` | the Caddy steps' data flow | added |
| `docs/red-first/SPEC-353.md` | the red-first record | added |
| `docs/specs/SPEC-127-a-failed-caddy-reload-restores-the-previous-site-file.md` | the parent SPEC | changed: an amendment appended after the last line; nothing above it is edited |
| `docs/decisions/ADR-198-the-install-undoes-every-write-and-a-linked-candidate-is-refused.md` | the parent ADR | changed: an amendment appended after the last line; nothing above it is edited |
| `changelog.d/both-caddyfile-layouts-serve-the-site.md` | the changelog fragment | added |

## 5. What this does NOT do

- It changes no host. The cured step reaches a host only through the Caddy step of #161's
  sync-server move, and the read-only checks around that step are recorded there (#161).
- It does not take out an import line that an earlier install wrote in the relative form. The
  check recorded on #161 reads the live Caddyfile for one before the install runs (#161).
- It leaves the candidate's name, the block's name, the four-name guard and the directory checks as
  ADR-198 decided them; only the candidate's directory moves (#424).
- It does not name a failed temporary directory of the operator's own (#451).
- It does not add `--adapter` to the reload, which reads a Caddyfile not named `Caddyfile*` as JSON;
  that is filed as its own requirement (#674).

## 6. Risks

- A live Caddyfile that still holds the relative import line of an earlier install would import
  the block twice, or a file that is not there. The read-only check recorded on #161 detects the
  line before the install runs, and the install's own validation refuses with the live file as it
  was.
- An install and a removal run with two spellings of the same Caddy directory write two different
  lines, so the removal keeps the install's line. Its reload then fails and is restored with a
  non-zero exit, as SPEC-127 requires; the operator sees the refusal.
- A test world whose temporary directory held a character outside R3's set would be refused in
  every Caddy test of the module at once, so it cannot pass silently; A3's positive control fails
  first.
- A change in how Caddy resolves an import pattern would make A1's oracle disagree with the edge.
  The install's validation resolves the real line with the real adapter on the host, and the check
  recorded on #161 reads the import after the install.

## 7. The mutation rows

S35300-S35399, in `scripts/mutation-rows.d/S35300-S35399.json`, table
`SCRIPT_MUTATIONS`. Each row's anchor and mutant are copied from the cured file.

- The import (A1): the line relative again (S35301).
- The candidate's directory (A2): the install's candidate in the Caddy directory again
  (S35302); the removal's (S35303).
- The Caddy directory's check (A3): the relative arm dropped (S35304); the plain-character arm
  dropped (S35305); the install's call dropped (S35306); the removal's call dropped
  (S35307).

## 8. References

ADR-364; ADR-198; SPEC-127 R2, A7 to A9, A38, A39; #452, #424, #451, #628, #161. Caddy's
documentation: the `import` directive, the Caddyfile's tokens and quotes and its environment
variables, and the `validate`, `adapt` and `reload` commands. `deploy/scripts/render-caddy.py`'s
web-root vocabulary.
