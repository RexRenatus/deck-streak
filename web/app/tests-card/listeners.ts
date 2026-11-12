// The suite's listeners (SPEC-341 R8, SPEC-461 R2): two HTTP servers and a UDP socket on the
// loopback address, each on a port the system chose, counting what really arrives. A request counts
// by its path at either server; the TCP listener, which the sentinel never reaches, also counts each
// connection it accepts as `tcp`, with a request or without one (a preconnect); and a datagram (a
// STUN request) counts on the socket. The suite reads every arrival here and never from the browser's
// own request events, which a channel outside the page's fetch machinery would not raise.
import { createSocket, type Socket as Datagrams } from 'node:dgram';
import { once } from 'node:events';
import { createServer as createHttpServer, type Server as HttpServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import type { Page } from '@playwright/test';
import type { Listener } from './planted';

const LOOPBACK = '127.0.0.1';
/** The path prefix of the harness page's own sentinel requests, which no card can name. */
const SENTINEL = '/sentinel/';

/** One reading: every arrival key with a count above zero (request paths, `tcp`, `udp`, `bridge:*`). */
export type Reading = Record<string, number>;

/** The harness page's bridge counters, as `main.ts` publishes them on `window.__counts`. */
export interface BridgeCounts {
  call: number;
  message: number;
  broadcast: number;
  storage: number;
  port: number;
  /** Each window message's `event.origin`, in arrival order. */
  origins: string[];
  /** Whether each window message's `event.source` was the card frame's `contentWindow`. */
  fromFrame: boolean[];
}

export class Listeners {
  private requests = new Map<string, number>();
  private connections = 0;
  private datagrams = 0;
  private sentinels = 0;

  private constructor(
    private readonly http: HttpServer,
    private readonly tcp: HttpServer,
    private readonly udp: Datagrams
  ) {
    http.on('request', (request, response) => {
      const path = new URL(request.url ?? '/', 'http://listener').pathname;
      if (path.startsWith(SENTINEL)) this.sentinels += 1;
      else this.requests.set(path, (this.requests.get(path) ?? 0) + 1);
      response.writeHead(200, { 'content-type': 'text/plain', 'cache-control': 'no-store', connection: 'close' });
      response.end();
    });
    tcp.on('connection', () => {
      this.connections += 1;
    });
    // every request by its path, with no sentinel branch: the sentinel is the HTTP listener's alone
    tcp.on('request', (request, response) => {
      const path = new URL(request.url ?? '/', 'http://listener').pathname;
      this.requests.set(path, (this.requests.get(path) ?? 0) + 1);
      response.writeHead(200, { 'content-type': 'text/plain', 'cache-control': 'no-store', connection: 'close' });
      response.end();
    });
    udp.on('message', () => {
      this.datagrams += 1;
    });
  }

  /** Starts the three listeners on the loopback address. */
  static async start(): Promise<Listeners> {
    const http = createHttpServer();
    const tcp = createHttpServer();
    const udp = createSocket('udp4');
    http.listen(0, LOOPBACK);
    tcp.listen(0, LOOPBACK);
    udp.bind(0, LOOPBACK);
    await Promise.all([once(http, 'listening'), once(tcp, 'listening'), once(udp, 'listening')]);
    return new Listeners(http, tcp, udp);
  }

  /** The addresses a planted card points at. */
  get address(): Listener {
    return {
      http: `http://${LOOPBACK}:${(this.http.address() as AddressInfo).port}`,
      tcp: `http://${LOOPBACK}:${(this.tcp.address() as AddressInfo).port}`,
      udp: this.udp.address().port
    };
  }

  /** The harness page's query for `card`, carrying the listeners' addresses. */
  query(card: string): string {
    const { http, tcp, udp } = this.address;
    return new URLSearchParams({ card, http, tcp, udp: String(udp) }).toString();
  }

  /** Forgets every arrival, before a visit. */
  reset(): void {
    this.requests.clear();
    this.connections = 0;
    this.datagrams = 0;
    this.sentinels = 0;
  }

  /**
   * What reached the listeners and the page's bridge since the last reset: every key with a count
   * above zero. A page that navigated away from the harness has no bridge counters to read.
   */
  async reading(page: Page): Promise<Reading> {
    const found: Reading = Object.fromEntries(this.requests);
    if (this.connections > 0) found.tcp = this.connections;
    if (this.datagrams > 0) found.udp = this.datagrams;
    const bridge = await this.bridge(page);
    for (const key of ['call', 'message', 'broadcast', 'storage', 'port'] as const) {
      if ((bridge?.[key] ?? 0) > 0) found[`bridge:${key}`] = bridge?.[key] ?? 0;
    }
    return found;
  }

  /** Whether `reading` holds every one of `paths`: a request path by prefix, any other key exactly. */
  static holds(reading: Reading, paths: readonly string[]): boolean {
    const keys = Object.keys(reading);
    return paths.every((path) => (path.startsWith('/') ? keys.some((key) => key.startsWith(path)) : (reading[path] ?? 0) > 0));
  }

  /** The harness page's bridge counters, or null when the page is no longer the harness. */
  async bridge(page: Page): Promise<BridgeCounts | null> {
    return page.evaluate(() => (window as unknown as { __counts?: BridgeCounts }).__counts ?? null).catch(() => null);
  }

  /**
   * Proves the HTTP listener still counts after a settle window: the harness page itself requests a
   * sentinel path, and the call returns once that request has arrived.
   */
  async sentinel(page: Page, timeout: number): Promise<void> {
    const url = `${this.address.http}${SENTINEL}${Date.now()}`;
    await page.evaluate((href) => {
      new Image().src = href;
    }, url);
    const deadline = Date.now() + timeout;
    while (this.sentinels === 0) {
      if (Date.now() > deadline) throw new Error(`the sentinel ${url} never arrived: the listener was not counting`);
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }

  async close(): Promise<void> {
    this.http.closeAllConnections();
    this.tcp.closeAllConnections();
    await Promise.all([
      new Promise((resolve) => this.http.close(resolve)),
      new Promise((resolve) => this.tcp.close(resolve)),
      new Promise((resolve) => this.udp.close(() => resolve(undefined)))
    ]);
  }
}
