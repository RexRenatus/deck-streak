---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/accessibility

WCAG 2.2 level AA as ONE pack (SPEC-V2-2223 / ADR-V2-2223). The pack does two things:

- **The coverage matrix**, [coverage.json](coverage.json), maps each of the 55 success criteria
  of levels A and AA to what judges it:
  - the sibling packs' rows, named `pack:row` and never copied;
  - this pack's own classes, for what no sibling checks;
  - the axe-core rules a rendered audit runs;
  - or a person, with the reason.
- **The classes** of `scripts/accessibility-probe.py`. It is standard-library Python and
  vendorable, it reads any tree through `--root`, and every row below runs it.

Which seats consume this pack is its catalog row's `consumes`, the one record of that edge
(ADR-V2-1990), so this body names none.

```
phxd pack probe --pack accessibility --root PATH --format json
```

WCAG 2.2 is a W3C Recommendation. It removed 4.1.1 Parsing as obsolete, and AA conformance means
all 31 level A and all 24 level AA criteria. The 31 AAA criteria are outside the target.
`accessibility-probe.py criteria` prints the 55 criteria, parsed from the Recommendation's own
markup, and `matrix-complete` holds the matrix to that list.

## How the packs share WCAG

No criterion is checked twice. Each row lives in the pack whose subject it is, and this matrix
names it there:

- **web-launch** checks the document layer of BUILT HTML: `lang`, titles, names of buttons and
  links, labels, frames, zoom, skip links, landmarks, headings, input purpose, tabindex,
  autoplay and paste. On a client-rendered SvelteKit app, the built HTML is only the shell, so
  those rows see little.
- **The Svelte compiler** checks the same things in the `.svelte` SOURCE, through its `a11y_*`
  warnings. Three rows make that a gate:
  - stack-selection's `svelte-check-ci` runs svelte-check in CI;
  - this pack's `svelte-a11y-not-disabled` refuses a filter or a flag that switches those
    warnings off;
  - `svelte-a11y-fails-ci` advises when a warning does not fail the run.
- **vibecode-polish** owns focus visibility, reduced motion and reflow at 320 px.
- **ux-laws** owns target size, drag and swipe alternatives, native controls, autoplay and endless
  motion.
- **ui-styles** TEACHES the contrast and focus-ring token rules (catalog rows, not checks). The
  matrix lists them under `taught_by`.
- **cjk-typography** owns 3.1.2 for CJK passages: every CJK run carries `lang`, and Chinese names
  its script.
- **The runtime audit** runs axe-core over the rendered pages and measures what a static read
  cannot: rendered colour pairs, target sizes and live regions. `runtime-audit` checks that one is
  wired. [templates/a11y.spec.ts](templates/a11y.spec.ts) is DeckStreak's, ready to copy.

## The rows

Twenty-three rows, all `tree`-scoped, one per class of `accessibility-probe.py`. Each runs
`python3 {skills}/../scripts/accessibility-probe.py --root {root} --skills {skills} check <class>`
under a 60-second wall. `{skills}` is the skills directory the catalog was read from, so the
script and the matrix always come from this pack, whatever tree `--root` names.

The `matrix` stage: 2 rows. They judge the pack's own data, so they answer the same for every tree.

| row | severity | reason | refuses when | WCAG |
|---|---|---|---|---|
| `matrix-complete` | block | `matrix-incomplete` | a criterion of WCAG 2.2 A or AA is missing or mapped twice, an entry is AAA or unknown, a level or name disagrees with the Recommendation, an entry has no static row, no axe rule and no manual reason, teaches nothing, or names a row that is not `pack:row`; or 4.1.1 is not recorded as removed | all 55 |
| `matrix-rows-resolve` | block | `matrix-row-unresolved` | a `pack:row` the matrix names (under `static` or `taught_by`) has no `checks.json`, or that pack has no such row; or a row of this pack is mapped to no criterion | all 55 |

The `markup` stage: 9 rows. They read HTML, Svelte, Astro and Vue templates. Test directories
(`tests`, `e2e`, `fixtures` and similar) and `*.spec.*` or `*.test.*` files are skipped: a test
that plants a failure to prove a guard is not one.

| row | severity | reason | refuses when | WCAG |
|---|---|---|---|---|
| `label-in-name` | block | `name-hides-label` | a button, link, `input type=submit/button/reset`, or an element with a widget role, carries an `aria-label` that does not contain its visible text (case and punctuation ignored; failure F96) | 2.5.3 |
| `aria-valid` | block | `aria-token-invalid` | a `role` token is not a WAI-ARIA role, is abstract, or is deprecated (`directory`), or an `aria-*` attribute is unknown or deprecated (`aria-grabbed`, `aria-dropeffect`). ARIA 1.2's roles and attributes are known, and so are the 1.3 draft's additions that browsers ship, such as `aria-description` | 4.1.2 |
| `aria-hidden-focus` | block | `hidden-yet-focusable` | an `aria-hidden="true"` element takes keyboard focus, or holds a descendant that does (`tabindex="-1"` and `disabled` do not) | 4.1.2, 1.3.1 |
| `keyboard-operable` | block | `pointer-only-handler` | an element that is not a native control handles a click or pointer event but takes no focus (`tabindex` 0 or above) or handles no key. A container whose own descendant is a control is left alone, because the click is that control's. Failure F54 | 2.1.1 |
| `video-captions` | block | `captions-missing` | a `<video>` that is not `muted` has no `<track kind="captions">`. Subtitles are not captions | 1.2.2 |
| `meta-refresh` | block | `timed-refresh` | a `<meta http-equiv="refresh">` refreshes or redirects after 1 to 72,000 seconds (failures F40, F41) | 2.2.1 |
| `context-change` | block | `context-change-unasked` | a focus handler calls `blur()`, navigates or submits (failure F55), or a change, input or radio-click handler on a form control navigates or submits (failures F36, F37). A handler named by reference is read from the component's own script | 3.2.1, 3.2.2 |
| `auth-autofill` | block | `autofill-blocked` | a password field sets `autocomplete="off"`, or sits in a form that does, which keeps password managers from filling it | 3.3.8 |
| `status-messages` | advisory | `status-not-announced` | a toast, snackbar, flash or notification surface, by class, id or component name, has no `role="status"`, `alert` or `log` and no `aria-live` on it, above it or inside it | 4.1.3 |

The `script` stage: 4 rows. They read script files and `<script>` blocks, with comments stripped,
plus the web app manifest.

| row | severity | reason | refuses when | WCAG |
|---|---|---|---|---|
| `orientation-lock` | block | `orientation-locked` | code calls Telegram's `lockOrientation()` or `screen.orientation.lock()`, or a web app manifest sets a portrait or landscape `orientation` (failure F97) | 1.3.4 |
| `pointer-cancel` | advisory | `down-event-activation` | a control acts on `mousedown`, `pointerdown` or `touchstart` with no up or click handler, or a file listens for a down event and never for the up event (failure F101) | 2.5.2 |
| `motion-actuation` | advisory | `motion-without-alternative` | code responds to device motion: `devicemotion`, `deviceorientation`, the Generic Sensor classes, or Telegram's `Accelerometer`, `Gyroscope` and `DeviceOrientation` (failure F106) | 2.5.4 |
| `key-shortcuts` | advisory | `single-key-shortcut` | a page-wide key handler (on `window`, `document` or `<svelte:window>`) acts on one printable key without checking Ctrl, Alt or Meta (failure F99) | 2.1.4 |

The `style` stage: 5 rows. They read CSS files and `<style>` blocks, nested rules included, and
the Tailwind classes that set the same things.

| row | severity | reason | refuses when | WCAG |
|---|---|---|---|---|
| `inline-spacing` | block | `spacing-locked` | an inline `style` or a Svelte `style:` directive sets `line-height`, `letter-spacing` or `word-spacing` with `!important`, which a user's text-spacing override cannot beat | 1.4.12 |
| `contrast-pairs` | block | `contrast-below-aa` | a rule that sets both `color` and `background` or `background-color` to colours a static read resolves (hex, `rgb()`, the basic named colours, `var()` from `:root` with its fallback) has a ratio under 4.5:1, or under 3:1 for large text (24 px, or 18.66 px bold) | 1.4.3 |
| `focus-not-obscured` | advisory | `focus-can-be-obscured` | a rule or a Tailwind class makes an element `fixed` or `sticky` at the top or bottom edge, and no rule sets `scroll-padding` (technique C43). A full-screen overlay (`inset: 0`) is a dialog the user opened, and is left alone | 2.4.11 |
| `hover-reveal` | advisory | `hover-only-reveal` | a `:hover` rule shows content (`display`, `visibility`, `opacity`) and no `:focus`, `:focus-visible` or `:focus-within` twin shows it, or a Tailwind `hover:`, `group-hover:` or `peer-hover:` reveal has no focus variant | 1.4.13, 2.1.1 |
| `flash-rate` | advisory | `may-flash` | an animation runs more than three times (or endlessly) with a cycle shorter than a third of a second | 2.3.1 |

The `toolchain` stage: 3 rows. They read Svelte and Vite configuration, `package.json` scripts, CI
workflows and tests.

| row | severity | reason | refuses when | WCAG |
|---|---|---|---|---|
| `svelte-a11y-not-disabled` | block | `a11y-warnings-disabled` | a `warningFilter` or `onwarn` drops the compiler's `a11y_*` warnings (a negated a11y match, an inequality with an a11y code, or an early return on one), or `svelte-check --compiler-warnings` sets an a11y code to `ignore` | the 12 criteria the compiler checks |
| `svelte-a11y-fails-ci` | advisory | `a11y-warnings-pass-ci` | a svelte-check invocation runs without `--fail-on-warnings`, or a `svelte-ignore a11y_*` comment gives no reason in parentheses | the same 12 |
| `runtime-audit` | advisory | `runtime-audit-missing` | a tree with markup has no test that runs axe-core over a rendered page, an audit's tags leave out any of `wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa` and `wcag22aa`, or no workflow runs `playwright test` | the 20 criteria axe-core checks |

Every class prints one line per finding, `<class>: <file>:<line> <finding>`, and ends with
`examined N`, the files it read (the matrix entries, for the two matrix rows). The script exits 0
when green, 1 on a finding, 2 on a usage error, and 3 when VOID. VOID means nothing was examined or
an input could not be read, and it is never a pass. A class with nothing to judge in a tree still
counts the files it read to decide that. The pack's card turns any non-zero exit of a `block` row
red, and prints an `advisory` row's failure as `advisory` without reddening the card.

## Exceptions: an essential use, recorded where it happens

Five criteria allow an exception for an essential use or an advised change:

- 1.3.4, an orientation that is essential;
- 2.2.1, a real-time, essential or over-20-hours time limit;
- 2.5.2, an essential down-event;
- 2.5.4, motion that is essential or runs through an accessibility-supported interface;
- 3.2.2, a change the user was told about first.

Their classes accept a comment on the finding's line or in the three lines above it:

```
// a11y-exception: the piano trainer's keyboard needs landscape
```

The reason is required. A waived site prints as `waived <file>:<line> (<reason>)` and is not a
finding.

## The matrix

`coverage.json` holds one entry per criterion:

- `static`: the rows that check it, as `pack:row`;
- `taught_by`: catalog rows that teach it without checking;
- `runtime`: the axe-core rules a rendered audit runs for it;
- `manual`: what only a person can judge, when nothing else can;
- `teach`: the rule as a builder applies it.

The numbers, which add up to all 55:

- 37 criteria have a static row in a pack. 19 of them have a class of this pack, and the rest
  are checked by a sibling pack.
- 1 (2.4.6) is checked statically only by the Svelte compiler.
- 1 (1.4.1) is checked only by the rendered audit.
- 16 are judged by a person alone, each with its reason in `manual`.

Across all 55, 20 criteria have an axe-core rule.

"Svelte compiler" below means the three toolchain rows plus stack-selection's `svelte-check-ci`,
which together make the compiler's `a11y_*` warnings a gate.

| SC | level | name | static rows | axe-core rules | a person judges |
|---|---|---|---|---|---|
| 1.1.1 | A | Non-text Content | `web-launch:alt-text`, Svelte compiler | aria-meter-name, aria-progressbar-name, image-alt, input-image-alt, object-alt, role-img-alt, svg-img-alt |  |
| 1.2.1 | A | Audio-only and Video-only (Prerecorded) | none | none | yes |
| 1.2.2 | A | Captions (Prerecorded) | `accessibility:video-captions`, Svelte compiler | video-caption |  |
| 1.2.3 | A | Audio Description or Media Alternative (Prerecorded) | none | none | yes |
| 1.2.4 | AA | Captions (Live) | none | none | yes |
| 1.2.5 | AA | Audio Description (Prerecorded) | none | none | yes |
| 1.3.1 | A | Info and Relationships | `web-launch:form-labels`, `web-launch:heading-order`, `web-launch:landmarks`, `accessibility:aria-hidden-focus`, Svelte compiler | aria-hidden-body, aria-required-children, aria-required-parent, definition-list, dlitem, list, listitem, td-headers-attr, th-has-data-cells |  |
| 1.3.2 | A | Meaningful Sequence | none | none | yes |
| 1.3.3 | A | Sensory Characteristics | none | none | yes |
| 1.3.4 | AA | Orientation | `accessibility:orientation-lock` | none |  |
| 1.3.5 | AA | Identify Input Purpose | `web-launch:input-purpose`, `ux-laws:controls.input-purpose`, Svelte compiler | autocomplete-valid |  |
| 1.4.1 | A | Use of Color | none | link-in-text-block | yes |
| 1.4.2 | A | Audio Control | `web-launch:autoplay-audio`, `ux-laws:motion.autoplay-controls` | no-autoplay-audio |  |
| 1.4.3 | AA | Contrast (Minimum) | `accessibility:contrast-pairs`, `web-launch:contrast`; taught by `ui-styles:contrast-text-normal`, `ui-styles:contrast-text-large` | color-contrast |  |
| 1.4.4 | AA | Resize Text | `web-launch:zoom-not-disabled`; taught by `ui-styles:type-scale-relative-units` | meta-viewport |  |
| 1.4.5 | AA | Images of Text | none | none | yes |
| 1.4.10 | AA | Reflow | `vibecode-polish:no-horizontal-scroll`, `vibecode-polish:mobile-menu-overflow`, `web-launch:mobile` | none |  |
| 1.4.11 | AA | Non-text Contrast | none; taught by `ui-styles:contrast-non-text`, `ui-styles:states-focus-contrast` | none | yes |
| 1.4.12 | AA | Text Spacing | `accessibility:inline-spacing` | avoid-inline-spacing |  |
| 1.4.13 | AA | Content on Hover or Focus | `accessibility:hover-reveal` | none |  |
| 2.1.1 | A | Keyboard | `accessibility:keyboard-operable`, `accessibility:hover-reveal`, `ux-laws:laws.jakob-native-controls`, Svelte compiler | frame-focusable-content, scrollable-region-focusable, server-side-image-map |  |
| 2.1.2 | A | No Keyboard Trap | none | none | yes |
| 2.1.4 | A | Character Key Shortcuts | `accessibility:key-shortcuts` | none |  |
| 2.2.1 | A | Timing Adjustable | `accessibility:meta-refresh` | meta-refresh |  |
| 2.2.2 | A | Pause, Stop, Hide | `ux-laws:motion.autoplay-controls`, `ux-laws:motion.infinite-animation`, `vibecode-polish:reduced-motion-support`, Svelte compiler | blink, marquee |  |
| 2.3.1 | A | Three Flashes or Below Threshold | `accessibility:flash-rate` | none | yes |
| 2.4.1 | A | Bypass Blocks | `web-launch:skip-link`, `web-launch:landmarks` | bypass |  |
| 2.4.2 | A | Page Titled | `web-launch:page-title` | document-title |  |
| 2.4.3 | A | Focus Order | `web-launch:positive-tabindex`, Svelte compiler | none |  |
| 2.4.4 | A | Link Purpose (In Context) | `web-launch:link-name`, Svelte compiler | area-alt, link-name |  |
| 2.4.5 | AA | Multiple Ways | none | none | yes |
| 2.4.6 | AA | Headings and Labels | Svelte compiler | none | yes |
| 2.4.7 | AA | Focus Visible | `vibecode-polish:focus-visible-outline`; taught by `ui-styles:states-focus-contrast` | none |  |
| 2.4.11 | AA | Focus Not Obscured (Minimum) | `accessibility:focus-not-obscured` | none |  |
| 2.5.1 | A | Pointer Gestures | `ux-laws:controls.drag-alternative` | none |  |
| 2.5.2 | A | Pointer Cancellation | `accessibility:pointer-cancel` | none |  |
| 2.5.3 | A | Label in Name | `accessibility:label-in-name` | none |  |
| 2.5.4 | A | Motion Actuation | `accessibility:motion-actuation` | none |  |
| 2.5.7 | AA | Dragging Movements | `ux-laws:controls.drag-alternative` | none |  |
| 2.5.8 | AA | Target Size (Minimum) | `ux-laws:controls.target-minimum` | target-size |  |
| 3.1.1 | A | Language of Page | `web-launch:html-lang`, Svelte compiler | html-has-lang, html-lang-valid, html-xml-lang-mismatch |  |
| 3.1.2 | AA | Language of Parts | `cjk-typography:cjk-lang` | valid-lang |  |
| 3.2.1 | A | On Focus | `accessibility:context-change` | none |  |
| 3.2.2 | A | On Input | `accessibility:context-change` | none |  |
| 3.2.3 | AA | Consistent Navigation | none | none | yes |
| 3.2.4 | AA | Consistent Identification | none | none | yes |
| 3.2.6 | A | Consistent Help | none | none | yes |
| 3.3.1 | A | Error Identification | `vibecode-polish:success-error-messages`, `web-launch:form-validation` | none |  |
| 3.3.2 | A | Labels or Instructions | `web-launch:form-labels`, `ux-laws:laws.postel-pattern-hint`, Svelte compiler | form-field-multiple-labels |  |
| 3.3.3 | AA | Error Suggestion | none | none | yes |
| 3.3.4 | AA | Error Prevention (Legal, Financial, Data) | none | none | yes |
| 3.3.7 | A | Redundant Entry | none | none | yes |
| 3.3.8 | AA | Accessible Authentication (Minimum) | `accessibility:auth-autofill`, `web-launch:paste-allowed`, `ux-laws:laws.postel-paste-allowed` | none |  |
| 4.1.2 | A | Name, Role, Value | `web-launch:button-name`, `web-launch:link-name`, `web-launch:iframe-title`, `web-launch:form-labels`, `web-launch:duplicate-ids`, `accessibility:aria-valid`, `accessibility:aria-hidden-focus`, `ux-laws:laws.jakob-native-controls`, Svelte compiler | area-alt, aria-allowed-attr, aria-braille-equivalent, aria-command-name, aria-conditional-attr, aria-deprecated-role, aria-hidden-body, aria-hidden-focus, aria-input-field-name, aria-prohibited-attr, aria-required-attr, aria-roles, aria-tab-name, aria-toggle-field-name, aria-tooltip-name, aria-valid-attr, aria-valid-attr-value, button-name, duplicate-id-aria, frame-title, frame-title-unique, input-button-name, input-image-alt, label, link-name, nested-interactive, select-name, summary-name |  |
| 4.1.3 | AA | Status Messages | `accessibility:status-messages` | none |  |

Each entry's `teach` field in `coverage.json` says how a builder meets the criterion. The
criteria a person judges are reviewed in the release checklist. Each carries its reason in
`manual`, and the review walks the rendered Mini App with a keyboard and a screen reader
(VoiceOver on iOS, TalkBack on Android, where Telegram's webview runs).

## How a project adopts this

A SvelteKit Mini App, such as DeckStreak, takes five steps.

1. **Run every pack the matrix names**, against the same tree: web-launch and vibecode-polish
   (on the built output, `build/`), ux-laws and cjk-typography, stack-selection (for
   `svelte-check-ci`), and this pack:

   ```
   phxd pack probe --pack accessibility --root PATH --format json
   ```

2. **Make the compiler's a11y warnings a gate.** Run `svelte-check --fail-on-warnings` in CI, never
   filter `a11y_*` codes out, and give every `<!-- svelte-ignore a11y_... (reason) -->` its reason.
3. **Wire the rendered audit.** Copy [templates/a11y.spec.ts](templates/a11y.spec.ts) to
   `tests/a11y.spec.ts`, list every route, and run `playwright test` in CI. It audits both of
   Telegram's colour schemes, because the `--tg-theme-*` colours exist only at runtime.
4. **Record essential exceptions** with `a11y-exception:` comments, each with its reason.
5. **Review what a person judges** before each release: the criteria marked "yes" above, with
   their `teach` text as the checklist.

## References

- WCAG 2.2: https://www.w3.org/TR/WCAG22/ and its Understanding documents,
  https://www.w3.org/WAI/WCAG22/Understanding/ (one per criterion; each names its failures and
  techniques, for example F96, F54, F55, F36, F37, F40, F41, F97, F99, F101, F106 and C43)
- WAI-ARIA 1.2: https://www.w3.org/TR/wai-aria-1.2/, and the 1.3 draft:
  https://www.w3.org/TR/wai-aria-1.3/
- axe-core's rules and their WCAG tags:
  https://github.com/dequelabs/axe-core/blob/develop/doc/rule-descriptions.md, and
  `@axe-core/playwright`: https://github.com/dequelabs/axe-core-npm/tree/develop/packages/playwright
- Svelte's compiler warnings: https://svelte.dev/docs/svelte/compiler-warnings, and svelte-check:
  https://github.com/sveltejs/language-tools/tree/master/packages/svelte-check
- Telegram Mini Apps (orientation lock and motion sensors): https://core.telegram.org/bots/webapps
- Context7 ids: `/websites/w3_tr_wcag22`, `/websites/w3_wai_wcag22`, `/mdn/content`,
  `/websites/svelte_dev_svelte`, `/sveltejs/language-tools`, `/dequelabs/axe-core`
