# The web literal-text census

SPEC-653 and ADR-675. One standard-library module, `scripts/tests/test_web_literal_text.py`,
reads every web component as text and reports each user-facing text written as a literal instead
of drawn from the message catalogs.

## Data flow

```mermaid
flowchart LR
    tree["web/app/src/**/*.svelte"] --> walk["components(root)"]
    walk --> count{"examined n component(s)"}
    count -- "n = 0" --> void["VOID: refused"]
    count -- "n > 0" --> read["component_sites(text)"]
    read --> blank["blank script, style, comments and every {...} expression"]
    blank --> text["text nodes with a letter"]
    blank --> attr["quoted values of the nine user-facing attributes"]
    read --> code["script code and expression code, comments removed"]
    code --> dialog["quoted literals handed to the five dialog calls"]
    text --> sites["sites: file, line, form, text, ordinal"]
    attr --> sites
    dialog --> sites
    sites --> verdict["verdict(sites, LISTED, LEVEL)"]
    listed["LISTED: file, form, text, ordinal, reason"] --> check["listing_problems(LISTED)"]
    listed --> verdict
    verdict --> warn["unlisted at warn: WARN line, pass"]
    verdict --> fail["unlisted at fail: FAIL line, refused"]
    verdict --> stale["entry with no site: STALE line, refused"]
```

## Planted components

```mermaid
flowchart LR
    fixtures["scripts/tests/fixtures/web-literal-text/*.svelte.fixture"] --> node["literal_text_node: 7 sites"]
    fixtures --> attribute["literal_attribute: 9 sites"]
    fixtures --> dialogs["literal_dialog: 6 sites"]
    fixtures --> control["catalog_only: 0 sites, 11 accepted forms"]
```

Each planted line carries a `planted:` comment and each control an `accepted:` one; a test checks
that the reported lines are exactly the planted lines.
