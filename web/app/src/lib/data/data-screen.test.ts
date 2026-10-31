/**
 * @vitest-environment jsdom
 */
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import type { Answer } from '#lib/api.js';
import type { BackupsListed, SnapshotRead } from '#lib/engine/protocol.js';
import { m } from '#lib/paraglide/messages.js';
import { getLocale } from '#lib/paraglide/runtime.js';
import { ROUTES } from '#lib/routes.js';
import { snapshotAge } from '#lib/sync/status.js';
import DataRoute from '../../routes/data/+page.svelte';
import DataScreen from './DataScreen.svelte';
import { EXPORT_FILE, parseServerBackup, saveFile, type DataApi, type DataClient } from './data';

// SPEC-516 R1, R2, R4, R6 to R8, B1, B4, B6, B7; ADR-534 D1, D4 to D6. The data screen: a JSON copy
// of the owner's data from the one export route, saved as a file; back up now through the Worker,
// then the list again; the server's backup age, unknown said as unknown; and, inside Telegram on a
// phone, the bot's `/export` named beside the download.

// The route draws the screen over the app's one engine, its one API client and Telegram's platform,
// each replaced before the route's import runs.
const route = vi.hoisted(() => ({
  client: vi.fn(),
  dataExport: vi.fn(),
  serverBackup: vi.fn(),
  platform: 'tdesktop'
}));
vi.mock('#lib/study/engine.js', () => ({ studyEngine: () => ({ client: route.client }) }));
vi.mock('#lib/api.js', () => ({
  api: { dataExport: route.dataExport, serverBackup: route.serverBackup }
}));
vi.mock('#lib/telegram.svelte.js', () => ({
  telegram: {
    get platform() {
      return route.platform;
    }
  }
}));

const APP = resolve(import.meta.dirname, '../../..');

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The Worker's backups as the data screen sees them: each call recorded, each answer set. */
class FakeData implements DataClient {
  calls: string[] = [];
  listed: BackupsListed = { backups: [{ id: 'backup-1', kind: 'backup', age: 60 }], removed: [] };
  /** What the next back up now does: answer an id, or throw as an engine that stopped does. */
  made: string | Error = 'backup-2';

  async backups() {
    this.calls.push('backups');
    return this.listed;
  }

  async backupExport(backup: string) {
    this.calls.push(`backupExport ${backup}`);
    return new Uint8Array([1]);
  }

  async backUpNow() {
    this.calls.push('backUpNow');
    if (this.made instanceof Error) throw this.made;
    this.listed = {
      backups: [{ id: this.made, kind: 'backup', age: 0 }, ...this.listed.backups],
      removed: []
    };
    return this.made;
  }
}

/** The API as the data screen sees it: each call recorded, each answer set. */
class FakeApi implements DataApi {
  calls: string[] = [];
  exported: Answer<Blob> = { kind: 'ok', value: new Blob(['{"schema":"deckstreak.export.v1"}\n']) };
  server: Answer<SnapshotRead> = { kind: 'ok', value: { found: true, age: 7200 } };

  async dataExport() {
    this.calls.push('dataExport');
    return this.exported;
  }

  async serverBackup() {
    this.calls.push('serverBackup');
    return this.server;
  }
}

async function settle(): Promise<void> {
  for (let turn = 0; turn < 4; turn += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

/** A message by its key, or a marker naming the key while the locale files lack it. */
function said(key: string, inputs: Record<string, unknown> = {}): string {
  const message = (m as unknown as Record<string, ((inputs: Record<string, unknown>) => string) | undefined>)[key];
  return message === undefined ? `no message ${key}` : message(inputs);
}

/** The text of each paragraph the screen shows, in order. */
function texts(container: HTMLElement): string[] {
  return [...container.querySelectorAll('p')].map((each) => each.textContent?.trim() ?? '');
}

/** The server's backup age as the screen says it. */
function serverText(container: HTMLElement): string {
  return container.querySelector('[data-server-age]')?.textContent?.trim() ?? 'no server age shown';
}

/** The backups the screen lists, by their row's text. */
function listedRows(container: HTMLElement): string[] {
  return [...container.querySelectorAll('tbody tr th')].map((each) => each.textContent?.trim() ?? '');
}

/** The screen over `client`, `api` and `platform`, with each save recorded. */
function drawn(client: FakeData, api: FakeApi, platform = 'tdesktop') {
  const saved: [Blob, string][] = [];
  const view = render(DataScreen, {
    client: async () => client,
    data: api,
    platform,
    save: (blob: Blob, name: string) => saved.push([blob, name])
  });
  return { view, saved };
}

function button(key: string): HTMLButtonElement {
  return screen.getByRole('button', { name: said(key) }) as HTMLButtonElement;
}

describe('the data screen is routed and audited', () => {
  it('/data is a route, so the coverage test and the rendered audit cover it', () => {
    // the rendered audit visits every path of ROUTES in both schemes, and the coverage test holds
    // ROUTES equal to the screens under src/routes
    expect(ROUTES).toContain('/data');
    expect(existsSync(resolve(APP, 'src/routes/data/+page.svelte'))).toBe(true);
  });

  it("the route draws the screen over the app's engine, its API and Telegram's platform", async () => {
    const client = new FakeData();
    const api = new FakeApi();
    route.client.mockResolvedValue(client);
    route.dataExport.mockImplementation(() => api.dataExport());
    route.serverBackup.mockImplementation(() => api.serverBackup());
    route.platform = 'ios';
    const { container } = render(DataRoute);
    await settle();

    expect(screen.getByRole('heading', { level: 1 }).textContent?.trim()).toBe(said('data_title'));
    expect(client.calls).toEqual(['backups']);
    expect(api.calls).toEqual(['serverBackup']);
    // the platform the wrapper read decides the phone's sentence
    expect(texts(container)).toContain(said('data_telegram_phone'));
  });
});

describe('the JSON copy', () => {
  it('a download posts the one export and saves its document as deck-streak-export.json', async () => {
    const client = new FakeData();
    const api = new FakeApi();
    const { view, saved } = drawn(client, api);
    await settle();

    await fireEvent.click(button('data_json_download'));
    await settle();

    expect(api.calls).toEqual(['serverBackup', 'dataExport']);
    expect(EXPORT_FILE).toBe('deck-streak-export.json');
    expect(saved.map(([blob, name]) => [blob, name])).toEqual([[(api.exported as { value: Blob }).value, EXPORT_FILE]]);
    expect(texts(view.container)).toContain(said('data_json_saved'));
  });

  it('a refused session or no answer saves nothing and says why', async () => {
    const cases: [Answer<Blob>, string][] = [
      [{ kind: 'unavailable' }, 'data_json_failed'],
      [{ kind: 'reopen' }, 'data_json_reopen']
    ];
    for (const [answer, key] of examined('refused export(s)', cases)) {
      const api = new FakeApi();
      api.exported = answer;
      const { view, saved } = drawn(new FakeData(), api);
      await settle();
      await fireEvent.click(button('data_json_download'));
      await settle();
      expect(saved).toEqual([]);
      expect(texts(view.container), key).toContain(said(key));
      expect(texts(view.container)).not.toContain(said('data_json_saved'));
      view.unmount();
    }
  });
});

describe('back up now', () => {
  it('back up now asks the Worker once, then lists the backups again', async () => {
    const client = new FakeData();
    const { view } = drawn(client, new FakeApi());
    await settle();
    expect(listedRows(view.container)).toHaveLength(1);

    await fireEvent.click(button('data_back_up_now'));
    await settle();

    // the list after it is the Worker's, which runs the core's retention first
    expect(client.calls).toEqual(['backups', 'backUpNow', 'backups']);
    expect(listedRows(view.container)).toHaveLength(2);
    expect(texts(view.container)).toContain(said('data_backup_made'));
  });

  it('a back up now the engine refused says so, and lists nothing new', async () => {
    const client = new FakeData();
    client.made = new Error('the engine stopped');
    const { view } = drawn(client, new FakeApi());
    await settle();

    await fireEvent.click(button('data_back_up_now'));
    await settle();

    expect(client.calls).toEqual(['backups', 'backUpNow']);
    expect(listedRows(view.container)).toHaveLength(1);
    expect(view.container.textContent).toContain(said('study_refused_engine'));
    expect(texts(view.container)).not.toContain(said('data_backup_made'));
  });

  it('the button waits while a back up now runs', async () => {
    const client = new FakeData();
    let finish: (id: string) => void = () => undefined;
    client.backUpNow = () => {
      client.calls.push('backUpNow');
      return new Promise<string>((resolve) => {
        finish = resolve;
      });
    };
    drawn(client, new FakeApi());
    await settle();

    await fireEvent.click(button('data_back_up_now'));
    await settle();
    expect(button('data_back_up_now').disabled).toBe(true);
    await fireEvent.click(button('data_back_up_now'));
    finish('backup-2');
    await settle();
    expect(button('data_back_up_now').disabled).toBe(false);
    expect(client.calls).toEqual(['backups', 'backUpNow', 'backups']);
  });
});

describe("the server's backup age", () => {
  it('found shows its age; none says so; unknown, a refusal or no answer say unknown in words', async () => {
    const cases: [Answer<SnapshotRead>, string][] = [
      [{ kind: 'ok', value: { found: true, age: 7200 } }, said('data_server_age', { age: snapshotAge(7200, getLocale()) })],
      [{ kind: 'ok', value: { found: true, age: 30 } }, said('data_server_age', { age: snapshotAge(30, getLocale()) })],
      [{ kind: 'ok', value: { found: false } }, said('data_server_none')],
      [{ kind: 'ok', value: { found: null } }, said('data_server_unknown')],
      [{ kind: 'unavailable' }, said('data_server_unknown')],
      [{ kind: 'reopen' }, said('data_server_unknown')]
    ];
    for (const [answer, expected] of examined("server's answer(s)", cases)) {
      const api = new FakeApi();
      api.server = answer;
      const { view } = drawn(new FakeData(), api);
      await settle();
      expect(serverText(view.container), JSON.stringify(answer)).toBe(expected);
      view.unmount();
    }
    // unknown is said in words, never as a number
    expect(said('data_server_unknown')).not.toMatch(/[0-9]/);
    expect(said('data_server_unknown')).not.toBe(said('data_server_none'));
  });
});

describe('inside Telegram on a phone', () => {
  it('on iOS and Android the screen names /export beside the download, and elsewhere it does not', async () => {
    const phones = examined('phone platform(s)', ['ios', 'android', 'android_x']);
    const others = examined('other platform(s)', ['tdesktop', 'macos', 'weba', 'webk', 'unigram', 'unknown', '']);
    expect(said('data_telegram_phone')).toContain('/export');
    for (const [platform, named] of [...phones.map((p) => [p, true] as const), ...others.map((p) => [p, false] as const)]) {
      const { view } = drawn(new FakeData(), new FakeApi(), platform);
      await settle();
      expect(texts(view.container).includes(said('data_telegram_phone')), platform).toBe(named);
      view.unmount();
    }
  });
});

// SPEC-516 R2, R6, R7. What each section says before any press, and how the screen reads a copy, a
// back up now and a server read that throw rather than answer.
describe('the data screen before any press, and when a read throws', () => {
  it('each section names itself, and no line about a copy, a backup or an alert shows before any press', async () => {
    const { view } = drawn(new FakeData(), new FakeApi());
    await settle();
    for (const key of examined('section title(s)', ['data_json_title', 'data_device_title', 'data_server_title'])) {
      expect(screen.getByRole('heading', { level: 2, name: said(key) }).id, key).not.toBe('');
    }
    expect(texts(view.container)).toEqual(expect.arrayContaining([said('data_json_about'), said('data_device_about')]));
    const shown = view.container.textContent ?? '';
    for (const key of ['data_json_saved', 'data_json_failed', 'data_json_reopen', 'data_working', 'data_backup_made']) {
      expect(shown, key).not.toContain(said(key));
    }
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('a copy whose fetch throws says it failed, saves nothing and frees the button', async () => {
    const api = new FakeApi();
    api.dataExport = async () => {
      api.calls.push('dataExport');
      throw new Error('no answer');
    };
    const { view, saved } = drawn(new FakeData(), api);
    await settle();
    await fireEvent.click(button('data_json_download'));
    await settle();
    expect(saved).toEqual([]);
    expect(texts(view.container)).toContain(said('data_json_failed'));
    expect(button('data_json_download').disabled).toBe(false);
  });

  it('a second press while the copy is fetched asks nothing more, and the button waits until it ends', async () => {
    const api = new FakeApi();
    let answer: (value: Answer<Blob>) => void = () => undefined;
    api.dataExport = () => {
      api.calls.push('dataExport');
      return new Promise((resolve) => (answer = resolve));
    };
    const { saved } = drawn(new FakeData(), api);
    await settle();
    const download = button('data_json_download');
    // two presses in one turn, before the screen redraws
    download.click();
    download.click();
    await settle();
    expect(api.calls.filter((call) => call === 'dataExport')).toEqual(['dataExport']);
    expect(download.disabled).toBe(true);
    answer({ kind: 'ok', value: new Blob(['{}']) });
    await settle();
    expect(saved.map(([, name]) => name)).toEqual([EXPORT_FILE]);
    expect(download.disabled).toBe(false);
    // the copy has ended, so the next press asks again
    download.click();
    await settle();
    expect(api.calls.filter((call) => call === 'dataExport')).toEqual(['dataExport', 'dataExport']);
  });

  it('a back up now says it works while it runs, then that it made a backup, and raises no alert', async () => {
    const client = new FakeData();
    let finish: () => void = () => undefined;
    const made = client.backUpNow.bind(client);
    client.backUpNow = () => new Promise((resolve) => (finish = () => resolve(made())));
    const { view } = drawn(client, new FakeApi());
    await settle();
    await fireEvent.click(button('data_back_up_now'));
    await settle();
    expect(texts(view.container)).toContain(said('data_working'));
    finish();
    await settle();
    expect(texts(view.container)).toContain(said('data_backup_made'));
    expect(texts(view.container)).not.toContain(said('data_working'));
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it("a server read that throws says unknown in words", async () => {
    const api = new FakeApi();
    api.serverBackup = async () => {
      api.calls.push('serverBackup');
      throw new Error('no answer');
    };
    const { view } = drawn(new FakeData(), api);
    await settle();
    expect(serverText(view.container)).toBe(said('data_server_unknown'));
  });

  it('an engine that throws when the screen opens raises the alert and lists nothing', async () => {
    const { container } = render(DataScreen, {
      client: () => Promise.reject(new Error('the engine stopped')),
      data: new FakeApi(),
      platform: 'tdesktop',
      save: () => undefined
    });
    await settle();
    expect(screen.getByRole('alert').textContent?.trim()).toBe(said('study_refused_engine'));
    expect(listedRows(container)).toEqual([]);
  });
});

// SPEC-516 R6, R7. The snapshot route's body as the screen reads it, and the anchor that saves a file.
describe("the data screen's reads", () => {
  it('a body that is no plain object, or found with no whole non-negative age, reads as none', () => {
    const odd = examined('body(ies) that are no plain object', [
      null,
      undefined,
      'found',
      7,
      Object.assign(() => undefined, { found: false }),
      Object.assign([], { found: false })
    ]);
    for (const body of odd) expect(parseServerBackup(body), typeof body).toBeNull();
    const ageless = examined('found body(ies) with no whole non-negative age', [
      { found: true, age_seconds: -1 },
      { found: true, age_seconds: 1.5 },
      { found: true },
      { found: 'true', age_seconds: 5 },
      { found: 1, age_seconds: 5 }
    ]);
    for (const body of ageless) expect(parseServerBackup(body), JSON.stringify(body)).toBeNull();
    // the three bodies it reads
    expect(parseServerBackup({ found: true, age_seconds: 0 })).toEqual({ found: true, age: 0 });
    expect(parseServerBackup({ found: false })).toEqual({ found: false });
    expect(parseServerBackup({ found: null })).toEqual({ found: null });
  });

  it('a save clicks one anchor that never joins the page, then revokes its address', async () => {
    const blob = new Blob(['{"schema":"deckstreak.export.v1"}\n']);
    const address = 'blob:https://app.example/copy-1';
    const made: Blob[] = [];
    const revoked: string[] = [];
    const clicked: { tag: string; href: string; download: string; joined: boolean }[] = [];
    const urls = URL as unknown as Record<string, unknown>;
    const before: Record<string, PropertyDescriptor | undefined> = Object.getOwnPropertyDescriptors(URL);
    Object.defineProperty(URL, 'createObjectURL', {
      configurable: true,
      value: (each: Blob) => {
        made.push(each);
        return address;
      }
    });
    Object.defineProperty(URL, 'revokeObjectURL', {
      configurable: true,
      value: (each: string) => revoked.push(each)
    });
    const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) {
      clicked.push({ tag: this.tagName, href: this.href, download: this.download, joined: this.isConnected });
    });
    try {
      saveFile(blob, EXPORT_FILE);
      expect(made).toEqual([blob]);
      expect(clicked).toEqual([{ tag: 'A', href: address, download: EXPORT_FILE, joined: false }]);
      // the address is revoked once the save has begun, not before
      expect(revoked).toEqual([]);
      await new Promise((resolve) => setTimeout(resolve, 0));
      expect(revoked).toEqual([address]);
    } finally {
      click.mockRestore();
      for (const name of ['createObjectURL', 'revokeObjectURL']) {
        const kept = before[name];
        if (kept === undefined) delete urls[name];
        else Object.defineProperty(URL, name, kept);
      }
    }
  });
});
