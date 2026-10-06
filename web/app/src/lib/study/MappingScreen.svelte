<script lang="ts">
  // SPEC-350 R18, A30; ADR-361 D15. The mapping screen: the remote's mapping on this device, one row
  // per action with its keys and its gamepad buttons. A learner changes one by choosing it and then
  // pressing the new key or button on the remote. The gamepads are read each animation frame only
  // while the screen waits for a button, and their first frame is a baseline, so a button already
  // held is not taken. Escape or Cancel ends the wait. Each mode returns to its default apart, and
  // the left stick's grades and the review's single-key shortcuts (WCAG 2.1.4) are switched here
  // too; all of it is kept on this device.
  import { m } from '$lib/paraglide/messages.js';
  import type { Intent } from '$lib/remote/actions';
  import { snapshotOf, type PadSnapshot } from '$lib/remote/gamepad';
  import { deviceStorage, KeySwitch } from './input';
  import { buttonOf, INTENTS, keyOf, MappingStore, type Mode } from './mapping-store';

  const store = new MappingStore(deviceStorage());
  const keySwitch = new KeySwitch(deviceStorage());

  /** Each action's name: the review's own, and the confirm key's two steps. */
  const NAMES: Record<Intent, () => string> = {
    confirm: m.study_mapping_confirm,
    again: m.study_again,
    hard: m.study_hard,
    good: m.study_good,
    easy: m.study_easy,
    undo: m.study_undo,
    bury: m.study_bury,
    flag: m.study_flag,
    replay: m.study_replay
  };

  /** What the screen waits for: the new key or the new button of one action. */
  interface Wait {
    mode: Mode;
    intent: Intent;
  }

  let mapping = $state.raw(store.mapping);
  let waiting: Wait | null = $state.raw(null);

  /** The inputs that fire `intent` in `map`, by name, or None. */
  function named<K>(map: ReadonlyMap<K, Intent>, intent: Intent, name: (input: K) => string): string {
    const inputs = [...map].filter(([, fired]) => fired === intent).map(([input]) => name(input));
    return inputs.length > 0 ? inputs.join(', ') : m.study_mapping_none();
  }

  function keyName(key: string): string {
    return key === ' ' ? m.study_mapping_space() : key;
  }

  function buttonName(index: number): string {
    return m.study_mapping_button({ index });
  }

  function prompt(wait: Wait): string {
    const action = NAMES[wait.intent]();
    return wait.mode === 'keyboard'
      ? m.study_mapping_press_key({ action })
      : m.study_mapping_press_button({ action });
  }

  /** The store's mapping on the screen, and the wait over. */
  function settle(): void {
    mapping = store.mapping;
    waiting = null;
  }

  function restore(mode: Mode): void {
    store.restore(mode);
    settle();
  }

  function keydown(event: KeyboardEvent): void {
    if (waiting === null) return;
    if (event.key === 'Escape') {
      waiting = null;
      return;
    }
    const key = keyOf(event);
    if (waiting.mode === 'keyboard' && key !== null) {
      // the key is the remote's answer, never a press of the control that holds the focus
      event.preventDefault();
      store.bindKey(waiting.intent, key);
      settle();
    }
  }

  // while the screen waits for a button, each gamepad is read every animation frame
  $effect(() => {
    if (waiting?.mode !== 'gamepad') return;
    const { intent } = waiting;
    const before = new Map<number, PadSnapshot>();
    let frame = requestAnimationFrame(function tick() {
      const pads = navigator
        .getGamepads()
        .filter((pad): pad is Gamepad => pad !== null)
        .map(snapshotOf);
      for (const pad of pads) {
        const button = buttonOf(before.get(pad.index), pad);
        before.set(pad.index, pad);
        if (button !== null) {
          store.bindButton(intent, button);
          settle();
          return;
        }
      }
      frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  });
</script>

<svelte:window onkeydown={keydown} />

<main class="mx-auto flex max-w-prose flex-col px-4 py-6">
  <h1 class="text-2xl font-semibold tracking-tight">{m.study_mapping_title()}</h1>
  <p class="mt-1 text-sm">{m.study_mapping_intro()}</p>

  <div class="mt-4 flex min-h-11 flex-wrap items-center gap-3">
    <p role="status">
      {#if waiting !== null}{prompt(waiting)}{/if}
    </p>
    {#if waiting !== null}
      <button
        type="button"
        class="min-h-11 min-w-11 rounded-md border px-3 transition-colors duration-150"
        onclick={() => (waiting = null)}
      >
        {m.study_mapping_cancel()}
      </button>
    {/if}
  </div>

  <table class="mt-2 w-full text-left">
    <thead>
      <tr>
        <th scope="col" class="py-2 pr-3 font-medium">{m.study_mapping_action()}</th>
        <th scope="col" class="py-2 pr-3 font-medium">{m.study_mapping_key()}</th>
        <th scope="col" class="py-2 font-medium">{m.study_mapping_gamepad()}</th>
      </tr>
    </thead>
    <tbody>
      {#each INTENTS as intent (intent)}
        {@const action = NAMES[intent]()}
        {@const keys = named(mapping.keys, intent, keyName)}
        {@const buttons = named(mapping.buttons, intent, buttonName)}
        <tr class="border-t">
          <th scope="row" class="py-2 pr-3 font-normal">{action}</th>
          <td class="py-2 pr-3">
            <button
              type="button"
              class="min-h-11 min-w-11 rounded-md border px-3 transition-colors duration-150"
              aria-label={m.study_mapping_change_key({ action, input: keys })}
              onclick={() => (waiting = { mode: 'keyboard', intent })}
            >
              {keys}
            </button>
          </td>
          <td class="py-2">
            <button
              type="button"
              class="min-h-11 min-w-11 rounded-md border px-3 transition-colors duration-150"
              aria-label={m.study_mapping_change_button({ action, input: buttons })}
              onclick={() => (waiting = { mode: 'gamepad', intent })}
            >
              {buttons}
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <div class="mt-4 flex flex-wrap gap-2">
    <button
      type="button"
      class="min-h-11 min-w-11 rounded-md border px-3 transition-colors duration-150"
      onclick={() => restore('keyboard')}
    >
      {m.study_mapping_restore_keys()}
    </button>
    <button
      type="button"
      class="min-h-11 min-w-11 rounded-md border px-3 transition-colors duration-150"
      onclick={() => restore('gamepad')}
    >
      {m.study_mapping_restore_buttons()}
    </button>
  </div>

  <label class="mt-6 inline-flex min-h-11 items-center gap-2">
    <input
      type="checkbox"
      class="size-6"
      checked={mapping.stick.length > 0}
      onchange={(event) => {
        store.setStick(event.currentTarget.checked);
        mapping = store.mapping;
      }}
    />
    {m.study_mapping_stick()}
  </label>
  <label class="inline-flex min-h-11 items-center gap-2">
    <input
      type="checkbox"
      class="size-6"
      checked={keySwitch.on}
      onchange={(event) => (keySwitch.on = event.currentTarget.checked)}
    />
    {m.study_character_keys()}
  </label>

  <nav class="mt-4">
    <a href="/study/review" class="inline-flex min-h-11 items-center rounded-md border px-4">{m.study_mapping_back()}</a>
  </nav>
</main>
