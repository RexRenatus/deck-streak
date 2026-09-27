Paste these three sections into the pack's SPEC, numbered to follow its requirements and
acceptance criteria. Every row id in `checks.json` must appear in the coverage matrix, and every
exclusion must cite a real ledger idea at or below the population frontier.

## 4. Coverage matrix

"b" is blocking and "a" is advisory. An exclusion names its reason (a live fetch, a browser,
judgement, a deprecation, another pack's subject) and the idea it belongs to.

| # | practice | source | mapped to |
|---|---|---|---|
| 1 | every page opens with a title | the style guide, https://example.org/style | `example-title` b |
| 2 | a page stays short enough to read in one sitting | the style guide | `example-length` a |
| 3 | a page reads clearly | the style guide | excluded: judgement of prose (i1547) |

**Totals.** 3 practices: 2 mapped to rows, 1 excluded.

## 6. What this does NOT do

- It does not judge how well a page reads. That is judgement, not a tree fact (i1547).

## References

Accessed 2026-09-27. Say here if WebSearch was unavailable, and which sources were fetched
directly instead.

- The style guide: https://example.org/style
- Context7 ids that answered: `/example/docs`. When none answers, write "Context7: none answered".
