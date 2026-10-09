// The planted cards of SPEC-341's suite (R8 to R11, ADR-352 D7): one card for every channel the
// schematic's web table names (docs/schematics/card-frame-channels.md, section 3), the layers the
// single-layer variants remove, the engines and cards whose reference frame reaches no listener a
// test can own, and the render-proof card. Each card points only at the suite's own listeners on
// the loopback address, and each of its arrivals carries the card's own path, so a reading names
// the channel that opened.

/** An engine the suite runs in: Playwright's project name. */
export type Engine = 'chromium' | 'webkit' | 'firefox';

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
  /**
   * The arrivals that show the channel open, every one of them: a request path prefix (`/img/`),
   * `tcp`, `udp`, or a `bridge:*` counter of the harness page.
   */
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

/** A 1x1 transparent image, for markup that needs an image which reaches nothing. */
const PIXEL = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';

/** A link or button laid over the whole frame, so a click at the frame's centre lands on it. */
const COVER = '.cover { position: fixed; inset: 0; display: block; margin: 0; }';

const none = (): string => '';

/** A card that loads from the HTTP listener under its own path, with no click. */
function loads(id: string, alone: Layer | null, html: (http: string) => string, css: (http: string) => string = none): PlantedCard {
  return { id, alone, click: false, html: (l) => html(`${l.http}/${id}`), css: (l) => css(`${l.http}/${id}`), paths: [`/${id}/`] };
}

/** A card whose full-frame link or button acts when clicked, toward the HTTP listener. */
function clicks(id: string, alone: Layer | null, html: (http: string) => string): PlantedCard {
  return { id, alone, click: true, html: (l) => html(`${l.http}/${id}`), css: () => COVER, paths: [`/${id}/`] };
}

/** The page's bridge, as a card script reaches for it: each call that throws is passed over. */
export const BRIDGE_STEPS = `for (const step of [
  () => parent.bridge(),
  () => parent.postMessage('card', '*'),
  () => top.postMessage('card', '*'),
  () => new BroadcastChannel('deck-streak').postMessage('card'),
  () => localStorage.setItem('deck-streak-card', 'card'),
  () => parent.enginePort.postMessage('card')
]) {
  try { step(); } catch { /* the step is closed to this frame */ }
}`;

/** A peer connection whose STUN server is the UDP listener: gathering sends it a datagram. */
export function peerConnection(host: string, port: number): string {
  return `const peer = new RTCPeerConnection({ iceServers: [{ urls: 'stun:${host}:${port}' }] });
peer.createDataChannel('card');
peer.createOffer().then((offer) => peer.setLocalDescription(offer));`;
}

export const PLANTED: readonly PlantedCard[] = [
  loads(
    'img',
    'W2',
    (h) =>
      `<img alt="" src="${h}/1"><img alt="" srcset="${h}/2 1x">` +
      `<picture><source srcset="${h}/3"><img alt=""></picture><input type="image" alt="go" src="${h}/4">` +
      `<svg width="10" height="10"><image href="${h}/5" width="10" height="10"/><use href="${h}/6#a"/></svg>` +
      `<video poster="${h}/7"></video>`
  ),
  loads(
    'css-url',
    'W2',
    (h) => `<div class="framed" style="background-image: url('${h}/1')">styled</div><ul><li>item</li></ul>`,
    (h) =>
      `body { background-image: image-set(url('${h}/2') 1x); cursor: url('${h}/3'), auto; }\n` +
      `li { list-style-image: url('${h}/4'); }\n` +
      `.framed { border: 10px solid; border-image: url('${h}/5') 30 round; }`
  ),
  loads(
    'css-import',
    'W2',
    () => '<p>imported</p>',
    (h) => `@import url('${h}/1');`
  ),
  loads(
    'font',
    'W2',
    () => '<p>a planted font</p>',
    (h) => `@font-face { font-family: planted; src: url('${h}/1'); }\nbody { font-family: planted; }`
  ),
  loads(
    'media',
    'W2',
    (h) =>
      `<audio preload="auto" src="${h}/1"></audio><video preload="auto" src="${h}/2"></video>` +
      `<video><track default kind="captions" src="${h}/3"></video>`
  ),
  loads('stylesheet', null, (h) => `<link rel="stylesheet" href="${h}/1"><p>linked</p>`),
  loads('preload', null, (h) => `<link rel="preload" as="image" href="${h}/1"><link rel="modulepreload" href="${h}/2"><p>preloaded</p>`),
  loads('prefetch', null, (h) => `<link rel="prefetch" href="${h}/1"><p>prefetched</p>`),
  {
    id: 'preconnect',
    alone: 'W3',
    click: false,
    html: (l) => `<link rel="preconnect" href="${l.tcp}"><p>connected</p>`,
    css: none,
    paths: ['tcp']
  },
  {
    id: 'dns-prefetch',
    alone: 'W3',
    click: false,
    html: (l) => `<link rel="dns-prefetch" href="${l.tcp}"><p>resolved</p>`,
    css: none,
    paths: ['tcp']
  },
  {
    id: 'shadow-link',
    alone: 'W3',
    click: false,
    html: (l) => `<div><template shadowrootmode="open"><link rel="preconnect" href="${l.tcp}"><p>shadowed</p></template></div>`,
    css: none,
    paths: ['tcp']
  },
  loads('meta-refresh', null, (h) => `<meta http-equiv="refresh" content="0; url=${h}/1"><p>refreshing</p>`),
  loads('base', null, (h) => `<base href="${h}/"><img alt="" src="1">`),
  clicks('nav-self', 'W4', (h) => `<a class="cover" href="${h}/1">open</a>`),
  clicks('nav-top', 'W1', (h) => `<a class="cover" target="_top" href="${h}/1">open</a>`),
  clicks('nav-blank', 'W1', (h) => `<a class="cover" target="_blank" href="${h}/1">open</a>`),
  clicks('download', 'W4', (h) => `<a class="cover" download="card.txt" href="${h}/1">save</a>`),
  clicks('ping', null, (h) => `<a class="cover" href="#planted" ping="${h}/1">ping</a><p id="planted">here</p>`),
  clicks('form', null, (h) => `<form method="post" action="${h}/1"><button class="cover" type="submit">send</button></form>`),
  loads(
    'nested-frame',
    'W2',
    (h) => `<iframe title="nested" src="${h}/1"></iframe><iframe title="nested document" srcdoc="<img alt='' src='${h}/2'>"></iframe>`
  ),
  clicks('external-scheme', null, () => '<a class="cover" href="mailto:card@example.invalid">mail</a>'),
  loads('object', null, (h) => `<object type="text/html" data="${h}/1"></object><embed type="text/html" src="${h}/2">`),
  loads('script-inline', null, (h) => `<script>new Image().src = '${h}/1';</script>`),
  loads('script-src', null, (h) => `<script src="${h}/1"></script>`),
  loads('event-handler', null, (h) => `<img alt="" src="${PIXEL}" onload="new Image().src = '${h}/1'">`),
  clicks('javascript-url', null, (h) => `<a class="cover" href="javascript:new Image().src = '${h}/1'; void 0">run</a>`),
  {
    id: 'webrtc',
    alone: null,
    click: false,
    html: (l) => `<script>${peerConnection(new URL(l.http).hostname, l.udp)}</script>`,
    css: none,
    paths: ['udp']
  },
  {
    id: 'bridge',
    alone: null,
    click: false,
    html: () => `<script>${BRIDGE_STEPS}</script>`,
    css: none,
    paths: ['bridge:call', 'bridge:message', 'bridge:broadcast', 'bridge:storage', 'bridge:port']
  }
];

export const LAYERS: readonly Layer[] = ['W1', 'W2', 'W3', 'W4'];

export const UNOBSERVABLE: readonly Unobservable[] = [
  { engine: 'chromium', id: 'dns-prefetch', why: 'a DNS lookup reaches no listener a test owns' },
  { engine: 'webkit', id: 'dns-prefetch', why: 'a DNS lookup reaches no listener a test owns' },
  { engine: 'webkit', id: 'prefetch', why: 'measured: WebKit under the suite sends no prefetch request' },
  { engine: 'chromium', id: 'preconnect', why: 'measured: Chromium under the suite opens no preconnect connection' },
  { engine: 'chromium', id: 'shadow-link', why: 'measured: Chromium under the suite opens no preconnect connection, in a shadow tree or out of one' },
  { engine: 'chromium', id: 'external-scheme', why: 'no listener sees a mail handler launch' },
  { engine: 'webkit', id: 'external-scheme', why: 'no listener sees a mail handler launch' }
];

/** The render-proof card: it points nowhere, so the frame shows it whole or not at all. */
export const RENDER: RenderCard | null = {
  html:
    '<p>Der Hund: the dog</p>' +
    '<img alt="a red square" width="40" height="40" src="data:image/gif;base64,R0lGODlhAQABAIABAP8AAP///ywAAAAAAQABAAACAkQBADs=">' +
    '<table><tbody><tr><td>der</td><td>Hund</td></tr><tr><td>die</td><td>Katze</td></tr></tbody></table>',
  css: 'body { font: 16px sans-serif; color: #123; background: #fefae0; } td { border: 1px solid #333; padding: 4px; }'
};
