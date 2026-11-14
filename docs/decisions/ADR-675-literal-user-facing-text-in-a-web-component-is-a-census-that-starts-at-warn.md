---
status: "accepted"
---

# ADR-675: literal user-facing text in a web component is a census that starts at warn

Decides SPEC-653. Path:
`docs/decisions/ADR-675-literal-user-facing-text-in-a-web-component-is-a-census-that-starts-at-warn.md`.
Amends nothing. Cites ADR-450 D6 (the promotion rule and the kill criteria, taken by reference),
SPEC-571 and ADR-590 (the seven catalogs held whole by one standard-library census) and ADR-005
(the web front end and its message compiler, unchanged). Read at dev `b9a08e60`.

## Context

- **The web app's text is drawn from seven catalogs.** A component imports `m` from
  `#lib/paraglide/messages.js` and calls `m.<key>()` inside a `{...}` expression; dev holds 57 such
  imports across its components.
- **The catalogs are held whole, the components are not.** SPEC-571's census refuses a message
  missing, empty or mismatched in any locale, but no check refuses a text a component writes in its
  markup instead of drawing it from a catalog. Such a text shows in one language in every locale.
- **Dev holds no such text.** The 53 components hold no literal text node, no literal value of a
  user-facing attribute and no literal dialog text (SPEC-653 section 1). The rule starts with no
  site to list.

## Decision Drivers

- No rule already in force is lowered.
- A new rule starts at warn and moves to fail by ADR-450 D6's promotion rule; demoting a fail rule
  is a signed ruling.
- Every check prints what it examined and fails on an empty population, which is VOID, never a
  pass.
- A catalog draw is always an expression, so a literal is told apart from a draw by the text alone;
  no compiler and no web tool runs.
- The standard library only: no new dependency, in the python stage that already runs every guard
  test.

## Decisions

**D1. A text reader with three forms (SPEC-653 R2-R5).** The reader blanks script and style
blocks, comments and every `{...}` expression, keeping each line break, then reads the text between
tags (`text`) and the quoted values of nine user-facing attributes (`attribute`). It reads the code
of script blocks and markup expressions, comments removed, for a quoted literal handed to one of
five dialog calls (`dialog`). A text is a site when it holds a letter and is not one listed name
alone.
- **False positives:** a literal that is the same in every locale but is not a listed name (a unit
  symbol spelt in letters, a code shown as text); none at dev.
- **False negatives:** a literal inside a markup expression (`{ok ? 'Yes' : 'No'}`), a text a `.ts`
  file builds and a component shows, an attribute outside the nine, and a dialog called through an
  alias. Each is outside the census, and SPEC-653 section 5 tracks the first two.

**D2. Warn, an empty listing, and a key without a line (SPEC-653 R6-R8).** `LEVEL` is `warn` and
`LISTED` is empty, since dev holds no site. A site's key is its file, form, shown text and ordinal,
so a moved line keeps its key; the line is reported for the reader. A listing entry carries a
reason, and a malformed or repeated entry fails at both levels, as a stale one does.

**D3. Kill criteria, by ADR-450 D6.** The rule takes ADR-450 D6's promotion rule and kill criteria
as they stand: `LEVEL` moves from warn to fail after three iterations of the rule review at
precision of at least 0.8 with at least five hits, by its own delivery; live precision below 0.8
with at least five hits proposes a demotion; this is a new zero-hit rule, so 2 or more false hits
among its first 5 propose a demotion; a demotion is a weakening and goes to the owner as a signed
ruling; `examined 0` is VOID, which blocks, never a pass.

**D4. One module, planted components and hand rows (SPEC-653 R9, R10, section 9).** The reader
and its tests sit in one module under `scripts/tests`, as the temporary-file census does. The
planted components carry a `.fixture` suffix, so no web tool reads them. The Python mutation lane
mutates only `scripts/<name>.py`, so the module's arms are proved by hand-written rows in the
SPEC's own band, each killed by one test.

**D5. Not in this delivery.** No `.ts` reader and no reader of literals inside markup
expressions (SPEC-653 section 5); no component or catalog change.

**D6. Tests and rows, red first.** One test per acceptance criterion. The tests, the planted
components and the documents are committed first, over a stub reader that keeps every input and
finds nothing; the reader, the rows and the record follow.

## What it was chosen against

### D1

- **The Svelte compiler or a lint plugin reading the template.** It needs node in the python stage
  and a pinned web toolchain for a check that reads committed text; chosen against.
- **Reading the compiled messages.** A literal never reaches the compiled catalogs, so it cannot be
  seen there; chosen against.
- **Every attribute.** Most attributes (`class`, `href`, `id`, `role`) are never read by a person;
  nine user-facing attributes are named instead.

### D2

- **Starting at fail, since dev holds no site.** #1370 states warn, and ADR-450 D6's promotion
  route applies to every new census; chosen against.
- **A key with the line.** A line moves with every edit above it, and a listing keyed by line goes
  stale on unrelated edits; chosen against.

### D3

- **Kill criteria of its own.** Two rules of the same kind under two promotion rules would be read
  apart; ADR-450 D6 is taken by reference instead.

### D4

- **A new `scripts/<name>.py` with a mutation-lane entry.** The reader is test code that reads the
  tree, like the other guard censuses; chosen against.
- **Amending SPEC-436 and ADR-450.** Their surface is async Rust, and their band has too few free
  ids for this census's rows; chosen against.

### D5

- **A `.ts` form now.** At dev it finds 28 candidates and 0 user-facing texts, so its precision
  would start under 0.8; chosen against.

## Consequences

- A new literal text in a component prints a `WARN` line in CI and passes, until the rule is
  promoted by its own delivery.
- A listing entry needs a reason, so a site kept at warn says why.
- The planted components grow with the reader: a new form is added with its planted line and its
  accepted control.

## Confirmation

Each of SPEC-653's acceptance criteria, read red or not red with its reason and then green,
recorded in `docs/red-first/SPEC-653.md`. The rows S65301 to S65361.

## What would make this wrong

- A component shows a literal through a form the reader does not name. Then the census holds 0
  while the text ships, and the form is the next rule.
- The census lists a site the review finds correct in more than 1 of 5 cases. Then its precision
  falls, and D3 proposes a demotion to the owner.
- The template syntax changes so that an expression is no longer a brace pair. Then D1's reader is
  wrong, and the census is re-read against the new syntax.

## More Information

SPEC-653 (requirements, criteria, manifest); ADR-450 D6 (the promotion rule and kill criteria);
SPEC-571 and ADR-590 (the catalogs held whole); #1370 (this rule); #1052 (the iPhone and iPad
half).
