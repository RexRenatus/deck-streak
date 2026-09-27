# Red-first record: SPEC-028

The thirteen acceptance tests were committed at 0faa515 with stubs that compile and fail by
assertion: the wrapper read nothing and called nothing, the startapp map read a token as a path,
the API client answered nothing, the token compiler emitted nothing, and About had no links. Each
acceptance line was run from the repository root there, selected exactly one test, and failed for
its criterion's reason. The implementation landed at 9804206, where all thirteen pass.

A10 pins behaviour the skeleton already had, so it is disclosed as not red.

```red-first
A1: red at 0faa515: AssertionError: expected [] to deeply equal [ 'src/lib/telegram.svelte.ts' ]: no module read Telegram's object, so the wrapper was not yet its one reader
A1: green at 9804206
A2: red at 0faa515: AssertionError: token "settings": expected '/settings' to be '/': an unknown token became a path to a screen that does not exist
A2: green at 9804206
A3: red at 0faa515: AssertionError: token "/about": expected '//about' to be '/': a token shaped like a path became a navigation
A3: green at 9804206
A4: red at 0faa515: AssertionError: expected [ '/' ] to deeply equal [ '/', '/about' ]: the audit's routes missed a screen, and the audit kept a list of its own
A4: green at 9804206
A5: red at 0faa515: AssertionError: expected { kind: 'unavailable' } to deeply equal { kind: 'ok', …(1) }: no handshake was sent and no session opened
A5: green at 9804206
A6: red at 0faa515: AssertionError: expected { kind: 'unavailable' } to deeply equal { kind: 'reopen' }: an ended session neither renewed nor asked the owner to reopen
A6: green at 9804206
A7: red at 0faa515: AssertionError: expected [ { version: '6.0', …(5) }, …(4) ] to deeply equal [ …(5) ]: ready and expand were called 0 times at every version, and at 8.0 and 9.6 the safe areas read 0 where Telegram set 47 and 56
A7: green at 9804206
A8: red at 0faa515: AssertionError: --background: expected '(not declared)' to match /^var\(\s*--tg-theme-([a-z]+(?:-[a-z]+)*)\s*,\s*(#[0-9a-f]{6})\s*\)$/: the compiled tokens declared no colour that reads a Telegram theme variable
A8: green at 9804206
A9: red at 0faa515: AssertionError: --foreground: expected '(not declared)' to match /^var\(\s*--tg-theme-([a-z]+(?:-[a-z]+)*)\s*,\s*(#[0-9a-f]{6})\s*\)$/: no declared pair could be painted in Telegram's palettes, so none could be measured
A9: green at 9804206
A10: not red: the skeleton on dev already compiled zh-Hans and zh-Hant and set the page language from the locale when the app starts (hooks.client.ts); the test pins both for every later screen
A11: red at 0faa515: TestingLibraryElementError: Unable to find an element with the text: Saturday, February 3, 2001: Today showed no study day
A11: green at 9804206
A12: red at 0faa515: TestingLibraryElementError: Unable to find an accessible element with the role "link" and name "Privacy policy"
A12: green at 9804206
A13: red at 0faa515: AssertionError: expected undefined to be 'hash': the page had no policy of its own
A13: green at 9804206
```

Running the tests before the red commit also caught a defect in the census behind A1: its first
pattern read the prose "outside Telegram." in a comment as a member of Telegram's object. The
pattern was narrowed to a member access before 0faa515, and that prose line is now one of the
census's planted controls.
