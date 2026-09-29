---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The kernel's loader refuses an empty credential by its id, so every unit that reads one fails and pages the same way

## Context and Problem Statement

ADR-038 delivers each credential to a unit at every start as
`LoadCredential=<id>:<the credential socket>`, and the kernel's `CredentialLoader` reads it from the
credentials directory (SPEC-020 R11). The loader refuses a missing credential by its id, and loads
any other file it can read, less one trailing newline: a file of zero bytes, or one holding only
that newline, loads as an empty value. systemd.exec(5) describes a socket's credential as "the
credential data read from the connection" and bounds a unit's credentials from above only ("an
accumulated credential size limit of 1 MB per unit"); it states no lower bound, and no start failure
when the data read is empty. Where is an empty credential refused, so that the unit fails and its
`OnFailure=` page says which credential it was?

## Decision Drivers

- One place every role passes through. Each role of `deckstreakd` reads its credentials through
  the loader, so a rule there holds for every present and future role, with nothing to repeat per
  unit.
- A refusal names the credential's id and never a value (SPEC-020 R11, CHARTER 15).
- A refused start must fail its unit, so that its `OnFailure=` alert fires (ADR-010, SPEC-031).
- The alert unit reads its credentials in its own script, not through the loader, and it cannot page
  about itself (SPEC-031).

## Considered Options (the alternatives it was chosen against)

- The loader refuses an empty credential, and the alert script the same way: chosen, because every
  role reads through the loader, so no unit or credential can be left out, and the refusal takes the
  path a missing credential already takes, to a failed unit and a page that quotes its error line.
- Relying on the service manager to refuse the start: rejected, because its manual promises no start
  failure when a credential source answers with nothing. systemd.exec(5) reads a socket's credential
  as the data read from the connection, and states an upper bound on its size and no lower one.
- An `ExecStartPre=` size check in every unit: rejected, because it takes one line per credential per
  unit, and a unit, or a credential added to one, that forgets its line starts with the empty value.
- The credential helper declining to answer: rejected, because declining is not a start failure the
  manual documents either. The service manager can still hand the unit an empty credential, so the
  refusal has to be where the value is read.
- The loader refusing a blank value too, one of whitespace only: rejected, because the loader judges
  presence and not shape: a value of one character loads by design (SPEC-066 R1), and each caller
  refuses a blank one by its own check or at the far end.

The census that holds the templates to the units' conditions (below) needs a reading of each
template, and two were considered for it:

- A census that reads the plain syntax the templates hold and refuses the rest: chosen, because no
  template under `deploy/` holds a continued line, a control character, an exit-status word other
  than a decimal or a status name, an empty or unknown restart or collect value, or a `[Unit]`
  condition, so each refusal gives up nothing the templates need, and what the census admits it
  reads the one way it can.
- Modelling systemd's unit-file reader and number parser in the census: rejected, because each
  review of a model found a spelling the model read otherwise than systemd, and a census that reads
  a word otherwise than systemd can admit what it means to refuse. A refusal holds for every
  spelling it does not read; a model holds only until its next divergence.
- Refusing the dependency directives one by one, then whichever the next review names: rejected,
  because a list of refused keys holds only until the next key, while a literal list of the keys
  each kind of unit uses refuses every other key by its name. The lists are the keys those units use,
  the units that load a credential and page on failure, each list is held equal to the keys its units
  hold, and a unit that needs another key adds it to its list in the same change (SPEC-066 R2).
- Bounding only the keys, and leaving the value of a listed key free: rejected, because a listed key
  can hold a value that changes what a unit does, and a list of keys cannot say so. A table holds the
  values those units use for `Restart=`, their restart budget and their ordering, and their
  `OnFailure=` is the alert template alone (SPEC-066 R2).

## Decision Outcome

Chosen option: the loader refuses an empty credential, because it is the one reader every role
shares, and a refusal there fails the unit the way a missing credential does.

- **The loader.** `CredentialLoader::load` trims the one trailing newline, and refuses a value with
  nothing left as `CredentialError::Empty { id }`, before it registers the value with the redactor
  or returns it. Its message is `the credential <id> is empty in the credentials directory`. A
  missing credential keeps `Missing`, and a value of one character or more loads as before, so a
  file of two newlines loads as one newline. The loader judges emptiness only: a value's shape
  stays its caller's check.
- **The units.** A role that refuses start exits 1 (SPEC-025 R1), and the `sync` job's login records
  the refusal as `missing_credentials`, which pages when it opens the job's error streak (SPEC-027
  R7). Every template that loads a credential, the alert template excepted, names `OnFailure=` the
  alert template, and none carries a setting that would skip the refused start, count it as a
  success or skip `OnFailure=`: an `ExecCondition=`, which skips the start when it exits 1 to 254,
  a `[Unit]` condition or assertion, which can stop the start before it runs, an `ExecStart=` with
  the `-` prefix, a `SuccessExitStatus=` word that reads as 1, or `RestartMode=direct`
  (systemd.service(5), systemd.unit(5)). A census over `deploy/` holds all six. It does not model
  how systemd reads a unit file: it reads the plain syntax the templates hold and refuses the rest,
  a line it does not read, an exit-status word other than a decimal of at most 255 or a status
  name, and an empty or unknown `Restart=`, `RestartMode=` or `CollectMode=`, each read at every
  assignment (SPEC-066 R2). It holds each unit to a literal list of `(section, key)` pairs for its
  kind, the drop-ins of the unit's own `<name>.d/` included, and refuses any key off it by name; a
  `*.d/` directory under `deploy/` that is not a shipped unit's own is refused, since only a unit's
  own drop-in directory is read.
- **The alert unit.** Its script refuses an empty credential of the two it loads by its id, with one
  line at error priority, before it reads the journal or makes a request, and exits 1. The unit then
  stays failed, in `systemctl --failed` and the journal, because its template counts no refusal a
  success, names no `[Unit]` condition or assertion, holds only the keys of its own list, which has no dependency directive, restarts no refused start and is never unloaded
  while failed (SPEC-066 R3's exit, restart and collection conditions). It names no `OnFailure=`, so
  its own failure pages nothing: a page about the alert unit's own failure needs a route that does
  not depend on the alert sender (#285).

### Consequences

- Good, because every role refuses an empty credential by its id with no line in its unit, and a
  future role inherits the refusal with the loader.
- Good, because the refusal reaches the owner as the page a missing credential already sends, whose
  quoted error line names the credential for a role, and names `missing_credentials` for the sync
  job.
- Bad, because the alert unit's own refusal pages no one until #285 builds a second route; it is
  visible in `systemctl --failed` and the journal only.
- Bad, because a credential that is present but blank (spaces, or a carriage return) still loads;
  its caller's shape check refuses it, as identity's owner gate does.
- Bad, because a template that needs what the census refuses, such as a continued line, an
  exit-status word the census does not read or a `[Unit]` condition, is refused until a delivery
  of its own widens the census and says why.

### Confirmation

SPEC-066's acceptance tests: the loader's refusal in each empty form and its message (A1 to A3),
the census of the templates (A4), which plants each refusal and a cross-check corpus of exit-status
words and admits none of the corpus, the alert unit's route (A5), and the sync's login, which reads
through the loader, never reaching the engine with an empty value (A6). Hand-proved rows S06601
to S06610 kill the mutants cargo-mutants does not make.

## What would make this wrong

- A credential whose empty value is meaningful, such as an optional feature's token. It would need
  an id and a loading call of its own that admit an empty value, decided in its own ADR.
- A service manager that documents a start failure for an empty credential. The loader's refusal
  would then be a second guard, and it would still name the credential.

## More Information

ADR-038 (its note of 2026-09-28 names this ADR), ADR-010, SPEC-020 R11, SPEC-025 R1, SPEC-027 R7,
SPEC-031, SPEC-066, #284, #285. systemd.exec(5), `LoadCredential=`, at
<https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html>; systemd.service(5),
`ExecCondition=`, `ExecStart=`, `SuccessExitStatus=`, `Restart=`, `RestartForceExitStatus=` and
`RestartMode=`, at <https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html>;
systemd.unit(5), `CollectMode=` and the `[Unit]` conditions and assertions, at
<https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html>; systemd-analyze(1),
`exit-status`, at <https://www.freedesktop.org/software/systemd/man/latest/systemd-analyze.html>.
