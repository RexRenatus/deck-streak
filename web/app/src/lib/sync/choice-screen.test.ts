/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { describe, expect, it } from 'vitest';
import type { ChoiceConfirmed, ChoiceCounted, Direction } from '$lib/engine/protocol';
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';
import ChoiceScreen from './ChoiceScreen.svelte';
import { snapshotAge, type ChoiceClient } from './status';

// SPEC-377 R9, R11, A12; ADR-388 D5, D13. The choice screen shows only the directions the engine
// offered, each with what it loses; preselects neither; shows an upload's snapshot age or that the
// upload waits; cancels at every step before the write; and returns to the counts with a sentence
// when they changed.
const UPLOAD = { reviews: 1, cards: 2, notes: 3 };
const DOWNLOAD = { reviews: 4, cards: 5, notes: 6 };

/** The engine's client as the choice screen sees it: each count and confirm answered in turn. */
class FakeChoice implements ChoiceClient {
  calls: string[] = [];
  counts: (ChoiceCounted | Promise<ChoiceCounted>)[];
  confirms: ChoiceConfirmed[] = [];

  constructor(...counts: (ChoiceCounted | Promise<ChoiceCounted>)[]) {
    this.counts = counts;
  }

  async choiceCount() {
    this.calls.push('choiceCount');
    return this.counts.shift() as ChoiceCounted;
  }

  async choiceConfirm(direction: Direction) {
    this.calls.push(`choiceConfirm ${direction}`);
    return this.confirms.shift() as ChoiceConfirmed;
  }

  async choiceCancel() {
    this.calls.push('choiceCancel');
    return null;
  }
}

async function settle(): Promise<void> {
  for (let turn = 0; turn < 3; turn += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

function shown(fake: FakeChoice, lost = false) {
  const done: string[] = [];
  const view = render(ChoiceScreen, { client: fake, lost, ondone: (outcome: string) => done.push(outcome) });
  return { view, done };
}

function counted(
  upload: typeof UPLOAD | null,
  download: typeof DOWNLOAD | null,
  snapshot: ChoiceCounted['snapshot'] = { found: true, age: 7200 }
): ChoiceCounted {
  return { status: 'held', counts: { upload, download }, snapshot };
}

describe('the choice screen', () => {
  it('shows only the offered directions with their counts, and preselects neither', async () => {
    const cases: [string, ChoiceCounted, Direction[]][] = [
      ['both', counted(UPLOAD, DOWNLOAD), ['upload', 'download']],
      ['upload alone', counted(UPLOAD, null), ['upload']],
      ['download alone', counted(null, DOWNLOAD), ['download']]
    ];
    for (const [what, answer, offered] of cases) {
      const { view } = shown(new FakeChoice(answer));
      await settle();
      expect(screen.queryByRole('button', { name: m.sync_choice_upload() }) !== null, what).toBe(offered.includes('upload'));
      expect(screen.queryByRole('button', { name: m.sync_choice_download() }) !== null, what).toBe(
        offered.includes('download')
      );
      expect(screen.queryByText(m.sync_choice_upload_loses(UPLOAD)) !== null, what).toBe(offered.includes('upload'));
      expect(screen.queryByText(m.sync_choice_download_loses(DOWNLOAD)) !== null, what).toBe(offered.includes('download'));
      // neither direction is chosen for the owner: no pressed button, no checked control, no focus
      expect(view.container.querySelectorAll('[aria-pressed="true"], [aria-checked="true"], :checked'), what).toHaveLength(0);
      expect(document.activeElement === document.body || document.activeElement === null, what).toBe(true);
      view.unmount();
    }
    console.log(`examined ${cases.length} offers`);
    // a lost collection's download is named as the restore
    const { view } = shown(new FakeChoice(counted(null, DOWNLOAD)), true);
    await settle();
    expect(screen.getByRole('button', { name: m.sync_choice_restore() })).toBeTruthy();
    expect(screen.queryByRole('button', { name: m.sync_choice_download() })).toBeNull();
    view.unmount();
  });

  it("shows an upload's snapshot age, or that the upload waits", async () => {
    const found = shown(new FakeChoice(counted(UPLOAD, DOWNLOAD, { found: true, age: 7200 })));
    await settle();
    expect(screen.getByText(m.sync_choice_snapshot_age({ age: snapshotAge(7200, getLocale()) }))).toBeTruthy();
    expect((screen.getByRole('button', { name: m.sync_choice_upload() }) as HTMLButtonElement).disabled).toBe(false);
    expect(screen.queryByText(m.sync_choice_snapshot_waits())).toBeNull();
    found.view.unmount();
    for (const snapshot of [{ found: false as const }, { found: null }]) {
      const each = shown(new FakeChoice(counted(UPLOAD, DOWNLOAD, snapshot)));
      await settle();
      expect(screen.getByText(m.sync_choice_snapshot_waits())).toBeTruthy();
      const upload = screen.getByRole('button', { name: m.sync_choice_upload() }) as HTMLButtonElement;
      expect(upload.disabled).toBe(true);
      // the download does not wait on the snapshot
      expect((screen.getByRole('button', { name: m.sync_choice_download() }) as HTMLButtonElement).disabled).toBe(false);
      each.view.unmount();
    }
    // the age reads as a past time in the page's language
    expect([snapshotAge(30, 'en'), snapshotAge(90, 'en'), snapshotAge(7200, 'en'), snapshotAge(259200, 'en')]).toEqual([
      '1 minute ago',
      '2 minutes ago',
      '2 hours ago',
      '3 days ago'
    ]);
    expect([snapshotAge(3599, 'en'), snapshotAge(3600, 'en'), snapshotAge(172799, 'en'), snapshotAge(172800, 'en')]).toEqual([
      '60 minutes ago',
      '1 hour ago',
      '48 hours ago',
      '2 days ago'
    ]);
  });

  it('cancels at every step before the write', async () => {
    // while counting
    let answer: (value: ChoiceCounted) => void = () => {};
    const slow = new FakeChoice(new Promise<ChoiceCounted>((resolve) => (answer = resolve)));
    const counting = shown(slow);
    await settle();
    expect(screen.getByText(m.sync_choice_counting())).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_cancel() }));
    await settle();
    expect([slow.calls, counting.done]).toEqual([['choiceCount', 'choiceCancel'], ['cancelled']]);
    answer(counted(UPLOAD, DOWNLOAD));
    counting.view.unmount();
    // with the counts shown
    const fake = new FakeChoice(counted(UPLOAD, DOWNLOAD));
    const showing = shown(fake);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_cancel() }));
    await settle();
    expect([fake.calls, showing.done]).toEqual([['choiceCount', 'choiceCancel'], ['cancelled']]);
    showing.view.unmount();
  });

  it('returns to the counts with a sentence when they changed, and says what the write did', async () => {
    const fake = new FakeChoice(counted(UPLOAD, DOWNLOAD));
    const NEW = { reviews: 9, cards: 9, notes: 9 };
    fake.confirms = [
      { status: 'held', outcome: 'changed', why: 'server', counts: { upload: NEW, download: DOWNLOAD } },
      { status: 'held', outcome: 'changed', why: 'device', counts: { upload: UPLOAD, download: NEW } },
      { status: 'held', outcome: 'refused', why: 'engine' },
      { status: 'held', outcome: 'written' }
    ];
    const { done } = shown(fake);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_upload() }));
    await settle();
    expect(screen.getByText(m.sync_choice_changed_server())).toBeTruthy();
    expect(screen.getByText(m.sync_choice_upload_loses(NEW))).toBeTruthy();
    expect(screen.queryByText(m.sync_choice_upload_loses(UPLOAD))).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_download() }));
    await settle();
    expect(screen.getByText(m.sync_choice_changed_device())).toBeTruthy();
    expect(screen.queryByText(m.sync_choice_changed_server())).toBeNull();
    expect(screen.getByText(m.sync_choice_download_loses(NEW))).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_download() }));
    await settle();
    expect(screen.getByText(m.sync_choice_refused())).toBeTruthy();
    // the counts stay, so the owner can tap again or cancel
    expect(screen.getByRole('button', { name: m.sync_choice_cancel() })).toBeTruthy();
    expect(done).toEqual([]);
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_download() }));
    await settle();
    expect(screen.getByText(m.sync_choice_written())).toBeTruthy();
    expect(done).toEqual(['written']);
    // no cancel after the write
    expect(screen.queryByRole('button', { name: m.sync_choice_cancel() })).toBeNull();
    expect(fake.calls).toEqual([
      'choiceCount',
      'choiceConfirm upload',
      'choiceConfirm download',
      'choiceConfirm download',
      'choiceConfirm download'
    ]);
  });
});
