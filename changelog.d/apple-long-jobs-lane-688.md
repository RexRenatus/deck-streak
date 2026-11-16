### Changed

- The two long jobs of the XCFramework workflow, harness and release-and-app, now also reach the
  self-hosted runner for a push or a dispatch on the dev branch with DS_MINI and DS_MINI_LONG on,
  so the internal upload lane's runs there use it, while every other event and ref, a tag, main, a
  schedule and a fork's pull request, keeps both long jobs hosted (SPEC-688).
