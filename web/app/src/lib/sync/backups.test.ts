/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { BackupsClient } from '$lib/engine/backups';
import type { BackupsListed } from '$lib/engine/protocol';
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';
import BackupList from './BackupList.svelte';
import { snapshotAge } from './status';

// SPEC-377 R15 to R17, B5; ADR-388 D3, D5, D14. The sync screen's backup list: the backups in the
// order the Worker answered, newest first, each by its kind and age with an export that is a 44 px
// target; an export saves the bytes the Worker sent as a file named by kind and age; each backup
// retention removed is named; and an empty list says so.

/** A message by its key, or a marker naming the key while the locale files lack it. */
function said(key: string, inputs: Record<string, unknown> = {}): string {
  const message = (m as unknown as Record<string, ((inputs: Record<string, unknown>) => string) | undefined>)[key];
  return message === undefined ? `no message ${key}` : message(inputs);
}

/** How a backup's kind and age read on the list. */
function described(kind: 'backup' | 'server', age: number): string {
  const key = kind === 'backup' ? 'sync_backup_device' : 'sync_backup_server';
  return said(key, { age: snapshotAge(age, getLocale()) });
}

const LISTED: BackupsListed = {
  backups: [
    { id: 'backup-2', kind: 'backup', age: 5 },
    { id: 'server-1', kind: 'server', age: 7200 },
    { id: 'backup-4', kind: 'backup', age: 90000 }
  ],
  removed: []
};

/** The client's export, each call recorded, its bytes set. */
class FakeBackups implements BackupsClient {
  calls: string[] = [];
  bytes = new Uint8Array([83, 81, 76, 105, 116, 101]);
  refuse = false;

  async backups() {
    this.calls.push('backups');
    return LISTED;
  }

  async backupExport(backup: string) {
    this.calls.push(`backupExport ${backup}`);
    if (this.refuse) throw new Error('the engine stopped');
    return this.bytes;
  }
}

async function settle(): Promise<void> {
  for (let turn = 0; turn < 3; turn += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

/** The text of each listed backup's row, in order. */
function rows(container: HTMLElement): string[] {
  return [...container.querySelectorAll('tbody tr th')].map((each) => each.textContent?.trim() ?? '');
}

function exports(): HTMLButtonElement[] {
  return screen.queryAllByRole('button', { name: said('sync_backup_export') }) as HTMLButtonElement[];
}

function shown(listed: BackupsListed, fake = new FakeBackups()) {
  return render(BackupList, { listed, client: async () => fake });
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("the sync screen's backup list", () => {
  it('the backups are listed in the order the Worker answered, each by its kind and age', () => {
    const view = shown(LISTED);
    expect(rows(view.container)).toEqual([
      described('backup', 5),
      described('server', 7200),
      described('backup', 90000)
    ]);
    expect(view.container.querySelector('caption')?.textContent?.trim()).toBe(said('sync_backups_title'));
    view.unmount();
  });

  it('each backup has one export, a target of at least 44 px', () => {
    const view = shown(LISTED);
    // min-h-11 is 2.75rem, 44 px at the page's root size
    expect(exports().map((button) => button.classList.contains('min-h-11'))).toEqual([true, true, true]);
    expect(exports().map((button) => button.type)).toEqual(['button', 'button', 'button']);
    view.unmount();
  });

  it('an export saves the bytes the Worker sent as a file named by its kind and age', async () => {
    const fake = new FakeBackups();
    const made: Blob[] = [];
    const revoked: string[] = [];
    Object.defineProperty(URL, 'createObjectURL', {
      configurable: true,
      value: (blob: Blob) => {
        made.push(blob);
        return 'blob:https://app.example/backup';
      }
    });
    Object.defineProperty(URL, 'revokeObjectURL', { configurable: true, value: (url: string) => revoked.push(url) });
    const clicked: [string, string][] = [];
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) {
      clicked.push([this.download, this.href]);
    });
    const view = shown(LISTED, fake);
    expect(exports()).toHaveLength(3);
    await fireEvent.click(exports()[1]);
    await settle();
    expect(fake.calls).toEqual(['backupExport server-1']);
    expect(clicked).toEqual([['deck-streak-server-7200s.anki2', 'blob:https://app.example/backup']]);
    expect(made).toHaveLength(1);
    expect([...new Uint8Array(await made[0].arrayBuffer())]).toEqual([...fake.bytes]);
    expect(revoked).toEqual(['blob:https://app.example/backup']);
    // the anchor was only a means: the page keeps no element and no text of the export
    expect(view.container.querySelectorAll('a')).toHaveLength(0);
    view.unmount();
  });

  it('an export the Worker refuses says the engine stopped and saves nothing', async () => {
    const fake = new FakeBackups();
    fake.refuse = true;
    const clicked = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => undefined);
    const view = shown(LISTED, fake);
    expect(exports()).toHaveLength(3);
    expect(screen.queryByText(m.study_refused_engine())).toBeNull();
    await fireEvent.click(exports()[0]);
    await settle();
    expect(fake.calls).toEqual(['backupExport backup-2']);
    expect(screen.queryByText(m.study_refused_engine())).not.toBeNull();
    expect(clicked).not.toHaveBeenCalled();
    view.unmount();
  });

  it("a refused export's alert clears when the next export saves", async () => {
    const fake = new FakeBackups();
    fake.refuse = true;
    Object.defineProperty(URL, 'createObjectURL', { configurable: true, value: () => 'blob:https://app.example/backup' });
    Object.defineProperty(URL, 'revokeObjectURL', { configurable: true, value: () => undefined });
    const clicked = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => undefined);
    const view = shown(LISTED, fake);
    await fireEvent.click(exports()[0]);
    await settle();
    expect(screen.queryByText(m.study_refused_engine()), 'refused').not.toBeNull();
    fake.refuse = false;
    await fireEvent.click(exports()[0]);
    await settle();
    expect(fake.calls).toEqual(['backupExport backup-2', 'backupExport backup-2']);
    expect(clicked, 'the second export saved').toHaveBeenCalledTimes(1);
    expect(screen.queryByText(m.study_refused_engine()), 'saved').toBeNull();
    view.unmount();
  });

  it('a list retention removed nothing from names no removal', () => {
    const view = shown(LISTED);
    expect(rows(view.container), 'the backups are drawn').toHaveLength(3);
    expect(view.container.querySelectorAll('ul, li')).toHaveLength(0);
    view.unmount();
  });

  it('each backup retention removed is named once', () => {
    const view = shown({
      backups: LISTED.backups,
      removed: [
        { kind: 'backup', age: 400000 },
        { kind: 'server', age: 300000 }
      ]
    });
    const removals = [...view.container.querySelectorAll('li')].map((each) => each.textContent?.trim() ?? '');
    expect(removals).toEqual([
      said('sync_backup_removed', { backup: described('backup', 400000) }),
      said('sync_backup_removed', { backup: described('server', 300000) })
    ]);
    view.unmount();
  });

  it('a browser with no backup says so and offers no export', () => {
    const view = shown({ backups: [], removed: [] });
    expect(screen.queryByText(said('sync_backups_none'))).not.toBeNull();
    expect(exports()).toHaveLength(0);
    expect(rows(view.container)).toEqual([]);
    view.unmount();
  });
});
