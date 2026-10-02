/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import CapturePage from '../../routes/capture/+page.svelte';
import { createApi } from '../api';
import QuickCapture from './QuickCapture.svelte';
import { QUICK_TEXT_CHARS, captureBody, fits, newCaptureId, parseSaved } from './capture';

// SPEC-118 R10, R11, A20, A21; ADR-118. The capture screen sends the text, its kind and a retry
// key: a fresh key for each capture and the same key when it retries one, so a capture whose
// answer was lost is saved once. The tests run the screen over the app's real API client and a
// synthetic server, and judge what reached the wire.
const LAUNCH = 'auth_date=1&hash=synthetic-launch-data';
const KEY = /^[0-9a-f]{32}$/;

/** A capture's body as the server read it. */
interface Posted {
  readonly capture_id: string;
  readonly kind: string;
  readonly text: string;
}

/** A 201 or 200 answer naming `name`, as the route writes it. */
function saved(name: string, already = false): () => Response {
  return () =>
    Response.json(already ? { name, already_captured: true } : { name }, {
      status: already ? 200 : 201
    });
}

/** A refusal of `status` naming `reason`, as the route writes it. */
function refused(status: number, reason: string): () => Response {
  return () => Response.json({ reason }, { status });
}

/** `answer`, held until the test releases it, so the screen can be read while a capture waits. */
function held(answer: () => Response): { answer: () => Promise<Response>; release: () => void } {
  let release = (): void => undefined;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  return {
    answer: async () => {
      await gate;
      return answer();
    },
    release: () => release()
  };
}

/**
 * A server whose session opens, and which answers the n-th capture with the n-th of `answers` (a
 * dropped connection once they run out). It records every capture body it was sent.
 */
function server(answers: readonly (() => Response | Promise<Response>)[]) {
  const posted: Posted[] = [];
  const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
    if (String(input) === '/api/session') return new Response(null, { status: 200 });
    posted.push(JSON.parse(String(init.body)) as Posted);
    const answer = answers[posted.length - 1];
    if (answer === undefined) throw new TypeError('the connection dropped');
    return answer();
  });
  const api = createApi({
    launchData: () => LAUNCH,
    fetch: fetch as unknown as typeof globalThis.fetch
  });
  render(QuickCapture, { props: { send: api.capture } });
  return posted;
}

function field(): HTMLTextAreaElement {
  return screen.getByRole('textbox', { name: 'What to capture' }) as HTMLTextAreaElement;
}

function button(): HTMLButtonElement {
  return screen.getByRole('button', { name: /Save to inbox|Saving/ }) as HTMLButtonElement;
}

function choice(): HTMLInputElement {
  return screen.getByRole('checkbox', { name: 'Journal entry' }) as HTMLInputElement;
}

function form(): HTMLFormElement {
  return screen.getByRole('form', { name: 'Quick capture' }) as HTMLFormElement;
}

/** Types `text` and, when asked, chooses the journal kind. */
async function write(text: string, journal = false): Promise<void> {
  await fireEvent.input(field(), { target: { value: text } });
  if (choice().checked !== journal) await fireEvent.click(choice());
}

/** Presses save, then waits until `count` captures have reached the server and the screen settled. */
async function save(posted: readonly Posted[], count: number): Promise<void> {
  await fireEvent.click(button());
  await vi.waitFor(() => expect(posted).toHaveLength(count));
  await vi.waitFor(() => expect(button().textContent?.trim()).toBe('Save to inbox'));
}

function status(): string {
  return (screen.getByRole('status').textContent ?? '').replace(/\s+/g, ' ').trim();
}

function alerts(): string[] {
  return screen.queryAllByRole('alert').map((alert) => (alert.textContent ?? '').trim());
}

describe('QuickCapture', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('sends the text, the kind and one capture id per capture, and the same id on a retry', async () => {
    const posted = server([
      saved('2026-10-02-text-aaaa.md'),
      saved('2026-10-02-journal-bbbb.md'),
      refused(503, 'vault_missing'),
      saved('2026-10-02-text-cccc.md')
    ]);

    await write('Buy oat milk');
    await save(posted, 1);
    await write('A calm morning by the river', true);
    await save(posted, 2);
    // the third capture's answer is a refusal; pressing save again retries that same capture
    await write('Call the dentist');
    await save(posted, 3);
    await save(posted, 4);

    expect(posted.map((body) => [body.kind, body.text])).toEqual([
      ['text', 'Buy oat milk'],
      ['journal', 'A calm morning by the river'],
      ['text', 'Call the dentist'],
      ['text', 'Call the dentist']
    ]);
    expect(posted.map((body) => Object.keys(body).sort())).toEqual(
      Array(4).fill(['capture_id', 'kind', 'text'])
    );
    const keys = posted.map((body) => body.capture_id);
    expect(keys.filter((key) => KEY.test(key))).toEqual(keys);
    // one key per capture: the first three are distinct, and the retry repeats the third's
    expect(new Set(keys.slice(0, 3)).size).toBe(3);
    expect(keys[3]).toBe(keys[2]);
  });

  it('shows the saved name or the failure line', async () => {
    const posted = server([
      saved('2026-10-02-text-aaaa.md'),
      saved('2026-10-02-text-aaaa.md', true),
      refused(503, 'vault_missing')
    ]);

    // 201: the saved name, and the field is cleared for the next capture
    await write('Buy oat milk');
    await save(posted, 1);
    expect(status()).toBe('Saved as 2026-10-02-text-aaaa.md');
    expect(alerts()).toEqual([]);
    expect(field().value).toBe('');

    // 200: a capture the server already holds answers the name it was saved under
    await write('Buy oat milk');
    await save(posted, 2);
    expect(status()).toBe('Saved as 2026-10-02-text-aaaa.md');

    // 503: one failure line, no saved name, and the text kept for the retry
    await write('Call the dentist');
    await save(posted, 3);
    expect(alerts()).toEqual(['Not saved. Your text is still here, so try again in a moment.']);
    expect(status()).toBe('');
    expect(field().value).toBe('Call the dentist');

    // no answer at all reads the same one line
    await save(posted, 4);
    expect(alerts()).toEqual(['Not saved. Your text is still here, so try again in a moment.']);
  });

  it('a capture edited after a failure is a new capture with its own id', async () => {
    const posted = server([refused(503, 'vault_missing'), saved('2026-10-02-journal-dddd.md')]);

    await write('First thought');
    await save(posted, 1);
    await write('First thought, said better', true);
    await save(posted, 2);

    expect(posted.map((body) => [body.kind, body.text])).toEqual([
      ['text', 'First thought'],
      ['journal', 'First thought, said better']
    ]);
    expect(posted[1].capture_id).not.toBe(posted[0].capture_id);
    expect(status()).toBe('Saved as 2026-10-02-journal-dddd.md');
  });

  it('a capture whose kind alone or text alone changed after a failure is a new capture', async () => {
    const posted = server([
      refused(503, 'vault_missing'),
      refused(503, 'vault_missing'),
      saved('2026-10-02-journal-ffff.md')
    ]);

    await write('Same words');
    await save(posted, 1);
    // the kind alone changes
    await write('Same words', true);
    await save(posted, 2);
    // the text alone changes
    await write('Other words', true);
    await save(posted, 3);

    expect(posted.map((body) => [body.kind, body.text])).toEqual([
      ['text', 'Same words'],
      ['journal', 'Same words'],
      ['journal', 'Other words']
    ]);
    expect(new Set(posted.map((body) => body.capture_id)).size).toBe(3);
  });

  it('opens empty, as a note, with save off, named by its title and bounded in its hint', () => {
    server([]);

    expect(form().getAttribute('aria-labelledby')).toBe('capture-title');
    expect(field().value).toBe('');
    expect(field().readOnly).toBe(false);
    const hint = document.getElementById(field().getAttribute('aria-describedby') ?? '');
    expect(hint?.textContent).toBe("Up to 4000 characters. It lands in your vault's inbox.");
    expect(choice().checked).toBe(false);
    expect(choice().disabled).toBe(false);
    expect(button().disabled).toBe(true);
    expect(button().textContent?.trim()).toBe('Save to inbox');
  });

  it('a submit never leaves the page, and a blank text reaches no server', async () => {
    const posted = server([saved('2026-10-02-text-eeee.md')]);

    expect(await fireEvent.submit(form())).toBe(false);
    await write('   ');
    expect(await fireEvent.submit(form())).toBe(false);
    await write('Water the plants');
    expect(await fireEvent.submit(form())).toBe(false);
    await vi.waitFor(() => expect(status()).toBe('Saved as 2026-10-02-text-eeee.md'));

    expect(posted.map((body) => body.text)).toEqual(['Water the plants']);
  });

  it('holds the field while a capture waits, then returns to an empty note', async () => {
    const slow = held(saved('2026-10-02-journal-gggg.md'));
    const posted = server([slow.answer]);

    await write('Slept well', true);
    await fireEvent.click(button());
    await vi.waitFor(() => expect(posted).toHaveLength(1));
    // while it waits: the saving label, save off, and the field and the choice held
    expect(button().textContent?.trim()).toBe('Saving…');
    expect(button().disabled).toBe(true);
    expect(field().readOnly).toBe(true);
    expect(choice().disabled).toBe(true);

    slow.release();
    await vi.waitFor(() => expect(status()).toBe('Saved as 2026-10-02-journal-gggg.md'));
    expect(button().textContent?.trim()).toBe('Save to inbox');
    expect(field().value).toBe('');
    expect(field().readOnly).toBe(false);
    expect(choice().checked).toBe(false);
    expect(choice().disabled).toBe(false);
  });

  it('asks to reopen from Telegram when the session is refused, and keeps the text', async () => {
    const sent: string[] = [];
    const fetch = vi.fn(async (input: RequestInfo | URL) => {
      sent.push(String(input));
      return new Response(null, { status: 401 });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });
    render(QuickCapture, { props: { send: api.capture } });

    await write('Call the dentist');
    await fireEvent.click(button());
    await vi.waitFor(() =>
      expect(alerts()).toEqual(['Reopen DeckStreak from Telegram to sign in again.'])
    );
    expect(status()).toBe('');
    expect(field().value).toBe('Call the dentist');
    // the session was refused, so the capture itself never reached the wire
    expect(sent).toEqual(['/api/session']);
  });

  it('keeps save off for a blank or over-long text, and bounds the field', async () => {
    server([]);

    expect(field().maxLength).toBe(QUICK_TEXT_CHARS);
    await write('   \n\t ');
    expect(button().disabled).toBe(true);
    await write('x'.repeat(QUICK_TEXT_CHARS));
    expect(button().disabled).toBe(false);
    await write('x'.repeat(QUICK_TEXT_CHARS + 1));
    expect(button().disabled).toBe(true);
  });
});

describe('the quick capture module', () => {
  it('mints a fresh 32-hex capture id each time', () => {
    const keys = Array.from({ length: 50 }, () => newCaptureId());
    expect(keys.filter((key) => KEY.test(key))).toHaveLength(50);
    expect(new Set(keys).size).toBe(50);
  });

  it('fits one to 4000 characters after the trim, counting characters, not UTF-16 units', () => {
    expect([fits(''), fits('  \n '), fits('a'), fits(` ${'a'.repeat(4000)} `)]).toEqual([
      false,
      false,
      true,
      true
    ]);
    expect(fits('a'.repeat(4001))).toBe(false);
    // 4000 characters outside the basic plane are 8000 UTF-16 units, and still fit
    expect(fits('😀'.repeat(4000))).toBe(true);
    expect(fits('😀'.repeat(4001))).toBe(false);
  });

  it('sends exactly the route body, and reads only a saved name', () => {
    expect(JSON.parse(captureBody({ captureId: 'ab12', kind: 'journal', text: 'x' }))).toEqual({
      capture_id: 'ab12',
      kind: 'journal',
      text: 'x'
    });
    expect(parseSaved(201, { name: 'a.md' })).toEqual({ name: 'a.md', alreadyCaptured: false });
    expect(parseSaved(200, { name: 'a.md', already_captured: true })).toEqual({
      name: 'a.md',
      alreadyCaptured: true
    });
    const none: [number, unknown][] = [
      [200, { name: 'a.md' }],
      [201, { name: '' }],
      [201, { name: 7 }],
      [201, null],
      [201, ['a.md']],
      [503, { name: 'a.md' }],
      [422, { reason: 'text_out_of_bounds' }]
    ];
    expect(none.map(([code, body]) => parseSaved(code, body))).toEqual(Array(none.length).fill(null));
  });

  it('reads no capture from a body that is no object, or under a status other than 201 and 200', () => {
    const none: [number, unknown][] = [
      [201, undefined],
      [201, 'a.md'],
      [201, 7],
      [409, { name: 'a.md', already_captured: true }],
      [503, { name: 'a.md', already_captured: true }]
    ];
    expect(none.map(([code, body]) => parseSaved(code, body))).toEqual(Array(none.length).fill(null));
  });
});

// SPEC-118 R11. The capture route holds the quick capture and the way back to Today.
describe('the capture screen page', () => {
  it('holds the quick capture and a way back to Today', () => {
    render(CapturePage);

    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('DeckStreak');
    expect(form().contains(field())).toBe(true);
    expect(screen.getByRole('link', { name: 'Back to Today' }).getAttribute('href')).toBe('/');
  });
});
