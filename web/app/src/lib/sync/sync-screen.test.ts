/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import type { ChoiceCounted, ChoiceConfirmed, Direction, StatusWord, Synced, Unsynced } from '$lib/engine/protocol';
import type { BackupsListed } from '$lib/engine/protocol';
import type { BackupsClient } from '$lib/engine/backups';
import { REQUIRED, STATUS_WORDS } from '$lib/engine/protocol';
import { m } from '$lib/paraglide/messages.js';
import SyncRoute from '../../routes/sync/+page.svelte';
import SyncScreen from './SyncScreen.svelte';
import { choosing, statusText, type SyncClient } from './status';

// SPEC-377 R8, R10, R11, A10, A11, A13; ADR-388 D5, D11, D13. The sync screen posts the sync user
// and password to the Worker once and keeps neither, says each status word in the owner's words,
// shows the unsynced count the Worker answers, offline too, and says when the browser holds no
// copy of the collection, offering the download alone.

// The route draws the screen over the app's one engine, which is replaced before the route's
// import runs.
const route = vi.hoisted(() => ({ client: vi.fn(), lost: vi.fn() }));
vi.mock('$lib/study/engine', () => ({ studyEngine: () => ({ client: route.client, lost: route.lost }) }));

/** The engine's client as the sync screen sees it: each call recorded, each answer set. */
class FakeSync implements SyncClient {
  calls: string[] = [];
  word: StatusWord;
  synced: Synced;
  pending: number[];
  counted: ChoiceCounted = { status: 'held', counts: null, snapshot: null };
  // the backup list's reads and exports, recorded apart from `calls`, which the route's test reads
  // whole (SPEC-377 R15, R16)
  listings = 0;
  exported: string[] = [];
  listed: BackupsListed = { backups: [{ id: 'backup-1', kind: 'backup', age: 60 }], removed: [] };

  constructor(word: StatusWord, synced: Synced = { status: word, required: 'no-changes' }, pending = [7]) {
    this.word = word;
    this.synced = synced;
    this.pending = pending;
  }

  async credentialStatus() {
    this.calls.push('credentialStatus');
    return this.word;
  }

  async syncLogin(user: string, password: string) {
    this.calls.push(`syncLogin ${user} ${password}`);
    this.word = 'held';
    return this.word;
  }

  async sync() {
    this.calls.push('sync');
    this.word = this.synced.status;
    return this.synced;
  }

  async unsynced(): Promise<Unsynced> {
    this.calls.push('unsynced');
    const reviews = this.pending.length > 1 ? (this.pending.shift() as number) : this.pending[0];
    return { reviews, changed: reviews > 0, schema: false };
  }

  async forgetSync() {
    this.calls.push('forgetSync');
    this.word = 'absent';
    return this.word;
  }

  async choiceCount() {
    this.calls.push('choiceCount');
    return this.counted;
  }

  async choiceConfirm(direction: Direction): Promise<ChoiceConfirmed> {
    this.calls.push(`choiceConfirm ${direction}`);
    return { status: 'held', outcome: 'written' };
  }

  async choiceCancel() {
    this.calls.push('choiceCancel');
    return null;
  }

  async backups() {
    this.listings += 1;
    return this.listed;
  }

  async backupExport(backup: string) {
    this.exported.push(backup);
    return new Uint8Array([1]);
  }
}

async function settle(): Promise<void> {
  for (let turn = 0; turn < 3; turn += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

function status(): string {
  return screen.getByRole('status').textContent?.trim() ?? '';
}

/** The text of each paragraph the screen shows, in order. */
function texts(container: HTMLElement): string[] {
  return [...container.querySelectorAll('p')].map((each) => each.textContent?.trim() ?? '');
}

/** A message by its key, or a marker naming the key while the locale files lack it. */
function said(key: string, inputs: Record<string, unknown> = {}): string {
  const message = (m as unknown as Record<string, ((inputs: Record<string, unknown>) => string) | undefined>)[key];
  return message === undefined ? `no message ${key}` : message(inputs);
}

/** The storage status's texts, in order: the browser's answer, then the eviction sentence. */
function storageText(container: HTMLElement): string[] {
  return [...container.querySelectorAll('[data-storage] > *')].map((each) => each.textContent?.trim() ?? '');
}

/** The backups the screen lists, by their row's text. */
function listedRows(container: HTMLElement): string[] {
  return [...container.querySelectorAll('tbody tr th')].map((each) => each.textContent?.trim() ?? '');
}

/** Whether the sync and the sign-out buttons are disabled, in that order. */
function disabled(): boolean[] {
  return [m.sync_now(), m.sync_sign_out()].map(
    (name) => (screen.getByRole('button', { name }) as HTMLButtonElement).disabled
  );
}

/** A sync client whose first sync throws, as an engine that stopped does. */
class FailingSync extends FakeSync {
  failures = 1;

  override async sync() {
    if (this.failures === 0) return super.sync();
    this.failures -= 1;
    this.calls.push('sync');
    throw new Error('the engine stopped');
  }
}

function shown(fake: FakeSync, lost = false, ended: string[] = []) {
  return render(SyncScreen, {
    client: async () => fake,
    lost: () => lost,
    fetch: (async (input: string) => {
      ended.push(String(input));
      return new Response(null, { status: 204 });
    }) as typeof globalThis.fetch
  });
}

describe('the sync screen', () => {
  it('the sign-in posts once and shows each status word', async () => {
    const fake = new FakeSync('absent');
    const { container, unmount } = shown(fake);
    await settle();
    expect(status()).toBe(m.sync_status_absent());
    const user = screen.getByLabelText(m.sync_user()) as HTMLInputElement;
    const password = screen.getByLabelText(m.sync_password()) as HTMLInputElement;
    // the browser's password manager may fill both, and the owner may paste into both
    expect([user.autocomplete, user.type, password.autocomplete, password.type]).toEqual([
      'username',
      'text',
      'current-password',
      'password'
    ]);
    expect([user.getAttribute('onpaste'), password.getAttribute('onpaste')]).toEqual([null, null]);
    await fireEvent.input(user, { target: { value: 'owner' } });
    await fireEvent.input(password, { target: { value: 'pass-phrase' } });
    await fireEvent.click(screen.getByRole('button', { name: m.sync_sign_in() }));
    await settle();
    // posted once, then kept nowhere: both fields are empty and no text of the screen holds either
    expect(fake.calls.filter((call) => call.startsWith('syncLogin'))).toEqual(['syncLogin owner pass-phrase']);
    expect([user.value, password.value]).toEqual(['', '']);
    expect(container.innerHTML).not.toContain('pass-phrase');
    expect(status()).toBe(m.sync_status_held());
    // a sync after the sign-in posts no credential again
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    expect(fake.calls.filter((call) => call.startsWith('syncLogin'))).toHaveLength(1);
    expect(fake.calls).toContain('sync');
    unmount();

    // each status word reads as the owner's words, never as the word itself
    const texts = new Set<string>();
    for (const word of STATUS_WORDS) {
      const each = new FakeSync(word);
      const view = shown(each);
      await settle();
      expect(status(), word).toBe(statusText(word));
      expect(status(), word).not.toContain(word);
      texts.add(status());
      // the sign-in is offered exactly when the store holds no key a sync could send
      const signsIn = word === 'absent' || word === 'needs-sign-in';
      expect(screen.queryByLabelText(m.sync_user()) !== null, word).toBe(signsIn);
      expect(screen.queryByRole('button', { name: m.sync_now() }) !== null, word).toBe(!signsIn);
      expect(screen.queryByRole('button', { name: m.sync_sign_out() }) !== null, word).toBe(!signsIn);
      view.unmount();
    }
    expect(texts.size).toBe(STATUS_WORDS.length);
    console.log(`examined ${STATUS_WORDS.length} status words`);
    expect([
      statusText('absent'),
      statusText('sealed'),
      statusText('held'),
      statusText('needs-sign-in'),
      statusText('offline')
    ]).toEqual([
      m.sync_status_absent(),
      m.sync_status_sealed(),
      m.sync_status_held(),
      m.sync_status_needs_sign_in(),
      m.sync_status_offline()
    ]);

    // the sign-out forgets the key, then ends the web session
    const out = new FakeSync('held');
    const ended: string[] = [];
    const view = shown(out, false, ended);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_sign_out() }));
    await settle();
    expect(out.calls).toContain('forgetSync');
    expect(ended).toEqual(['/api/session']);
    expect(status()).toBe(m.sync_status_absent());
    view.unmount();
  });

  it("the unsynced count is the worker's", async () => {
    // offline: the count is still the Worker's reply, read when the screen opens
    const fake = new FakeSync('offline', { status: 'offline', required: null }, [7, 9]);
    const view = shown(fake);
    await settle();
    expect(screen.getByText(m.sync_unsynced({ count: 7 }))).toBeTruthy();
    // a sync that stays offline asks the Worker again, and shows its new reply
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    expect(screen.getByText(m.sync_unsynced({ count: 9 }))).toBeTruthy();
    expect(screen.queryByText(m.sync_unsynced({ count: 7 }))).toBeNull();
    expect(fake.calls.filter((call) => call === 'unsynced')).toHaveLength(2);
    expect(status()).toBe(m.sync_status_offline());
    view.unmount();
    // a sync in step says so
    const quiet = new FakeSync('held', { status: 'held', required: 'no-changes' }, [0]);
    const after = shown(quiet);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    expect(screen.getByText(m.sync_in_step())).toBeTruthy();
    expect(screen.getByText(m.sync_unsynced({ count: 0 }))).toBeTruthy();
    after.unmount();
  });

  it('a collection the browser lost offers the download alone', async () => {
    const fake = new FakeSync('held', { status: 'held', required: 'full-download' }, [0]);
    fake.counted = {
      status: 'held',
      counts: { upload: null, download: { reviews: 0, cards: 0, notes: 0 } },
      snapshot: { found: false }
    };
    const view = shown(fake, true);
    await settle();
    expect(screen.getByText(m.sync_lost())).toBeTruthy();
    // nothing is offered before the owner syncs
    expect(screen.queryByRole('button', { name: m.sync_choice_restore() })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    expect(screen.getByText(m.sync_full_required())).toBeTruthy();
    expect(screen.getByRole('button', { name: m.sync_choice_restore() })).toBeTruthy();
    expect(screen.queryByRole('button', { name: m.sync_choice_upload() })).toBeNull();
    expect(screen.queryByRole('button', { name: m.sync_choice_download() })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_restore() }));
    await settle();
    expect(fake.calls).toContain('choiceConfirm download');
    expect(screen.getByText(m.sync_choice_written())).toBeTruthy();
    view.unmount();
    // a collection the browser holds says nothing of a loss
    const held = shown(new FakeSync('held'), false);
    await settle();
    expect(screen.queryByText(m.sync_lost())).toBeNull();
    held.unmount();
  });

  it('offers the choice exactly when a sync answers that only a full sync can go on', () => {
    expect(REQUIRED.map((required) => [required, choosing(required)])).toEqual([
      ['no-changes', false],
      ['normal-sync', false],
      ['full-sync', true],
      ['full-download', true],
      ['full-upload', true]
    ]);
    expect(choosing(null)).toBe(false);
    console.log(`examined ${REQUIRED.length + 1} sync answers`);
  });

  it('before the worker answers, the screen says it is working and offers nothing', async () => {
    const view = render(SyncScreen, {
      client: () => new Promise<SyncClient & BackupsClient>(() => {}),
      lost: () => true,
      fetch: (async () => new Response(null, { status: 204 })) as typeof globalThis.fetch
    });
    // the first paint already says so: the open's act starts as the screen mounts
    expect(texts(view.container)).toEqual(['', m.sync_working()]);
    await settle();
    expect(screen.getByRole('heading', { level: 1, name: m.sync_title() })).toBeTruthy();
    expect(texts(view.container)).toEqual(['', m.sync_working()]);
    expect(screen.queryAllByRole('button')).toEqual([]);
    view.unmount();
  });

  it('a failed act says the engine stopped, and the next act clears it', async () => {
    const fake = new FailingSync('held', { status: 'held', required: 'no-changes' }, [0]);
    const view = shown(fake);
    await settle();
    expect(screen.queryByText(m.study_refused_engine())).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    expect(texts(view.container)).toEqual([m.sync_status_held(), m.study_refused_engine(), m.sync_unsynced({ count: 0 })]);
    // the failed act has ended, so the owner may act again
    expect(disabled()).toEqual([false, false]);
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    expect(screen.queryByText(m.study_refused_engine())).toBeNull();
    expect(screen.getByText(m.sync_in_step())).toBeTruthy();
    expect(fake.calls.filter((call) => call === 'sync')).toHaveLength(2);
    view.unmount();
  });

  it('the sync buttons wait while the worker works and while the choice is open', async () => {
    const fake = new FakeSync('held', { status: 'held', required: 'full-sync' }, [5]);
    fake.counted = {
      status: 'held',
      counts: { upload: { reviews: 1, cards: 1, notes: 1 }, download: { reviews: 2, cards: 2, notes: 2 } },
      snapshot: { found: false }
    };
    const answer = fake.sync.bind(fake);
    let release: () => void = () => {};
    fake.sync = () => new Promise<Synced>((resolve) => (release = () => resolve(answer())));
    const view = shown(fake);
    await settle();
    expect(disabled()).toEqual([false, false]);
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    // the sync is with the Worker
    expect(disabled()).toEqual([true, true]);
    expect(screen.getByText(m.sync_working())).toBeTruthy();
    release();
    await settle();
    // the sync answered that only a full sync can go on: the choice is open, and nothing is working
    expect(screen.queryByText(m.sync_working())).toBeNull();
    expect(screen.getByRole('heading', { level: 2, name: m.sync_choose() })).toBeTruthy();
    expect(disabled()).toEqual([true, true]);
    // a cancel closes the choice alone: nothing is written and the count is not read again
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_cancel() }));
    await settle();
    expect(screen.queryByRole('heading', { level: 2 })).toBeNull();
    expect(disabled()).toEqual([false, false]);
    expect(screen.queryByText(m.sync_choice_written())).toBeNull();
    expect(fake.calls.filter((call) => call === 'unsynced')).toHaveLength(2);
    view.unmount();
  });

  it('a written choice closes, says so and reads the count again, and the next sync clears the word', async () => {
    const fake = new FakeSync('held', { status: 'held', required: 'full-download' }, [4, 4, 0]);
    fake.counted = {
      status: 'held',
      counts: { upload: null, download: { reviews: 0, cards: 1, notes: 1 } },
      snapshot: { found: false }
    };
    const view = shown(fake, true);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    expect(screen.getByText(m.sync_full_required())).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_restore() }));
    await settle();
    // the choice is closed, the browser holds the collection again, and the count is read again
    expect(screen.queryByRole('heading', { level: 2 })).toBeNull();
    expect(texts(view.container)).toEqual([m.sync_status_held(), m.sync_unsynced({ count: 0 }), m.sync_choice_written()]);
    expect(fake.calls.filter((call) => call === 'unsynced')).toHaveLength(3);
    fake.synced = { status: 'held', required: 'no-changes' };
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    expect(texts(view.container)).toEqual([m.sync_status_held(), m.sync_unsynced({ count: 0 }), m.sync_in_step()]);
    view.unmount();
  });

  it('the sign-in keeps the page where it is', async () => {
    const view = shown(new FakeSync('absent'));
    await settle();
    const form = view.container.querySelector('form') as HTMLFormElement;
    expect(await fireEvent.submit(form)).toBe(false);
    view.unmount();
  });

  it("the storage status is the browser's answer", async () => {
    // SPEC-377 R18, B7: the page asks `persisted()` itself, since a Worker cannot ask `persist()`
    const answers: [string, Pick<StorageManager, 'persisted'>, string][] = [
      ['kept', { persisted: async () => true }, 'sync_storage_persisted'],
      ['not kept', { persisted: async () => false }, 'sync_storage_not_persisted'],
      ['refused', { persisted: () => Promise.reject(new TypeError('no storage shelf')) }, 'sync_storage_unknown']
    ];
    for (const [answer, storage, key] of answers) {
      const view = render(SyncScreen, { client: async () => new FakeSync('held'), lost: () => false, storage });
      await settle();
      expect(storageText(view.container), answer).toEqual([said(key), said('sync_storage_evicted')]);
      // the store's status word stays the screen's one status, and no paragraph the screen reads moved
      expect(status(), answer).toBe(m.sync_status_held());
      expect(texts(view.container), answer).toEqual([m.sync_status_held(), m.sync_unsynced({ count: 7 })]);
      view.unmount();
    }
    // with no storage given, the screen asks the page's own navigator, and a page without one
    // cannot tell
    try {
      vi.stubGlobal('navigator', { storage: { persisted: async () => true } });
      const kept = shown(new FakeSync('held'));
      await settle();
      expect(storageText(kept.container), 'navigator').toEqual([said('sync_storage_persisted'), said('sync_storage_evicted')]);
      kept.unmount();
      vi.stubGlobal('navigator', undefined);
      const none = shown(new FakeSync('held'));
      await settle();
      expect(storageText(none.container), 'no navigator').toEqual([said('sync_storage_unknown'), said('sync_storage_evicted')]);
      none.unmount();
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('the backup list is read at the start and after every write or cancel', async () => {
    // SPEC-377 R15, R17; ADR-388 D18: the Worker runs retention before each list, and never while a
    // choice is held, so the screen lists at its start and once each choice has ended
    const fake = new FakeSync('held', { status: 'held', required: 'full-download' }, [4, 4, 0]);
    fake.counted = {
      status: 'held',
      counts: { upload: null, download: { reviews: 0, cards: 1, notes: 1 } },
      snapshot: { found: false }
    };
    const view = shown(fake, true);
    await settle();
    expect(fake.listings).toBe(1);
    expect(listedRows(view.container)).toHaveLength(1);
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    // the choice is held: the list is not read while it is open
    expect(fake.listings).toBe(1);
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_cancel() }));
    await settle();
    expect(fake.listings).toBe(2);
    fake.listed = {
      backups: [
        { id: 'backup-2', kind: 'backup', age: 1 },
        { id: 'backup-1', kind: 'backup', age: 61 }
      ],
      removed: []
    };
    await fireEvent.click(screen.getByRole('button', { name: m.sync_now() }));
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_restore() }));
    await settle();
    expect(fake.listings).toBe(3);
    expect(listedRows(view.container)).toHaveLength(2);
    expect(fake.calls.filter((call) => call === 'unsynced')).toHaveLength(4);
    view.unmount();
  });

  it("the route draws the sync screen over the app's one engine, and asks it whether the collection was lost", async () => {
    for (const lost of [true, false]) {
      const fake = new FakeSync('held');
      route.client.mockResolvedValue(fake);
      route.lost.mockReturnValue(lost);
      const view = render(SyncRoute);
      await settle();
      const said = lost ? [m.sync_lost()] : [];
      expect(texts(view.container), String(lost)).toEqual([m.sync_status_held(), ...said, m.sync_unsynced({ count: 7 })]);
      expect(fake.calls, String(lost)).toEqual(['credentialStatus', 'unsynced']);
      view.unmount();
      route.client.mockReset();
      route.lost.mockReset();
    }
  });
});
