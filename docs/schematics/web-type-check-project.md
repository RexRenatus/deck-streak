# The web type-check project, and the census that holds it

Kind: component and data flow. Read at `877b3dfbebd32768f61b817a36e70d519c81d432`. The project of
SPEC-628 (ADR-650): what the web app's type check reads, which jobs run the check and the browser
suites, and what the census reads and names.

## Component and data flow

```mermaid
flowchart LR
  subgraph pkg["web/app/package.json"]
    chk["check :12<br/>svelte-check --tsconfig"]
    pwrun["test:engine :16<br/>playwright test"]
  end
  subgraph proj["web/app/tsconfig.json include"]
    appsrc["the app's sources and configurations<br/>src, tests, vite.config.ts,<br/>sveltekit.config.js, playwright.config.ts"]
    cardsuite["the card suite and its two configurations<br/>SPEC-341"]
    studysuite["the study suite and its two configurations<br/>SPEC-350"]
    enginesuite["the engine suite, its harness page<br/>and its two configurations<br/>SPEC-628"]
  end
  subgraph cen["web/app/src/lib/type-check-project.test.ts"]
    incl["the include list<br/>read by TypeScript's parser"]
    listing["the top level of web/app<br/>examined n top-level members"]
    censustest["A1 and A2"]
    named["each omitted member, sorted"]
  end
  subgraph jobs["CI"]
    webjob["web job<br/>the check, then Vitest"]
    enginejob["web-engine job<br/>Playwright strips types unchecked"]
  end
  webjob --> chk
  chk --> appsrc
  chk --> cardsuite
  chk --> studysuite
  chk --> enginesuite
  webjob --> censustest
  incl --> censustest
  listing --> censustest
  censustest --> named
  enginejob --> pwrun
```

## What each part holds

- The check reads only the files the `include` list names, so a browser suite the list leaves out
  is never type-checked: Playwright runs it with its types stripped.
- The census selects each top-level directory named `src`, `tests`, `tests-<name>` or
  `<name>-harness`, and each top-level `.ts` or `.js` file that is no declaration file (SPEC-628
  R3), and names, sorted, each one the list does not name. A1 reads it over the real listing, A2
  over planted ones.
- `web/app/scripts` stays outside both: its scripts run in Node beside the app.
