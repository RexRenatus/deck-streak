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

/** The device's local storage, or undefined where there is none or the browser refuses it. */
export function deviceStorage(): Storage | undefined {
  try {
    return typeof localStorage === 'undefined' ? undefined : localStorage;
  } catch {
    return undefined;
  }
}

/** The stored choice: `system` when nothing is stored, the value is unknown or storage is refused. */
export function readColourMode(storage: Pick<Storage, 'getItem'> | undefined): ColourMode {
  let stored: string | null;
  try {
    stored = storage === undefined ? null : storage.getItem(COLOUR_MODE_KEY);
  } catch {
    return COLOUR_MODES[0];
  }
  return COLOUR_MODES.find((mode) => mode === stored) ?? COLOUR_MODES[0];
}

/** Sets `mode` on the root element: its class for `light` or `dark`, and neither for `system`. */
export function applyColourMode(root: ModeRoot, mode: ColourMode): void {
  if (mode === 'light') {
    root.classList.remove('dark');
    root.classList.add('light');
  } else if (mode === 'dark') {
    root.classList.remove('light');
    root.classList.add('dark');
  } else {
    root.classList.remove('light', 'dark');
  }
}

/** Applies the stored choice to the root element after mount, and answers it. */
export function applyStoredColourMode(root: ModeRoot, storage: Pick<Storage, 'getItem'> | undefined): ColourMode {
  const mode = readColourMode(storage);
  applyColourMode(root, mode);
  return mode;
}

/**
 * Keeps `mode` as the learner's choice and applies it. Storage the browser refuses keeps nothing,
 * and the choice still holds on this page until it reloads.
 */
export function storeColourMode(storage: Pick<Storage, 'setItem'> | undefined, root: ModeRoot, mode: ColourMode): void {
  try {
    storage?.setItem(COLOUR_MODE_KEY, mode);
  } catch {
    // the choice is applied below all the same
  }
  applyColourMode(root, mode);
}
