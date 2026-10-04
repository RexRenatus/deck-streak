// The planted cards of SPEC-341's suite (R8 to R11, ADR-352 D7): one card for every channel the
// schematic's web table names (docs/schematics/card-frame-channels.md, section 3), the layers the
// single-layer variants remove, and the engines and cards whose reference frame reaches no listener
// a test can own. A stub until the suite is written: every table is empty.

/** An engine the suite runs in: Playwright's project name. */
export type Engine = 'chromium' | 'webkit';

/** A web layer of the card frame (the schematic's section 3). */
export type Layer = 'W1' | 'W2' | 'W3' | 'W4';

/** Where a planted card's markup points: the listeners' addresses, from the test. */
export interface Listener {
  /** The HTTP listener's origin, counting requests by path. */
  http: string;
  /** The TCP listener's origin, counting connections that carry no request (a preconnect). */
  tcp: string;
  /** The UDP listener's port on the loopback address, counting datagrams (a STUN request). */
  udp: number;
}

/** One channel's planted card. */
export interface PlantedCard {
  /** The schematic's channel id. */
  id: string;
  /** The one layer whose removal alone opens the channel, or null when two or more hold it. */
  alone: Layer | null;
  /** Whether the card acts only when its full-frame link or button is clicked. */
  click: boolean;
  /** The card's markup, pointed at the listeners. */
  html(listener: Listener): string;
  /** The card's CSS, pointed at the listeners. */
  css(listener: Listener): string;
  /** The arrivals that show the channel open: request paths, `tcp`, `udp` and `bridge:*` counters. */
  paths: readonly string[];
}

/** An engine and card whose reference frame reaches nothing a listener can count, and why. */
export interface Unobservable {
  engine: Engine;
  id: string;
  why: string;
}

/** The render-proof card: text, a `data:` image and a table, and no channel. */
export interface RenderCard {
  html: string;
  css: string;
}

export const PLANTED: readonly PlantedCard[] = [];

export const LAYERS: readonly Layer[] = [];

export const UNOBSERVABLE: readonly Unobservable[] = [];

export const RENDER: RenderCard | null = null;
