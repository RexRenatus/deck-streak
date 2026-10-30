### Security

- The web lock moves `qs` from 6.16.1 to 6.17.0, the newest release inside the
  `typed-rest-client@2>qs` override's range, as the Rust advisory entry's lock had it before the
  SvelteKit 3 lock replaced that lock. No `package.json` range moves. This supersedes the Rust
  advisory entry's line that three low web advisories stay open: SvelteKit 3 clears
  GHSA-pfhg-q3qv-9q6r and GHSA-27mw-82g8-phjj, and the fixed `deepmerge` copy clears
  GHSA-gcqx-w48m-w2h4, so the web advisory audit lists none.
