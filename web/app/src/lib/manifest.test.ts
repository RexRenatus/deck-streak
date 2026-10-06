import { readFileSync } from 'node:fs';
import { crc32, inflateSync } from 'node:zlib';
import { describe, expect, it } from 'vitest';

// SPEC-350 R17, A28; ADR-361 D9, D14, D16. The Home Screen: a web app manifest the browser's install
// criteria accept (a name, a short name, a 192 and a 512 pixel icon, the start URL and a standalone
// display), a 180 pixel touch icon, and both linked from the shell's head. There is no service
// worker, and the page policy does not change (csp.test.ts). Each icon is built from text when the
// site is built: an endpoint at the icon's path, which the build prerenders into the file the site
// serves there, so the tree holds no binary file (ADR-361 D16).
const STATIC = new URL('../../static/', import.meta.url);
const SHELL = readFileSync(new URL('../app.html', import.meta.url), 'utf8');
const ENDPOINTS = import.meta.glob<Endpoint>('../routes/*.png/+server.ts');
const PNG = '89504e470d0a1a0a';

/** What an endpoint module exports, as the build reads it. */
interface Endpoint {
  prerender?: unknown;
  GET?: () => Promise<Response>;
}

/** A file the app serves from `static/`, or null where the tree holds none. */
function served(name: string): Buffer | null {
  try {
    return readFileSync(new URL(name, STATIC));
  } catch {
    return null;
  }
}

/**
 * The PNG the built site holds at `/<name>`: the answer of the endpoint at that path, which the build
 * prerenders, or null where no prerendered endpoint answers a PNG there.
 */
async function built(name: string): Promise<Buffer | null> {
  const load = ENDPOINTS[`../routes/${name}/+server.ts`];
  if (load === undefined) return null;
  const endpoint = await load();
  if (endpoint.prerender !== true || endpoint.GET === undefined) return null;
  const answer = await endpoint.GET();
  if (answer.headers.get('content-type') !== 'image/png') return null;
  return Buffer.from(await answer.arrayBuffer());
}

/**
 * A PNG's width and height, read from its header chunk, when every chunk's checksum holds and its one
 * image data chunk inflates to rows of that many 8-bit red, green and blue pixels, each row unfiltered
 * and every pixel one colour; anything else has none.
 */
function pixels(file: Buffer | null): number[] | null {
  if (file === null || file.subarray(0, 8).toString('hex') !== PNG) return null;
  const chunks: { kind: string; data: Buffer }[] = [];
  for (let at = 8; at < file.length; ) {
    const length = file.readUInt32BE(at);
    const body = file.subarray(at + 4, at + 8 + length);
    if (crc32(body) !== file.readUInt32BE(at + 8 + length)) return null;
    chunks.push({ kind: body.subarray(0, 4).toString('latin1'), data: body.subarray(4) });
    at += 12 + length;
  }
  if (chunks.map((chunk) => chunk.kind).join(' ') !== 'IHDR IDAT IEND') return null;
  const [header, image, end] = chunks.map((chunk) => chunk.data);
  if (header.length !== 13 || end.length !== 0) return null;
  // 8 bits per channel, colour type 2 (red, green and blue), the standard compression and filter, no interlace
  if (header.subarray(8).toString('hex') !== '0802000000') return null;
  const [width, height] = [header.readUInt32BE(0), header.readUInt32BE(4)];
  const rows = inflateSync(image);
  const pixel = rows.subarray(1, 4);
  const row = Buffer.concat([Buffer.of(0), ...Array.from({ length: width }, () => pixel)]);
  if (!rows.equals(Buffer.concat(Array.from({ length: height }, () => row)))) return null;
  return [width, height];
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

interface Icon {
  src: string;
  sizes: string;
  type: string;
}

describe('the install surface', () => {
  it('the manifest meets the install criteria', async () => {
    const file = served('manifest.webmanifest');
    const manifest: { icons: Icon[] } | null = file === null ? null : JSON.parse(file.toString('utf8'));
    expect(manifest).toEqual({
      name: 'DeckStreak',
      short_name: 'DeckStreak',
      start_url: '/study',
      display: 'standalone',
      icons: [
        { src: '/icon-192.png', sizes: '192x192', type: 'image/png' },
        { src: '/icon-512.png', sizes: '512x512', type: 'image/png' }
      ]
    });

    // each icon is the size it declares, read from the PNG the built site holds at its path, and the
    // touch icon is 180 pixels
    for (const icon of examined('manifest icons', manifest?.icons ?? [])) {
      expect(pixels(await built(icon.src.slice(1))), icon.src).toEqual(icon.sizes.split('x').map(Number));
    }
    expect(pixels(await built('apple-touch-icon.png'))).toEqual([180, 180]);

    // the shell's head links both
    const head = SHELL.slice(SHELL.indexOf('<head>'), SHELL.indexOf('</head>'));
    expect(head).toContain('<link rel="manifest" href="%sveltekit.assets%/manifest.webmanifest" />');
    expect(head).toContain('<link rel="apple-touch-icon" href="%sveltekit.assets%/apple-touch-icon.png" />');
  });
});
