### Changed

- The three short Apple jobs of `xcframework.yml` (the XCFramework, the codec and the card view)
  can run on the repository's self-hosted runner, labelled `self-hosted`, `macOS`, `ARM64` and
  `ds-mini`: for a push, a dispatch and a pull request from this repository, when the repository
  variable `DS_MINI` is `on` and the actor is not Dependabot. In every other case, a fork's pull
  request included, they run on `xcode-27`, and the Swift harness, the release and app job and the
  verdict always do. With `DS_MINI` unset nothing changes.
