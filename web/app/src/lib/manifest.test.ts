import { readFileSync } from 'node:fs';
import { crc32 } from 'node:zlib';
import { describe, expect, it } from 'vitest';

// SPEC-350 R17, A28, as SPEC-579 amends it (R3, A13 and A14); ADR-361 D9 and D14; ADR-598 D4. The
// Home Screen: a web app manifest the browser's install criteria accept (a name, a short name, a 192
// and a 512 pixel icon, the start URL and a standalone display), a 180 pixel touch icon and the
// DeckStreak logo as the favicon, each a static file the site serves and the shell's head links.
// The files come from the DeckStreak brand source byte for byte, which test_brand_files.py holds
// (SPEC-579 A1); this test reads what a browser reads: the manifest's entries, each icon's size from
// its own header, both favicons and the head's links. There is no service worker, and the page
// policy does not change (csp.test.ts).
const STATIC = new URL('../../static/', import.meta.url);
const SHELL = readFileSync(new URL('../app.html', import.meta.url), 'utf8');
const PNG = '89504e470d0a1a0a';

interface Icon {
  src: string;
  sizes: string;
  type: string;
  purpose: string;
}

interface Manifest {
  icons?: Icon[];
}

// The brand source's three manifest entries: 192 and 512 pixels for any purpose, and 512 maskable.
const EXPECTED: Icon[] = [
  { src: 'icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'any' },
  { src: 'icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any' },
  { src: 'icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' }
];

// The shell's head links, in their order: the manifest, the touch icon and the vector favicon.
const LINKS = [
  '<link rel="manifest" href="%sveltekit.assets%/manifest.webmanifest" />',
  '<link rel="apple-touch-icon" href="%sveltekit.assets%/apple-touch-icon.png" />',
  '<link rel="icon" href="%sveltekit.assets%/favicon.svg" type="image/svg+xml" />'
];

/** A file the app serves from `static/`, or null where the tree holds none. */
function served(name: string): Buffer | null {
  try {
    return readFileSync(new URL(name, STATIC));
  } catch {
    return null;
  }
}

/**
 * A PNG's width and height, read from its header chunk, when it opens with the PNG signature, every
 * chunk's checksum holds, the 13-byte header chunk comes first, the empty end chunk comes last with at
 * least one image data chunk before it, and the header names 8 bits per channel in colour type 2 (red,
 * green and blue) or 6 (with alpha); anything else has none.
 */
function size(file: Buffer | null): number[] | null {
  if (file === null || file.subarray(0, 8).toString('hex') !== PNG) return null;
  const chunks: { kind: string; data: Buffer }[] = [];
  for (let at = 8; at < file.length; ) {
    if (at + 12 > file.length) return null;
    const length = file.readUInt32BE(at);
    if (at + 12 + length > file.length) return null;
    const body = file.subarray(at + 4, at + 8 + length);
    if (crc32(body) !== file.readUInt32BE(at + 8 + length)) return null;
    chunks.push({ kind: body.subarray(0, 4).toString('latin1'), data: body.subarray(4) });
    at += 12 + length;
  }
  const kinds = chunks.map((chunk) => chunk.kind);
  if (kinds[0] !== 'IHDR' || chunks[0].data.length !== 13) return null;
  if (kinds[kinds.length - 1] !== 'IEND' || chunks[chunks.length - 1].data.length !== 0) return null;
  if (!kinds.includes('IDAT')) return null;
  const header = chunks[0].data;
  // 8 bits per channel, colour type 2 (red, green and blue) or 6 (red, green, blue and alpha)
  if (header[8] !== 8 || (header[9] !== 2 && header[9] !== 6)) return null;
  return [header.readUInt32BE(0), header.readUInt32BE(4)];
}

/** Each of the brand source's entries the manifest lacks, and each entry it holds beyond them. */
function iconFindings(manifest: Manifest | null): string[] {
  const key = (icon: Icon) => [icon.src, icon.sizes, icon.type, icon.purpose].join(' ');
  const held = (manifest?.icons ?? []).map(key);
  const wanted = EXPECTED.map(key);
  return [
    ...EXPECTED.filter((icon) => !held.includes(key(icon))).map((icon) => `missing: ${icon.src} (${key(icon)})`),
    ...(manifest?.icons ?? []).filter((icon) => !wanted.includes(key(icon))).map((icon) => `unexpected: ${key(icon)}`)
  ];
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

describe('the install surface', () => {
  it('the manifest meets the install criteria', () => {
    const file = served('manifest.webmanifest');
    const manifest: Manifest | null = file === null ? null : JSON.parse(file.toString('utf8'));
    expect(manifest).toEqual({
      name: 'DeckStreak',
      short_name: 'DeckStreak',
      start_url: '/study',
      display: 'standalone',
      icons: EXPECTED
    });
    expect(iconFindings(manifest)).toEqual([]);

    // each icon is the size it declares, read from the static file the site serves at its path, and
    // the touch icon is 180 pixels
    for (const icon of examined('manifest icons', manifest?.icons ?? [])) {
      expect(size(served(icon.src)), icon.src).toEqual(icon.sizes.split('x').map(Number));
    }
    expect(size(served('apple-touch-icon.png'))).toEqual([180, 180]);

    // both favicons: the icon file with its two images, and the vector icon
    expect(served('favicon.ico')?.subarray(0, 6).toString('hex')).toBe('000001000200');
    expect(served('favicon.svg')?.toString('utf8')).toContain('<svg');

    // the shell's head links all three and no other, and no endpoint answers at an icon's path
    const head = SHELL.slice(SHELL.indexOf('<head>'), SHELL.indexOf('</head>'));
    expect(head.match(/<link\b[^>]*>/g)).toEqual(examined('head links', LINKS));
    expect(Object.keys(import.meta.glob('../routes/*.png/+server.ts'))).toEqual([]);
  });

  it('a manifest missing an icon entry is refused', () => {
    const complete: Manifest = { icons: EXPECTED };
    expect(iconFindings(complete), 'the complete manifest is the control').toEqual([]);
    const planted: Manifest = { icons: EXPECTED.filter((icon) => icon.purpose !== 'maskable') };
    const found = iconFindings(planted);
    expect(found).toHaveLength(1);
    expect(found[0]).toContain('icon-maskable-512.png');
  });
});
