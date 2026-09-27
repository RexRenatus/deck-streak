# Changelog fragments

Each pull request adds one file here instead of editing `CHANGELOG.md`, so concurrent pull
requests never collide on the same lines. Name it for the branch or the pull request, for example
`changelog.d/fix-expired-session.md`, and write it in the changelog's own shape:

```markdown
### Fixed

- A session that expired while the app was open no longer loses the answer being typed.
```

Use only the six Keep a Changelog headings: Added, Changed, Deprecated, Removed, Fixed and
Security. At release time the fragments are compiled, newest first, into a new version section of
`CHANGELOG.md` and then deleted.
