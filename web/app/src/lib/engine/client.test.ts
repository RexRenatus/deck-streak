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
    const answered = client.answer(3, 1200);

    expect(port.sent).toEqual([
      { id: 1, op: 'open' },
      { id: 2, op: 'next' },
      { id: 3, op: 'answer', rating: 3, ms: 1200 }
    ]);
    expect(client.waiting).toBe(3);
    // out of order, with a reply to an id never sent, a reply with no id and a message that is
    // not a reply at all among them
    port.reply({ id: 2, ok: true, value: 1001n });
    port.reply({ id: 99, ok: true, value: 'stray' });
    port.reply({ ok: true, value: 'no id' });
    port.reply(null);
    port.reply({ id: 3, ok: false, code: 'not-open', message: 'answer before open' });
    port.reply({ id: 1, ok: true, value: { existed: false, notes: 0 } });

    await expect(next).resolves.toBe(1001n);
    await expect(opened).resolves.toEqual({ existed: false, notes: 0 });
    const refused = await answered.then(
      () => 'resolved',
      (error: unknown) => error
    );
    expect(refused).toBeInstanceOf(EngineError);
    expect(refused).toBeInstanceOf(Error);
    expect(refused).toMatchObject({ code: 'not-open', message: 'answer before open' });
    expect(client.waiting).toBe(0);
  });

  it('each operation sends its own request', async () => {
    const port = new FakePort();
    const client = new EngineClient(port, ORIGIN);
    const calls = [
      client.seed(25),
      client.undo(),
      client.snapshot(1001n),
      client.close(),
      client.answer(1, 0)
    ];

    expect(port.sent).toEqual([
      { id: 1, op: 'seed', count: 25 },
      { id: 2, op: 'undo' },
      { id: 3, op: 'snapshot', card: 1001n },
      { id: 4, op: 'close' },
      { id: 5, op: 'answer', rating: 1, ms: 0 }
    ]);
    for (const id of [1, 2, 3, 4, 5]) port.reply({ id, ok: true, value: id * 10 });
    await expect(Promise.all(calls)).resolves.toEqual([10, 20, 30, 40, 50]);
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
});
