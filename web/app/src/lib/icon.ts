// SPEC-350 R17, A28; ADR-361 D16. The Home Screen's icons are plain squares, built from this text
// when the site is built: each icon's path is an endpoint that answers `icon(side)`, and the build
// prerenders it into the file the site serves there. So the tree and its history hold no binary
// file, which the public scrub refuses. The image data is compressed by the platform's own deflate,
// so the app adds no dependency.

/** The icons' one colour, as 8-bit red, green and blue. */
const COLOUR = [15, 23, 42];

/** The eight bytes every PNG file opens with. */
const SIGNATURE = Uint8Array.of(0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a);

/** The CRC-32 step of each byte value, for the checksum every PNG chunk ends with. */
const CRC = Array.from({ length: 256 }, (_, byte) =>
  Array.from({ length: 8 }).reduce<number>((crc) => (crc & 1 ? 0xedb88320 ^ (crc >>> 1) : crc >>> 1), byte)
);

/** The CRC-32 of `bytes`. */
function crc32(bytes: Uint8Array): number {
  return (bytes.reduce((crc, byte) => CRC[(crc ^ byte) & 0xff] ^ (crc >>> 8), 0xffffffff) ^ 0xffffffff) >>> 0;
}

/** `value` as four bytes, most significant first, as PNG writes every number. */
function uint32(value: number): Uint8Array<ArrayBuffer> {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setUint32(0, value);
  return bytes;
}

/** The parts, one after another. */
function join(parts: Uint8Array[]): Uint8Array<ArrayBuffer> {
  const whole = new Uint8Array(parts.reduce((length, part) => length + part.length, 0));
  let at = 0;
  for (const part of parts) {
    whole.set(part, at);
    at += part.length;
  }
  return whole;
}

/** A PNG chunk: the length of its data, its kind, the data, and the checksum of the kind and data. */
function chunk(kind: string, data: Uint8Array): Uint8Array {
  const body = join([new TextEncoder().encode(kind), data]);
  return join([uint32(data.length), body, uint32(crc32(body))]);
}

/** A plain square `side` pixels wide and high: each row a filter byte of 0 (none), then its pixels. */
function rows(side: number): Uint8Array<ArrayBuffer> {
  const row = [0, ...Array.from({ length: side }, () => COLOUR).flat()];
  return Uint8Array.from(Array.from({ length: side }, () => row).flat());
}

/** `bytes` in the zlib format PNG's image data takes, compressed by the platform's own deflate. */
async function deflate(bytes: Uint8Array<ArrayBuffer>): Promise<Uint8Array> {
  const compressed = new Blob([bytes]).stream().pipeThrough(new CompressionStream('deflate'));
  return new Uint8Array(await new Response(compressed).arrayBuffer());
}

/**
 * What an icon's endpoint answers: a plain square `side` pixels wide and high, as a PNG of 8-bit red,
 * green and blue pixels with no interlace.
 */
export async function icon(side: number): Promise<Response> {
  const header = join([uint32(side), uint32(side), Uint8Array.of(8, 2, 0, 0, 0)]);
  const png = join([
    SIGNATURE,
    chunk('IHDR', header),
    chunk('IDAT', await deflate(rows(side))),
    chunk('IEND', new Uint8Array(0))
  ]);
  return new Response(png, { headers: { 'content-type': 'image/png' } });
}
