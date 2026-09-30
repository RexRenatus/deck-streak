// @ts-check
/**
 * SPEC-195 R9, ADR-195. Records GitHub's rendering of every generated fence member into
 * `src/lib/docs-mermaid.fences.json`, the truth the docs Mermaid check's generated test compares its
 * reader against. Run it after changing the grammar in `docs-mermaid-fences.js`, from the repository
 * root, with an authenticated `gh`:
 *
 *   node web/app/scripts/record-docs-mermaid-fences.js
 *
 * It renders through GitHub's Markdown API (`gh api markdown`, mode gfm, this repository as the
 * context), a read. Distinct texts go in batches, each followed by a separator paragraph; a batch
 * whose separators do not all come back once (a fence left open swallows one) is split in half until
 * every text is rendered alone. Then a sample of members is rendered one per request, and the file is
 * written only if every one agrees with its batch.
 */
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { GRAMMAR, digestOf, fenceMembers } from '../src/lib/docs-mermaid-fences.js';

const REPOSITORY = 'RexRenatus/deck-streak';
const TRUTH = resolve(import.meta.dirname, '../src/lib/docs-mermaid.fences.json');
const BATCH = 250;
const CONTROLS = 40;

/** GitHub's HTML for `text`. */
function render(/** @type {string} */ text) {
  const input = JSON.stringify({ text, mode: 'gfm', context: REPOSITORY });
  return execFileSync('gh', ['api', 'markdown', '--input', '-'], { input, encoding: 'utf8', maxBuffer: 1 << 28 });
}

/** The text of an HTML attribute or element, its character references decoded. */
function decode(/** @type {string} */ html) {
  const named = /** @type {Record<string, string>} */ ({ quot: '"', amp: '&', lt: '<', gt: '>', apos: "'" });
  return html.replace(/&(#x[0-9a-f]+|#[0-9]+|[a-z]+);/gi, (whole, ref) =>
    ref[0] !== '#'
      ? (named[ref.toLowerCase()] ?? whole)
      : String.fromCodePoint(Number.parseInt(ref.slice(ref[1] === 'x' || ref[1] === 'X' ? 2 : 1), ref[1] === 'x' || ref[1] === 'X' ? 16 : 10))
  );
}

/** The source of every diagram in a rendered fragment, in order. */
function sourcesOf(/** @type {string} */ fragment) {
  return [...fragment.matchAll(/data-type="mermaid".*?data-json="([^"]*)"/gs)].map((match) =>
    decode(JSON.parse(decode(match[1])).data)
  );
}

/** GitHub's diagram sources for each of `texts`, batched and split until every separator comes back once. */
function batch(/** @type {string[]} */ texts, /** @type {string} */ tag) {
  /** @type {Map<string, string[]>} */
  const out = new Map();
  if (texts.length === 1) {
    out.set(texts[0], sourcesOf(render(texts[0])));
    return out;
  }
  const mark = (/** @type {number} */ i) => `ZZSEP${tag}x${i}ZZ`;
  let rest = render(texts.map((text, i) => `${text}\n\n${mark(i)}\n\n`).join('\n'));
  for (const [i, text] of texts.entries()) {
    const parts = rest.split(`<p dir="auto">${mark(i)}</p>`);
    if (parts.length !== 2) {
      const half = Math.ceil(texts.length / 2);
      return new Map([...batch(texts.slice(0, half), `${tag}a`), ...batch(texts.slice(half), `${tag}b`)]);
    }
    out.set(text, sourcesOf(parts[0]));
    rest = parts[1];
  }
  return out;
}

const members = fenceMembers(GRAMMAR);
const distinct = [...new Set(members.map((member) => member.text))];
/** @type {Map<string, string[]>} */
const rendered = new Map();
for (let at = 0; at < distinct.length; at += BATCH) {
  for (const [text, sources] of batch(distinct.slice(at, at + BATCH), `${at}`)) rendered.set(text, sources);
}
const step = Math.max(1, Math.floor(members.length / CONTROLS));
for (let at = 0; at < members.length; at += step) {
  const alone = sourcesOf(render(members[at].text));
  if (JSON.stringify(alone) !== JSON.stringify(rendered.get(members[at].text))) {
    throw new Error(`${members[at].id}: rendered alone ${JSON.stringify(alone)}, in a batch otherwise`);
  }
}
/** @type {string[]} */
const sources = [];
const indexOf = (/** @type {string} */ source) => {
  if (!sources.includes(source)) sources.push(source);
  return sources.indexOf(source);
};
const renders = members.map((member) => [member.id, (rendered.get(member.text) ?? []).map(indexOf)]);
const lines = [
  '{',
  `  "renderer": ${JSON.stringify(`GitHub's Markdown API: gh api markdown, mode gfm, context ${REPOSITORY}`)},`,
  `  "refresh": "node web/app/scripts/record-docs-mermaid-fences.js",`,
  `  "grammar": ${JSON.stringify(GRAMMAR)},`,
  `  "members": ${members.length},`,
  `  "digest": ${JSON.stringify(digestOf(members))},`,
  `  "sources": ${JSON.stringify(sources)},`,
  '  "renders": {',
  renders.map(([id, list], i) => `    ${JSON.stringify(id)}: ${JSON.stringify(list)}${i + 1 < renders.length ? ',' : ''}`).join('\n'),
  '  }',
  '}',
  ''
];
writeFileSync(TRUTH, lines.join('\n'));
const diagrams = renders.filter(([, list]) => list.length > 0).length;
process.stdout.write(
  `recorded ${members.length} members (${distinct.length} distinct texts): ${diagrams} rendered as a diagram, ` +
    `${members.length - diagrams} as code; ${Math.ceil(members.length / step)} rendered alone agree\n`
);
