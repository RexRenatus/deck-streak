### Changed

- The verdict job of the XCFramework workflow now takes the short Apple jobs' runs-on expression,
  and harness and release-and-app take one new expression that reaches the self-hosted runner only
  for this repository's own pull requests with DS_MINI and DS_MINI_LONG on, so the upload lanes'
  and the tag's runs keep both long jobs hosted (SPEC-688).
