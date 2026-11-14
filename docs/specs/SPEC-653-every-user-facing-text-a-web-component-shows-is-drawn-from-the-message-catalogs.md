# SPEC-653: every user-facing text a web component shows is drawn from the message catalogs

Path:
`docs/specs/SPEC-653-every-user-facing-text-a-web-component-shows-is-drawn-from-the-message-catalogs.md`.

- **Issue:** #1370 (refuse user-facing text written outside the web app's message catalogs). The
  iPhone and iPad half is #1052.
- **Context(s):** the web app's components (`web/app/src`, read as text and never changed) and the
  repository's own guard tests (`scripts/tests`). No product source, catalog, workflow,
  `scripts/check.sh` or test-runner configuration changes.
- **Decided by:** ADR-675 (literal user-facing text in a web component is a census that starts at
  warn). It cites ADR-450 D6 (the promotion rule and the kill criteria), SPEC-571 and ADR-590 (the
  seven catalogs held whole by their census), and amends none of them.
- **Schematic:** `docs/schematics/web-literal-text-census.md`, added.
- **Mutation band:** `S65300-S65399` (section 9).
- **Status:** one pull request into dev, with its tests and `docs/red-first/SPEC-653.md`. Read at
  dev `b9a08e60`.

## 1. The problem, measured

Read at dev `b9a08e60`, by `git show <dev>:<path>` and a standard-library reader over the copies.

- The web app's text lives in seven catalogs under `web/app/messages/`, listed by
  `web/app/project.inlang/settings.json` and held whole by `scripts/tests/test_locale_census.py`
  (SPEC-571). A component draws a message by importing `m` from `#lib/paraglide/messages.js` (57
  imports) and calling `m.<key>()` or `m.<key>({...})` inside a `{...}` expression.
- No check refuses a text a component writes in its markup instead of drawing it from a catalog.
  Such a text ships in one language to every locale, and the catalog census cannot see it.
- The 53 `.svelte` files under `web/app/src` hold 0 literal text nodes, 0 literal values of a
  user-facing attribute and 0 literal dialog texts, by the forms of R2 to R4. Each count is a lower
  bound: it counts only the forms this SPEC names.
- Outside those forms: 93 quoted literals holding a letter sit inside markup expressions, 71 of
  them where their value could be shown as text; none holds a space or a capital letter (state
  names such as `ok` and `reopen`, compared and never shown). The 61 non-test `.ts` sources under
  `web/app/src` hold 28 literals of two or more words, in 9 files: frame document markup, test
  stand-ins, worker and protocol error texts, a protocol label and a class list; no component
  expression names an error or a reason. Neither population is in this census (section 5).

## 2. Requirements

- **R1. The population.** The census reads every `.svelte` file under `web/app/src` as text, prints
  `examined <n> component(s)` and refuses an empty population, which is VOID, never a pass.
- **R2. A text node.** A run of markup text before, between or after tags that holds a letter
  outside every `{...}` expression is a site of form `text`.
- **R3. A user-facing attribute.** A quoted value, in double or single quotes, that holds a letter
  outside every expression is a site of form `attribute` when its attribute is one of `alt`,
  `aria-description`, `aria-label`, `aria-placeholder`, `aria-roledescription`, `aria-valuetext`,
  `label`, `placeholder` or `title`. Every other attribute (`class`, `id`, `href`, `role`, `data-*`
  and the rest) is not read by a person.
- **R4. A dialog.** A quoted literal holding a letter, handed as the first argument to
  `window.alert`, `window.confirm`, `window.prompt`, `.showAlert` or `.showConfirm` in a script
  block or a markup expression, is a site of form `dialog`; a `${...}` part of a template literal
  is shown as `{…}`. A bare `confirm(` is a component's own function, not a dialog.
- **R5. Not user-facing.** Script and style blocks are not markup; an HTML comment and a `//` or
  `/* */` comment in code are not read; every `{...}` expression is code, so a catalog draw is
  never a site; a character reference is read as the character it names; a text with no letter (a
  number, a sign, a symbol) and a text that is one of `Anki`, `DeckStreak` or `Telegram` alone are
  not sites.
- **R6. A site and its key.** Each site reports its file, line, form, shown text (each expression
  shown as `{…}`, whitespace collapsed) and ordinal among equal texts of its form in its file. Its
  key is the file, form, text and ordinal, never the line, so an edit above a site keeps its key.
- **R7. Level and listing.** `LEVEL` is `warn`. A site `LISTED` names passes. A site it does not
  name prints `WARN literal-text <file>:<line> <form> '<text>' #<ordinal>` and passes at warn, and
  prints `FAIL ...` and fails at `fail`. An entry no site matches prints `STALE ...` and fails at
  both levels. Each entry is five cells (file, form, text, ordinal, reason) with a reason that is
  not blank, and names a key no other entry names; any other entry fails.
- **R8. Today's listing.** Dev holds no site, so `LISTED` is empty. Moving `LEVEL` to `fail` is its
  own delivery, under ADR-450 D6's promotion rule (ADR-675 D3).
- **R9. Planted components.** Four planted components sit under
  `scripts/tests/fixtures/web-literal-text/`, each with a `.fixture` suffix so that no web tool and
  no `*.svelte` glob reads them. A line the census must report carries a `planted:` comment, a form
  it must pass carries an `accepted:` one, and each test checks that the reported lines are exactly
  the planted lines.
- **R10. One module.** `scripts/tests/test_web_literal_text.py` holds the reader beside its tests,
  imports the standard library and `_support` only, and runs in the python stage of
  `scripts/check.sh` as its own `unittest discover` run.

## 3. Acceptance criteria of SPEC-653

| # | criterion | decided by |
|---|---|---|
| A1 | each planted literal text node is a site, with its line, shown text and ordinal; a catalog draw, a name alone, a script string and a style block are not | `test_web_literal_text.py`; red: the stub finds no site |
| A2 | each of the nine user-facing attributes given a quoted literal is a site, in either quote and holding a quoted `>`; a catalog draw, an empty value, a value with no letter, another attribute and a name alone are not | `test_web_literal_text.py`; red: the stub finds no site |
| A3 | each of the five dialog calls given a quoted literal is a site, in a script block or a markup expression, with a template part shown as `{…}`; a catalog draw, a commented call, a literal with no letter and a bare `confirm(` are not | `test_web_literal_text.py`; red: the stub finds no site |
| A4 | a component that draws every text from a catalog, with nested braces, blocks, references, a number, names alone, a quoted brace, comments, a script and a style block, has no site | `test_web_literal_text.py`; the control |
| A5 | at warn an unlisted site warns and passes, at fail it fails, a listed site passes and a stale entry fails at both; an entry with a blank reason, a repeated key or other than five cells is refused by its exact line | `test_web_literal_text.py`; red: the stub passes everything |
| A6 | a tree with no component is refused with `examined 0 component(s)` | `test_web_literal_text.py`; red: the stub skips the refusal |
| A7 | the repository's components are read, the route layout among them and no planted one, the level is warn, the listing is well formed and the verdict passes | `test_web_literal_text.py`; not red: dev holds no site |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_web_literal_text.py -k a_planted_literal_text_node_is_a_site
A2: python3 -m unittest discover -s scripts/tests -p test_web_literal_text.py -k a_planted_literal_user_facing_attribute_is_a_site
A3: python3 -m unittest discover -s scripts/tests -p test_web_literal_text.py -k a_planted_dialog_literal_is_a_site
A4: python3 -m unittest discover -s scripts/tests -p test_web_literal_text.py -k a_component_that_draws_every_text_from_a_catalog_has_no_site
A5: python3 -m unittest discover -s scripts/tests -p test_web_literal_text.py -k the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does
A6: python3 -m unittest discover -s scripts/tests -p test_web_literal_text.py -k an_empty_population_is_refused
A7: python3 -m unittest discover -s scripts/tests -p test_web_literal_text.py -k the_repository_lists_exactly_todays_sites
```

The planted components hold 7 text-node sites, 9 attribute sites and 6 dialog sites, each spelt in
its test with its line, form, text and ordinal, and the control holds 11 accepted forms and no
site. The level and listing test plants one site, two entries and a malformed listing of three
problems, and asserts each line exactly.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-653-every-user-facing-text-a-web-component-shows-is-drawn-from-the-message-catalogs.md` | document | new |
| `docs/decisions/ADR-675-literal-user-facing-text-in-a-web-component-is-a-census-that-starts-at-warn.md` | document | new |
| `docs/schematics/web-literal-text-census.md` | document | new |
| `docs/red-first/SPEC-653.md` | record | new |
| `scripts/tests/test_web_literal_text.py` | test | new |
| `scripts/tests/fixtures/web-literal-text/literal_text_node.svelte.fixture` | test | new |
| `scripts/tests/fixtures/web-literal-text/literal_attribute.svelte.fixture` | test | new |
| `scripts/tests/fixtures/web-literal-text/literal_dialog.svelte.fixture` | test | new |
| `scripts/tests/fixtures/web-literal-text/catalog_only.svelte.fixture` | test | new |
| `scripts/mutation-rows.d/S65300-S65399.json` | rows | new |
| `changelog.d/web-literal-text.md` | changelog | new |

## 5. What this does NOT cover

- No text of the iPhone and iPad app: tracked on #1052.
- No `.ts` source: 28 literals of two or more words in 9 of 61 files at dev, none shown by a
  component; a reader for text a `.ts` file builds for a component is tracked on #1402.
- No literal inside a markup expression, such as `{ok ? 'Yes' : 'No'}`: 71 at a text position at
  dev, none shaped as prose; tracked on #1403.
- No component and no catalog changes: the census reads, and dev holds no site to cure.

## 6. Risks

- **A site in a form the reader does not name** (a literal in a markup expression, a text built in
  a `.ts` file, an attribute outside the nine): the census holds 0 while a component shows a
  literal. Each count is a lower bound, and section 5's issues name the next forms.
- **A planted component read as a real one.** The `.fixture` suffix keeps it out of every
  `*.svelte` glob, and the repository test asserts that no planted path is in the population.
- **An open change adds a component with a literal before this lands.** At warn its site prints a
  `WARN` line in the merge's CI and passes; it is cured by drawing the text from the catalogs, never
  by a listing entry added to pass.
- **A name alone changes by locale.** The names set is edited only by its own delivery.

## 7. What only CI proves

- The python stage of the `hygiene` job runs the module over the merge's tree and prints
  `examined <n> component(s)`; the red push's run reads A1, A2, A3, A5 and A6 red by assertion.
- The rows job runs each row's killer against its mutant.

## 8. Formal model

No state, actor or write path is added, and no covered span is edited: every file of this delivery
is new. The census is a pure reader of committed text; its classification is closed by the planted
components, whose expected sites are spelt literally, and by section 9's rows. No model checker
runs.

## 9. Mutation rows (S65300-S65399)

Rows S65301 to S65361, in `scripts/mutation-rows.d/S65300-S65399.json`, table
`SCRIPT_MUTATIONS`, each `[id, path, find, replace, description, killer]` on
`scripts/tests/test_web_literal_text.py`. Each find is taken from the green commit's text and
occurs exactly once in the module; each killer is one test by its full id.

| rows | what each mutant removes or breaks | killer |
|---|---|---|
| S65301-S65309 | one user-facing attribute: `alt`, `aria-description`, `aria-label`, `aria-placeholder`, `aria-roledescription`, `aria-valuetext`, `label`, `placeholder`, `title` | A2 |
| S65310-S65314 | one dialog call: `alert`, `confirm`, `prompt`, `showAlert`, `showConfirm` | A3 |
| S65315-S65319 | the `window` requirement, the template quote, the text's end before its quote, the letter test, the template part | A3 |
| S65320-S65324 | the script span and the expression span of the dialog scan, the `//` and `/* */` comments, the code literal kept whole | A3 |
| S65325-S65328 | one name alone: `Anki`, `DeckStreak`, `Telegram`; the names check | A4 |
| S65329-S65337 | the script block, the style block, the comment, the expression blank, nested braces, the quoted brace, the character reference, the text node's letter test, the digit | A4 |
| S65338-S65340 | the quoted `>` in a tag, the attribute's letter test, the single-quoted value | A2 |
| S65341-S65347 | the text before a tag, the text after the last tag, the closing tag, the expression mark, the first letter's line, the line count, the ordinal | A1 |
| S65348-S65353 | the verdict's word, its warn pass, its fail refusal, its stale refusal, the listed lookup, the key lookup | A5 |
| S65354-S65357 | the listing's five cells, its reason, its repeated key, its key memory | A5 |
| S65358 | the empty-population refusal | A6 |
| S65359-S65361 | the nested walk, the components root, the warn level | A7 |
