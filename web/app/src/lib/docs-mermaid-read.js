// @ts-check
/**
 * SPEC-195 R9, ADR-195. The docs Mermaid check's reader. GitHub draws a diagram for each
 * `<pre lang="mermaid">` in the HTML it makes from a Markdown text, and it makes that element from a
 * fenced code block whose language word is `mermaid`, or from raw HTML. The reader reads a declared
 * subset of Markdown, on which a CommonMark 0.31.2 parse (commonmark.js) opens the same fences as
 * GitHub's cmark-gfm with the same text, and it refuses by name every form outside that subset that it
 * has found could make, hide or change a diagram: a line on which cmark-gfm could open a `mermaid`
 * fence and the reader reads none, a block deeper than cmark-gfm opens one, a block GitHub may nest
 * past its HTML depth, a line that may open raw HTML, and a tag a CommonMark reading may hide in a
 * construct cmark-gfm does not form. Its tables are exported, and the check's generated test
 * (`docs-mermaid-fences.js`) draws members from them, so an entry added here is a member there with no
 * test edit.
 */
import { Parser } from 'commonmark';

/**
 * Code point ranges the reader refuses wherever they stand. cmark-gfm skips a byte-order mark that
 * opens a document and commonmark.js does not; a vertical tab or form feed ends a list marker for one
 * and not the other; a line or paragraph separator ends commonmark.js's `.` and not cmark-gfm's line;
 * and GitHub shows each other C0 and C1 control, DEL and each noncharacter in a diagram as U+FFFD.
 */
export const REFUSED_CHARACTERS = /** @type {[number, number][]} */ ([
  [0x00, 0x08],
  [0x0b, 0x0c],
  [0x0e, 0x1f],
  [0x7f, 0x9f],
  [0x2028, 0x2029],
  [0xfdd0, 0xfdef],
  [0xfeff, 0xfeff],
  ...Array.from({ length: 17 }, (_, plane) => /** @type {[number, number]} */ ([plane * 0x10000 + 0xfffe, plane * 0x10000 + 0xffff]))
]);

/**
 * Raw HTML tag names the reader refuses: the HTML standard's `special` elements, whose parse is not
 * that of an ordinary element (a `<pre>`, a `<select>` or a `<textarea>` among them), the roots of
 * SVG and MathML content, `image`, and every HTML block tag of cmark-gfm's and CommonMark 0.31.2's
 * lists. A raw tag of any other name is an ordinary element to GitHub's HTML parse, and cannot make,
 * take in or drop a `<pre>`.
 */
export const SPECIAL_TAGS = [
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

/**
 * What may stand before a fence on its line, in any container: blanks, a block quote's `>`, a bullet
 * or an ordered list marker followed by a blank. A line that opens a fence has this, then three or
 * more backticks or tildes, then its info string.
 */
export const FENCE_PREFIX = String.raw`(?:[ \t>]|[-*+](?=[ \t])|[0-9]{1,9}[.)](?=[ \t]))*`;

/**
 * An info string in which cmark-gfm may find the language word `mermaid`: one holding the word, in
 * any case, or a character reference, which cmark-gfm decodes before it takes the word. Nothing else
 * can make the word: cmark-gfm's trim and cut only remove, and it removes a backslash only before
 * ASCII punctuation.
 */
export const MERMAID_INFO = /mermaid|&/i;

/** The one info string the reader reads a `mermaid` fence by: the word, and blanks around it. */
export const MERMAID_WORD = /^[ \t]*mermaid[ \t]*$/;

/**
 * The most block quotes and list items together a block the reader reads may stand in. cmark-gfm opens
 * no list item as the 100th or later block it opens on one line (`MAX_LIST_DEPTH`), and it counts block
 * quotes among those blocks.
 */
export const CONTAINER_DEPTH = 99;

/**
 * The most elements the reader lets GitHub's HTML nest around the text of a block. GitHub draws no
 * diagram at or after the first point where its HTML nests more than 254 elements deep (a paragraph
 * holds 253 nested tags and not 254), so the reader refuses a block whose bound passes this one, with
 * room to spare: a block quote counts one, a list item two, and each character that may open an
 * element inside the block (`*`, `~`, `[`, `<`, and `_` not between letters or digits), with every
 * tag earlier or later in the text, one more.
 */
export const PAGE_DEPTH = 240;

/** What ends a line for cmark-gfm: a line feed, a carriage return and line feed, or a carriage return alone. */
export const LINE_END = /\r\n|\r|\n/;

/** The longest run of backticks cmark-gfm opens a code span with (`MAXBACKTICKS`); commonmark.js has no such limit. */
export const CODE_TICKS = 80;

/** The most parentheses cmark-gfm nests in a link destination; commonmark.js has no such limit. */
export const LINK_PARENS = 32;

/**
 * The fewest characters in brackets the reader takes for a link label cmark-gfm may refuse. cmark-gfm
 * refuses a label of more than 1,000 bytes and commonmark.js one of more than 1,000 characters, and no
 * character takes more than four bytes, so a shorter label is one both parsers take or both refuse.
 */
export const LABEL_UNITS = 250;

const OPENER = new RegExp(`^${FENCE_PREFIX}(?:\`{3,}|~{3,})(.*)$`);
const SPECIAL = new Set(SPECIAL_TAGS);
/** A `<` that may begin raw HTML: `!` or `?`, or a tag name after an optional `/`, the name captured. */
const MARKUP = /<(?:[!?]|\/?([A-Za-z][A-Za-z0-9-]*))/g;
/** Characters that may open an element inside a block. */
const OPENS = /[*~[<]|(?<![\p{L}\p{N}])_|_(?![\p{L}\p{N}])/gu;
/**
 * A backslash escape of a character the reader's counts read: a parenthesis, a bracket or a backslash.
 * cmark-gfm and commonmark.js both skip it as a pair, so it opens, closes and ends nothing.
 */
const ESCAPE = /\\[()[\]\\]/g;
/**
 * A backtick after text cmark-gfm's extended autolink may take up to it: it stops only at an ASCII blank
 * or `<`. The lines it is tested on are joined with a line feed, so no carriage return is among them.
 */
const AUTOLINK = /(?::\/\/|www\.)[^ \t\n<]*`/i;
/** A table's delimiter row, in the shape cmark-gfm opens a table with, and wider. */
const DELIMITER = /^[|: \t-]*-[|: \t-]*$/;

/**
 * The refused characters as one pattern. It is built each time a text is read, not when the module
 * loads, so a malformed entry fails the read that meets it, inside a test, rather than the import of
 * every file that uses the reader.
 */
function refusedPattern() {
  const ranges = REFUSED_CHARACTERS.map(([low, high]) => `\\u{${low.toString(16)}}-\\u{${high.toString(16)}}`);
  return new RegExp(`[${ranges.join('')}]`, 'u');
}

/**
 * @typedef {{
 *   blockStarts: ((parser: ParserState, container: import('commonmark').Node) => number)[],
 *   tip: import('commonmark').Node & { _fenceOffset: number },
 *   currentLine: string, offset: number, nextNonspace: number
 * }} ParserState
 * commonmark.js 0.31.2's state while it opens a block: its block starts (a block quote, an ATX
 * heading, a fenced code block, ...), the block just opened, the line and the offsets into it.
 */

const FENCED_CODE = 2;

/**
 * @typedef {{ node: import('commonmark').Node, info: string }} Fence
 * A fenced code block as the wrapper opened it: the block, and its info string as the line holds it,
 * before commonmark.js trims it and decodes its character references.
 */

/**
 * A CommonMark parser that opens a fenced code block as GitHub's cmark-gfm does. CommonMark counts a
 * fence's indentation in columns and cmark-gfm in characters, so when a container prefix consumes
 * part of a tab, GitHub's block keeps the tab's remaining columns on each line. The wrapper runs the
 * fenced code start, sets the offset cmark-gfm would, and adds each fence it opens to `fences` with
 * its raw info string.
 */
function gfmParser(/** @type {Fence[]} */ fences) {
  const parser = new Parser();
  const state = /** @type {ParserState} */ (/** @type {unknown} */ (parser));
  const starts = [...state.blockStarts];
  const fenced = starts[FENCED_CODE];
  starts[FENCED_CODE] = (current, container) => {
    const { offset, nextNonspace } = current;
    const started = fenced(current, container);
    if (started === 2) {
      current.tip._fenceOffset = nextNonspace - offset;
      fences.push({ node: current.tip, info: current.currentLine.slice(current.offset) });
    }
    return started;
  };
  state.blockStarts = starts;
  return parser;
}

/**
 * @typedef {{ line: number, source: string }} Block
 * A `mermaid` block: its first line in the text, counted from 1, and the text GitHub renders as the diagram.
 *
 * @typedef {{ line: number, form: string }} Refusal
 * A form the reader does not read: its line, counted from 1 (for a block GitHub may nest too deep, the
 * block's first line), and what it is.
 *
 * @typedef {{ blocks: Block[], refused: Refusal[] }} Reading
 */

/**
 * Reads `text`: the `mermaid` blocks, in order, each a fenced code block whose info string is
 * `mermaid` alone and whose text holds a diagram, with that text as GitHub shows it; and every form
 * the reader refuses. A text with a refusal is not read as GitHub renders it, and the check names it.
 */
export function readMermaid(/** @type {string} */ text) {
  /** @type {Block[]} */
  const blocks = [];
  /** @type {Refusal[]} */
  const refused = [];
  const lines = text.split(LINE_END);
  const refusedCharacter = refusedPattern();
  lines.forEach((line, at) => {
    if (refusedCharacter.test(line)) refused.push({ line: at + 1, form: 'a character GitHub reads otherwise' });
  });
  /** @type {Fence[]} */
  const fences = [];
  const walker = gfmParser(fences).parse(text).walker();
  const prefix = new RegExp(`^${FENCE_PREFIX}`);
  const lineStart = new RegExp(`^${FENCE_PREFIX}<(?:[!?]|/?([A-Za-z][A-Za-z0-9-]*)(?=[\\s/>]|$)(.*>[ \\t]*$)?)`);
  const body = new Set();
  for (const { node } of fences) for (let n = node.sourcepos[0][0] + 1; n <= node.sourcepos[1][0]; n += 1) body.add(n);
  /** Each line outside every fence's body, with its index: the lines raw HTML may stand on. */
  const outside = [...lines.entries()].filter(([at]) => !body.has(at + 1));
  const tags = outside.map(([, line]) => line).join('\n').match(/<[A-Za-z]/g)?.length ?? 0;
  /** @type {(line: string) => number} */
  const special = (line) => [...line.matchAll(MARKUP)].filter((m) => m[1] === undefined || SPECIAL.has(m[1].toLowerCase())).length;
  /** @typedef {{ node: import('commonmark').Node, code: number, breaks: number, risk: boolean, url: boolean }} Leaf */
  /** @type {Map<number, Leaf>} */
  const owner = new Map();
  let quotes = 0;
  let items = 0;
  /** @type {Leaf | undefined} */
  let leaf;
  for (let step = walker.next(); step; step = walker.next()) {
    const { node, entering } = step;
    const container = node.type === 'block_quote' || node.type === 'item';
    if (container) {
      const change = entering ? 1 : -1;
      if (node.type === 'item') items += change;
      else quotes += change;
      if (entering && quotes + items > CONTAINER_DEPTH) {
        refused.push({ line: node.sourcepos[0][0], form: `a block in more than ${CONTAINER_DEPTH} block quotes and list items` });
      }
    }
    if (leaf && (node.type === 'softbreak' || node.type === 'linebreak')) leaf.breaks += 1;
    if (leaf && node.type === 'text' && /:\/\/|www\./i.test(/** @type {string} */ (node.literal))) leaf.url = true;
    if (node.type === 'code' && leaf) {
      const literal = /** @type {string} */ (node.literal);
      if (literal.includes('|')) leaf.risk = true;
      leaf.code += special(literal);
    }
    if (!entering || !node.sourcepos || container || node.type === 'list' || node.type === 'document') continue;
    const [first, last] = [node.sourcepos[0][0], node.sourcepos[1][0]];
    let opens = 0;
    if (node.type !== 'code_block') {
      for (let n = first; n <= last; n += 1) {
        const line = /** @type {string} */ (lines[n - 1]);
        opens += line.slice(/** @type {RegExpExecArray} */ (prefix.exec(line))[0].length).match(OPENS)?.length ?? 0;
      }
    }
    if (quotes + 2 * items + opens + tags > PAGE_DEPTH) {
      refused.push({ line: first, form: `a block GitHub may nest more than ${PAGE_DEPTH} elements deep` });
    }
    leaf = { node, code: 0, breaks: 0, risk: false, url: false };
    for (let n = first; n <= last; n += 1) owner.set(n, leaf);
  }
  const labels = new RegExp(`\\[[^[\\]]{${LABEL_UNITS},}\\]`).test(text.replace(ESCAPE, '__'));
  const ticks = new RegExp(`\`{${CODE_TICKS + 1},}`);
  /**
   * Whether each special tag of a block stands in a code span that cmark-gfm forms as commonmark.js
   * does. Only a paragraph or a heading holds a code span, so a tag on a line of any other block, or of
   * none, is never trusted. It is asked only for a line that holds such a tag, so a text with none pays
   * nothing for it.
   */
  const trusted = (/** @type {Leaf} */ { node, code, breaks, risk, url }) => {
    const held = lines.slice(node.sourcepos[0][0] - 1, node.sourcepos[1][0]);
    const all = held.reduce((sum, line) => sum + special(line), 0);
    const raw = held.join('\n');
    const plain = raw.replace(ESCAPE, '__');
    const table = held.some((line) => DELIMITER.test(line.replace(prefix, '')));
    let parens = 0;
    let deepest = 0;
    for (const c of plain) {
      parens += c === '(' ? 1 : c === ')' && parens > 0 ? -1 : 0;
      deepest = Math.max(deepest, parens);
    }
    const cells = table && (risk || breaks < held.length - 1 || raw.includes('\\|'));
    return all === code && !cells && !labels && deepest <= LINK_PARENS && !ticks.test(raw) && !(url && AUTOLINK.test(raw)) && !/\]\[[^[\]]*`/.test(plain);
  };
  outside.forEach(([at, line]) => {
    const start = lineStart.exec(line);
    if (start && (start[1] === undefined || SPECIAL.has(start[1].toLowerCase()) || start[2] !== undefined)) {
      refused.push({ line: at + 1, form: 'a line that may open raw HTML' });
    } else if (special(line) > 0) {
      const leaf = owner.get(at + 1);
      if (leaf === undefined || !trusted(leaf)) refused.push({ line: at + 1, form: 'raw HTML a CommonMark reading may hide' });
    }
  });
  for (const { node, info } of fences) {
    const line = node.sourcepos[0][0];
    if (MERMAID_INFO.test(info) && !MERMAID_WORD.test(info)) {
      refused.push({ line, form: 'a fence whose info string is not `mermaid` alone' });
    } else if (MERMAID_WORD.test(info)) {
      if (!/\S/u.test(/** @type {string} */ (node.literal))) refused.push({ line, form: 'a `mermaid` block that holds no diagram' });
      else blocks.push({ line, source: /** @type {string} */ (node.literal) });
    }
  }
  const read = new Set(blocks.map((block) => block.line));
  lines.forEach((line, at) => {
    const opener = OPENER.exec(line);
    if (opener && MERMAID_INFO.test(opener[1]) && !read.has(at + 1)) {
      refused.push({ line: at + 1, form: 'a line that may open a `mermaid` fence and is not read as one' });
    }
  });
  return { blocks, refused: refused.sort((a, b) => a.line - b.line) };
}
