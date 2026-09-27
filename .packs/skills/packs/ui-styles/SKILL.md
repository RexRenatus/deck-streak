---
requires_phxd_schema: phxd.pack.run.v1
tip_floor: e996e5b8
---

# packs/ui-styles

Builder **brief catalog** (SPEC-V2-1196 R33; SPEC-V2-1204), extended with the
**system layer** every style needs (SPEC-V2-2192). Closed taxonomy vocab plus
a nineteen-row system layer — **not a gate**. Twenty-six vocab tokens, first
`minimalism`, last `editorial`; forty-five rows total. Repo `skills/` is
authoritative, and the closed forbid set is `skills/deny/DENY.md`.

Which seats consume this pack is its catalog row's `consumes`, the one
record of that edge (ADR-V2-1990), so this body names none. The pack names
vocabulary and system structure; it decides nothing.

## Probe (headless `phxd`)

`--format json` is required.

```
phxd pack run --pack ui-styles --project ID --format json
```

Schema `phxd.pack.run.v1`. Every row reports `catalog`, so the pack
has no red state to report and no green one to earn. Every walk
therefore reads `verdict: void` with `examined: 0`,
`refuse_reason: declarative_walk` and exit 3. That is the expected
answer, and it is not a pass.

`phxd pack probe` is the WRONG verb for this pack: `run_probe` in
`phxd/src/pack_cli.rs` requires `BodyStatus::Seeded`, and every row here is a
bare-string `Probe::Reason` — never runnable (ADR-V2-1977/1989 D6) — so
`ChecksFile::body_status()` reads `Reserved1198` and `pack probe` refuses
`stub_probe` regardless of content. That is a property of the declarative
walk, not a defect this delivery introduces or could fix; use `phxd pack run`.

## Never a gate

`ui-styles` never refuses land — not on a red row, not on a project
opt-in, not on any policy member. `PackId::UiStyles` is absent from
every arm of `refuse_on_red` in
`crates/phxd/src/completion_graph.rs`, and SPEC-V2-1207 R9 keeps
that rule in `refuse_on_red` ALONE so no second copy can rot. This
file holds no copy of it either.

Each `checks.json` row is closed: `id`, `stage`, `severity=catalog`,
`reason=catalog-only`, `probe`. The brief-vocab sentence a builder
names is the `brief assert` column of the table below — prose
belongs in this file, and `reason` is the ONE spelling a row carries
(SPEC-V2-1618 §1). Naming a token never gates land.

## Taxonomy (26) — brief vocab

| id | brief assert |
|---|---|
| `minimalism` | Spare surfaces, few elements, high whitespace; the brief names restraint as the style. |
| `maximalism` | Dense ornament, layered type, high visual energy; the brief names abundance as the style. |
| `flat` | Two-dimensional planes with no fake depth; the brief names flat color and type over relief. |
| `material` | Elevation, ink, and motion as Material; the brief names sheet, shadow, and ripple. |
| `skeuomorphism` | Real-world material cues (leather, glass, metal); the brief names physical resemblance. |
| `brutalism` | Raw type, visible structure, anti-polish; the brief names exposed construction. |
| `neo-brutalism` | Thick borders, hard offset shadows, high contrast; the brief names the 2020s brutalist revival. |
| `glassmorphism` | Frosted translucency, blur, layered panes; the brief names glass over a vivid field. |
| `neumorphism` | Soft extruded surfaces and low-contrast relief; the brief names the surface as the control. |
| `claymorphism` | Plump three-dimensional clay forms, pastel, soft shadow; the brief names clay volume. |
| `swiss` | International Typographic Style: grid, sans, asymmetry; the brief names Swiss modern. |
| `bauhaus` | Geometric primaries and function-first composition; the brief names Bauhaus geometry. |
| `art-deco` | Stepped geometry, metallic accents, luxury; the brief names Deco ornament. |
| `memphis` | Eighties pattern clash, primary shapes, playful; the brief names Memphis Milano. |
| `organic` | Curves, nature palettes, irregular rhythm; the brief names living form over grid. |
| `retro` | Period revival (fifties–seventies cues) without parody; the brief names a dated era as style. |
| `vintage` | Aged paper, worn type, patina; the brief names wear as the surface. |
| `vaporwave` | Neon, chrome, eighties/nineties net nostalgia; the brief names vaporwave. |
| `cyberpunk` | High-contrast neon on dark, tech grit; the brief names cyberpunk. |
| `dark` | Dark canvas as the default surface; the brief names dark as the style, not a theme toggle. |
| `light` | Light canvas as the default surface; the brief names light as the style, not a theme toggle. |
| `illustration-led` | Drawn art carries the hierarchy; the brief names illustration as the lead surface. |
| `photographic` | Photography carries the hierarchy; the brief names photos as the lead surface. |
| `isometric` | Thirty-degree axonometric scenes; the brief names isometric projection. |
| `kinetic` | Motion and transition as the style; the brief names movement as the look. |
| `editorial` | Magazine layout, pull-quotes, typographic story; the brief names editorial as the last token. |

Order is closed (ADR-V2-1204 D3). A later vocab token is an ADR
amendment, not a brief invention. Naming a token never gates MERGE.

## System layer (19) — every style needs this, regardless of vocab

SPEC-V2-2192. However a builder names the style (Taxonomy, above), the
result is not "full best practice" unless it also carries a token system, a
contrast story, closed type and spacing scales, dark-mode pairing, named
component states, one consistent icon set, and a Telegram theme mapping.
These rows are catalog-only exactly like the taxonomy: naming the practice
never gates land, and this pack is never a gate (ADR-V2-1204 D3, extended by
ADR-V2-2192).

Primary sources, with URLs and access dates, are in
`docs/specs/SPEC-V2-2192-*.md`'s `## References`. This section states only
the closed row set; it does not re-derive the research.

**Wave ownership.** Four pack deliveries split the UI-practice space this
wave so no two packs assert the same practice: `web-launch` (d2189) owns
document-level accessibility; `vibecode-polish` (d2190) owns keyboard
focus-visible behavior, `prefers-reduced-motion`, UI states as BEHAVIOR
(loading/empty/error/offline), dark-mode support/toggle behavior, and
Telegram-native runtime behavior (the `theme_changed` event, `BackButton`,
`MainButton`, haptics, safe areas); `ux-laws` (d2191) owns target size,
div-as-button, and the heuristic/ethics rubric. This pack owns the STRUCTURAL,
token-level half of contrast, dark mode, and component states — not their
runtime behavior.

### Design tokens (6) — stage `tokens`

W3C Design Tokens Community Group (DTCG) format.

| id | brief assert |
|---|---|
| `token-dollar-value` | Every design token in the tree is `{ "$value": ... }`, DTCG's required key, not a bare literal. |
| `token-dollar-type` | Every token names its `$type` (color, dimension, fontFamily, fontWeight, duration, ...) from the DTCG type list, so a consuming tool resolves it without guessing. |
| `token-dollar-description` | A token group, or a token that is not self-explanatory, carries `$description`, DTCG's documentation key. |
| `token-alias-reference` | A token that repeats another's value instead aliases it with a `{group.token}` reference, DTCG's alias syntax, so the source of truth stays singular. |
| `token-tiered-groups` | Tokens are organized in tiers — reference/raw values, then system/semantic roles, then component-specific tokens — the pattern Material Design 3 calls ref/sys/comp and Style Dictionary calls tiers, so a rename touches one layer. |
| `token-theming-extends` | A themed variant (dark, brand, density) is expressed with DTCG's `$extends`/group composition rather than a hand-copied duplicate tree. |

### Colour contrast (4) — stage `contrast`

WCAG 2.2, with APCA noted as emerging.

| id | brief assert |
|---|---|
| `contrast-text-normal` | Normal-size body text meets WCAG 2.2 SC 1.4.3: at least 4.5:1 contrast against its background. |
| `contrast-text-large` | Large-scale text (18pt, or 14pt bold — WCAG's large-text threshold) meets the relaxed SC 1.4.3 minimum of 3:1. |
| `contrast-non-text` | Icons, borders, and UI-component boundaries that carry meaning meet WCAG 2.2 SC 1.4.11 Non-text Contrast: at least 3:1 against adjacent colors. |
| `contrast-apca-emerging` | Where a token's contrast is computed, APCA (Advanced Perceptual Contrast Algorithm) is noted as the emerging, non-normative successor metric. WCAG 2.2 still specifies the WCAG 2.x ratio; APCA is a candidate direction for WCAG 3, not a shipped criterion of any published WCAG version. |

### Type scale (2) — stage `type-scale`

| id | brief assert |
|---|---|
| `type-scale-named-roles` | The type scale names roles (display, headline, title, body, label — Material 3's naming, or an equivalent closed set) rather than bare pixel sizes, so a reviewer can ask "is this a body or a label" instead of "is this 14px or 15px." |
| `type-scale-relative-units` | Type sizes are expressed in relative units (rem/em, or a platform's Dynamic-Type-equivalent scale) rather than hard-coded pixels, so a user's or the OS's text-size preference is honored. |

### Spacing scale (1) — stage `spacing-scale`

| id | brief assert |
|---|---|
| `spacing-scale-closed-steps` | Spacing is drawn from a closed numeric step scale (for example a 4px or 8px base grid) rather than arbitrary one-off values, so two components at the "same" gap actually match. |

### Dark-mode token pairs (2) — stage `dark-mode`

Structural pairing only. Whether/how the app SWITCHES between the pairs
at runtime is `vibecode-polish` (d2190).

| id | brief assert |
|---|---|
| `dark-mode-token-pairs` | Every color token that appears in a light surface has a paired dark-mode `$value`, so the pair is a token-authoring fact, not a runtime guess. |
| `dark-mode-pairing-mechanism` | The pairing mechanism itself is named and closed — a DTCG-composed theme group, a `prefers-color-scheme` media-query pair, or an explicit `--tg-theme-*` mapping — rather than left to be invented per component. |

### Component states (2) — stage `states`

Token-level presence only. Runtime keyboard focus-visible behavior is
`vibecode-polish` (d2190).

| id | brief assert |
|---|---|
| `states-interactive-set` | Every interactive component's token set names the closed state list — hover, focus, active, disabled, error — as tokens (color, opacity, or elevation deltas), not left to be improvised per instance. |
| `states-focus-contrast` | The focus state's token satisfies the same non-text contrast minimum as `contrast-non-text` (WCAG 2.2 SC 1.4.11) against its background, so a focus ring drawn from the token set is never accidentally invisible. |

### Icons (1) — stage `icons`

| id | brief assert |
|---|---|
| `icons-consistent-set` | Icons are drawn from one consistent family and one consistent stroke weight and grid (the way Material Symbols or Apple's SF Symbols are one closed set), rather than mixed from unrelated icon packs. |

### Telegram Mini App theming (1) — stage `telegram-theme`

Mapping only. Reacting to the `theme_changed` event at runtime, and the
rest of Telegram-native behavior, is `vibecode-polish` (d2190).

| id | brief assert |
|---|---|
| `telegram-theme-params-mapped` | The token set maps its semantic roles onto the Telegram Mini Apps `themeParams` fields and the `--tg-theme-*` CSS custom properties the client injects (`bindCssVars()`), so the app matches the host client's own theme rather than fighting it. |

Order is closed within each stage exactly as the taxonomy's is (ADR-V2-1204
D3, extended by ADR-V2-2192): a later row in any stage is an ADR amendment,
not a brief invention. Naming a system-layer practice never gates MERGE.
