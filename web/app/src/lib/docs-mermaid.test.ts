/**
 * SPEC-195, ADR-195. Every fenced mermaid block under docs/ must parse with Mermaid's own parser,
 * so a diagram that would render as an error box is refused before merge. The parser sanitises
 * labels through the document, hence jsdom.
 *
 * @vitest-environment jsdom
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import mermaid from 'mermaid';
import { describe, expect, it } from 'vitest';

const DOCS = resolve(import.meta.dirname, '../../../../docs');

/** Every Markdown file under `dir`, in path order. */
function markdownFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true })
    .sort((a, b) => a.name.localeCompare(b.name))
    .flatMap((entry) => {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) return markdownFiles(path);
      return entry.name.endsWith('.md') ? [path] : [];
    });
}

interface Block {
  name: string;
  source: string;
}

/** The fenced `mermaid` blocks of one file, each named `<file> block <n>` counted from 1. */
function blocksOf(name: string, text: string): Block[] {
  const found = text.matchAll(/^```mermaid[^\n]*\n([\s\S]*?)^```[ \t]*$/gm);
  return [...found].map((match, index) => ({ name: `${name} block ${index + 1}`, source: match[1] }));
}

/** Whether Mermaid's own parser accepts the diagram. */
async function parses(source: string): Promise<boolean> {
  return (await mermaid.parse(source, { suppressErrors: true })) !== false;
}

const BLOCKS = markdownFiles(DOCS).flatMap((file) =>
  blocksOf(relative(DOCS, file), readFileSync(file, 'utf8'))
);

describe('the Mermaid diagrams under docs', () => {
  it('reads every fenced block', () => {
    const opened = markdownFiles(DOCS)
      .map((file) => (readFileSync(file, 'utf8').match(/^```mermaid/gm) ?? []).length)
      .reduce((sum, count) => sum + count, 0);

    expect(BLOCKS.length).toBeGreaterThan(100);
    expect(BLOCKS.length).toBe(opened);
  });

  it('parses every block', async () => {
    const refused: string[] = [];
    for (const block of BLOCKS) {
      if (!(await parses(block.source))) refused.push(block.name);
    }

    expect(BLOCKS.length).toBeGreaterThan(100);
    expect(refused).toEqual([]);
  });

  it('refuses a block whose node id is a reserved word', async () => {
    const planted = blocksOf('planted.md', '```mermaid\nflowchart TD\n  call --> done\n```\n');

    expect(planted.map((block) => block.name)).toEqual(['planted.md block 1']);
    expect(await parses(planted[0].source)).toBe(false);
  });

  it('accepts the same block once its node id is not a reserved word', async () => {
    const planted = blocksOf('planted.md', '```mermaid\nflowchart TD\n  caller --> done\n```\n');

    expect(await parses(planted[0].source)).toBe(true);
  });
});
