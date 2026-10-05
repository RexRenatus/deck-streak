// SPEC-341 R8, R13. A SvelteKit directive map written as a Content-Security-Policy header value, the
// way SvelteKit writes its meta element: keyword sources quoted, sources joined by spaces and
// directives by semicolons. The card harness serves the page policy the app ships through it, so
// the planted suite measures the policy svelte.config.js declares rather than a copy of it.

/** The CSP keywords SvelteKit's directive map spells without their quotes. */
const KEYWORDS = new Set([
  'self',
  'none',
  'unsafe-inline',
  'unsafe-eval',
  'wasm-unsafe-eval',
  'unsafe-hashes',
  'strict-dynamic',
  'report-sample'
]);

/** The header value of `directives`; a directive that is not a source list is left out. */
export function policyHeader(directives: object): string {
  return Object.entries(directives)
    .flatMap(([name, sources]: [string, unknown]) =>
      Array.isArray(sources)
        ? [[name, ...sources.map((source: string) => (KEYWORDS.has(source) ? `'${source}'` : source))].join(' ')]
        : []
    )
    .join('; ');
}
