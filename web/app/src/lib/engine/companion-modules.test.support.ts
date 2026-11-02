// SPEC-511 C9 to C11: the companion modules the loader's tests serve, assembled here byte by byte,
// so no test reads a module a toolchain built. Each exports the raw byte-frame ABI, `memory`,
// `alloc`, `free`, `open` and `call`, unless a plant drops one; `open` and `call` answer their
// own input back, so a reply proves which bytes went in.

/** An unsigned LEB128 number. */
function leb(value: number): number[] {
  const out: number[] = [];
  let rest = value;
  do {
    let byte = rest & 0x7f;
    rest >>>= 7;
    if (rest !== 0) byte |= 0x80;
    out.push(byte);
  } while (rest !== 0);
  return out;
}

/** A name: its length, then its UTF-8 bytes. */
function name(text: string): number[] {
  const bytes = [...new TextEncoder().encode(text)];
  return [...leb(bytes.length), ...bytes];
}

/** A vector: its count, then its items. */
function vector(items: number[][]): number[] {
  return [...leb(items.length), ...items.flat()];
}

/** A section: its id, its size, then its body. */
function section(id: number, body: number[]): number[] {
  return [id, ...leb(body.length), ...body];
}

const I32 = 0x7f;
const I64 = 0x7e;
const END = 0x0b;

/** `alloc(len) -> ptr`: a bump allocator over the one global, starting at 16. */
const ALLOC = [0x23, 0, 0x23, 0, 0x20, 0, 0x6a, 0x24, 0, END];
/** `free(ptr, len)`: counts each call in the word at address 0. */
const FREE = [0x41, 0, 0x41, 0, 0x28, 2, 0, 0x41, 1, 0x6a, 0x36, 2, 0, END];
/** `open` and `call`: the input's own pointer and length, packed high and low. */
const ECHO = [0x20, 0, 0xad, 0x42, 32, 0x86, 0x20, 1, 0xad, 0x84, END];
/** A body that traps at once. */
const TRAP = [0x00, END];
/** `open` that answers 4 bytes at 2 MiB, far past the module's 256 KiB of memory. */
const OUT_OF_BOUNDS = [0x42, ...[0x84, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x10], END];

/** What a planted module changes from the echo module. */
export interface Plant {
  /** An import the module asks for, as `[module, name]`. */
  imports?: [string, string];
  /** An export the module does not have. */
  drop?: 'memory' | 'alloc' | 'free' | 'open' | 'call';
  /** `open`'s body, when not the echo. */
  open?: 'trap' | 'out-of-bounds';
  /** `call`'s body, when not the echo. */
  call?: 'trap';
  /** One more byte in a custom section, so two modules differ by one byte and nothing else. */
  marker?: number;
  /** The module's whole length, reached by a padding custom section. */
  padTo?: number;
}

const BODIES = { trap: TRAP, 'out-of-bounds': OUT_OF_BOUNDS } as const;

/** A companion module: the echo module, with `plant`'s changes. */
export function companionModule(plant: Plant = {}): Uint8Array {
  // types: 0 alloc (i32) -> i32; 1 free (i32, i32) -> (); 2 open and call (i32, i32) -> i64; 3 () -> ()
  const types = section(
    1,
    vector([
      [0x60, ...vector([[I32]]), ...vector([[I32]])],
      [0x60, ...vector([[I32], [I32]]), ...vector([])],
      [0x60, ...vector([[I32], [I32]]), ...vector([[I64]])],
      [0x60, ...vector([]), ...vector([])]
    ])
  );
  const imported = plant.imports === undefined ? 0 : 1;
  const imports =
    plant.imports === undefined ? [] : section(2, vector([[...name(plant.imports[0]), ...name(plant.imports[1]), 0x00, 3]]));
  const functions = section(3, vector([[0], [1], [2], [2]]));
  const memory = section(5, vector([[0x00, 4]]));
  const globals = section(6, vector([[I32, 0x01, 0x41, 16, END]]));
  const exported: [string, number, number][] = [
    ['memory', 0x02, 0],
    ['alloc', 0x00, imported + 0],
    ['free', 0x00, imported + 1],
    ['open', 0x00, imported + 2],
    ['call', 0x00, imported + 3]
  ];
  const exports = section(
    7,
    vector(exported.filter(([what]) => what !== plant.drop).map(([what, kind, index]) => [...name(what), kind, ...leb(index)]))
  );
  const body = (instructions: readonly number[]) => {
    const whole = [0x00, ...instructions];
    return [...leb(whole.length), ...whole];
  };
  const open = plant.open === undefined ? ECHO : BODIES[plant.open];
  const call = plant.call === undefined ? ECHO : BODIES[plant.call];
  const code = section(10, vector([body(ALLOC), body(FREE), body(open), body(call)]));
  const marker = plant.marker === undefined ? [] : section(0, [...name('marker'), plant.marker]);
  const module = [
    ...[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00],
    ...types,
    ...imports,
    ...functions,
    ...memory,
    ...globals,
    ...exports,
    ...code,
    ...marker
  ];
  return plant.padTo === undefined ? new Uint8Array(module) : padded(module, plant.padTo);
}

/** `module` with a custom section `pad` of zero bytes after it, so its whole length is `length`. */
function padded(module: number[], length: number): Uint8Array {
  const label = name('pad');
  for (let width = 1; width <= 5; width += 1) {
    const zeros = length - module.length - 1 - width - label.length;
    if (zeros >= 0 && leb(label.length + zeros).length === width) {
      const out = new Uint8Array(length);
      out.set(module);
      out.set([0, ...leb(label.length + zeros), ...label], module.length);
      return out;
    }
  }
  throw new Error(`no padding makes the module ${length} bytes long`);
}
