---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Both Caddyfile layouts serve the site: the import names the block by its absolute path, each step checks its candidate in the Caddyfile's own directory, and the Caddy directory is a plain absolute path

## Context and Problem Statement

#452 asks that the site be served in both documented layouts: the live Caddyfile in the Caddy
directory, which is the default, and a Caddyfile that `DECKSTREAK_DEPLOY_CADDYFILE` names elsewhere.
Done is one of two things. Either the import resolves to the site block from the live Caddyfile in
both layouts and the validation checks the file the reload loads, or the set-apart layout is
refused before any write and ADR-198 says so.

`deploy.sh caddy-install` writes the site block in the Caddy directory and adds one `import` line to
a candidate copy of the Caddyfile. It validates and adapts that candidate, renames it onto the live
Caddyfile and reloads (SPEC-127, ADR-198). Caddy's documentation of the `import` directive says a
filename pattern "is always relative to the file the `import` appears in". The adapter joins a
relative pattern to the directory of the absolute path of the file it is reading, and uses an
absolute pattern as written. The working directory decides nothing once the config path is
absolute. `caddy validate`, `caddy adapt` and `caddy reload` each read the file named by
`--config` and adapt it in the command's own process. So the only path input of import resolution
is the directory of the file each command is given. This record decides what the import line names
(D1), where the candidate is checked (D2), and what the Caddy directory may hold now that the line
carries it (D3).

## Decision Drivers

- The cure is the smallest that makes the issue's done true and keeps every criterion of SPEC-127
  true as written, so no row of S12700-S12746 loses its anchor.
- `DECKSTREAK_DEPLOY_CADDYFILE` is a documented setting. ADR-198 rejected refusing a Caddyfile
  outside the Caddy directory because that would remove a documented configuration.
- The check must read the configuration the reload loads: a check of anything else is not a check
  of the change.
- Nothing runs on a host. A host check is a recorded step on #161, waiting for the owner's go.
- No new privilege, no new setting, and no write outside the two directories the steps already
  check.

## Considered Options (the alternatives it was chosen against)

D1, what the import line names:

- The block by its absolute path in the Caddy directory: chosen, because Caddy uses an absolute
  pattern as written, so `IMPORT_LINE="import $CADDY_DIR/deck-streak.caddy"` names the block from
  any directory the Caddyfile is in. One assignment changes, and the block stays where SPEC-127's
  guards, tests and rows expect it. The install's `grep -qxF` and the removal's `grep -vxF` read
  the same variable, so they still agree.
- Refuse the set-apart layout before any write (the issue's option b): rejected, because ADR-198
  already rejected it, since the setting is documented. SPEC-127's A38 and A39 also run both steps
  to success with the Caddyfile set apart. A38 tells the two directories apart only in that
  layout, so A38, A39 and the rows S12731, S12733, S12736 and S12737 would be rewritten or
  retired. A test over both layouts could then show only a refusal in one of them. Its merit is
  size: one directory comparison per script and no change to the line.
- Write the block in the Caddyfile's directory instead of the Caddy directory: rejected, because
  `DECKSTREAK_DEPLOY_CADDY_DIR` would then choose nothing the steps write. Every block path in the
  four-name guard, the tests and SPEC-127's rows would also move.
- A relative pattern computed from the two directories: rejected, because the host's tools do not
  spell a relative path between two directories portably, and the pattern breaks when either path
  holds a link.
- A link to the block beside the Caddyfile: rejected, because ADR-198 refuses links at the steps'
  names, and a second name for the block adds a write, an undo and a guard.

D2, where each step's candidate is written and checked:

- In the live Caddyfile's own directory: chosen, because the directory of the file each command
  is given is the only path input of import resolution. Both scripts spell it
  `copy=$(dirname -- "$file")/deck-streak.candidate`. A candidate there is checked under the resolution the reload uses,
  for the site import and for every relative import of the operator's own. The rename onto the
  Caddyfile also stays within one directory. The existing directory checks already require that
  directory to be writable (SPEC-127 A33 to A38). The four-name guard reads `$copy`, so it follows
  the candidate with no edit.
- D1 alone, with the candidate kept in the Caddy directory: rejected, because an operator's own
  relative import would then resolve from the Caddy directory at the check and from the
  Caddyfile's directory at the reload. The check would read a configuration the reload does not
  load, and #452's done asks that the validation check the file the reload loads.
- Validate the live Caddyfile after the rename and before the reload: rejected, because the
  refused configuration would then already be in place, against SPEC-127's rule that a refusal
  leaves the live file as it was.
- A fresh `mktemp` name in the Caddyfile's directory: rejected, for ADR-198's reason. It adds a
  second cleanup and a name the tests and rows cannot anchor on.
- Run the checks from the Caddyfile's directory as the working directory: rejected, because Caddy
  joins a relative pattern to the directory of the absolute path of the file being read. With an
  absolute `--config`, the working directory changes nothing.

D3, what the Caddy directory may hold:

- An absolute path of letters, digits and `._@+/-`: chosen, because D1 carries the directory into
  one Caddyfile token. Whitespace splits tokens, and `{$…}` is substituted before the Caddyfile is
  parsed. `#` opens a comment at the start of a line, and quotes, backticks and `<<` delimit
  tokens. The import's glob reads `*`, `?`, `[` and `\`. Each Caddy step checks the setting first,
  `case $CADDY_DIR in [!/]* | /*[!A-Za-z0-9._@+/-]*) die ...`, before the tag is read or the host
  is reached. A plain absolute path means one file to the step's shell and to Caddy. A relative directory would
  name one place to the host script and another to Caddy, which joins a relative pattern to the
  Caddyfile's directory. The set is the vocabulary `render-caddy.py` already admits for a web root,
  so one set of characters reaches the Caddyfile from either path.
- Quote the token, `import "<dir>/deck-streak.caddy"`: rejected, because quotes protect whitespace
  only. Placeholders are substituted before parsing, and the glob reads its characters inside
  quotes too. A refusal list would still be needed beside an escaping rule for `"` and `\`.
- No check, leaving it to `caddy validate`: rejected, because the validation refuses only some of
  these directories. A placeholder or a wildcard can resolve to another existing file with no
  error, and the import would then name something other than the block.

## Decision Outcome

The chosen option of each of D1 to D3. `deploy.sh` gains an absolute import line, a check of the
Caddy directory called first by each Caddy step, and a candidate in the Caddyfile's own directory.
The tests gain Caddy's import rule as a helper and A1 to A3. Two of SPEC-127's tests are rewritten
to the new spelling without changing their criteria: the helper of A7 to A9 resolves the import
instead of matching its text, and A38's stale copies are the names each step now writes in each
directory. SPEC-127 and this record's parent, ADR-198, each gain an appended amendment.

### Consequences

- Good, because the site is served in both layouts, and each check reads the configuration the
  reload loads.
- Good, because the rename onto the live Caddyfile is within one directory in both layouts.
- Good, because no criterion of SPEC-127 changes and no row of its band moves.
- Bad, because a live Caddyfile that still holds the relative line of an earlier install is
  refused at the check until that line is taken out. The read-only check recorded on #161 finds it
  first.
- Bad, because a Caddy directory outside D3's characters, which the relative line could carry, is
  now refused. The default directory is inside the set.

### Confirmation

SPEC-353 A1 to A3 in `scripts/tests/test_deploy_scripts.py`, with SPEC-127's acceptance lines
unchanged. Each of rows S35301 to S35307 is proved killed by its full id.

## What would make this wrong

- If Caddy resolved a relative import against the working directory, or adapted the reload's file
  in the server instead of the command, D2's reason would change. A1's oracle states the documented
  rule, and the check recorded on #161 reads the import after the install.
- If an operator needed a Caddy directory outside D3's characters, D3 would have to grow an escape
  rule. No documented layout needs one.

## More Information

#452, #424, #451, #628, #161; SPEC-127; ADR-198. Caddy's documentation: the `import` directive,
Caddyfile concepts (tokens and quotes, environment variables), and the command line (`validate`,
`adapt`, `reload`). Caddy's adapter source, `caddyconfig/caddyfile/parse.go` (`doImport`), and its
command line, `cmd/commandfuncs.go` (`cmdReload`, `cmdValidateConfig`).
