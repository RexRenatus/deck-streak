### Security

- `deepmerge` 4.3.1 (GHSA-gcqx-w48m-w2h4, low), which SvelteKit's Vite plugin uses to merge its
  options and which has no fixed release, is replaced by our own fixed copy in `vendor/deepmerge`,
  through an override scoped to the plugin's major. The copy takes the change of the open upstream
  pull request 277 in TehShrike/deepmerge: an own `__proto__` key in a merged object stays an own
  data property and never sets the result's prototype. A new test reaches deepmerge as the plugin
  resolves it, and the lock no longer holds the registry's 4.3.1 (ADR-517, SPEC-499). This
  supersedes the SvelteKit 3 entry's line that `deepmerge` 4.3.1 "has no fixed release and stays".
