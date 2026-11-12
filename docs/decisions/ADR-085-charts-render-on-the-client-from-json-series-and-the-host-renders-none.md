---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Charts are drawn on the client from series the server shapes, on the predecessor's dark panel, and the host renders none

## Context and Problem Statement

The predecessor draws its charts as images on its host (`charts.py`), sends three of them with
`/chart` (`bot.py:CommandBot._send_charts`), and lets its agent read thirteen as image resources
(`server.py:_CHART_RESOURCES`), on a dedicated one-worker render rail
(`offload.py:run_render_offloaded`). All use one dark theme: gold and cream on a navy panel, with
the seven Okabe-Ito colours for a series per course. One of them, the collection atlas, paints a
pixel for every card of the collection and is never published.

DeckStreak's Mini App can draw charts itself: ADR-005 admits Chart.js 4, lazy-loaded, for charts
rendered on the client. The owner's rule is that every number comes from a golden of the
predecessor's own function (ADR-012). The Mini App's design tokens all resolve to Telegram theme
variables (SPEC-028 R5), so they follow the chat's light or dark palette. Where is each chart's data
shaped, where is it drawn, which colours does it use in each Telegram theme, and what becomes of the
agent's image resources and the atlas?

## Decision Drivers

- The host renders nothing it does not have to: its budget is the service's (ADR-032).
- Every number of a chart, its windows, its grid, its colours and its labels, is proved by a golden
  in one language.
- The charts keep the predecessor's look and stay readable: a chart's marks need 3:1 contrast
  against their background (WCAG 2.2 success criterion 1.4.11), and a canvas needs a text
  alternative.
- The library loads only on a screen that draws a chart.
- No dependency the stack has not admitted.

## Considered Options (the alternatives it was chosen against)

- The server shapes each chart's data from the goldens, and the Mini App draws it with the lazy-loaded Chart.js on the predecessor's dark panel — chosen: the host renders nothing, every number is proved once in Rust, and the series keep their colours and their contrast in both Telegram themes.
- Render images on the host, as the predecessor does — rejected because an image per view costs the host memory and work that its budget keeps for the service (ADR-032).
- Render SVG on the host in Rust — rejected because it is still server work for every view, and a picture gives the owner no interaction.
- Shape the series in TypeScript on the client — rejected because the rules would live in a second language, apart from the Rust tests that read the goldens, and the client would fetch raw rows instead of one small payload.
- Draw the charts in the chat's own theme — rejected because on Telegram's light palette the gold, cream, yellow and sky-blue series fall below the 3:1 contrast a chart's marks need.
- Put the chart palette in the design tokens — rejected because SPEC-028 R5 makes every design token resolve to a Telegram theme variable, and the chart keeps its own panel in both themes.
- A chart plugin for the calendar heatmaps — rejected because a CSS grid of cells paints the predecessor's calendar with no second dependency.
- Import `chart.js/auto`, or import the library statically in each screen — rejected because the first ships every chart type and the second puts the library in the main bundle, for screens that draw no chart.
- Port the collection atlas now — rejected because it is an image of every card rendered on the host, and whether it returns in another form is the owner's decision (#270).
- Serve the agent's chart resources as images from the host — rejected because the MCP server can serve the agent the same series as data, with no renderer (#157).

## Decision Outcome

Chosen option: "the server shapes each chart's data from the goldens, and the Mini App draws it with
the lazy-loaded Chart.js on the predecessor's dark panel", because it keeps every number proved in
one place, keeps the host free of rendering, and keeps the charts both faithful and readable.

- **Shaping.** `deck-streak-insights` holds each chart's shaping as pure functions: the calendar's
  grid and each cell's colour, the cumulative lines, the race, the ladder, the weekly stack and the
  scatter, each proved against the golden of the predecessor function that drew it. Coordination
  reads each chart's owner through its port, and the API serves one chart's data by name to the
  owner only.
- **Drawing.** `chart.js` enters the Mini App's runtime dependencies. One loader imports it
  dynamically when the first chart mounts, and registers only the controllers, elements, scales and
  plugins the charts use. The calendars are a CSS grid painted from the payload, with no plugin.
  Every chart is an image with a label that names it, its window and its latest value, beside a
  table of its values, and no chart animates when reduced motion is requested.
- **Colours.** A chart draws on the predecessor's dark panel in both Telegram themes, a figure
  framed by the page's own tokens. Its palette is a module held equal to the golden, apart from the
  design tokens. On that panel every series colour and the chart's text reach 3:1.
- **What is not ported.** Nothing renders on the host, so the predecessor's render rail is not
  ported. The atlas is not ported (#270). The agent reads the same series when the MCP server
  arrives (#157), and the public publish job keeps the only server rendering (#156). `/chart`
  answers with a button that opens the charts screen.

### Consequences

- Good, because the host renders nothing, and a chart's weight falls on the phone that draws it.
- Good, because every number of every chart is proved against its golden in one language.
- Good, because the series keep the predecessor's colours with enough contrast in either Telegram
  theme.
- Bad, because a dark chart panel sits inside Telegram's light theme, which reads as a figure rather
  than as part of the page.
- Bad, because every chart needs a payload shape of its own, and an owner's change of its series
  changes the chart's read model with it.

### Confirmation

SPEC-085's acceptance tests: the palette against the golden in Rust and in the Mini App, the 3:1
contrast on the panel, every chart's shaping against its golden, the census that no route answers an
image and no crate renders one, the census that the atlas is absent, the loader's dynamic import and
its registrations, and each chart's label and table; the accessibility and stack-selection packs in
the box run (ADR-069).

## What would make this wrong

- The owner asks for charts in the chat's own colours: the palette then needs light and dark
  variants, each proved for contrast, which the predecessor's palette does not provide.
- A chart needs more points than a phone draws smoothly: its series would then be thinned on the
  server, with the thinning proved like any other shaping.
- The agent needs pictures rather than series: the MCP server would then render them, off the
  service's own path.

## More Information

SPEC-085; ADR-005 (Chart.js admitted, lazy-loaded); ADR-028 (the design tokens); SPEC-028 R5;
ADR-032 (the host budget); the predecessor's `charts.py`, `server.py:_CHART_RESOURCES` and
`bot.py:CommandBot._send_charts` at `27ee2bc`; #152, #156, #157, #270.
