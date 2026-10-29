### Changed

- The Apple build (`xcframework.yml`: the XCFramework, the codec, the card view and the Swift harness)
  runs on the repository's self-hosted Apple-silicon runner for a push or a dispatch, and stays on
  GitHub's `xcode-27` image for a pull request and every other event.
