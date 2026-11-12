# Each mastery figure, from its computation to its plain description

The data flow of SPEC-408 (ADR-422): each mastery figure the learner sees, from the function that computes it, through the store, the route and the view, to every surface that shows it, and the sentence that now stands beside it. Every `path:line` citation was read at commit `a68db18a`. Nodes marked NEW are what SPEC-408 adds; everything else exists at that commit.

## Data flow

```mermaid
flowchart LR
  subgraph curriculumCtx["curriculum: read, never changed"]
    cardMastery["progress.rs card_mastery :85-112<br/>0 to 1 per counted card"]
    courseProgress["progress.rs course_progress :141-236<br/>band and course mean, times 100"]
    pillar["law.rs mastery_pillar :10-19<br/>100 less 3 per active leech, floor 70"]
  end
  subgraph coordinationCtx["coordination"]
    progressView["progress_view StoredProgress<br/>mastery_pct per course"]
    lawBlockFn["law/mod.rs law_block :126<br/>shown when leeches and mastery are above 0 :79-86"]
  end
  subgraph apiCtx["api"]
    progressRoute["progress_routes.rs :84<br/>mastery_pct"]
    lawRoute["law_routes.rs :71-72<br/>mastery, mastery_pending"]
  end
  subgraph webCtx["web Mini App"]
    courseLadder["CourseLadder.svelte :17-38<br/>progress_summary, progress_band_mastery, progress_band_meter"]
    courseAbout["NEW: paragraph after each summary<br/>progress_mastery_about"]
    lawView["LawBlock.svelte :30, :48<br/>law_mastery"]
    lawAbout["NEW: text inside the mastery line<br/>law_mastery_about"]
  end
  subgraph botCtx["bot"]
    courseLine["progress_commands.rs course_line :59-70<br/>pct mastery per course"]
    botAbout["NEW: the reply's italic last line<br/>MASTERY_ABOUT"]
  end
  messages["web/app/messages<br/>en, es, fr, ja, ko, zh-Hans, zh-Hant<br/>both keys in every file"]
  leechPort["leech port, not wired<br/>issue 133"]

  cardMastery --> courseProgress
  courseProgress --> progressView
  progressView --> progressRoute
  progressRoute --> courseLadder
  courseLadder --> courseAbout
  progressView --> courseLine
  courseLine --> botAbout
  pillar --> lawBlockFn
  leechPort -.->|no count yet, so pending| lawBlockFn
  lawBlockFn --> lawRoute
  lawRoute --> lawView
  lawView --> lawAbout
  messages --> courseAbout
  messages --> lawAbout
  messages -.->|en.json text equals the constant| botAbout
```

## The surfaces that are not changed

```mermaid
flowchart LR
  scorePillar["analytics score.rs :167-176<br/>graduations against 3, less a leech penalty"]
  scoreRoute["api analytics_routes.rs :130"]
  scoreView["web ScoreBreakdown.svelte :10-16, :32<br/>pillar_mastery: a score from 0 to 100, not a percentage"]
  toolText["mcp tools.rs :130-134<br/>agent read tool text, golden-pinned, issue 157"]
  exportField["data_rights.rs<br/>mastery_pct in the data export"]
  scorePillar --> scoreRoute
  scoreRoute --> scoreView
```

The score pillar is a third measure, shown as a score rather than a percentage. ADR-422 D7 keeps it as it is. The agent read tool and the data export are read by machines, not shown to a reader (ADR-422 D3).

## Each figure, its sentence and what pins it

| figure | computed by | shown on | its sentence's key | locales | pinned by |
|---|---|---|---|---|---|
| course mastery, per course and per band | `card_mastery`, averaged by `course_progress` | Road to C2 (summary and band cells), the bot's progress reply | `progress_mastery_about`, and the bot's `MASTERY_ABOUT`, equal to its English text | all seven | A2 (Road to C2), A3 (every locale), A4 (the bot, its golden and the bot role) |
| law mastery | `mastery_pillar` | the law block's mastery line, when it is shown | `law_mastery_about` | all seven | A1 (the law block), A3 (every locale), A5 (its figures equal the pillar's) |

## Where each sentence sits

- **Road to C2:** `section[aria-labelledby=course-<code>]` holds `h3`, then `p[data-course-summary]`, then the NEW `p[data-mastery-about=course]`, then the band list. It appears once per course.
- **Law block:** `ul` holds `li[data-line=mastery]`, which holds the figure text and then the NEW `span[data-mastery-about=law]`. The pending list holds no sentence.
- **Bot:** the reply is "Road to C2" in bold, one line per course, then the NEW italic course sentence. The no-course and failed replies are unchanged.
