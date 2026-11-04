// SPEC-593 R8, ADR-616 D1: the web colour mode. The learner's choice, `system`, `light` or `dark`,
// is kept on the device under one local-storage key. Until the app mounts the stylesheet alone
// paints the system mode (the `prefers-color-scheme` rule in `layout.css`); after mount the root
// layout calls `applyStoredColourMode`, which sets the stored `light` or `dark` as a class on the
// root element and clears both for `system`. No script enters the page shell for any of this.

/** The local-storage key the choice is kept under. */
export const COLOUR_MODE_KEY = 'deckstreak.colour-mode';

/** The three choices, the first the default. */
export const COLOUR_MODES = ['system', 'light', 'dark'] as const;
export type ColourMode = (typeof COLOUR_MODES)[number];

/** The part of the root element the mode touches: its classes. */
export interface ModeRoot {
  readonly classList: Pick<DOMTokenList, 'add' | 'remove'>;
}

/** The stored choice. The inert seam: it reads nothing yet and answers the default. */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function readColourMode(storage: Pick<Storage, 'getItem'> | undefined): ColourMode {
  return COLOUR_MODES[0];
}

/** Sets `mode` on the root element. The inert seam: it applies no mode yet. */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function applyColourMode(root: ModeRoot, mode: ColourMode): void {}

/** Applies the stored choice to the root element after mount, and answers it. */
export function applyStoredColourMode(root: ModeRoot, storage: Pick<Storage, 'getItem'> | undefined): ColourMode {
  const mode = readColourMode(storage);
  applyColourMode(root, mode);
  return mode;
}

/** Keeps `mode` as the learner's choice and applies it. The inert seam: it keeps nothing yet. */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function storeColourMode(storage: Pick<Storage, 'setItem'> | undefined, root: ModeRoot, mode: ColourMode): void {}
