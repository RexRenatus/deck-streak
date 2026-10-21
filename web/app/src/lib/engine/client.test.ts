import { describe, expect, it } from 'vitest';
import { EngineClient, EngineError } from './client';

// SPEC-338 A7: the page's client numbers each request and settles it by its reply's id alone.
class FakePort {
  sent: unknown[] = [];
  #listeners: ((event: MessageEvent) => void)[] = [];

  postMessage(message: unknown) {
    this.sent.push(message);
  }

  addEventListener(type: 'message', listener: (event: MessageEvent) => void) {
    expect(type).toBe('message');
    this.#listeners.push(listener);
  }

  reply(data: unknown, origin = '') {
    for (const listener of this.#listeners) listener(new MessageEvent('message', { data, origin }));
  }
}

const ORIGIN = 'https://app.example';

describe('EngineClient', () => {
  it('the client pairs each reply with its request and rejects an error with its code', async () => {
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const opened = client.open();
    const next = client.next();
    const rated = client.rate(1001n, 3, 1200);

    expect(port.sent).toEqual([
      { id: 1, op: 'open' },
      { id: 2, op: 'next' },
      { id: 3, op: 'rate', card: 1001n, rating: 3, ms: 1200 }
    ]);
    expect(client.waiting).toBe(3);
    // out of order, with a reply to an id never sent, a reply with no id and a message that is
    // not a reply at all among them
    port.reply({ id: 2, ok: true, value: 1001n });
    port.reply({ id: 99, ok: true, value: 'stray' });
    port.reply({ ok: true, value: 'no id' });
    port.reply(null);
    port.reply({ id: 3, ok: false, code: 'not-open', message: 'rate before open' });
    port.reply({ id: 1, ok: true, value: { existed: false, notes: 0 } });

    await expect(next).resolves.toBe(1001n);
    await expect(opened).resolves.toEqual({ existed: false, notes: 0 });
    const refused = await rated.then(
      () => 'resolved',
      (error: unknown) => error
    );
    expect(refused).toBeInstanceOf(EngineError);
    expect(refused).toBeInstanceOf(Error);
    expect(refused).toMatchObject({ code: 'not-open', message: 'rate before open' });
    expect(client.waiting).toBe(0);
  });

  it('each operation sends its own request', async () => {
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const calls = [
      client.seed(25),
      client.undo(1001n, 4),
      client.snapshot(1001n),
      client.close(),
      client.rate(1001n, 1, 0),
      client.undoOffer()
    ];

    // the undo names the answer an offer named, by its card and its step (SPEC-371 R12)
    expect(port.sent).toEqual([
      { id: 1, op: 'seed', count: 25 },
      { id: 2, op: 'undo', card: 1001n, step: 4 },
      { id: 3, op: 'snapshot', card: 1001n },
      { id: 4, op: 'close' },
      { id: 5, op: 'rate', card: 1001n, rating: 1, ms: 0 },
      { id: 6, op: 'undo-offer' }
    ]);
    for (const id of [1, 2, 3, 4, 5, 6]) port.reply({ id, ok: true, value: id * 10 });
    await expect(Promise.all(calls)).resolves.toEqual([10, 20, 30, 40, 50, 60]);
    // a second reply to a settled id settles nothing
    port.reply({ id: 1, ok: false, code: 'engine-failed', message: 'late' });
    expect(client.waiting).toBe(0);
  });

  it('memory asks for the linear memory and resolves to its bytes', async () => {
    // SPEC-338 A18: the page reads the Worker's memory through its own request
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const memory = client.memory();

    expect(port.sent).toEqual([{ id: 1, op: 'memory' }]);
    expect(client.waiting).toBe(1);
    port.reply({ id: 1, ok: true, value: 37683200 });
    await expect(memory).resolves.toBe(37683200);
    expect(client.waiting).toBe(0);
  });

  it('the client ignores a reply from another origin', async () => {
    // SPEC-338 A21: a reply that names another origin settles nothing; the empty origin a
    // dedicated Worker's channel carries, and the page's own origin, settle their request
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const first = client.next();
    const second = client.next();
    port.reply({ id: 1, ok: true, value: 7n }, 'https://other.example');
    expect(client.waiting).toBe(2);
    port.reply({ id: 1, ok: true, value: 1001n });
    port.reply({ id: 2, ok: true, value: 1002n }, ORIGIN);
    await expect(first).resolves.toBe(1001n);
    await expect(second).resolves.toBe(1002n);
    expect(client.waiting).toBe(0);
  });

  it('the client posts each study operation and settles its answer', async () => {
    // SPEC-350 A8: the review's operations, each with its own request, each settled by its reply
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const calls = [
      client.open(['ja', 'en']),
      client.decks(),
      client.study(9007199254740993n),
      client.card(),
      client.rate(1001n, 3, 1500),
      client.bury(1001n),
      client.flag(1001n)
    ];

    expect(port.sent).toEqual([
      { id: 1, op: 'open', languages: ['ja', 'en'] },
      { id: 2, op: 'decks' },
      { id: 3, op: 'study', deck: 9007199254740993n },
      { id: 4, op: 'card' },
      { id: 5, op: 'rate', card: 1001n, rating: 3, ms: 1500 },
      { id: 6, op: 'bury', card: 1001n },
      { id: 7, op: 'flag', card: 1001n }
    ]);
    const values = [
      { existed: true, notes: 2 },
      [{ id: 1n, name: 'Default', level: 1, new: 2, learning: 0, review: 0, children: [] }],
      null,
      { counts: { new: 0, learning: 0, review: 0 }, card: null },
      null,
      null,
      1
    ];
    values.forEach((value, at) => port.reply({ id: at + 1, ok: true, value }));
    await expect(Promise.all(calls)).resolves.toEqual(values);
    // a stale card's refusal settles as an EngineError with its code
    const stale = client.rate(1002n, 3, 0);
    port.reply({ id: 8, ok: false, code: 'not-shown', message: 'not-shown: the card is not the one on screen' });
    await expect(stale).rejects.toMatchObject({ code: 'not-shown' });
    expect(client.waiting).toBe(0);
  });

  // Mutation coverage: an open with no languages names none, as a key of its own or an undefined one.
  it('an open with no languages sends none', () => {
    const port = new FakePort();
    new EngineClient(port, ORIGIN).open();
    expect(port.sent).toStrictEqual([{ id: 1, op: 'open' }]);
  });

  it('faces sends the card and resolves to both faces', async () => {
    // SPEC-350 A24: the page asks for the shown card's faces by its id alone
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const faces = client.faces(1001n);
    expect(port.sent).toEqual([{ id: 1, op: 'faces', card: 1001n }]);
    const face = { text: '<img src="data:,">', css: '.card {}', autoplay: [], replay: [], omitted: ['gone.png'] };
    const value = { question: face, answer: face, wanted: [] };
    port.reply({ id: 1, ok: true, value }, ORIGIN);
    expect(await faces).toEqual(value);
  });

  it('the credential operations send no argument and resolve to a status word', async () => {
    // SPEC-363 R14, R15: the page asks the Worker for the credential's status and to forget it, and
    // hears back one word
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const status = client.credentialStatus();
    const forgotten = client.forgetSync();
    expect(port.sent).toEqual([
      { id: 1, op: 'credential-status' },
      { id: 2, op: 'credential-forget' }
    ]);
    port.reply({ id: 2, ok: true, value: 'absent' }, ORIGIN);
    port.reply({ id: 1, ok: true, value: 'held' }, ORIGIN);
    expect(await status).toBe('held');
    expect(await forgotten).toBe('absent');
  });

  it('the sync operations send the login and nothing for a sync, and resolve to their answers', async () => {
    // SPEC-364 R17, R18: the page sends the user and password once, to the Worker, and hears back a
    // status word; a sync carries nothing and hears back the status and what the collections need
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const login = client.syncLogin('a user', 'a password');
    const synced = client.sync();
    expect(port.sent).toEqual([
      { id: 1, op: 'sync-login', user: 'a user', password: 'a password' },
      { id: 2, op: 'sync' }
    ]);
    port.reply({ id: 2, ok: true, value: { status: 'held', required: 'full-upload' } }, ORIGIN);
    port.reply({ id: 1, ok: true, value: 'held' }, ORIGIN);
    expect(await login).toBe('held');
    expect(await synced).toEqual({ status: 'held', required: 'full-upload' });
  });

  it('the choice operations send the direction alone, and resolve to their answers', async () => {
    // SPEC-377 R6, R8: the page names a direction and nothing else, and hears back the counts, the
    // outcome, nothing for a cancel, and the unsynced count
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const counted = client.choiceCount();
    const confirmed = client.choiceConfirm('download');
    const cancelled = client.choiceCancel();
    const unsynced = client.unsynced();
    expect(port.sent).toEqual([
      { id: 1, op: 'choice-count' },
      { id: 2, op: 'choice-confirm', direction: 'download' },
      { id: 3, op: 'choice-cancel' },
      { id: 4, op: 'unsynced' }
    ]);
    const counts = { upload: null, download: { reviews: 0, cards: 1, notes: 1 } };
    port.reply({ id: 4, ok: true, value: { reviews: 2, changed: true, schema: false } }, ORIGIN);
    port.reply({ id: 3, ok: true, value: null }, ORIGIN);
    port.reply({ id: 2, ok: true, value: { status: 'held', outcome: 'written' } }, ORIGIN);
    port.reply({ id: 1, ok: true, value: { status: 'held', counts, snapshot: { found: false } } }, ORIGIN);
    expect(await counted).toEqual({ status: 'held', counts, snapshot: { found: false } });
    expect(await confirmed).toEqual({ status: 'held', outcome: 'written' });
    expect(await cancelled).toBeNull();
    expect(await unsynced).toEqual({ reviews: 2, changed: true, schema: false });
  });
});
