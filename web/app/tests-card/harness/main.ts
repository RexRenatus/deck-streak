// The card harness page (SPEC-341 R8 to R11). One script serves the three pages, by path:
// - index.html mounts the shipped CardFrame with the planted card, under the page policy;
// - open.html is the reference frame: a plain frame whose document is the raw card, every layer off;
// - variant.html removes one layer (off=W1 to W4), every other layer on, or turns card scripts on
//   for the measurement (off=scripts), or leaves the host policy alone (off=W3meta: the strip and the
//   card document's own policy meta both off). Every variant but the scripts one is built through
//   the host, as CardFrame builds it.
// Every page installs stand-ins for what a card must never reach: the bridge, window messages, a
// BroadcastChannel, the page's storage and a worker's port. It counts each and publishes the counts
// on window.__counts, which the suite reads through listeners.ts.
import { mount } from 'svelte';
import CardFrame from '../../src/lib/card/CardFrame.svelte';
import { frameDocument } from '../../src/lib/card/frame-document';
import { frameHost } from '../../src/lib/card/frame-host';
import { FRAME_POLICY, FRAME_SANDBOX } from '../../src/lib/card/policy.js';
import { PLANTED, RENDER, type Listener } from '../planted';

const counts = { call: 0, message: 0, broadcast: 0, storage: 0, port: 0, origins: [] as string[], fromFrame: [] as boolean[] };
const host = document.getElementById('card') as HTMLElement;
const frame = (): Window | null => host.querySelector('iframe')?.contentWindow ?? null;

Object.assign(window, {
  __counts: counts,
  bridge: () => {
    counts.call += 1;
  }
});
window.addEventListener('message', (event) => {
  counts.message += 1;
  counts.origins.push(event.origin);
  counts.fromFrame.push(event.source !== null && event.source === frame());
});
new BroadcastChannel('deck-streak').addEventListener('message', () => {
  counts.broadcast += 1;
});
window.addEventListener('storage', () => {
  counts.storage += 1;
});
const engine = new MessageChannel();
engine.port2.addEventListener('message', () => {
  counts.port += 1;
});
engine.port2.start();
Object.assign(window, { enginePort: engine.port1 });

const params = new URLSearchParams(location.search);
const listener: Listener = { http: params.get('http') ?? '', tcp: params.get('tcp') ?? '', udp: Number(params.get('udp')) };
const id = params.get('card') ?? '';
const planted = PLANTED.find((card) => card.id === id);
const card =
  id === 'render' && RENDER !== null
    ? RENDER
    : planted === undefined
      ? { html: '', css: '' }
      : { html: planted.html(listener), css: planted.css(listener) };
const page = location.pathname.split('/').pop();
const off = page === 'variant.html' ? params.get('off') : null;

/** The frame policy's meta element, as frameDocument writes it. */
const policyMeta = (policy: string): string => `<meta http-equiv="Content-Security-Policy" content="${policy}">`;
/** A frame document the harness composes itself, with `head` before the card's style. */
const composed = (head: string, css: string, body: string): string =>
  `<!doctype html><html><head>${head}<style>${css}</style></head><body>${body}</body></html>`;

/** The host `CardFrame` would build for `doc`, as the frame's `srcdoc`; a refusal fails the visit. */
const hostOf = (doc: string, policy?: string): string => {
  const hosted = frameHost(doc, 'The card', policy);
  if (hosted.srcdoc === undefined) throw new Error(`the host refused the card ${id}`);
  return hosted.srcdoc;
};

if (page === 'index.html' || off === 'W4') {
  // the shipped frame; with frame-src off, only the page header differs (vite.card.config.ts)
  mount(CardFrame, { target: host, props: { html: card.html, css: card.css, title: 'The card' } });
} else {
  const element = document.createElement('iframe');
  element.title = 'The card';
  const shipped = frameDocument(card.html, card.css).srcdoc ?? '';
  if (off !== null && off !== 'W1') element.setAttribute('sandbox', FRAME_SANDBOX);
  if (page === 'open.html') element.srcdoc = composed('', card.css, card.html);
  else if (off === 'W1') element.srcdoc = hostOf(shipped);
  else if (off === 'W2') element.srcdoc = hostOf(shipped.replace(policyMeta(FRAME_POLICY), ''), '');
  else if (off === 'W3') element.srcdoc = hostOf(composed(policyMeta(FRAME_POLICY), card.css, card.html));
  else if (off === 'W3meta') element.srcdoc = hostOf(composed('', card.css, card.html));
  else if (off === 'scripts') {
    // card scripts on, for the measurement: one admitted script source, the harness's own
    element.setAttribute('sandbox', 'allow-scripts');
    const script = `<script src="${location.origin}/planted/${id}.js" data-host="${new URL(listener.http).hostname}" data-port="${listener.udp}"></script>`;
    element.srcdoc = composed(policyMeta(`${FRAME_POLICY}; script-src ${location.origin}`), '', script);
  }
  host.append(element);
}
