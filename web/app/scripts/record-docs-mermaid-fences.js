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
 * context), a read, one request about every 1.1 s. Distinct texts go in batches, each followed by a
 * control diagram that names it; a text whose control does not come back as the next diagram (a fence
 * or HTML block left open, or GitHub's HTML parse left inside a tag, a comment or a table) is rendered
 * alone and the batch resumes after it, and a batch that renders a diagram after its last control is
 * split in half. A text that opens with a byte-order mark or holds a `<` is rendered alone: a mark
 * acts only at the start of a document, and raw HTML can leave GitHub's HTML parse in a state (a
 * table, a foreign element, a template) that changes how the next text's raw HTML is read without
 * moving any control. Then a sample of members is rendered one per request, and the file is written
 * only if every one agrees with its batch.
 */
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { GRAMMAR, digestOf, fenceMembers } from '../src/lib/docs-mermaid-fences.js';

const REPOSITORY = 'RexRenatus/deck-streak';
const TRUTH = resolve(import.meta.dirname, '../src/lib/docs-mermaid.fences.json');
const BATCH = 250;
const CONTROLS = 40;
const PACE_MS = 1100;
let last = 0;

/** GitHub's HTML for `text`, one request at a time and about `PACE_MS` apart. */
function render(/** @type {string} */ text) {
  const wait = last + PACE_MS - Date.now();
  if (wait > 0) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, wait);
  last = Date.now();
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

/**
 * GitHub's diagram sources for each of `texts`, rendered in one request with a control diagram after
 * each, whose text names it. A text whose control is not the next diagram, once and in order (it left
 * a fence or an HTML block open, or left GitHub's HTML parse inside a tag, a comment or a table), is
 * rendered alone, the texts before it keep their rendering, and the rest go again; when a diagram
 * follows the last control, the batch is split in half.
 */
function batch(/** @type {string[]} */ texts, /** @type {string} */ tag) {
  /** @type {Map<string, string[]>} */
  const out = new Map();
  if (texts.length === 1) {
    out.set(texts[0], sourcesOf(render(texts[0])));
    return out;
  }
  const mark = (/** @type {number} */ i) => `ZZSEP${tag}x${i}ZZ\n`;
  const got = sourcesOf(render(texts.map((text, i) => `${text}\n\n\`\`\`mermaid\n${mark(i)}\`\`\`\n\n`).join('')));
  let at = 0;
  for (const [i, text] of texts.entries()) {
    const end = got.indexOf(mark(i), at);
    const own = got.slice(at, end);
    if (end < 0 || own.some((source) => source.startsWith('ZZSEP')) || got.lastIndexOf(mark(i)) !== end) {
      out.set(text, sourcesOf(render(text)));
      return i + 1 < texts.length ? new Map([...out, ...batch(texts.slice(i + 1), `${tag}r`)]) : out;
    }
    out.set(text, own);
    at = end + 1;
  }
  if (got.length > at) {
    const half = Math.ceil(texts.length / 2);
    return new Map([...batch(texts.slice(0, half), `${tag}a`), ...batch(texts.slice(half), `${tag}b`)]);
  }
  return out;
}

const members = fenceMembers(GRAMMAR);
const distinct = [...new Set(members.map((member) => member.text))];
/** @type {Map<string, string[]>} */
const rendered = new Map();
const ownRequest = (/** @type {string} */ text) => text.startsWith('\uFEFF') || text.includes('<');
for (const text of distinct.filter(ownRequest)) rendered.set(text, sourcesOf(render(text)));
const batched = distinct.filter((text) => !ownRequest(text));
for (let at = 0; at < batched.length; at += BATCH) {
  for (const [text, sources] of batch(batched.slice(at, at + BATCH), `${at}`)) rendered.set(text, sources);
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
