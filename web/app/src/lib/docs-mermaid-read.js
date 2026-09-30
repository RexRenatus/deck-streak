// @ts-check
/**
 * SPEC-195 R9, ADR-195. The docs Mermaid check's reader. GitHub draws a diagram for each
 * `<pre lang="mermaid">` in the HTML it makes from a Markdown text, and it makes that element from a
 * fenced code block whose language word is `mermaid`, or from raw HTML. The reader reads a declared
 * subset of Markdown, on which a CommonMark 0.31.2 parse (commonmark.js) opens the same fences as
 * GitHub's cmark-gfm with the same text, and it refuses by name everything outside that subset that
 * could make, hide or change a diagram. It reads nothing it has not positively read: every line on
 * which cmark-gfm could open a `mermaid` fence, in any container, is either a block it reads or a
 * line it refuses. Its tables are exported, and the check's generated test (`docs-mermaid-fences.js`)
 * draws members from them, so an entry added here is a member there with no test edit.
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

/** The most lists a fence the reader reads may stand in: cmark-gfm opens no list deeper (`MAX_LIST_DEPTH`). */
export const LIST_DEPTH = 99;

const OPENER = new RegExp(`^${FENCE_PREFIX}(?:\`{3,}|~{3,})(.*)$`);
const TAG = /^<\/?([A-Za-z][A-Za-z0-9-]*)/;
const SPECIAL = new Set(SPECIAL_TAGS);
/** What follows a `<` that opens a tag: a letter, or `/` and a letter. */
const MARKUP = /^\/?[A-Za-z]/;

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
 * A form the reader does not read: its line, counted from 1 (for raw HTML inside a paragraph, the
 * paragraph's first line), and what it is.
 *
 * @typedef {{ blocks: Block[], refused: Refusal[] }} Reading
 */

/** Whether an inline raw HTML node is the first thing on its line. */
function opensLine(/** @type {import('commonmark').Node} */ node) {
  return node.prev === null || node.prev.type === 'softbreak' || node.prev.type === 'linebreak';
}

/** The line a node stands on: its own, or its nearest block's. */
function lineOf(/** @type {import('commonmark').Node | null} */ node) {
  for (let at = node; at; at = at.parent) if (at.sourcepos) return at.sourcepos[0][0];
  return 1;
}

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
  const lines = text.split(/\r\n|\r|\n/);
  const refusedCharacter = refusedPattern();
  lines.forEach((line, at) => {
    if (refusedCharacter.test(line)) refused.push({ line: at + 1, form: 'a character GitHub reads otherwise' });
  });
  // Every node examined below is a leaf, which the walker visits once, entering.
  /** @type {Fence[]} */
  const fences = [];
  const walker = gfmParser(fences).parse(text).walker();
  for (let step = walker.next(); step; step = walker.next()) {
    const { node } = step;
    if (node.type === 'html_block') refused.push({ line: lineOf(node), form: 'a raw HTML block' });
    // Only a text node or a code span holds `<` alone; the rule refuses a line-opening `<` in either.
    if (node.literal === '<' && opensLine(node)) {
      const next = node.next?.type === 'text' ? /** @type {string} */ (node.next.literal) : '';
      if (MARKUP.test(next)) refused.push({ line: lineOf(node), form: 'a `<` that may open raw HTML' });
    }
    if (node.type === 'html_inline') {
      const name = TAG.exec(/** @type {string} */ (node.literal))?.[1].toLowerCase();
      if (name === undefined || SPECIAL.has(name) || opensLine(node)) {
        refused.push({ line: lineOf(node), form: `raw HTML \`${node.literal}\`` });
      }
    }
  }
  for (const { node, info } of fences) {
    const line = node.sourcepos[0][0];
    if (MERMAID_INFO.test(info) && !MERMAID_WORD.test(info)) {
      refused.push({ line, form: 'a fence whose info string is not `mermaid` alone' });
    } else if (MERMAID_WORD.test(info)) {
      let lists = 0;
      for (let at = node.parent; at; at = at.parent) if (at.type === 'list') lists += 1;
      if (lists > LIST_DEPTH) refused.push({ line, form: `a fence in more than ${LIST_DEPTH} lists` });
      else if (!/\S/u.test(/** @type {string} */ (node.literal))) refused.push({ line, form: 'a `mermaid` block that holds no diagram' });
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
