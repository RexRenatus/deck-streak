### Added

- A census reads every workflow and every script a workflow step runs by path, and lists each
  command that resolves or builds dependencies without its lock flag: a cargo command without
  `--locked` or `--frozen`, a `cargo install` without `--locked`, a pnpm install without
  `--frozen-lockfile`, an npm install where `npm ci` is the locked form, a Swift build without
  its resolved-file flag, every xcodebuild action or project call, and a pip install without `--require-hashes`. It starts at
  warn with today's sites listed, each with its reason, and fails on a listed entry that matches no
  current command (SPEC-671).
