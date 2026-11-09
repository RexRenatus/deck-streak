# Red-first record: SPEC-643

SPEC-643, ADR-666, the schematic and the changelog fragment were committed first and alone
(93d355ed), before any test. The shape test and this record came next, with the browser audit
absent, and the commit after them adds the audit. Each red below is quoted from a run of the shape
test at the commit that adds it, and each green from a run at the commit that adds the audit, both
from the repository root by the acceptance lines with `--reporter=verbose`.

## The fence, line by line

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `web/app/src/lib/settings/sign-in-methods-a11y.test.ts` "the passkey list audit runs axe over the list with every row closed in both colour schemes" | added; holds `web/app/tests/sign-in-methods-a11y.spec.ts` |
| 2 | A2 | `web/app/src/lib/settings/sign-in-methods-a11y.test.ts` "the passkey list audit runs axe over the list with a confirm step open in both colour schemes" | added; holds `web/app/tests/sign-in-methods-a11y.spec.ts` |

## Reds and greens

```red-first
A1: red at 8a526007: sign-in-methods-a11y.test.ts "the passkey list audit runs axe over the list with every row closed in both colour schemes": AssertionError: tests/sign-in-methods-a11y.spec.ts, the passkey list audit, is absent: expected false to be true // Object.is equality
A1: green at 3d4febd4
A2: red at 8a526007: sign-in-methods-a11y.test.ts "the passkey list audit runs axe over the list with a confirm step open in both colour schemes": AssertionError: tests/sign-in-methods-a11y.spec.ts, the passkey list audit, is absent: expected false to be true // Object.is equality
A2: green at 3d4febd4
```

The SPEC, the ADR, the schematic and the changelog fragment were committed first and alone
(93d355ed). The shape test came next, with the audit absent (8a526007), and read red by name for
both criteria; 3d4febd4 adds the browser audit with the shape test unchanged and reads both green.
The browser audit's own two tests run in the end-to-end suite in CI at the pull request head.
The commit after 3d4febd4 changes only this record.
