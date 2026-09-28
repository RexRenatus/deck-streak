# Schematic: charts, from each owner's series to the client's drawing

Kind: data flow. Read at DeckStreak `dev` c3d769b and at the predecessor's `27ee2bc` (`charts.py`,
`server.py:_CHART_RESOURCES`, `bot.py:CommandBot._send_charts`, `offload.py:run_render_offloaded`).
Added by SPEC-085 (ADR-085). It extends `docs/schematics/data-flow.md`, whose domain contexts are
the owners below: the charts add a read path from them to the Mini App, and no path to the host's
disk or to an image.

## The path

```mermaid
flowchart LR
  analytics["analytics: rollups by study day"] --> rm["coordination: one read model per chart"]
  progression["progression: XP per study day, both XP tables"] --> rm
  habits["habits: reading and writing series"] --> rm
  focus["focus: completed minutes by study day"] --> rm
  curriculum["curriculum: bands by course"] --> rm
  quests["quests: the race of the week"] --> rm
  rm --> shape["insights: grids, lines, race, ladder, stack, scatter and the palette, each proved by its golden"]
  shape --> api["API: one chart by name, to the owner only"]
  api -->|JSON| app["Mini App: the chart component"]
  app --> lazy["chart.js, imported on the first chart only"]
  app --> grid["calendars painted as a CSS grid, no plugin"]
  bot["bot: the chart command"] -->|a button that opens the charts screen| app
  agent["the agent, when the MCP server arrives"] -.->|the same series, no image| api
```

The host renders nothing: no route answers an image, and no crate depends on a plotting or image
library. The predecessor's render rail has no work and is not ported.

## The charts

| chart | the owner's series | drawn as | shown on |
|---|---|---|---|
| review heatmap | reviews per study day (analytics) | calendar grid, gold ramp | `/charts` |
| streak calendar | a study day or not (analytics) | calendar grid, gold ramp | `/charts` |
| XP over time | XP per study day, cumulative (progression) | line with fill | `/charts` |
| reading heatmap | reading minutes per day (habits) | calendar grid, ember ramp | `/habits` |
| writing heatmap | writing languages confirmed per day (habits) | calendar grid, jade ramp | `/habits` |
| weekly reading | minutes per course per week, and the goal (habits) | stacked bars and a goal line | `/habits` |
| reading and retention | weekly minutes against the next week's retention (habits) | scatter by course | `/habits` |
| focus heatmap | completed focus minutes per day (focus) | calendar grid, violet ramp | `/focus` |
| focus over time | completed focus minutes, cumulative (focus) | line with fill | `/focus` |
| Road to C2 | bands and mastery per course (curriculum) | ladder cells and mastery bars | `/progress` |
| race | the owner's week against the ghost's (quests) | two lines | `/quests` |

Not drawn here: the collection atlas (not ported, the owner's decision), the Oracle calibration
(with the prediction markets), the leech breakdown (with leech remediation) and the badge gallery
image (the badges screen lists badges instead).

## A calendar

```mermaid
flowchart TD
  days["values by study day, and today"] --> start["the window's first day, moved back to its Monday"]
  start --> cols["one column a week, rows Monday to Sunday, cells through today"]
  cols --> range["each cell's place in the grid's range of values"]
  range --> ramp["the colour ramp's table gives each cell its colour"]
  ramp --> payload["grid, cells, colours and the table of values, in one payload"]
  payload --> paint["the client paints the cells and offers the table"]
```

## Colours

Every chart draws on the predecessor's dark panel, in both Telegram colour schemes, as a figure the
page's own tokens frame. The chart palette is held equal to the golden, apart from the design
tokens, which all resolve to Telegram theme variables (SPEC-028 R5). On the panel, every series
colour and the chart's text reach 3:1, the contrast WCAG 2.2 asks of a chart's marks.
