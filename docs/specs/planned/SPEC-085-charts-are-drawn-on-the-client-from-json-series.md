# SPEC-085: charts are drawn on the client from JSON series, and the host renders none

- **Wave:** W3. **Issue:** #152 (epic #4). **Context(s):** `deck-streak-insights` (the shaping of
  every chart's data and the chart palette); `deck-streak-coordination` (one read model per chart
  over its owner's port); `deck-streak-api` (the chart route); `deck-streak-bot` (`/chart`); the
  Mini App (`web/app`: the lazy chart component, the calendar grid, the ladder, the charts screen
  and the owners' screens).
- **Decided by:** ADR-005 (the Mini App's stack admits Chart.js 4, lazy-loaded), ADR-012 (the parity
  oracle proves the numbers), ADR-028 (the design tokens and the route table), ADR-085 (the server
  shapes each chart, the client draws it on the predecessor's dark panel, and the host renders none)
  and ADR-087 (the courses the series are named by).
- **Prerequisites:** SPEC-071 (the rollups: reviews and study days), SPEC-072 (XP per study day),
  SPEC-077 (the bands), SPEC-078 (the reading and writing series), SPEC-079 (the focus days),
  SPEC-080 (the race's week), SPEC-028 (the Mini App shell) and SPEC-026 (the bot's command table).
  **Mutation band:** `S08500-S08599`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-085.md` (ADR-016).

## 1. The problem, measured

- **Nothing draws a chart.** `crates/insights/src/` holds only `lib.rs`, `web/app/package.json` has
  no chart library, and `stack.json` lists `chart-js` as planned (read at `dev` c3d769b). ADR-005
  already admits Chart.js 4, lazy-loaded, for charts rendered on the client.
- **What is ported.** The predecessor's `charts-and-resources` feature: `charts.py` draws each of
  its charts as an image, `/chart` sends three of them (`bot.py:CommandBot._send_charts`: Road to
  C2, the review heatmap and the reading heatmap), and the agent reads thirteen as image resources
  (`server.py:_CHART_RESOURCES`), rendered on a dedicated one-worker rail
  (`offload.py:run_render_offloaded`). All share one dark theme, gold on a navy panel, with the
  Okabe-Ito colours for series by course.
- **What each chart plots, read at `27ee2bc`.** Five calendars share `charts.py:_heatmap`: a grid
  from the Monday on or before the window's first day through today, rows Monday to Sunday, one
  column a week, each cell coloured by the day's value through a colour ramp. The calendars are the
  review heatmap (reviews per day), the streak calendar (a study day or not), the reading heatmap
  (reading minutes), the writing heatmap (writing languages confirmed) and the focus heatmap
  (completed focus minutes). Two lines are cumulative (`charts.py:xp_over_time`,
  `charts.py:focus_over_time`); the race draws the owner's running total against the ghost's
  (`charts.py:ghost_race_chart`); the ladder colours each band cell by achieved, in progress or not
  started (`charts.py:combined_road_to_c2_bars`), beside mastery bars
  (`charts.py:road_to_c2_chart`); the weekly reading chart stacks courses against a goal line
  (`charts.py:reading_weekly_trend`); and the scatter plots weekly reading against the next week's
  retention (`charts.py:reading_vs_retention`).
- **Corrections to the issue, from the decision.** #152's render rail criterion has no subject:
  nothing renders on the host (ADR-085), so this SPEC replaces it with a census that no route
  answers an image and no crate depends on a plotting or image library. Its atlas criterion is kept
  in the stronger form that the atlas is not ported at all (#270). Its palette criterion is kept,
  against the golden, in a chart palette held apart from the design tokens: SPEC-028 R5 makes every
  design token resolve to a Telegram theme variable, and the chart keeps the predecessor's own panel
  in both themes.
- **Contrast, measured for ADR-085.** On the predecessor's panel every series colour reaches 3:1,
  the non-text contrast WCAG 2.2 asks of a chart's marks; on a white page the gold, cream, yellow
  and sky-blue series do not. The palette test re-measures it from the golden (A3).
- **What the parity oracle proves.** Eight goldens (§7): the palette, the windows, the calendars'
  grids and each cell's colour as the predecessor's image gives it, the cumulative lines, the race,
  the ladder, the weekly stack and the scatter, over seeded synthetic series.
- **Prerequisites.** Each owner's series comes from the SPEC named in the header; this SPEC shapes
  and draws them, and holds none of their rules.

## 2. Requirements

R1. The Mini App draws every chart on the client, and the host renders none: no API route answers an
    image, and no crate depends on a plotting, image or raster library. The predecessor's render
    rail has nothing to carry and is not ported.
R2. `chart.js` (the stack's pick, admitted by ADR-005) enters `web/app/package.json`'s
    `dependencies`, so the web audit examines it, and `stack.json` marks `chart-js` in use. One
    module, `web/app/src/lib/charts/load.ts`, imports it, only dynamically and only when the first
    chart mounts, and registers exactly the controllers, elements, scales and plugins the charts
    use, never `chart.js/auto`, so the library ships in a chunk that only a chart screen loads.
R3. `deck-streak-insights` shapes every chart's data as pure functions over the kernel's types, each
    equal to its golden: the calendar grid and each cell's colour (`goldens/charts_calendar.json`,
    `charts.py:_heatmap` through the five calendars), the cumulative lines
    (`goldens/charts_cumulative.json`), the race's two lines (`goldens/charts_race.json`), the
    ladder's cells, labels and bars (`goldens/charts_ladder.json`), the weekly stack and its goal
    line (`goldens/charts_reading_trend.json`), and the scatter's points
    (`goldens/charts_reading_scatter.json`). A cell's colour is the one the predecessor's image
    gives it, through the ramp's table and the grid's own range; the golden's cases include a grid
    whose values are all equal.
R4. Every window equals the predecessor's (`goldens/charts_windows.json`: the default weeks of each
    windowed chart), and the palette equals `goldens/charts.constants.json`: the canvas, panel,
    golds, cream, grid, muted and ember colours, the focus violet, the ghost blue, the seven
    Okabe-Ito series colours in order, and the band order.
R5. Coordination serves each chart's data through one read model per chart
    (`crates/coordination/src/charts/`), reading its owner's port and holding none of its rules:
    analytics' rollups (SPEC-071, the rows `GET /api/analytics/days` serves) for the review heatmap
    and the streak calendar; progression's XP per study day across both XP tables (SPEC-072) for XP
    over time; habits' series (SPEC-078) for the reading and writing heatmaps, the weekly stack and
    the scatter; focus's daily minutes (SPEC-079) for the focus heatmap and line; curriculum's bands
    (SPEC-077) for the ladder and the bars; and quests' week (SPEC-080) for the race.
R6. `GET /api/charts/{name}` serves one chart's data, for the closed set of names above, to the
    owner's session only. An unknown name is answered 404, and any other caller 401 or 403 with no
    data.
R7. A chart draws on the predecessor's dark panel in both Telegram colour schemes, framed by the
    page's own tokens. The chart palette lives in `web/app/src/lib/charts/palette.ts`, not in the
    design tokens (SPEC-028 R5), and a test holds it equal to the golden. Every series colour and
    the chart's text reach 3:1 against the panel (WCAG 2.2 success criterion 1.4.11).
R8. Every chart is an image with a label that names the chart, its window and its latest value,
    beside a table of the same values the owner can open. A calendar is painted from the payload as
    a CSS grid of cells, with no chart plugin. With reduced motion requested, no chart animates.
R9. The `/charts` screen shows the review heatmap, the streak calendar and XP over time, and joins
    the route table. The owners' screens gain their charts beside the data they already show:
    `/progress` the ladder and the mastery bars, `/habits` the reading and writing heatmaps, the
    weekly stack and the scatter, `/focus` the focus heatmap and line, and `/quests` the race.
R10. The bot keeps `/chart`: it answers with one line and a button that opens `/charts`, and sends
    no image.
R11. The collection atlas is not ported (#270): no source, route or series names it. The agent's
    chart resources become these same series when the MCP server arrives (#157), and server
    rendering stays only in the public publish job (#156).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the chart palette in `insights` equals the constants golden | `the_chart_palette_equals_the_predecessors` |
| A2 | the Mini App's chart palette equals the constants golden | `the chart palette equals the predecessor's` |
| A3 | every series colour and the chart's text reach 3:1 against the chart panel | `every series colour reaches three to one on the chart panel` |
| A4 | the five calendars' grids and every cell's colour equal the golden of `charts.py:_heatmap` | `the_calendars_match_the_predecessors_golden` |
| A5 | every chart window equals the predecessor's default | `the_chart_windows_equal_the_predecessors` |
| A6 | the cumulative XP and focus lines equal their golden | `the_cumulative_lines_match_the_predecessors_golden` |
| A7 | the race's two lines equal the golden of `charts.py:ghost_race_chart` | `the_race_lines_match_the_predecessors_golden` |
| A8 | the ladder's cells, labels and bars equal their golden | `the_ladder_matches_the_predecessors_golden` |
| A9 | the weekly stack and its goal line equal the golden of `charts.py:reading_weekly_trend` | `the_reading_stack_matches_the_predecessors_golden` |
| A10 | the scatter's points and colours equal the golden of `charts.py:reading_vs_retention` | `the_reading_scatter_matches_the_predecessors_golden` |
| A11 | the chart route answers the owner's session only, and an unknown name is 404 | `the_chart_route_answers_only_the_owner` |
| A12 | each chart's read model reads its owner's port and returns the shaped series | `each_chart_reads_its_owners_series` |
| A13 | no API route answers an image and no crate depends on a plotting or image library; a planted route and a planted dependency are refused | `test_no_route_or_crate_renders_an_image` |
| A14 | no source, route or series names the collection atlas; a planted mention is refused | `test_the_collection_atlas_is_not_ported` |
| A15 | only the loader imports `chart.js`, and only dynamically; a planted static import is refused | `only the loader imports chart.js, and only on the first chart` |
| A16 | the loader registers exactly the components the charts use, and never `chart.js/auto` | `registers only the components the charts use` |
| A17 | every chart is an image with a label and a table of its values | `every chart carries a label and a table of its values` |
| A18 | with reduced motion requested, a chart does not animate | `a chart does not animate when reduced motion is requested` |
| A19 | a calendar paints each day in its week's column and its weekday's row, in the payload's colour | `paints each day in its week column and weekday row` |
| A20 | `/chart` answers with a button that opens the charts screen, and sends no image | `chart_opens_the_charts_screen_and_sends_no_image` |
| A21 | the charts screen shows the review heatmap, the streak calendar and XP over time | `the charts screen shows the three activity charts` |

```acceptance
A1: cargo test -p deck-streak-insights --test charts_palette -- --exact the_chart_palette_equals_the_predecessors
A2: pnpm exec vitest run web/app/src/lib/charts/palette.test.ts -t "the chart palette equals the predecessor's"
A3: pnpm exec vitest run web/app/src/lib/charts/palette.test.ts -t "every series colour reaches three to one on the chart panel"
A4: cargo test -p deck-streak-insights --test charts_calendar -- --exact the_calendars_match_the_predecessors_golden
A5: cargo test -p deck-streak-insights --test charts_calendar -- --exact the_chart_windows_equal_the_predecessors
A6: cargo test -p deck-streak-insights --test charts_lines -- --exact the_cumulative_lines_match_the_predecessors_golden
A7: cargo test -p deck-streak-insights --test charts_lines -- --exact the_race_lines_match_the_predecessors_golden
A8: cargo test -p deck-streak-insights --test charts_ladder -- --exact the_ladder_matches_the_predecessors_golden
A9: cargo test -p deck-streak-insights --test charts_reading -- --exact the_reading_stack_matches_the_predecessors_golden
A10: cargo test -p deck-streak-insights --test charts_reading -- --exact the_reading_scatter_matches_the_predecessors_golden
A11: cargo test -p deck-streak-api --test charts_routes -- --exact the_chart_route_answers_only_the_owner
A12: cargo test -p deck-streak-coordination --test charts_read_models -- --exact each_chart_reads_its_owners_series
A13: python3 -m unittest discover -s scripts/tests -p test_charts_no_host_rendering.py -k test_no_route_or_crate_renders_an_image
A14: python3 -m unittest discover -s scripts/tests -p test_charts_no_host_rendering.py -k test_the_collection_atlas_is_not_ported
A15: pnpm exec vitest run web/app/src/lib/charts/load.test.ts -t "only the loader imports chart.js, and only on the first chart"
A16: pnpm exec vitest run web/app/src/lib/charts/load.test.ts -t "registers only the components the charts use"
A17: pnpm exec vitest run web/app/src/lib/charts/Chart.test.ts -t "every chart carries a label and a table of its values"
A18: pnpm exec vitest run web/app/src/lib/charts/Chart.test.ts -t "a chart does not animate when reduced motion is requested"
A19: pnpm exec vitest run web/app/src/lib/charts/CalendarHeatmap.test.ts -t "paints each day in its week column and weekday row"
A20: cargo test -p deck-streak-bot --test charts_commands -- --exact chart_opens_the_charts_screen_and_sends_no_image
A21: pnpm exec vitest run web/app/src/routes/charts/charts.test.ts -t "the charts screen shows the three activity charts"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The accessibility, stack-selection and
telegram-platform packs stay enforced; no row of theirs is deferred for this delivery, and none
changes state when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `web/app/src/routes/charts/`, `web/app/src/lib/charts/` and the four owners' screens this SPEC changes, examining every chart component, the accessibility checks pass, each canvas carrying its text alternative | the accessibility pack |
| B2 | over `stack.json`, `web/app/package.json` and `pnpm-lock.yaml`, examining every declared element, `chart-js` is in use at the stack's pinned major and no held item enters | the stack-selection pack |
| B3 | over `crates/bot/src/charts_commands.rs` and `crates/bot/src/commands.rs`, examining the `/chart` command, the menu is the owner's and the reply opens the Mini App | the telegram-platform pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/insights/src/lib.rs` | `deck-streak-insights` | changed: the modules below |
| `crates/insights/src/charts.rs` | `deck-streak-insights` | added: the calendar grid, the cumulative lines, the race, the ladder, the weekly stack and the scatter |
| `crates/insights/src/palette.rs` | `deck-streak-insights` | added: the chart palette and the calendar ramps |
| `crates/insights/Cargo.toml` | `deck-streak-insights` | changed: dev-dependencies `serde` and `serde_json` (the golden reader, SPEC-029 R8) |
| `crates/insights/tests/charts_palette.rs` | `deck-streak-insights` | added: A1 |
| `crates/insights/tests/charts_calendar.rs` | `deck-streak-insights` | added: A4, A5 |
| `crates/insights/tests/charts_lines.rs` | `deck-streak-insights` | added: A6, A7 |
| `crates/insights/tests/charts_ladder.rs` | `deck-streak-insights` | added: A8 |
| `crates/insights/tests/charts_reading.rs` | `deck-streak-insights` | added: A9, A10 |
| `crates/coordination/src/charts/mod.rs` | `deck-streak-coordination` | added: one read model per chart |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the charts module |
| `crates/coordination/tests/charts_read_models.rs` | `deck-streak-coordination` | added: A12 |
| `crates/api/src/charts_routes.rs` | `deck-streak-api` | added: `GET /api/charts/{name}` |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the chart route mounted |
| `crates/api/tests/charts_routes.rs` | `deck-streak-api` | added: A11 |
| `crates/bot/src/charts_commands.rs` | `deck-streak-bot` | added: the chart command |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table and the owner's menu |
| `crates/bot/tests/charts_commands.rs` | `deck-streak-bot` | added: A20 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the chart read models joined to their owners' ports |
| `web/app/package.json` | miniapp | changed: `chart.js` in `dependencies` |
| `pnpm-lock.yaml` | repo | changed |
| `stack.json` | repo | changed: `chart-js` in use |
| `web/app/src/lib/charts/load.ts` | miniapp | added: the one dynamic import of `chart.js` and its registration |
| `web/app/src/lib/charts/load.test.ts` | miniapp | added: A15, A16 |
| `web/app/src/lib/charts/palette.ts` | miniapp | added: the chart palette |
| `web/app/src/lib/charts/palette.test.ts` | miniapp | added: A2, A3 |
| `web/app/src/lib/charts/Chart.svelte` | miniapp | added: the chart, its label and its table of values |
| `web/app/src/lib/charts/Chart.test.ts` | miniapp | added: A17, A18 |
| `web/app/src/lib/charts/CalendarHeatmap.svelte` | miniapp | added: the calendar as a CSS grid |
| `web/app/src/lib/charts/CalendarHeatmap.test.ts` | miniapp | added: A19 |
| `web/app/src/lib/charts/Ladder.svelte` | miniapp | added: the band cells per course |
| `web/app/src/routes/charts/+page.svelte` | miniapp | added: the charts screen |
| `web/app/src/routes/charts/charts.test.ts` | miniapp | added: A21 |
| `web/app/src/routes/progress/+page.svelte` | miniapp | changed: the ladder and the mastery bars |
| `web/app/src/routes/habits/+page.svelte` | miniapp | changed: the reading and writing heatmaps, the weekly stack and the scatter |
| `web/app/src/routes/focus/+page.svelte` | miniapp | changed: the focus heatmap and line |
| `web/app/src/routes/quests/+page.svelte` | miniapp | changed: the race |
| `web/app/src/lib/routes.ts` | miniapp | changed: the charts screen joins the route table |
| `scripts/tests/test_charts_no_host_rendering.py` | repo | added: A13, A14 |
| `tools/parity-oracle/registry/spec_085.py` | repo | added: this SPEC's registrations (§7) |
| `tools/parity-oracle/goldens/charts.constants.json` | repo | added: the constants golden of `charts.py`'s palette and band order (constants) |
| `tools/parity-oracle/goldens/charts_windows.json` | repo | added: the golden of the windowed charts' default weeks (adapter: reads each function's signature) |
| `tools/parity-oracle/goldens/charts_calendar.json` | repo | added: the golden of `charts.py:_heatmap` through its five callers (adapter: records the grid and each cell's colour from the image) |
| `tools/parity-oracle/goldens/charts_cumulative.json` | repo | added: the golden of `charts.py:xp_over_time` and `charts.py:focus_over_time` (adapter: records the plotted line) |
| `tools/parity-oracle/goldens/charts_race.json` | repo | added: the golden of `charts.py:ghost_race_chart` (adapter: records both lines) |
| `tools/parity-oracle/goldens/charts_ladder.json` | repo | added: the golden of `charts.py:combined_road_to_c2_bars` and `charts.py:road_to_c2_chart` (adapter: records the cells, labels and bars) |
| `tools/parity-oracle/goldens/charts_reading_trend.json` | repo | added: the golden of `charts.py:reading_weekly_trend` (adapter: records the stack and the goal line) |
| `tools/parity-oracle/goldens/charts_reading_scatter.json` | repo | added: the golden of `charts.py:reading_vs_retention` (adapter: records the points and colours) |
| `scripts/mutation-rows.d/S08500-S08599.json` | repo | added: the hand-proved rows (§9) |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-085-charts-are-drawn-on-the-client-from-json-series.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-085-charts-render-on-the-client-from-json-series-and-the-host-renders-none.md` | docs | changed: accepted |
| `docs/red-first/SPEC-085.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It serves no chart to the agent; the MCP server serves these same series when it arrives (#157).
- It renders nothing on the host for the public page; the public publish job keeps its own rendering
  (#156).
- It does not port the collection atlas; the owner decides whether it returns in another form
  (#270).
- It draws no Oracle calibration chart; the prediction markets bring it (#119).
- It draws no leech breakdown; leech remediation brings it (#133).
- It draws no badge gallery image; the badges screen lists earned and locked badges (#74).
- It builds no chart settings screen (#57).

## 6. Risks

- **A cell's colour drifts from the predecessor's.** The ramp is a table of fixed length indexed by
  the value's place in the grid's range; an index off by one shifts every cell. Detected by A4,
  whose golden records each cell's colour, with cases at the range's ends and an all-equal grid.
- **A label rounds a tie the other way.** The ladder's percent labels are the predecessor's
  whole-number format, which rounds a half to even. Detected by A8's tie cases.
- **Chart.js lands in the main bundle.** A static import anywhere else pulls it in. Detected by
  A15's census, with a planted static import it must refuse.
- **A chart is unreadable to a screen reader.** A canvas has no content of its own. Detected by A17
  and the accessibility pack (B1).
- **An owner's series changes shape.** Each read model reads its owner's port, and A12 runs every
  chart over the owners' real ports with synthetic rows.
- **The dark panel reads as foreign in Telegram's light theme.** Recorded by ADR-085 as the price of
  the series' contrast; the page's own tokens still frame it.

## 7. Parity goldens

Every golden is registered in `tools/parity-oracle/registry/spec_085.py` and generated from the
predecessor at `27ee2bc` over seeded synthetic series. Each adapter replaces `charts.py:_render`
with one that returns no bytes and records only data, colours and in-chart value labels from the
chart's axes, never a title or an axis's tick labels, which can carry a date, so no golden carries a
date string (SPEC-029 R3).

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `charts.constants` | `charts.py`'s palette and band order | constants | nothing |
| `charts_windows` | the windowed charts' signatures in `charts.py` | adapter | reads each function's default `weeks` |
| `charts_calendar` | `charts.py:_heatmap`, through `review_heatmap`, `streak_calendar`, `reading_heatmap`, `writing_heatmap` and `focus_heatmap` | adapter | the per-day values keyed by dates from epoch days; records the grid the axes are given and the colour the image gives each cell |
| `charts_cumulative` | `charts.py:xp_over_time`, `charts.py:focus_over_time` | adapter | rollups and daily minutes from JSON; records the plotted line and its colour |
| `charts_race` | `charts.py:ghost_race_chart` | adapter | the week and today as dates; records both lines and their colours |
| `charts_ladder` | `charts.py:combined_road_to_c2_bars`, `charts.py:road_to_c2_chart` | adapter | synthetic courses and bands; records each cell's fill, edge and text colour, each label, and each bar |
| `charts_reading_trend` | `charts.py:reading_weekly_trend` | adapter | synthetic courses and weeks; records each stacked series, its colour and the goal line |
| `charts_reading_scatter` | `charts.py:reading_vs_retention` | adapter | synthetic courses and points; records each course's points and colour, and the empty state |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S08501-THE-CALENDAR-STARTS-ON-A-MONDAY` | `crates/insights/src/charts.rs` | the grid's first column starts on the Monday on or before the window's first day | `charts_calendar::the_calendars_match_the_predecessors_golden` |
| `S08502-THE-WINDOW-IN-WEEKS` | `crates/insights/src/charts.rs` | the calendars' and lines' default window | `charts_calendar::the_chart_windows_equal_the_predecessors` |
| `S08503-THE-RAMP-TABLE-LENGTH` | `crates/insights/src/palette.rs` | the ramp's table length, which every cell's colour index depends on | `charts_calendar::the_calendars_match_the_predecessors_golden` |
| `S08504-THE-PANEL-COLOUR` | `crates/insights/src/palette.rs` | the chart panel's colour | `charts_palette::the_chart_palette_equals_the_predecessors` |
| `S08505-THE-SERIES-ORDER` | `crates/insights/src/palette.rs` | the Okabe-Ito series colours in their order | `charts_palette::the_chart_palette_equals_the_predecessors` |
| `S08506-THE-RACE-STOPS-AT-TODAY` | `crates/insights/src/charts.rs` | the owner's line ends at today's weekday | `charts_lines::the_race_lines_match_the_predecessors_golden` |
| `S08507-AN-ACHIEVED-BAND-IS-GOLD` | `crates/insights/src/charts.rs` | an achieved band's cell takes the gold fill | `charts_ladder::the_ladder_matches_the_predecessors_golden` |
