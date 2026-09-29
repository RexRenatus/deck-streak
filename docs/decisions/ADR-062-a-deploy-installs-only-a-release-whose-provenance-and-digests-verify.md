---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A deploy installs only a release whose build provenance and digests verify, checked on the maintainer's machine

## Context and Problem Statement

ADR-010 deploys "from a verified release artifact of a `main` tag", and RELEASING.md's deploy step
says the script "downloads the release's artifacts and checks their digests". Neither says what the
verification proves. The release is built once by CI from the tag (release-ops, SPEC-062), attached
to a public GitHub release, and later downloaded by `deploy/deploy.sh` on the maintainer's machine,
which then installs it as root on the host. What must a deploy check before an artifact reaches the
host, so that the host runs exactly what this repository's release workflow built from that tag?

## Decision Drivers

- The host runs only artifacts built from a tag on `main` by this repository's own workflow
  (CHARTER 3, ADR-017, ADR-034).
- No long-lived signing secret to store and rotate (CHARTER 15).
- The check runs before anything reaches the host, where a bad artifact would run as the service.
- cyber-pipeline's `cp.release-provenance` asks a release workflow to attest provenance or sign.

## Considered Options (the alternatives it was chosen against)

- A build-provenance attestation and the digest file, both verified on the maintainer's machine before the host is touched: chosen, because the attestation proves the artifact came from this repository's release workflow at that tag, signed with a short-lived identity the workflow is issued for that run, so no key is kept anywhere.
- The digest file alone: rejected because a `SHA256SUMS` attached beside the artifact proves only that the two agree, and whoever can replace one can replace both.
- A signing key held by the maintainer: rejected because it is a long-lived secret to store, rotate and protect, which the attestation makes unnecessary.
- Building on the maintainer's machine at deploy time: rejected because the deployed binary would then not be the one CI built and tested once, and the build machine's state would enter the release.
- Compiling on the host at deploy time: rejected because the host never compiles (CHARTER 3, ADR-010).

## Decision Outcome

Chosen option.
- **The release workflow** (SPEC-062 R1) attaches the tarball, its `SHA256SUMS` and a
  build-provenance attestation, with the job's `id-token: write` and `attestations: write`
  permissions only, and publishes the draft after the last upload.
- **The deploy** (SPEC-062 R3) verifies, in this order, before it reaches the host: the tag is
  annotated and its commit is on `origin/main`; `gh attestation verify` accepts the tarball for this
  repository, signed by this repository's release workflow; every digest in `SHA256SUMS` matches.
  Any failure stops the deploy with the host untouched.
- **Rollback** verifies the same way whenever it downloads a tag again (SPEC-062 R5); a release
  directory already on the host was verified when it was installed.
- The owner's repository settings may add immutable releases (release-ops recommends it); the
  deploy's checks do not depend on it.

### Consequences

- Good, because a replaced asset, a release from another workflow and a tag off `main` all stop the
  deploy before the host.
- Good, because nothing secret is kept for signing.
- Bad, because the deploy needs the GitHub CLI and network access to GitHub's attestation store on
  the maintainer's machine; without them, the deploy refuses rather than skipping the check.

### Confirmation

SPEC-062's A2 and A8; cyber-pipeline's `cp.release-provenance` on the box.

## What would make this wrong

- GitHub withdraws artifact attestations, or the repository moves off GitHub. A signature with a
  keyless signer of the same kind would replace it, recorded in a new ADR.
- The release starts to include a component built outside the release workflow; that component
  would need its own provenance.

## More Information

ADR-010; ADR-017; ADR-034; RELEASING.md; SPEC-062; the release-ops pack's release rows and
cyber-pipeline's `cp.release-provenance`; the GitHub CLI's `gh attestation verify`, with its
`--repo` and `--signer-workflow` options, read through Context7.
