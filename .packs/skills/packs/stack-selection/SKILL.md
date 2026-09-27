---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/stack-selection

How our projects choose the language and the framework for a modern app or webpage, and the
check that keeps a repository on what was chosen (SPEC-V2-2198 / ADR-V2-2198). The owner's words,
2026-09-27: "lets brainstorm how our packs determine what coding language and framework it uses to
develop modern apps and webpages", and the answers that followed: "A stack-selection pack",
"Rust-first, TS at the UI", a monthly refresh, and "You approve adopt/hold moves".

Four things live here:

- the **decision procedure** an architect runs for a new project or a new part of one;
- the **rubric** that scores a choice, with the owner's weights;
- the **golden paths**: the owner-approved default for each kind of app, with pinned majors;
- a **dated tech radar**, `radar.json`, re-researched monthly with WebSearch and Context7.

`scripts/stack-probe.py` is the check. It is standard-library Python and vendorable, it judges any
tree through `--root`, and every row below runs it. Which seats consume this pack is its catalog
row's `consumes`, the one record of that edge (ADR-V2-1990), so this body names none.

```
phxd pack probe --pack stack-selection --root PATH --format json
```

## The rows

Ten rows, all `tree`-scoped, one per class of `stack-probe.py`. Each runs
`python3 {skills}/../scripts/stack-probe.py --root {root} --radar {skills}/packs/stack-selection/radar.json check <class>`
under a 60-second wall. `{skills}` is the skills directory the catalog was read from (SPEC-V2-2195),
so the script and the radar always come from this pack, whatever tree `--root` names.

The `stack` stage: 4 rows. They read the tree's root `stack.json` and its manifests.

| row | severity | reason | refuses when |
|---|---|---|---|
| `stack-declared` | block | `stack-undeclared` | `stack.json` is missing, is not JSON, has no elements (VOID), or is malformed: a wrong schema, an unknown key, an element with no item slug, a duplicate item, a `planned` that is not a boolean, `pins` that are not major series, an `adr` that is not a path |
| `stack-matches-detected` | block | `stack-drifted` | a manifest, lockfile or build config uses a radar item that `stack.json` does not declare, or `stack.json` declares a detectable item that nothing uses and that is not `planned`, or a manifest cannot be read |
| `no-hold-items` | block | `held-item-used` | an item in the radar's `hold` ring is declared, or used anywhere in the tree |
| `deviation-has-adr` | block | `deviation-without-adr` | an element off its golden path, or one that re-pins a golden item's major, cites no ADR, cites one that does not exist or lies outside the tree, or cites one that never names the item; or an app type is on no golden path |

The `toolchain` stage: 4 rows. They enforce the owner's version and guardrail decisions.

| row | severity | reason | refuses when |
|---|---|---|---|
| `pinned-majors` | block | `major-off-pin` | a Cargo or npm dependency the radar pins sits on another major series (workspace and pnpm catalog requirements resolved), Node is used with no pin in `.nvmrc`, `.node-version` or `engines.node`, a Node pin names another major, or `packageManager` pins another pnpm major |
| `svelte-runes` | block | `runes-not-forced` | a package that depends on `svelte` has no `svelte.config.*`, `vite.config.*` or `astro.config.*` beside it that sets `runes: true`, or one sets `runes: false` |
| `svelte-check-ci` | block | `svelte-check-not-in-ci` | a package that depends on `svelte` is checked by no workflow under `.github/workflows` or `.depot/workflows`: none names `svelte-check`, and none runs a package script whose command runs it |
| `no-corepack` | block | `corepack-used` | a workflow line or a package script runs `corepack`, or a tree with `pnpm-lock.yaml` has a root `package.json` with no `packageManager: pnpm@<version>` |

The `radar` stage: 2 rows. They judge the radar itself, so they answer the same for every tree.

| row | severity | reason | refuses when |
|---|---|---|---|
| `radar-fresh` | advisory | `radar-stale` | `refreshed_at` is more than 35 days old (the monthly refresh plus five days' grace), is after today, or an entry's `since` is after `refreshed_at`. Advisory: it reports and never refuses the pack (SPEC-V2-2195) |
| `radar-approvals` | block | `approval-missing` | an `adopt` or `hold` entry has no `owner_approval`, or one for another ring, dated after today, or later than the entry's `since`; a golden path is unapproved or names an item outside `adopt`; the rubric's weights do not sum to 1; or an entry is malformed (no evidence, a non-https source, an unknown ring or key, a duplicate id, a dependency two entries both claim, pins or scores out of shape) |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`, the
population it read. The script exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when
VOID: nothing was examined or an input could not be read, which is never a pass. The pack's card
turns any non-zero exit of a `block` row red (exit 4), and prints an `advisory` row's failure as
`advisory` without reddening the card. A class that has nothing to judge in a tree, such as
`svelte-runes` in a tree with no Svelte, still counts the files it read to decide that, so a scan
that read nothing is VOID rather than green.

## The decision procedure

An architect runs it for a new project, and again for any new part of one.

1. **Classify the app type.** One or more of the five types the golden paths name: `app-ui` (SPAs,
   dashboards, Telegram mini apps), `webpage` (landing, docs, SEO-critical), `backend-api`
   (services and APIs), `cli`, and `mobile-desktop`.
2. **Take the golden path.** The items of every golden path whose `applies_to` names one of the
   types are the default, at their pinned majors. Taking them needs no argument.
3. **Deviate only with an ADR.** An element that is not on the golden path, a trial or assess item
   included, or one that re-pins a golden item's major, needs an ADR in the repository. The ADR
   scores the choice and its golden-path counterpart on the rubric below, names what it was
   chosen against, and names the item. Nothing in the `hold` ring may be chosen at all.
4. **Record the result.** Write `stack.json` at the repository root: the app types, one element
   per chosen item, the ADR each deviation cites, and `planned: true` on an element whose code has
   not landed yet.
5. **Keep it true.** Run the pack against the tree, in the repository's CI and before each release.

## The rubric

The owner's four criteria, with the owner's weights, "AI .30 / fit .25 / maturity .25 / modern .20
(Recommended)". Each is scored 1 to 5. The weighted score is the sum of weight times score, out
of 5.

| criterion | weight | a 5 means |
|---|---|---|
| AI-buildability | 0.30 | a current model writes correct, current-version code the first time; Context7 covers it; an official llms.txt, MCP server or autofixer exists; old-syntax traps are caught by a compiler or a linter |
| runtime fit | 0.25 | it fits the runtime: a Telegram webview on a mid-range phone, a 2 vCPU / 1.9 GiB VM, systemd, static assets behind Caddy |
| ecosystem maturity | 0.25 | a stable major, a predictable release cadence, a security process, more than one maintainer, not stalled |
| modern, clean, fast, and UI customizability | 0.20 | the owner's words: current idioms, small and fast output, and full control of look and feel. For a non-UI item, "clean to compose" stands in for customizability |

AI-buildability leads because AI agents build these projects. The rubric decides between options,
and the posture, "Rust-first, TS at the UI", breaks a tie. Every radar entry records its four
scores in `scores`.

## The golden paths

Every path is owner-approved and recorded in `radar.json` with the owner's words. The pinned
majors follow the owner's version answer, "Stable majors, newer at trial (Recommended)": the
stable major is adopted, and the next major waits at trial.

| path | applies to | items and pinned majors |
|---|---|---|
| `app-ui` | app-ui | SvelteKit 2, Svelte 5, TypeScript 6, Tailwind CSS 4, shadcn-svelte, Bits UI 2, DTCG design tokens (format 2025.10) |
| `app-ui-kit` | app-ui, mobile-desktop | shadcn-svelte, Bits UI 2 |
| `app-ui-state` | app-ui, mobile-desktop | Svelte 5 runes for client state, TanStack Query 6 (`@tanstack/svelte-query`) for server state |
| `app-ui-forms` | app-ui, mobile-desktop | Superforms 2 in SPA mode, Formsnap 2, Zod 4 (`zod/mini` in the client); authoritative validation stays in Rust |
| `i18n` | app-ui, webpage, mobile-desktop | Paraglide JS 2, with the CJK rules below |
| `charts` | app-ui, webpage, mobile-desktop | Chart.js 4, tree-shaken and lazy-loaded |
| `telegram-mini-app` | app-ui, backend-api | Telegram's own `telegram-web-app.js` behind a typed runes wrapper; initData validated in-house by an axum extractor |
| `webpage` | webpage | Astro 7 with `@astrojs/svelte` 9 and Svelte islands |
| `ui-language` | app-ui, webpage, mobile-desktop | TypeScript 6 |
| `rust-core` | backend-api, cli, mobile-desktop | Rust (stable toolchain, edition 2024) |
| `backend-api` | backend-api | axum 0.8, tokio 1 (the `~1.53` LTS), sqlx 0.9, tracing 0.1 with tracing-subscriber 0.3 |
| `data` | backend-api | SQLite in WAL mode, Litestream 0.5 (0.5.2 or later) |
| `telegram-bot` | backend-api | frankenstein |
| `ops` | backend-api, cli, app-ui, webpage | tracing JSON to journald, tower-http's TraceLayer, web-vitals beacons, hardened systemd units, Caddy 2 |
| `mobile-desktop` | mobile-desktop | Tauri 2 (desktop), Svelte |
| `js-toolchain` | app-ui, webpage, mobile-desktop | pnpm 11, Node 24 LTS |
| `testing-js` | app-ui, webpage, mobile-desktop | Vitest 4, Playwright 1 |
| `testing-rust` | backend-api, cli, mobile-desktop | cargo-nextest, insta 1 |

What each golden path requires of the repository, and which row checks it:

- **Svelte runes, mandatory.** Every package that depends on `svelte` sets
  `compilerOptions: { runes: true }` in `svelte.config.js` (or in its `vite.config` or
  `astro.config`). Runes mode turns every Svelte 4 construct an AI builder emits (`$:`,
  `export let`, `on:click`) into a compile error. Checked by `svelte-runes`.
- **svelte-check in CI, mandatory.** A workflow runs `svelte-check`, directly or through a package
  script. Checked by `svelte-check-ci`. The owner chose "Compile + check only", so these two are the
  whole mandatory guardrail.
- **The Svelte MCP autofixer is optional.** `npx sv add ai-tools` installs the official Svelte MCP
  server and its `svelte-autofixer`; use it when it helps, and nothing checks for it.
- **pnpm without Corepack.** Pin pnpm with `"packageManager": "pnpm@11.x.y"` in the root
  `package.json` and install it with `pnpm/action-setup` or the standalone installer. Node 25 and
  later no longer ship Corepack, and pnpm advises against it. Checked by `no-corepack` and
  `pinned-majors`.
- **Node pinned.** `.nvmrc`, `.node-version` or `engines.node` names Node 24. Checked by
  `pinned-majors`.
- **axum's router is tested.** axum 0.8 writes paths as `/{id}`. A model trained on 0.7 writes
  `/:id`, which panics when the router is built, so every service has a test that builds its
  router.
- **tokio on its LTS.** Pin `tokio = { version = "~1.53" }`. tokio's own policy recommends a tilde on
  an LTS minor, and 1.53 is supported until September 2027.
- **Playwright on Linux.** `npx playwright install --with-deps chromium` installs the browser and
  its apt packages; add `--only-shell` for headless CI. On a non-Debian image, use the
  `mcr.microsoft.com/playwright:*-noble` image instead, because `--with-deps` needs apt.
- **nextest skips doctests.** CI runs `cargo test --doc` beside `cargo nextest run`.
- **A SvelteKit SPA behind Caddy.** Use adapter-static with a fallback page and `ssr = false`.
  Caddy serves it with `try_files {path} {path}.html /index.html` and proxies `/api/*` to axum.
  Telegram passes launch data in the URL hash, so route by path and read the hash once at startup.
- **initData is validated on the server.** Compute HMAC-SHA256 with the key
  HMAC-SHA256("WebAppData", bot token), check a maximum `auth_date` age, and compare in constant
  time. Never trust `initDataUnsafe`.
- **Hardened units.** Each service ships as one release binary under a unit with
  `ProtectSystem=strict`, `StateDirectory=`, `NoNewPrivileges=` and
  `SystemCallFilter=@system-service`, gated by `systemd-analyze security --offline=true --threshold=<n>`.
- **WebAssembly only by measured exception.** `rust-wasm` is adopted as a rule, and it is on no
  golden path, so every use cites an ADR. The ADR records a CPU-bound hot path measured above
  50 ms on a mid-range phone, or logic that must be bit-identical with the Rust server; the code
  runs in a Web Worker; and the ADR states a gzipped size budget. FSRS scheduling does not qualify:
  `ts-fsrs` runs on the client and the `fsrs` crate stays authoritative on the server.

## CJK text: checkable guidance

The owner approved "i18n: Paraglide 2 + CJK rules". These rules are platform rules, so they apply
whatever the i18n library. Each rule below has a check a builder or a test can run.

| rule | check |
|---|---|
| `<html lang>` is set per locale, and every CJK content node carries its own `lang`. Chinese names its script, `zh-Hans` or `zh-Hant`, because Han unification makes glyph choice follow `lang` | a Playwright test reads `document.documentElement.lang`, and every rendered CJK node reports the `lang` its content needs |
| every CJK language has its own `:lang()` font stack. Fonts are subset by `unicode-range` or come from the system; a whole CJK font is never shipped | grep the CSS for a `:lang(ja)`, `:lang(ko)`, `:lang(zh-Hans)` and `:lang(zh-Hant)` rule; no font file over 1 MB is in the build output |
| Japanese and Korean headings use `word-break: auto-phrase` as a progressive enhancement; Korean body text uses `word-break: keep-all`; Japanese uses `line-break: strict` | grep the CSS for the three declarations under their `:lang()` selectors |
| `word-break: break-all` is never applied to mixed CJK and Latin text; URLs and code use `overflow-wrap: anywhere` | grep the CSS: `break-all` does not appear |
| CJK line height is about 1.7 | grep the CSS for the `:lang()` line-height |
| dates, numbers and plurals go through `Intl.DateTimeFormat`, `Intl.NumberFormat` and `Intl.PluralRules`, and word boundaries through `Intl.Segmenter`, never hand-rolled | a Vitest test formats one date and one count in each locale |

## The radar

`radar.json` holds one entry per item: its `category`, its `ring`, the date it entered the ring
(`since`), its evidence URLs, a rationale, its `pins`, its `current` version and release date, the
Context7 ids that answered for it, its rubric `scores`, and a `detect` rule naming the Cargo crates,
npm packages, lockfiles, manifests or build-config tokens that show a tree uses it. The file is
dated by `refreshed_at`, and it carries the golden paths and the rubric.

The four rings:

- **adopt**: the default. It is owner-approved, and the golden paths draw from it.
- **trial**: recommended for its category, but not yet a default. Using it is a deviation that
  cites an ADR.
- **assess**: a researched alternative. Using it is a deviation that cites an ADR.
- **hold**: refused. It is owner-approved, and `no-hold-items` refuses a tree that declares it or
  uses it.

The governance rule is the owner's, "You approve adopt/hold moves":

- A refresh may move an item between `assess` and `trial` on its own evidence. It updates `since`,
  the `evidence` and the `rationale`.
- A move into `adopt` or into `hold` needs the owner's approval, recorded on the entry as
  `owner_approval`: the ring, the owner's words verbatim, the date, and how it was given.
- `radar-approvals` refuses an adopt or hold entry without one, an approval for another ring, and
  an entry dated before its approval.

On 2026-09-27 the radar holds 73 entries: 38 adopt, 7 trial, 23 assess and 5 hold, on 18 golden
paths. These are the moves the owner approved that day:

- **Held:** teloxide, stalled at Bot API 9.1; SvelteKit remote functions in Rust-backend
  repositories; the deprecated `@telegram-apps/*` packages; and Leptos and Dioxus for UI work.
- **Split:** Tauri 2 desktop adopts, and Tauri 2 mobile is at trial. Capacitor 8 is at assess as
  the named fallback, revisited at the first app-store submission.
- **At trial:** SvelteKit 3, TypeScript 7, pnpm 12, Vitest 5, Node 26 and LayerChart.
- **At assess, by the owner's answers:** PostgreSQL 18 (move to it at a second app server or
  sustained concurrent writers), OpenTelemetry and containers.

## The monthly refresh

The owner chose "Monthly radar refresh", and `radar-fresh` turns advisory after 35 days. Run the
refresh in a delivery of its own:

1. For every entry, re-research with WebSearch and Context7 against primary sources: the
   project's release notes, its changelog, its security advisories, and the standards or official
   docs. Update `current` (version and release date), `context7` (the library ids that answered),
   `evidence` and `rationale`.
2. Record every deprecation and end-of-life date that took effect since the last refresh, and
   every new major.
3. Re-score an entry whose facts moved, and apply the rubric's weights.
4. Move entries between `assess` and `trial` on the evidence, and update `since`.
5. Propose each move into `adopt` or `hold`, with the evidence and the rubric scores. Ask the owner
   through AskUserQuestion. Record only an approved move, with the owner's verbatim words and the
   date.
6. Set `refreshed_at` to the refresh date.
7. Run `stack-probe.py check radar-approvals` and `check radar-fresh`, then this pack against every
   repository that declares a `stack.json`. A newly pinned major, or a newly held item, turns those
   repositories red until they move or record a deviation.

The next scheduled owner decision is Node 26's move into adopt at the November refresh. Node 26
becomes Active LTS on 2026-10-28.

## How a project adopts this

A Rust and Svelte repository, such as DeckStreak, takes four steps.

1. **Declare the stack.** Write `stack.json` at the root:

   ```json
   {
     "schema": "phx.stack.v1",
     "app_types": ["backend-api", "app-ui"],
     "elements": [
       {"item": "rust"}, {"item": "axum"}, {"item": "tokio"}, {"item": "sqlx"},
       {"item": "tracing"}, {"item": "sqlite"}, {"item": "litestream"},
       {"item": "frankenstein"}, {"item": "tma-initdata-auth"}, {"item": "telegram-web-app-js"},
       {"item": "sveltekit"}, {"item": "svelte"}, {"item": "typescript"},
       {"item": "tailwindcss"}, {"item": "shadcn-svelte"}, {"item": "bits-ui"},
       {"item": "tanstack-query"}, {"item": "paraglide-js"}, {"item": "chart-js", "planned": true},
       {"item": "pnpm"}, {"item": "node"}, {"item": "vitest"}, {"item": "playwright"},
       {"item": "cargo-nextest"}, {"item": "insta"}, {"item": "systemd"}, {"item": "caddy"}
     ]
   }
   ```

   A deviation adds `"adr": "docs/decisions/ADR-0003-....md"`, pointing at an ADR that names the
   item. A version held back adds `"pins": {"<package>": "<major>"}`, with the same ADR.

2. **Configure the tree to the golden path.**
   - Set `compilerOptions: { runes: true }` in `svelte.config.js`.
   - Add a `check` script, `svelte-kit sync && svelte-check`, and run it in CI.
   - Pin `"packageManager": "pnpm@11.x.y"` and `"engines": {"node": ">=24"}`, and add an `.nvmrc` of `24`.
   - Install pnpm with `pnpm/action-setup`, never Corepack.

3. **Run the check.** From this repository against DeckStreak's tree:

   ```
   phxd pack probe --pack stack-selection --root PATH --format json
   ```

   Or vendor the check into DeckStreak: copy `scripts/stack-probe.py`, and a copy of `radar.json`
   named by `stack.json`'s `"radar"`. Then run each class in its own CI:
   `python3 scripts/stack-probe.py --root . check <class>`.

4. **Read what refuses.** A dependency nobody declared. A held item. A deviation without an ADR. A
   major off its pin: SvelteKit 3, TypeScript 7 or pnpm 12 before the owner adopts them. A Svelte
   package without runes mode. No `svelte-check` in CI. Corepack. A radar missing an approval.
   The radar's age only advises.

`python3 scripts/stack-probe.py --root PATH detect` prints what a tree uses, item by item, and is
the quickest way to write a first `stack.json`.

## References

Accessed 2026-09-27. The full study, with a scored table per category, is SPEC-V2-2198 §4.

- Svelte and SvelteKit: https://svelte.dev/blog/whats-new-in-svelte-september-2026 ·
  https://svelte.dev/blog/sveltekit-3-release-candidate · https://svelte.dev/docs/svelte/svelte-compiler ·
  https://svelte.dev/docs/ai · https://svelte.dev/docs/cli/paraglide
- Astro: https://astro.build/blog/astro-7/ · https://docs.astro.build/en/guides/integrations-guide/svelte/
- Rust backend: https://github.com/tokio-rs/axum/releases · https://github.com/tokio-rs/tokio/blob/master/README.md ·
  https://github.com/launchbadge/sqlx/blob/main/CHANGELOG.md · https://github.com/tokio-rs/tracing/releases
- Tauri and Capacitor: https://v2.tauri.app/blog/tauri-2-0-0-release-candidate/ · https://v2.tauri.app/release/ ·
  https://ionic.io/blog/announcing-capacitor-8
- Toolchain: https://pnpm.io/blog/releases/11.0 · https://pnpm.io/blog/releases/12.0 ·
  https://nodejs.org/en/about/previous-releases · https://github.com/orgs/pnpm/discussions/9044 ·
  https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/
- Testing: https://playwright.dev/docs/ci · https://vitest.dev/blog/vitest-4-1.html · https://nexte.st/changelog/ ·
  https://insta.rs/changelog/
- Telegram: https://core.telegram.org/bots/webapps · https://core.telegram.org/bots/api-changelog ·
  https://github.com/teloxide/teloxide/releases · https://github.com/ayrat555/frankenstein/blob/master/CHANGELOG.md
- Data and ops: https://sqlite.org/whentouse.html · https://fly.io/blog/litestream-v050-is-here/ ·
  https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html ·
  https://caddyserver.com/docs/caddyfile/directives/try_files
- CJK and WebAssembly: https://developer.chrome.com/blog/css-i18n-features ·
  https://w3c.github.io/i18n-drafts/articles/typography/fontstyles.en.html ·
  https://web.dev/articles/webassembly-performance-patterns-for-web-apps
- Design tokens: https://www.designtokens.org/tr/2025.10/format/
- Context7 ids: `/websites/svelte_dev_svelte`, `/websites/svelte_dev_kit`, `/withastro/docs`,
  `/tokio-rs/axum`, `/websites/rs_tokio_tokio`, `/transact-rs/sqlx`, `/tauri-apps/tauri-docs`,
  `/websites/pnpm_io`, `/vitest-dev/vitest`, `/microsoft/playwright`, `/websites/nexte_st`,
  `/mitsuhiko/insta`, `/websites/shadcn-svelte`, `/websites/bits-ui`, `/tanstack/query`,
  `/ciscoheat/sveltekit-superforms`, `/websites/valibot_dev`, `/opral/paraglide-js`,
  `/techniq/layerchart`, `/telegram-mini-apps/tma.js`, `/ayrat555/frankenstein`,
  `/websites/rs_teloxide`, `/websites/litestream_io`, `/open-telemetry/opentelemetry-rust`
