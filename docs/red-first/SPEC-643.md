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
