### Added

- The TestFlight lane for the iPhone and iPad app (SPEC-352, #634): `testflight-internal.yml`, a
  dispatch on `dev`, and `testflight-release.yml`, a SemVer tag on `main`, each plan the build
  number and version on Linux, call the one Apple job body, and sign and upload the harness to
  internal TestFlight from a GitHub environment that only that lane's `app` job names. Until the
  owner places the credential, a run builds unsigned and says it stopped before the upload. The
  hardening test admits the lanes' secret reads by file, job and step and nothing else, and the
  app icon is generated at build time, never committed. ADR-363 records the choices and what each
  was chosen against.
