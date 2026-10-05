The owner rules on the TestFlight upload lane, in the owner's own words: "Env secrets (Recommended)" and "Narrow exception (Recommended)" (owner, 2026-10-05, answering where the upload credential lives and whether two public workflows may name its secrets), and "send the commits to me on telegram for signature" (owner, 2026-10-04): the upload credential and its signing material live as GitHub environment secrets in two environments, the internal one admitting dispatches on `dev` only and the release one admitting `v*` tags only, with the owner as its required reviewer and admin bypass off; CHARTER 15's Secret Manager clause governs host services, not this credential, which reaches only the one step that uses it, through that step's own environment; and, as a narrow exception to CHARTER 11, the lane's six neutral secret role names may appear only as `secrets.<NAME>` in the two lane workflow files and in the hardening test's admission table, never beside a value, host, project or account, and never on the scrubber's deny-list.

# OWNER RULING 2026-10-05: the TestFlight lane's secrets

## What was held

At dev `9241161a`, three texts meet the lane that uploads a build to TestFlight:

| text | what it says | what the lane needs |
|---|---|---|
| `CHARTER.md:61-66` (item 11) | no secret name enters the repository, an issue, a pull request or a commit message | GitHub hands a job only the secrets its workflow names, so a lane that uploads must name them |
| `CHARTER.md:77-79` (item 15) | secrets live in Secret Manager and reach a process as a systemd credential at start | the lane runs on a GitHub-hosted macOS runner, which no systemd credential reaches |
| ADR-344:69-72 | the upload credential is placed "as repository secrets, available to this workflow only" | every workflow on every branch reads a repository secret, so "this workflow only" cannot hold |

Under those texts as written, no CI upload lane can be built: every internal build would wait on the owner's own
hands.

## What replaces it

- **Where the credential lives.** Two GitHub environments, one per lane, each holding that lane's six parts as
  environment secrets:
  - the internal environment admits dispatches on the branch `dev` only, and has no required reviewer, because the
    dispatch is already the ask (ADR-344);
  - the release environment admits tags matching `v*` only, has the owner as its required reviewer, and has admin
    bypass off.

  An environment secret reaches only a job that names its environment, on a ref its rule admits, so neither a pull
  request's run nor a branch's edited workflow can read it. The owner keeps the original key in their own store of
  record.
- **CHARTER 15's scope.** Its Secret Manager and systemd-credential clause governs DeckStreak's host services. It does
  not govern this upload credential, just as CHARTER 3 does not govern the upload itself (ADR-344:73-74). For this
  lane the rest of CHARTER 15 reads:
  - a secret reaches only the one step that uses it, through that step's own environment, which is how GitHub hands
    a step a secret; never a workflow-level or job-level environment variable, so no other step or child of the job
    can read it;
  - never in argv, a log line, an uploaded artifact, a database row or a committed file;
  - the upload key exists as a file only for the one step that uses it, and is removed on any exit;
  - the signing certificate is imported non-extractable into a keychain that lives only for the job.
- **The CHARTER 11 exception, and its whole reach.** The six parts are the upload key, its key id, its issuer id, the
  distribution certificate, the certificate's password and the provisioning profile. Each has one neutral role name,
  and a name may appear ONLY:
  - as `secrets.<NAME>` in the two lane workflow files;
  - in the hardening test's admission table, which allows a secret read only in the lanes' `app` jobs, by file, job
    and step, and holds every other workflow at zero reads.

  Nowhere else: no prose, issue, pull request, commit message, changelog or document names them, this ruling
  included. No name sits beside a value, host, project, team, app id or account. The names are not added to the
  scrubber's deny-list, because the scrubber would then refuse the two files that must carry them.
- **What stays bound, verbatim.** CHARTER 11 keeps its text for every other secret name and for every other file.
  This ruling admits no seventh name, no third file and no second table.
- **Order.** No workflow names a secret before this ruling is signed and on dev. ADR-344 keeps its text; the delivery
  that builds the lane records, in its own ADR, that this ruling supersedes ADR-344's "repository secrets".

## Why it is admitted

It is a weakening twice over. CHARTER 11 bars any secret name in the public repository, and this admits six; its
reach is two files and one test table. CHARTER 15 bars a secret in any environment variable, and this admits one
step's own environment on a hosted runner; its reach is the step that uses the secret. A name alone opens nothing:
the value lives only in an environment that a pull request cannot reach, and the release value waits on the owner's
review.

The rejected alternatives:
- **Secret Manager reached by a cloud workload-identity binding.** It keeps CHARTER 15's host wording, but the public
  workflow would then carry a cloud provider and project identifier, which CHARTER 11 also bars, and a binding scoped
  to release tags cannot serve the internal builds dispatched on `dev`.
- **Plain repository secrets (ADR-344 as written).** Every workflow on every branch reads them, so a pull request from
  a branch here that edits a workflow could print them.
- **One bundled secret under one name.** The exception would cover one name, but every certificate renewal would
  re-place all six parts, and a failure could no longer say which part is missing.
- **No CI credential, hand uploads.** No secret is named, and every internal build waits for the owner to archive and
  upload it from a Mac.
