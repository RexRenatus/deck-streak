// @ts-check
/**
 * SPEC-195 R9, ADR-195. The container and fence grammar that the docs Mermaid check's generated test
 * draws its members from, and the one function that writes a member's text. The test
 * (`src/lib/docs-mermaid.test.ts`) and the refresh script (`record-docs-mermaid-fences.js`) both
 * import it, so the members the test generates are the members GitHub rendered, and adding an
 * alternative to the table adds members with no test edit.
 */
import { createHash } from 'node:crypto';

/**
 * CommonMark 0.31.2 (GFM 0.29) tokens, the first of each list the base. A block quote marker is up to
 * three blanks, `>` and an optional blank (section 5.1). A list item marker is up to three blanks, a
 * bullet or one to nine digits and `.` or `)`, then one to four columns of blanks, and a fifth column
 * starts indented code (section 5.2); its fence opens on the line after the marker or on the marker
 * line, and its continuation lines carry its content column as spaces or as the marker line with the
 * marker blanked. A fence is three or more backticks or tildes (section 4.5). Values GitHub shows as
 * code are here too (ten digits, two backticks, a fifth column of indentation, another language word,
 * a blank body), so the check is held to reading nothing there.
 */
export const GRAMMAR = {
  depth: 4,
  quote: { lead: ['', ' ', '   '], after: [' ', '', '\t'] },
  item: {
    lead: ['', ' ', '   '],
    marker: ['-', '+', '*', '1.', '1)', '123456789.', '123456789)', '1234567890.'],
    gap: [' ', '    ', '\t', '     '],
    layout: ['next', 'same'],
    continuation: ['spaces', 'blanked']
  },
  fence: {
    open: ['```', '````', '~~~', '``'],
    info: [
      'mermaid',
      ' mermaid',
      '\tmermaid',
      'mermaid title',
      'mermai&#100;',
      'Mermaid',
      'mermaidx',
      'mermaid,x',
      'mermaid x'
    ],
    indent: ['', ' ', '   ', '    ', '\t']
  },
  body: ['broken', 'valid', 'blank', 'lazy', 'empty', 'blank-only']
};
// A body is a flowchart whose first edge's node id is `call`, a word Mermaid reserves, so it does not
// parse (`broken`), or `caller` (`valid`, and `blank`, which holds an empty line); `lazy` drops every
// container prefix from its last line, and `empty` and `blank-only` hold no diagram.

/** @typedef {typeof GRAMMAR} Grammar */
/** @typedef {{ kind: 'quote', lead: number, after: number }} Quote */
/** @typedef {{ kind: 'item', lead: number, marker: number, gap: number, layout: number, continuation: number }} Item */
/** @typedef {Quote | Item} Level */
/** @typedef {{ open: number, info: number, indent: number }} Fence */
/** @typedef {{ id: string, body: string, text: string }} Member */

/** The column after `text` written from column 0, a tab advancing to the next multiple of 4 (section 2.2). */
function columns(/** @type {string} */ text) {
  let column = 0;
  for (const char of text) column = char === '\t' ? column + 4 - (column % 4) : column + 1;
  return column;
}

/** A member's id: each level's alternative indices outermost first, then the fence's and the body's. */
function idOf(/** @type {Level[]} */ levels, /** @type {Fence} */ fence, /** @type {number} */ body) {
  const level = (/** @type {Level} */ l) =>
    l.kind === 'quote'
      ? `q${l.lead}${l.after}`
      : `i${l.lead}${l.marker}${l.gap}${l.layout}${l.continuation}`;
  return `${levels.map(level).join('.') || 'top'}:f${fence.open}${fence.info}${fence.indent}:b${body}`;
}

/** The text of one member: its containers' markers on the first line or lines, their continuations on every other. */
function textOf(/** @type {Grammar} */ g, /** @type {Level[]} */ levels, /** @type {Fence} */ fence, /** @type {number} */ body) {
  const lines = [];
  let continued = '';
  /** @type {string | null} */
  let pending = '';
  for (const level of levels) {
    /** @type {string} */
    const start = pending ?? continued;
    let marker;
    let continuation;
    if (level.kind === 'quote') {
      marker = `${g.quote.lead[level.lead]}>${g.quote.after[level.after]}`;
      continuation = marker;
    } else {
      marker = g.item.lead[level.lead] + g.item.marker[level.marker] + g.item.gap[level.gap];
      continuation =
        g.item.continuation[level.continuation] === 'spaces'
          ? ' '.repeat(columns(start + marker) - columns(start))
          : marker.replace(/[-*+]|[0-9]+[.)]/g, (token) => ' '.repeat(token.length));
    }
    if (level.kind === 'item' && g.item.layout[level.layout] === 'next') {
      lines.push(`${start}${marker}item`, (continued + continuation).trimEnd());
      pending = null;
    } else {
      pending = start + marker;
    }
    continued += continuation;
  }
  const indent = g.fence.indent[fence.indent];
  const open = g.fence.open[fence.open];
  const node = g.body[body] === 'valid' || g.body[body] === 'blank' ? 'caller' : 'call';
  const rows = {
    broken: ['flowchart TD', `  ${node} --> done`],
    valid: ['flowchart TD', `  ${node} --> done`],
    blank: ['flowchart TD', '', `  ${node} --> done`],
    lazy: ['flowchart TD', null],
    empty: [],
    'blank-only': ['   ']
  }[g.body[body]];
  if (rows === undefined) throw new Error(`no body named ${g.body[body]}`);
  const inner = rows.map((row) =>
    row === null ? `  ${node} --> done` : row === '' ? (continued + indent).trimEnd() : continued + indent + row
  );
  const first = `${pending ?? continued}${indent}${open}${g.fence.info[fence.info]}`;
  return [...lines, first, ...inner, continued + indent + open, ''].join('\n');
}

/** Every sequence of `kinds` of length 0 to `depth`, shortest first. */
function shapes(/** @type {number} */ depth) {
  /** @type {('quote' | 'item')[][]} */
  let layer = [[]];
  /** @type {('quote' | 'item')[][]} */
  const all = [[]];
  for (let n = 1; n <= depth; n++) {
    layer = layer.flatMap((shape) => [
      [...shape, /** @type {const} */ ('quote')],
      [...shape, /** @type {const} */ ('item')]
    ]);
    all.push(...layer);
  }
  return all;
}

/**
 * The members of `g`, generated: for every container shape up to `g.depth` levels, with every list
 * item on the line after its marker and again on its marker line, the base member and every member
 * that differs from it in exactly one token (one level's axis, the fence's open, info or indent, or
 * the body); and at one level, every combination of a level's tokens.
 */
export function fenceMembers(/** @type {Grammar} */ g = GRAMMAR) {
  /** @type {Map<string, Member>} */
  const members = new Map();
  const add = (/** @type {Level[]} */ levels, /** @type {Fence} */ fence, /** @type {number} */ body) => {
    const id = idOf(levels, fence, body);
    if (!members.has(id)) members.set(id, { id, body: g.body[body], text: textOf(g, levels, fence, body) });
  };
  const base = /** @type {Fence} */ ({ open: 0, info: 0, indent: 0 });
  /** @type {Record<'quote' | 'item', string[]>} */
  const axes = { quote: ['lead', 'after'], item: ['lead', 'marker', 'gap', 'layout', 'continuation'] };
  const count = (/** @type {'quote' | 'item'} */ kind, /** @type {string} */ axis) =>
    /** @type {Record<string, string[]>} */ (g[kind])[axis].length;
  for (const shape of shapes(g.depth)) {
    for (const layout of g.item.layout.keys()) {
      /** @type {Level[]} */
      const levels = shape.map((kind) =>
        kind === 'quote'
          ? { kind, lead: 0, after: 0 }
          : { kind, lead: 0, marker: 0, gap: 0, layout, continuation: 0 }
      );
      add(levels, base, 0);
      levels.forEach((level, at) => {
        for (const axis of axes[level.kind]) {
          for (let value = 0; value < count(level.kind, axis); value++) {
            const changed = /** @type {Level} */ ({ ...level, [axis]: value });
            add(levels.map((l, i) => (i === at ? changed : l)), base, 0);
          }
        }
      });
      for (const key of /** @type {const} */ (['open', 'info', 'indent'])) {
        for (let value = 0; value < g.fence[key].length; value++) add(levels, { ...base, [key]: value }, 0);
      }
      for (let body = 0; body < g.body.length; body++) add(levels, base, body);
    }
  }
  /** @type {(axes: string[], kind: 'quote' | 'item') => Record<string, number>[]} */
  const product = (names, kind) =>
    names.reduce(
      (acc, axis) => acc.flatMap((partial) => Array.from({ length: count(kind, axis) }, (_, v) => ({ ...partial, [axis]: v }))),
      /** @type {Record<string, number>[]} */ ([{}])
    );
  for (const kind of /** @type {const} */ (['quote', 'item'])) {
    for (const tokens of product(axes[kind], kind)) add([/** @type {Level} */ ({ kind, ...tokens })], base, 0);
  }
  return [...members.values()];
}

/** One digest over every member's id and text, in order, so a change to how a text is written is seen. */
export function digestOf(/** @type {Member[]} */ members) {
  const hash = createHash('sha256');
  for (const member of members) hash.update(`${member.id}\0${member.text}\0`);
  return hash.digest('hex');
}
