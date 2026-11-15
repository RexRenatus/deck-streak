---
status: accepted
decision-makers: "the DeckStreak architect"
---

# The signing step names the placed certificate's shape and openssl's refusals by fixed classes, and re-wraps the key under PBES2

## Context and Problem Statement

The lane's signing step decodes the placed certificate and reads it with `openssl pkcs12`.
A failed read prints one class of a closed set and nothing of openssl's text (SPEC-352
section 8), but the step names nothing of what the decode gave, and a refusal outside the
three classes reads `unrecognised`: a run whose reads both fail names neither how the
certificate was placed nor what openssl refused. The re-wrap then protects the key with a
SHA-1 based algorithm and MAC, and a refused import stops the step with the whole-tool
line. SPEC-692 states the change.

## Decision Drivers

- A wrong placement names itself on its first run, with no byte of the certificate in the
  log.
- No existing read case changes its class.
- The keychain is first offered a bundle that names no SHA-1 based algorithm, and the lane
  keeps signing if the keychain refuses it.
- One bounded retry: two forms, never a third import.

## Decision Outcome, and what each decision was chosen against

- One line naming the decoded certificate's shape by a closed class before the first read —
  chosen, because a wrong placement then names itself on its first run without printing a byte
  of the certificate.
- Printing the certificate's first bytes or its length — rejected, because they are the secret's
  own bytes and size.
- Two read classes of their own for openssl's could-not-read and ASN.1 refusals, after the three
  that precede them — chosen, because neither changes an existing case's class.
- The PBES2 re-wrap first, with the SHA-1 based form once after a refused import — chosen,
  because one run then shows which form the keychain imports, and the lane keeps signing.
- The PBES2 form alone — rejected, because a refusal would stop signing until a revert.
- What would make this wrong: the keychain refuses both forms; the step then refuses naming both,
  and the bundle or the runner image is what changes.

## Consequences

- Good: every placed text gets one class line before any tool runs, and a text that does
  not decode gets one fixed refusal in place of an exception.
- Good: openssl's could-not-read and ASN.1 refusals each name themselves, and the three
  classes before them keep every case.
- Bad: which form the keychain imports is not measured until a lane run reads the import
  line, and the compatibility form stays in the lane until then.

## Confirmation

After the land, the first lane run on `dev` whose tree holds the merge is read in its app
job's step `the archive, signed`, by name: its shape line, any read class, and its import
line. `pbes2` accepted confirms the first form; `sha1-compat` accepted after a refused
`pbes2` is recorded here with that run's id. No such run has been read yet.

## What would make this wrong

- A placed certificate whose shape reads `der-sequence` and whose read passes, refused by
  the keychain under both forms.
- An openssl refusal that holds a phrase of an earlier class and so reads under that class.

## More Information

SPEC-692; SPEC-352 R9, R13 and section 8; ADR-363 D3 and D6; SPEC-566.
