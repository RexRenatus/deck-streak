// @ts-check
/**
 * SPEC-195 R9, ADR-195. The container and fence grammar that the docs Mermaid check's generated test
 * draws its members from, and the one function that writes a member's text. The test
 * (`src/lib/docs-mermaid.test.ts`) and the refresh script (`record-docs-mermaid-fences.js`) both
 * import it, so the members the test generates are the members GitHub rendered, and adding an
 * alternative to the table adds members with no test edit. The class axes also read the reader's own
 * tables (`docs-mermaid-read.js`): every character it refuses, every raw HTML tag name it refuses and
 * the most lists it reads a fence in, so an entry added to one of them is a member too.
 */
import { createHash } from 'node:crypto';

/** Stand-ins for the reader's tables, which the next commit adds. */
const REFUSED_CHARACTERS = /** @type {[number, number][]} */ ([
  [0x00, 0x08],
  [0x0b, 0x0c],
  [0x0e, 0x1f],
  [0x7f, 0x9f],
  [0x2028, 0x2029],
  [0xfdd0, 0xfdef],
  [0xfeff, 0xfeff],
  ...Array.from({ length: 17 }, (_, plane) => /** @type {[number, number]} */ ([plane * 0x10000 + 0xfffe, plane * 0x10000 + 0xffff]))
]);
const SPECIAL_TAGS = [
  'address', 'applet', 'area', 'article', 'aside', 'base', 'basefont', 'bgsound', 'blockquote', 'body',
  'br', 'button', 'caption', 'center', 'col', 'colgroup', 'dd', 'details', 'dialog', 'dir', 'div', 'dl',
  'dt', 'embed', 'fieldset', 'figcaption', 'figure', 'footer', 'form', 'frame', 'frameset', 'h1', 'h2',
  'h3', 'h4', 'h5', 'h6', 'head', 'header', 'hgroup', 'hr', 'html', 'iframe', 'image', 'img', 'input',
  'keygen', 'legend', 'li', 'link', 'listing', 'main', 'marquee', 'math', 'menu', 'menuitem', 'meta',
  'nav', 'noembed', 'noframes', 'noscript', 'object', 'ol', 'optgroup', 'option', 'p', 'param',
  'plaintext', 'pre', 'script', 'search', 'section', 'select', 'source', 'style', 'summary', 'svg',
  'table', 'tbody', 'td', 'template', 'textarea', 'tfoot', 'th', 'thead', 'title', 'tr', 'track', 'ul',
  'wbr', 'xmp'
];
const LIST_DEPTH = 99;

/** A byte-order mark, which cmark-gfm skips at the start of a document and nowhere else. */
const BOM = '\uFEFF';

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
  body: ['broken', 'valid', 'blank', 'lazy', 'empty', 'blank-only'],
  blanks: [
    ' ', '\t', '\n', '\v', '\f', '\r', '\0', '\u0085', '\u00a0', '\u1680', '\u180e', '\u2000', '\u2001',
    '\u2002', '\u2003', '\u2004', '\u2005', '\u2006', '\u2007', '\u2008', '\u2009', '\u200a', '\u200b',
    '\u2028', '\u2029', '\u202f', '\u205f', '\u3000', '\ufeff'
  ],
  names: { Tab: '\t', NewLine: '\n', nbsp: '\u00a0', emsp: '\u2003', ThinSpace: '\u2009', MediumSpace: '\u205f', ZeroWidthSpace: '\u200b' },
  spelling: ['raw', 'decimal', 'hex', 'padded', 'overlong', 'padded-hex', 'overlong-hex', 'named'],
  digits: 8,
  place: ['before', 'after', 'inside', 'around'],
  shown: [
    [0x01, 0x08], [0x0b, 0x0b], [0x0e, 0x1f], [0x7f, 0x9f], [0xfdd0, 0xfdef],
    ...Array.from({ length: 17 }, (_, plane) => [plane * 0x10000 + 0xfffe, plane * 0x10000 + 0xffff])
  ],
  bom: ['{bom}', '{bom}{bom}', '{bom} ', '{bom}    ', '{bom}\n', '{bom}Text.\n', 'Text.\n{bom}', '\n\n{bom}'],
  html: {
    tags: [
      'address', 'article', 'aside', 'base', 'basefont', 'blockquote', 'body', 'caption', 'center', 'col',
      'colgroup', 'dd', 'details', 'dialog', 'dir', 'div', 'dl', 'dt', 'fieldset', 'figcaption', 'figure',
      'footer', 'form', 'frame', 'frameset', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'head', 'header', 'hr',
      'html', 'iframe', 'legend', 'li', 'link', 'main', 'menu', 'menuitem', 'nav', 'noframes', 'ol',
      'optgroup', 'option', 'p', 'param', 'search', 'section', 'source', 'summary', 'table', 'tbody', 'td',
      'tfoot', 'th', 'thead', 'title', 'tr', 'track', 'ul', 'script', 'pre', 'textarea', 'style'
    ],
    form: ['<{}>', '</{}>', '<{} x', '<{}', '<{U}>', '<x-y <{}>'],
    delimited: ['<script{}', '<div{}', '<x-y{}a>', '<x-y>{}'],
    starts: [
      '<!-- x', '<? x', '<!DOCTYPE x', '<!doctype x', '<!A x', '<!Z x', '<![CDATA[ x', '<x-y <!--', '<x-y <? x',
      '<x-y <!DOCTYPE x', '<x-y <![CDATA[ x', '    <div>', '<x-y>', '</x-y>', '<x-y/>',
      '<x-y a>', "<x-y a='v'>", '<x-y a="v">', '<x-y a=v>', '<x-y a = v />', '</x-y >', '<x-y a=v\u0001w>',
      '<x-y a=v\u001fw>', '<x-y a=v"w>', '<x-y a="v>', '<x-y 1a>', '<x-y> x', '<1x>', '<x-y a==v>',
      '<', '< x', '<a x', '<z x', '<A x', '<Z x', '</a x', '</ x'
    ],
    place: ['interrupting', 'opening', 'lazy', 'lazy-break']
  },
  parse: {
    runs: [
      '<div', '<div a="x', "<div a='x", '<div>\n<!-- x', '<!-- a --> <!-- b', '<div>\n</div', '<div>\n<![CDATA[ x',
      '<div>\n<?x', '<div>\n<!x', '<select>', '<table>', '<template>', '<svg>', '<math>', '<noscript>', '<frameset>',
      '<col>', '<p', '<details>\n<summary', '<div>\n<div>', '<pre lang="mermaid">\nflowchart TD\n  call --> done\n</pre>'
    ],
    tags: ['title', 'textarea', 'style', 'xmp', 'iframe', 'noembed', 'noframes', 'script', 'plaintext'],
    filtered: ['<{}>', '<{U}>', '<{} x>', '<{}\tx>', '<{}/>', '<{}', '<{}\fx>', '<{}/x>', '<<{}>', '<{}><{}>'],
    states: /** @type {[string, string][]} */ ([
      ['', ''],
      ['<x a="', 'say " <select>'],
      ["<x a='", "it's <select>"],
      ['<!--', '<select> -->'],
      ['<select>', '<select>']
    ]),
    inline: ['Text <select>']
  },
  inline: {
    tags: ['a', 'b', 'code', 'em', 'font', 'kbd', 'nobr', 'rt', 'span', 'sub', 'u', 'x-y', 'select', 'pre', 'textarea', 'plaintext'],
    form: ['<{}>', '</{}>'],
    starts: ['<!-- x -->', '<? x ?>', '<!X x>', '<![CDATA[ x ]]>', '<!-- <b> -->'],
    text: ['<x-y x', '<source x', '< x', '<= x', '</ x']
  },
  gfm: [
    ['Text.[^1]', '', '[^1]: ```mermaid', '    {}', '    ```'],
    ['Text.[^1]', '', '[^1]: ```mermaid', '{}', '```'],
    ['Text.[^1]', '', '[^1]: Note.', '', '    ```mermaid', '    {}', '    ```'],
    ['Text.[^1]', '', '[^1]:', '    ```mermaid', '    {}', '    ```'],
    ['Text.[^1]', '', '[^1]: Note.', '    ```mermaid', '    {}', '    ```'],
    ['Text.[^1]', '', '[^1]: Note.', '```mermaid', '{}', '```'],
    ['> Text.[^1]', '>', '> [^1]: ```mermaid', '>     {}', '>     ```'],
    ['- Text.[^1]', '', '  [^1]: ```mermaid', '      {}', '      ```'],
    ['[^1]: ```mermaid', '    {}', '    ```'],
    ['Text.[^x y]', '', '[^x y]: ```mermaid', '    {}', '    ```'],
    ['| a |', '| - |', '| b |', '```mermaid', '{}', '```'],
    ['| a |', '| - |', '```mermaid', '{}', '```'],
    ['> | a |', '> | - |', '```mermaid', '{}', '```'],
    ['| a |', '| - |', '| b', '~~~mermaid', '{}', '~~~'],
    ['| a |', '| - |', '  ```mermaid', '  {}', '  ```'],
    ['- [ ] ```mermaid', '  {}', '  ```'],
    ['- [ ] Text.', '', '  ```mermaid', '  {}', '  ```']
  ],
  opener: ['~~mermaid', '0.     ```mermaid', '09)     ```mermaid']
};
// A body is a flowchart whose first edge's node id is `call`, a word Mermaid reserves, so it does not
// parse (`broken`), or `caller` (`valid`, and `blank`, which holds an empty line); `lazy` drops every
// container prefix from its last line, and `empty` and `blank-only` hold no diagram.
//
// The class axes, where cmark-gfm reads a line otherwise than CommonMark 0.31.2: `blanks` is every
// character a reader may take for a blank (each ASCII blank and NUL, each Unicode Zs, Zl and Zp
// character, and the others JavaScript trims or that look blank); `names` spells some by name, and
// `spelling` writes one raw, as a decimal or hexadecimal reference, padded to the reader's digit
// count (`digits`, eight: a decimal reference's longest), one digit past it, or by name; `place` puts it
// before `mermaid`, after it or inside the info string. `shown` is every code point range GitHub shows
// as U+FFFD in a diagram's text (each C0 and C1 control but tab, LF, FF and CR, DEL, and the
// noncharacters). `bom` puts a byte-order mark before a fence. `html` is every HTML block tag of both
// type 6 lists and the type 1 list, written in each `form`, the other start conditions, and each placed
// interrupting a paragraph, opening a block, or on a lazy line after a quote's paragraph. `parse` is
// the raw HTML GitHub's HTML parse reads before a fence: `runs` of raw HTML blocks that may leave it
// inside a tag, a quoted value, a comment or an element that takes the fence's `<pre>` in; each
// filtered tag in each `filtered` form; for each state raw HTML may leave the parse in, a run that
// state disarms, after a paragraph (`states`); and `inline` HTML that leaves it there. `inline` is raw
// HTML inside a line before a fence: ordinary and special tag names in each `form`, the other inline
// starts, and `text` a `<` that opens no tag, each in a paragraph on the third line. `gfm` places a
// fence (its body on each line that ends `{}`, after that line's lead) in the GitHub extensions
// commonmark.js does not parse: a footnote definition, a table and a task list item. `opener` is a
// line that holds `mermaid` after a run of fence characters or a list marker and opens no fence.

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
    let onNextLine = false;
    if (level.kind === 'quote') {
      marker = `${g.quote.lead[level.lead]}>${g.quote.after[level.after]}`;
      continuation = marker;
    } else {
      marker = g.item.lead[level.lead] + g.item.marker[level.marker] + g.item.gap[level.gap];
      onNextLine = g.item.layout[level.layout] === 'next';
      continuation =
        g.item.continuation[level.continuation] === 'spaces'
          ? ' '.repeat(columns(start + marker) - columns(start))
          : marker.replace(/[-*+]|[0-9]+[.)]/g, (token) => ' '.repeat(token.length));
    }
    if (onNextLine) {
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
    members.set(id, { id, body: g.body[body], text: textOf(g, levels, fence, body) });
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
  for (const [id, text] of classMembers(g)) members.set(id, { id, body: 'broken', text });
  return [...members.values()];
}

/** Each of `lists`' entries once, in order of first appearance. */
const union = (/** @type {string[][]} */ ...lists) => [...new Set(lists.flat())];

/** A character's id: its code point in hexadecimal. */
const hexOf = (/** @type {string} */ char) => /** @type {number} */ (char.codePointAt(0)).toString(16);

/** A container's prefix on its first line and on every other. */
const PREFIX = /** @type {Record<string, [string, string]>} */ ({ top: ['', ''], quote: ['> ', '> '], item: ['- ', '  '] });

/** `lines` in a container: at the top, in a block quote, or in a list item opened on the first line. */
function nest(/** @type {string} */ context, /** @type {string[]} */ lines) {
  const [first, other] = PREFIX[context];
  return lines.map((line, at) => (at === 0 ? first : other) + line).join('\n') + '\n';
}

/** A fence opened by `open`, holding a body that does not parse. */
const fenced = (/** @type {string} */ open, /** @type {string[]} */ body = ['flowchart TD', '  call --> done']) => [
  open,
  ...body,
  open.startsWith('~') ? '~~~' : '```'
];

/**
 * The members of the class axes: every blank of the grammar in every spelling and place in a fence's
 * info string, before a backtick in a backtick fence's info string, after a list marker interrupting
 * a paragraph, and as a fence's whole body; each end of each range GitHub shows as U+FFFD and of each
 * range the reader refuses, and its neighbours, in a fence's body; a byte-order mark before a fence;
 * every HTML block start of the grammar, in each placement; the raw HTML before a fence that GitHub's
 * HTML parse reads: each run, each filtered tag in each form, and each state; raw HTML inside a line,
 * with every tag name of the grammar and of the reader's refused list; a fence in each GitHub
 * extension; each line that holds `mermaid` and opens no fence; and a fence in the most lists the
 * reader reads one in, and in one more.
 */
function* classMembers(/** @type {Grammar} */ g) {
  const blanks = g.blanks;
  const names = g.names;
  const decimal = (/** @type {string} */ c, /** @type {number} */ width) => `&#${String(c.codePointAt(0)).padStart(width, '0')};`;
  const hex = (/** @type {string} */ c, /** @type {number} */ width) => `&#x${hexOf(c).padStart(width, '0')};`;
  /** @type {Record<string, (c: string) => string[]>} */
  const spellings = {
    raw: (c) => (c === '\n' || c === '\r' ? [] : [c]),
    decimal: (c) => [decimal(c, 1)],
    hex: (c) => [hex(c, 1)],
    padded: (c) => [decimal(c, g.digits)],
    overlong: (c) => [decimal(c, g.digits + 1)],
    'padded-hex': (c) => [hex(c, g.digits)],
    'overlong-hex': (c) => [hex(c, g.digits + 1)],
    named: (c) => Object.entries(names).flatMap(([name, value]) => (value === c ? [`&${name};`] : []))
  };
  /** @type {Record<string, (w: string) => string>} */
  const places = {
    before: (w) => `${w}mermaid`,
    after: (w) => `mermaid${w}`,
    inside: (w) => `mermaid${w}x`,
    around: (w) => `${w}mermaid${w}x`
  };
  for (const c of blanks) {
    for (const spelling of g.spelling) {
      for (const [n, written] of spellings[spelling](c).entries()) {
        for (const place of g.place) {
          for (const context of ['top', 'quote']) {
            yield [`info.${hexOf(c)}.${spelling}${n}.${place}.${context}`, nest(context, fenced(`\`\`\`${places[place](written)}`))];
          }
        }
      }
    }
    yield [`backtick.${hexOf(c)}`, nest('top', [`\`\`\`x${c}\``, ...fenced('```mermaid')])];
    yield [`backtick.${hexOf(c)}.tilde`, nest('top', [`~~~x${c}\``, ...fenced('~~~mermaid')])];
    yield [`backtick.${hexOf(c)}.runs`, nest('top', fenced(`~~~mermaid${c}\`x\``))];
    for (const [at, marker] of g.item.marker.entries()) {
      yield [`item.${hexOf(c)}.${at}`, nest('top', ['Text.', `${marker}    ${c}`, ...fenced('```mermaid').map((line) => ' '.repeat(marker.length + 4) + line)])];
    }
    for (const context of ['top', 'quote', 'item']) {
      yield [`body.${hexOf(c)}.${context}`, nest(context, fenced('```mermaid', [c]))];
    }
  }
  // Each end of each range GitHub shows as U+FFFD and of each range the reader refuses, and the code
  // point on either side of one, in a diagram's text between two letters and alone.
  const shown = [...g.shown, ...REFUSED_CHARACTERS]
    .flatMap(([low, high]) => [low - 1, low, high, high + 1])
    .filter((code) => code >= 0 && code <= 0x10ffff);
  for (const c of union(shown.map((code) => String.fromCodePoint(code)))) {
    yield [`shown.${hexOf(c)}.inside`, nest('top', fenced('```mermaid', [`x${c}y`]))];
    yield [`shown.${hexOf(c)}.alone`, nest('top', fenced('```mermaid', [c]))];
  }
  for (const [at, lead] of g.bom.entries()) {
    for (const context of ['top', 'quote', 'item']) {
      yield [`bom.${at}.${context}`, lead.replaceAll('{bom}', BOM) + nest(context, fenced('```mermaid'))];
    }
  }
  // Every tag in every form at the top and in a quote; each blank in every delimited form (after a
  // type 1 or type 6 tag name, between a type 7 tag's name and an attribute, after a type 7 tag) in
  // every container; the other start conditions in every container, and on a lazy line too, after a
  // soft and after a hard line break.
  const starts = [
    ...g.html.tags.flatMap((tag) =>
      g.html.form.map((form, at) => [`${tag}.${at}`, form.replace('{}', tag).replace('{U}', tag.toUpperCase()), 2])
    ),
    ...blanks.flatMap((c) => g.html.delimited.map((form, at) => [`delimited.${at}.${hexOf(c)}`, form.replace('{}', c), 3])),
    ...g.html.starts.map((start, at) => [`start.${at}`, start, 3])
  ];
  for (const [id, start, contexts] of /** @type {[string, string, number][]} */ (starts)) {
    for (const place of g.html.place) {
      const lazy = { lazy: '> Text.\n', 'lazy-break': '> Text.\\\n' }[place];
      if (lazy !== undefined && contexts < 3) continue;
      for (const context of lazy === undefined ? ['top', 'quote', 'item'].slice(0, contexts) : ['quote']) {
        const lines = place === 'opening' ? [start, ...fenced('```mermaid')] : ['Text.', start, ...fenced('```mermaid')];
        yield [
          `html.${id}.${place}.${context}`,
          lazy === undefined ? nest(context, lines) : `${lazy}${[start, ...fenced('```mermaid')].join('\n')}\n`
        ];
      }
    }
  }
  // Each run of raw HTML blocks before a fence, in every container; each filtered tag in each form; each
  // state that earlier raw HTML leaves the parse in, across a paragraph, before the run that disarms it;
  // and each inline start of a state, before a run that ends it.
  for (const [at, run] of g.parse.runs.entries()) {
    for (const context of ['top', 'quote', 'item']) {
      yield [`parse.run.${at}.${context}`, nest(context, [...run.split('\n'), '', ...fenced('```mermaid')])];
    }
  }
  for (const tag of g.parse.tags) {
    for (const [at, form] of g.parse.filtered.entries()) {
      yield [`parse.filtered.${tag}.${at}`, nest('top', ['<div>', form.replaceAll('{}', tag).replace('{U}', tag.toUpperCase()), '', ...fenced('```mermaid')])];
    }
  }
  for (const [at, [state, run]] of g.parse.states.entries()) {
    yield [`parse.state.${at}`, nest('top', ['<div>', `${state}x`, '', 'Text.', '', '<div>', run, '', ...fenced('```mermaid')])];
  }
  for (const [at, inline] of g.parse.inline.entries()) {
    yield [`parse.inline.${at}`, nest('top', [inline, '', '<select>', '', ...fenced('```mermaid')])];
  }
  // Raw HTML inside a line of a paragraph before a fence: every tag of the grammar and every tag the
  // reader refuses, in each form; each other inline start; and each `<` that opens no tag.
  const inline = (/** @type {string} */ line) => nest('top', ['Text.', '', line, '', ...fenced('```mermaid')]);
  for (const tag of union(g.inline.tags, SPECIAL_TAGS)) {
    for (const [at, form] of g.inline.form.entries()) yield [`inline.${tag}.${at}`, inline(`Text ${form.replace('{}', tag)} x.`)];
  }
  for (const [at, start] of g.inline.starts.entries()) yield [`inline.start.${at}`, inline(`Text ${start} x.`)];
  for (const [at, text] of g.inline.text.entries()) yield [`inline.text.${at}`, inline(`Text ${text}.`)];
  // A fence in each GitHub extension commonmark.js does not parse, each line that looks like an opener
  // and is not one, and a fence in as many lists as the reader reads one in, and in one more.
  const rows = ['flowchart TD', '  call --> done'];
  for (const [at, form] of g.gfm.entries()) {
    yield [`gfm.${at}`, `${form.flatMap((line) => (line.endsWith('{}') ? rows.map((row) => line.slice(0, -2) + row) : [line])).join('\n')}\n`];
  }
  for (const [at, line] of g.opener.entries()) yield [`opener.${at}`, nest('top', [line, ...rows, '```'])];
  for (const depth of [LIST_DEPTH, LIST_DEPTH + 1]) {
    const indent = ' '.repeat(2 * depth);
    yield [`depth.${depth}`, `${'- '.repeat(depth)}${fenced('```mermaid').map((line, at) => (at === 0 ? line : indent + line)).join('\n')}\n`];
  }
}

/** One digest over every member's id and text, in order, so a change to how a text is written is seen. */
export function digestOf(/** @type {Member[]} */ members) {
  const hash = createHash('sha256');
  for (const member of members) hash.update(`${member.id}\0${member.text}\0`);
  return hash.digest('hex');
}
