/**
 * SPEC-474, ADR-491. The docs check's reader against its Lean statement. The statement's writer,
 * `formal/lean/Formal/DocsMermaidReadVectors.lean`, prints `formal/vectors/docs-mermaid-read.jsonl`:
 * for each of 841 texts, the blocks the statement's port reads and the forms it refuses by name.
 * `readMermaid` must read every text so, and the reader without its opener rule
 * (`docs-mermaid-read.js:295-301`) is the planted fault the vectors must catch.
 *
 * @vitest-environment node
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { describe, expect, it } from 'vitest';
import { readMermaid } from './docs-mermaid-read.js';

/**
 * The repository root, found by walking up to `pnpm-workspace.yaml`. A fixed `../../../../` would
 * miss it when StrykerJS runs this file from its sandbox, which sits below `web/app/.stryker-tmp`.
 */
function repositoryRoot(from: string): string {
  for (let dir = from; ; dir = dirname(dir)) {
    if (existsSync(join(dir, 'pnpm-workspace.yaml'))) return dir;
    if (dirname(dir) === dir) throw new Error(`no pnpm-workspace.yaml above ${from}`);
  }
}

/** The vectors file, from the repository root. */
const VECTORS_FILE = join('formal', 'vectors', 'docs-mermaid-read.jsonl');

/** How many vectors the writer prints: 819 grid texts and 22 named texts. */
const VECTORS = 841;

/** The reader's three fence names, as `docs-mermaid-read.js` spells them. */
const INFO_FORM = 'a fence whose info string is not `mermaid` alone';
const EMPTY_FORM = 'a `mermaid` block that holds no diagram';
const OPENER_FORM = 'a line that may open a `mermaid` fence and is not read as one';

/** The first witness's text: a `mermaid` opener inside a tilde fence's body. */
const WITNESS_NAME = 'witness.opener-in-a-tilde-fence';
const WITNESS = '~~~\n```mermaid\nflowchart TD\n```\n~~~\n';

/** The header line the writer prints. */
const HEADER = {
  schema: 'phx.formal.vectors.v1',
  entry: 'DocsMermaidRead',
  reader: 'web/app/src/lib/docs-mermaid-read.js',
  function: 'readMermaid',
  vectors: VECTORS
};

interface Block {
  line: number;
  source: string;
}

interface Refusal {
  line: number;
  form: string;
}

interface Reading {
  blocks: Block[];
  refused: Refusal[];
}

interface Vector extends Reading {
  name: string;
  text: string;
}

/** A reading with only the fields a vector holds. */
function normalized(reading: Reading): Reading {
  return {
    blocks: reading.blocks.map(({ line, source }) => ({ line, source })),
    refused: reading.refused.map(({ line, form }) => ({ line, form }))
  };
}

/**
 * The vectors file's header and every vector, each line parsed. It is read inside the test that asks
 * for it, not when the file loads, so a missing or malformed file fails that test by name.
 */
function readVectors(): { header: unknown; vectors: Vector[] } {
  const text = readFileSync(join(repositoryRoot(import.meta.dirname), VECTORS_FILE), 'utf8');
  expect(text.endsWith('\n'), 'the vectors file does not end with a line feed').toBe(true);
  const [header, ...vectors] = text
    .slice(0, -1)
    .split('\n')
    .map((line) => JSON.parse(line));
  return { header, vectors };
}

/** Reports how many vectors were examined, and refuses none. */
function examined(count: number): void {
  console.log(`examined ${count} vectors`);
  expect(count, 'examined 0 vectors: nothing was judged').toBeGreaterThan(0);
}

/**
 * Compares `read` with every vector: the names of the vectors it reads otherwise, the first three
 * differences, and how many vectors it examined.
 */
function compare(vectors: Vector[], read: (text: string) => Reading) {
  const otherwise: string[] = [];
  const differences: string[] = [];
  let count = 0;
  for (const vector of vectors) {
    count += 1;
    const reading = normalized(read(vector.text));
    const stated = normalized(vector);
    if (!isDeepStrictEqual(reading, stated)) {
      otherwise.push(vector.name);
      if (differences.length < 3) {
        differences.push(`${vector.name}: the reader reads ${JSON.stringify(reading)}, the statement ${JSON.stringify(stated)}`);
      }
    }
  }
  return { otherwise, differences, count };
}

/**
 * The reader without its opener rule: its reading with every refusal of that name removed, which is
 * the reading of the reader with lines 295-301 removed, because no other rule writes that name.
 */
function readWithoutOpenerRule(text: string): Reading {
  const reading = readMermaid(text);
  return { blocks: reading.blocks, refused: reading.refused.filter((refusal) => refusal.form !== OPENER_FORM) };
}

describe("the docs Mermaid reader against its Lean statement's vectors", () => {
  it('answers every vector its Lean statement writes', () => {
    const { header, vectors } = readVectors();

    expect(header).toEqual(HEADER);
    expect(vectors.length).toBe(VECTORS);
    const { otherwise, differences, count } = compare(vectors, readMermaid);
    examined(count);
    expect(otherwise.length, `${differences.join('; ')}; ${otherwise.length} of ${VECTORS} vectors read otherwise`).toBe(0);
    const forms = vectors.flatMap((vector) => vector.refused.map((refusal) => refusal.form));
    expect(forms).toContain(INFO_FORM);
    expect(forms).toContain(EMPTY_FORM);
    expect(forms).toContain(OPENER_FORM);
    expect(
      vectors.some((vector) => vector.blocks.length > 0),
      'no vector reads a block'
    ).toBe(true);
    expect(
      vectors.some((vector) => vector.blocks.length === 0 && vector.refused.length === 0),
      'no vector reads neither a block nor a refusal'
    ).toBe(true);
  });

  it('reads otherwise on those vectors once its opener rule is removed', () => {
    const { vectors } = readVectors();
    const { otherwise, count } = compare(vectors, readWithoutOpenerRule);
    examined(count);

    expect(otherwise).toContain(WITNESS_NAME);
    expect(normalized(readMermaid(WITNESS))).toEqual({ blocks: [], refused: [{ line: 2, form: OPENER_FORM }] });
    expect(normalized(readWithoutOpenerRule(WITNESS))).toEqual({ blocks: [], refused: [] });
  });
});
