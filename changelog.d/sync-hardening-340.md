### Added

- The sync server is hardened before its first deploy (SPEC-340, ADR-351, #628): the sync family
  runs as its own user with its own archive and drill units, the server reaches loopback peers only,
  the edge logs the sync route alone with no key, the route drops the web cookie both ways, and a
  ban filter and jail count a refused sync login.
- The launcher admits only the house's hash shape: a floor and a ceiling on the rounds, a 16-byte
  salt and a 32-byte digest, each in canonical form.
- The release audits the sync server's dependencies before it builds anything: the audit job reads
  the pin in the engine's patch entry and the fork's own lockfile, and the release job needs it.
- The offsite copy is sealed before it leaves and only the sealed files are copied; they are removed
  afterwards, and an unset seal copies nothing and fails the run.
- The privacy policy states the offsite period and discloses the edge log and the ban list, and the
  runbook names the rekey triggers, the hash command and the bucket rule.
- Tests for each, a red-first record, and mutation rows S34001 to S34021 for the new checks; rows
  S33708 and S33739 to S33743 are re-anchored under their ids.
