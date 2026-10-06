import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';
import { readMedia, type MediaFolder } from './media';

// SPEC-350 A24, ADR-361 D12: the Worker's reader of the media directory, and the census that keeps
// the media rules in the core alone.

/** A directory in memory: its files by name, and the directories under it. */
class FakeFolder implements MediaFolder {
  constructor(
    readonly files = new Map<string, Uint8Array>(),
    readonly folders = new Map<string, FakeFolder>()
  ) {}

  async getDirectoryHandle(name: string): Promise<FakeFolder> {
    const folder = this.folders.get(name);
    if (folder === undefined) throw new DOMException(`${name} is absent`, 'NotFoundError');
    return folder;
  }

  async getFileHandle(name: string): Promise<{ getFile(): Promise<Blob> }> {
    const bytes = this.files.get(name);
    if (bytes === undefined) throw new DOMException(`${name} is absent`, 'NotFoundError');
    return { getFile: async () => new Blob([bytes]) };
  }
}

/** The repository's root, found by walking up to its Cargo workspace. */
function repositoryRoot(from: string): string {
  let at = from;
  while (!statSync(join(at, 'Cargo.toml'), { throwIfNoEntry: false })?.isFile()) at = dirname(at);
  return at;
}

const ROOT = repositoryRoot(import.meta.dirname);

/** Every source file under `directory`, tests and generated messages aside. */
function sources(directory: string): string[] {
  return readdirSync(join(ROOT, directory), { recursive: true, encoding: 'utf8' })
    .map((path) => join(directory, path))
    .filter((path) => /\.(ts|js|svelte|rs|html|css|json)$/.test(path))
    .filter((path) => !/\.(test|spec)\./.test(path) && !path.includes('paraglide'));
}

describe('the media directory', () => {
  it('the worker reads each name the engine asks for, no further than its limit', async () => {
    const media = new FakeFolder(
      new Map([
        ['cat.mp3', new Uint8Array([1, 2, 3])],
        ['long.png', new Uint8Array([4, 5, 6, 7, 8])]
      ])
    );
    const root = new FakeFolder(new Map(), new Map([['deck-streak-media', media]]));
    const wanted = [
      { name: 'cat.mp3', limit: 9n },
      { name: 'gone.ogg', limit: 9n },
      { name: 'long.png', limit: 2n }
    ];
    expect(await readMedia({ getDirectory: async () => root }, 'deck-streak-media', wanted)).toEqual([
      { name: 'cat.mp3', bytes: new Uint8Array([1, 2, 3]) },
      { name: 'long.png', bytes: new Uint8Array([4, 5]) }
    ]);

    // no directory, and a storage that refuses, read as no file
    const empty = new FakeFolder();
    expect(await readMedia({ getDirectory: async () => empty }, 'deck-streak-media', wanted)).toEqual([]);
    const refused = () => Promise.reject(new DOMException('refused', 'SecurityError'));
    expect(await readMedia({ getDirectory: refused }, 'deck-streak-media', wanted)).toEqual([]);
    expect(await readMedia(undefined, 'deck-streak-media', wanted)).toEqual([]);
  });

  it('the media rules have one copy', () => {
    // The core's caps and the media types of its closed table are read from the core itself, so
    // this census holds no copy either.
    const core = readFileSync(join(ROOT, 'crates/engine-core/src/media.rs'), 'utf8');
    const caps = [...core.matchAll(/^pub const (?:FILE|FACE)_CAP: u64 = ([^;]+);$/gm)].map((match) => match[1]);
    const bytes = caps.map((cap) => String(cap.split('*').reduce((product, factor) => product * Number(factor), 1)));
    const table = /^pub const TYPES[^=]*= \[([^\]]*)\];$/m.exec(core)?.[1] ?? '';
    const types = [...new Set([...table.matchAll(/\("[^"]+", "([^"]+)"\)/g)].map((match) => match[1]))];
    const rules = [...caps, ...bytes, ...types];
    const copies = (text: string) => rules.filter((rule) => text.includes(rule));

    // planted copies of a cap, a cap's bytes and a media type are refused by name
    const planted: [string, string][] = [
      ['planted/cap.ts', `export const CAP = ${caps[0]};`],
      ['planted/face.rs', `const FACE: u64 = ${bytes[1]};`],
      ['planted/sound.svelte', `<source type="${types[types.length - 1]}" />`]
    ];
    for (const [path, text] of planted) expect(copies(text), path).not.toEqual([]);

    const files = [...sources('web/app/src'), ...sources('crates/web-engine/src')];
    console.log(`examined ${files.length} source file(s) against ${rules.length} rule(s)`);
    expect(files).toContain(join('web/app/src/lib/engine', 'media.ts'));
    expect(files).toContain(join('crates/web-engine/src', 'wasm.rs'));
    expect(caps).toHaveLength(2);
    const found = files.flatMap((path) =>
      copies(readFileSync(join(ROOT, path), 'utf8')).map((rule) => `${path}: ${rule}`)
    );
    expect(found).toEqual([]);
  });
});
