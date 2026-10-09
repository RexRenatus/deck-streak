/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { describe, expect, it } from 'vitest';
import type { ChoiceConfirmed, ChoiceCounted, Direction } from '$lib/engine/protocol';
import { m } from '$lib/paraglide/messages.js';
import { getLocale, overwriteGetLocale } from '$lib/paraglide/runtime.js';
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

/** The text of each paragraph the screen shows, in order. */
function texts(container: HTMLElement): string[] {
  return [...container.querySelectorAll('p')].map((each) => each.textContent?.trim() ?? '');
}

/** Each button the screen shows, by its name, with whether it is disabled. */
function buttons(): [string, boolean][] {
  return screen
    .queryAllByRole('button')
    .map((each) => [each.textContent?.trim() ?? '', (each as HTMLButtonElement).disabled]);
}

/** A choice whose count fails, as a Worker that stopped does. */
class FailingChoice extends FakeChoice {
  override async choiceCount(): Promise<ChoiceCounted> {
    this.calls.push('choiceCount');
    throw new Error('the Worker stopped');
  }
}

/** A choice whose confirm waits until the test answers it, with an answer or a failure. */
class HeldChoice extends FakeChoice {
  answer: (value: ChoiceConfirmed | Error) => void = () => {};

  override async choiceConfirm(direction: Direction): Promise<ChoiceConfirmed> {
    this.calls.push(`choiceConfirm ${direction}`);
    const value = await new Promise<ChoiceConfirmed | Error>((resolve) => (this.answer = resolve));
    if (value instanceof Error) throw value;
    return value;
  }
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

  it('a cancel leaves nothing to tap, and a count answered after it changes nothing', async () => {
    // with the counts shown
    const fake = new FakeChoice(counted(UPLOAD, DOWNLOAD));
    const showing = shown(fake);
    await settle();
    expect(screen.getByRole('heading', { level: 2, name: m.sync_choose() })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_cancel() }));
    await settle();
    expect([texts(showing.view.container), buttons(), showing.done]).toEqual([[], [], ['cancelled']]);
    showing.view.unmount();
    // while counting, the count answering after the cancel
    let answer: (value: ChoiceCounted) => void = () => {};
    const slow = new FakeChoice(new Promise<ChoiceCounted>((resolve) => (answer = resolve)));
    const counting = shown(slow);
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_cancel() }));
    await settle();
    answer(counted(UPLOAD, DOWNLOAD));
    await settle();
    expect([texts(counting.view.container), buttons(), counting.done]).toEqual([[], [], ['cancelled']]);
    counting.view.unmount();
  });

  it('a count that counted nothing, or failed, offers no direction and says the choice replaced nothing', async () => {
    const cases: [string, FakeChoice][] = [
      ['nothing counted', new FakeChoice({ status: 'needs-sign-in', counts: null, snapshot: null })],
      ['the count failed', new FailingChoice()]
    ];
    for (const [what, fake] of cases) {
      const { view } = shown(fake);
      await settle();
      expect(texts(view.container), what).toEqual([m.sync_choice_refused()]);
      expect(buttons(), what).toEqual([[m.sync_choice_cancel(), false]]);
      view.unmount();
    }
    console.log(`examined ${cases.length} counts that offer nothing`);
    // counts with both directions say no sentence beside them, and an upload with no snapshot
    // answer waits
    const { view } = shown(new FakeChoice(counted(UPLOAD, DOWNLOAD, null)));
    await settle();
    expect(texts(view.container)).toEqual([
      m.sync_choice_upload_loses(UPLOAD),
      m.sync_choice_snapshot_waits(),
      m.sync_choice_download_loses(DOWNLOAD)
    ]);
    expect(buttons()).toEqual([
      [m.sync_choice_upload(), true],
      [m.sync_choice_download(), false],
      [m.sync_choice_cancel(), false]
    ]);
    view.unmount();
  });

  it('a tap holds every button until the Worker answers, and a failed tap says it replaced nothing', async () => {
    const fake = new HeldChoice(counted(UPLOAD, DOWNLOAD));
    const { view, done } = shown(fake);
    await settle();
    const all = (held: boolean): [string, boolean][] => [
      [m.sync_choice_upload(), held],
      [m.sync_choice_download(), held],
      [m.sync_choice_cancel(), held]
    ];
    expect(buttons()).toEqual(all(false));
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_download() }));
    await settle();
    expect(buttons()).toEqual(all(true));
    fake.answer({ status: 'held', outcome: 'changed', why: 'server', counts: { upload: UPLOAD, download: DOWNLOAD } });
    await settle();
    expect(buttons()).toEqual(all(false));
    expect(screen.getByText(m.sync_choice_changed_server())).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: m.sync_choice_upload() }));
    await settle();
    expect(buttons()).toEqual(all(true));
    fake.answer(new Error('the Worker stopped'));
    await settle();
    expect(buttons()).toEqual(all(false));
    expect(texts(view.container)).toEqual([
      m.sync_choice_refused(),
      m.sync_choice_upload_loses(UPLOAD),
      m.sync_choice_snapshot_age({ age: snapshotAge(7200, getLocale()) }),
      m.sync_choice_download_loses(DOWNLOAD)
    ]);
    expect([fake.calls, done]).toEqual([['choiceCount', 'choiceConfirm download', 'choiceConfirm upload'], []]);
    view.unmount();
  });

  it("the snapshot's age reads in the page's language", async () => {
    const original = getLocale;
    const fallback = new Intl.RelativeTimeFormat().resolvedOptions().locale;
    // the page's language and the runtime's own say the age differently, so the screen's is judged
    expect(snapshotAge(7200, 'fr')).not.toBe(snapshotAge(7200, fallback));
    overwriteGetLocale(() => 'fr');
    try {
      const { view } = shown(new FakeChoice(counted(UPLOAD, DOWNLOAD, { found: true, age: 7200 })));
      await settle();
      expect(screen.getByText(m.sync_choice_snapshot_age({ age: snapshotAge(7200, 'fr') }))).toBeTruthy();
      view.unmount();
    } finally {
      overwriteGetLocale(original);
    }
  });
});
