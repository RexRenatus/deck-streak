### Added

- A census reads every web component and reports user-facing text written in the markup instead of
  drawn from the message catalogs: a text node, the value of one of nine user-facing attributes
  such as `aria-label` or `placeholder`, or the text of a browser or Mini App dialog. It starts at
  warn with an empty listing, since no component holds such a text today, prints what it examined
  and refuses an empty population (SPEC-653).
