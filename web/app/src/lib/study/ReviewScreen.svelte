<script lang="ts">
  // SPEC-350 R7, R8, R9, R10, R11; ADR-361. The review screen. The machine (`review.ts`) decides
  // what shows and which request leaves; the input (`input.ts`) turns a key, a gamepad, the stick
  // and a click into its one handler; this screen only draws the machine's face and wires the
  // browser's events to them. The card shows in the sandboxed frame, which fills the card area;
  // its body carries the card's classes, and the night-mode classes in Telegram's dark palette. The
  // status region announces a refusal, a card the frame refused and a done deck, whose designed
  // end is that message and the way back to the deck list. The screen lock is #663's holder, held
  // while a card is shown, a gamepad is connected and the page is visible. The face's clips play
  // through the page's one audio element and its speaker (SPEC-350 R15, R16): the Replay control
  // shows while the side has replay clips, and a voice picker for each language the card speaks.
  // The input reads the remote's mapping this device stores, and the nav links to its screen
  // (SPEC-350 R18, ADR-361 D15). SPEC-371 R10; ADR-382: while the review asks to undo its own last
  // answer, the control bar becomes a dialog that names the card as text, the answer and the state
  // it goes back to, with the focus on keeping it; Escape keeps it too. SPEC-376 R5, R6; ADR-387: a
  // card the engine marks past its due day shows one line, below the status line and above the card,
  // on both sides, saying its review does not count toward the streak for that day; a card on time
  // shows none.
  import { onMount } from 'svelte';
  import CardFrame from '$lib/card/CardFrame.svelte';
  import type { Returns } from '$lib/engine/protocol';
  import { m } from '$lib/paraglide/messages.js';
  import { snapshotOf } from '$lib/remote/gamepad';
  import { WakeLockHolder, type Conditions } from '$lib/remote/wake-lock';
  import { telegram } from '$lib/telegram.svelte';
  import AnswerButtons from './AnswerButtons.svelte';
  import { Player } from './audio';
  import { deviceStorage, StudyInput } from './input';
  import { MappingStore } from './mapping-store';
  import { statusText } from './refusal';
  import { Review, type ClipPlayer, type StudyClient } from './review';
  import { deviceSpeaker } from './speech';
  import { VoiceChoices } from './voice';
  import VoicePicker from './VoicePicker.svelte';

  let { client, player }: { client: () => Promise<StudyClient>; player?: ClipPlayer } = $props();

  let region: HTMLElement;

  const choices = new VoiceChoices(deviceStorage());
  const devicePlayer = new Player(new Audio(), URL, deviceSpeaker(choices));

  const review = new Review(
    () => client(),
    () => performance.now(),
    () => (shown = read()),
    { play: (clips) => (player ?? devicePlayer).play(clips) }
  );

  /** What the screen draws, read from the machine after each of its steps. */
  function read() {
    return {
      phase: review.phase,
      face: review.face,
      counts: review.counts,
      status: review.status,
      controls: review.controls,
      languages: review.languages,
      offer: review.offer
    };
  }

  let shown = $state.raw(read());
  const face = $derived(shown.face);

  /** The state an offered answer's card goes back to, by its message. */
  const RETURNS: Record<Returns, () => string> = {
    new: () => m.undo_returns_new(),
    learning: () => m.undo_returns_learning(),
    review: () => m.undo_returns_review(),
    relearning: () => m.undo_returns_relearning(),
    preview: () => m.undo_returns_preview()
  };

  /** "Keep it", which takes the focus while the review asks (SPEC-371 R10). */
  let keeper = $state<HTMLButtonElement>();

  const input = new StudyInput(
    {
      act: (action) => review.act(action),
      side: () => review.side,
      focus: () => region.focus()
    },
    deviceStorage(),
    new MappingStore(deviceStorage()).mapping
  );

  const holder = new WakeLockHolder(navigator.wakeLock);

  /** The gamepads connected, by index: those the page already had, then each event's. */
  const connected = new Set(
    navigator
      .getGamepads?.()
      .filter((pad): pad is Gamepad => pad !== null)
      .map((pad) => pad.index)
  );
  let gamepad = $state(connected.size > 0);
  let visible = $state(document.visibilityState === 'visible');

  /** The screen lock's condition, with whether a card is shown. */
  function conditions(card: boolean): Conditions {
    return { review: card, gamepad, visible };
  }

  function connect(event: GamepadEvent): void {
    connected.add(event.gamepad.index);
    gamepad = true;
  }

  function disconnect(event: GamepadEvent): void {
    connected.delete(event.gamepad.index);
    input.forget(event.gamepad.index);
    gamepad = connected.size > 0;
  }

  /** The window lost focus; the frame is the active element only once the blur has been dispatched. */
  function blurred(): void {
    setTimeout(() => input.blur(document.activeElement, region.querySelector('iframe')), 0);
  }

  function retry(): void {
    review.retry();
    region.focus();
  }

  /** Keeps the offered answer, and gives the focus back to the review. */
  function keep(): void {
    review.keep();
    region.focus();
  }

  /** A key on the page: while the review asks, Escape keeps the answer; every key else is the
   * input's. */
  function key(event: KeyboardEvent): void {
    if (shown.phase === 'confirming' && event.key === 'Escape') {
      keep();
      return;
    }
    input.key(event);
  }

  // while the review asks, the focus is on keeping the answer
  $effect(() => keeper?.focus());

  $effect(() => holder.set(conditions(face !== null)));
  $effect(() => () => holder.set(conditions(false)));

  // each gamepad is read every animation frame, only while one is connected and the page is visible
  $effect(() => {
    if (!gamepad || !visible) return;
    let frame = requestAnimationFrame(function tick() {
      input.pads(
        navigator
          .getGamepads()
          .filter((pad): pad is Gamepad => pad !== null)
          .map(snapshotOf)
      );
      frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  });

  onMount(() => review.start());
</script>

<!-- a11y-exception: the pointer's down event activates nothing; it marks the focus move it starts as a pointer's, which the window's blur reads -->
<svelte:window
  onkeydown={key}
  onpointerdown={() => input.pointer()}
  onblur={blurred}
  ongamepadconnected={connect}
  ongamepaddisconnected={disconnect}
/>
<svelte:document onvisibilitychange={() => (visible = document.visibilityState === 'visible')} />

<main class="mx-auto flex max-w-prose flex-col px-4 py-6">
  <h1 class="text-2xl font-semibold tracking-tight">{m.study_review_title()}</h1>
  {#if shown.counts !== null}
    <p class="mt-1 text-sm">
      {m.study_deck_counts({ fresh: shown.counts.new, learning: shown.counts.learning, review: shown.counts.review })}
    </p>
  {/if}
  {#if shown.phase === 'loading'}
    <p class="mt-1 text-sm">{m.loading()}</p>
  {/if}

  <section bind:this={region} tabindex="-1" aria-label={m.study_review_title()} class="mt-4 flex flex-col gap-3">
    <p role="status" class="min-h-6">
      {#if shown.status !== null}{statusText(shown.status)}{/if}
    </p>
    {#if face !== null}
      <!-- a withheld card shows its line in the status region alone: no late line and no frame (SPEC-380 R5) -->
      {#if !face.view.withheld}
        {#if face.view.late}
          <p class="text-sm">{m.study_late_review()}</p>
        {/if}
        <div class="h-[55dvh] *:size-full *:border-0">
          <CardFrame
            html={face.side === 'answer' ? face.view.answer : face.view.question}
            css={face.view.css}
            title={face.side === 'answer' ? m.study_answer_frame() : m.study_question_frame()}
            classes={`card card${face.view.ordinal + 1}${telegram.colorScheme === 'dark' ? ' nightMode night_mode' : ''}`}
          />
        </div>
      {/if}
      {#if shown.offer !== null}
        {@const offer = shown.offer}
        <div
          role="alertdialog"
          aria-labelledby="undo-title"
          aria-describedby="undo-unsynced"
          class="flex flex-col gap-3 rounded-md border p-4"
        >
          <h2 id="undo-title" class="text-lg font-semibold">{m.undo_title()}</h2>
          <dl class="flex flex-col gap-1">
            <div>
              <dt class="inline font-medium">{m.undo_card()}</dt>{' '}<dd class="inline">
                {offer.text === '' ? m.undo_no_text() : offer.text}
              </dd>
            </div>
            <div>
              <dt class="inline font-medium">{m.undo_answer()}</dt>{' '}<dd class="inline">
                {offer.grade === 'again' ? m.study_again() : m.study_good()}
              </dd>
            </div>
            <div>
              <dt class="inline font-medium">{m.undo_returns()}</dt>{' '}<dd class="inline">
                {RETURNS[offer.returns]()}
              </dd>
            </div>
          </dl>
          <p id="undo-unsynced">{m.undo_unsynced()}</p>
          <div class="flex flex-wrap gap-2">
            <button
              type="button"
              class="min-h-11 rounded-md border px-4 transition-colors duration-150"
              onclick={() => input.click('undo')}
            >
              {m.study_undo_answer()}
            </button>
            <button
              bind:this={keeper}
              type="button"
              class="min-h-11 rounded-md bg-foreground px-4 font-medium text-background transition-colors duration-150"
              onclick={keep}
            >
              {m.undo_keep()}
            </button>
          </div>
        </div>
      {:else}
        {#if shown.controls.includes('show-answer')}
          <button
            type="button"
            class="min-h-11 rounded-md bg-foreground px-4 font-medium text-background transition-colors duration-150"
            onclick={() => input.click('show-answer')}
          >
            {m.study_show_answer()}
          </button>
        {:else if shown.controls.includes('good')}
          <AnswerButtons labels={face.view.labels} onanswer={(grade) => input.click(grade)} />
        {/if}
        <div class="flex flex-wrap gap-2">
          <button
            type="button"
            class="min-h-11 min-w-11 rounded-md border px-3 transition-colors duration-150 disabled:opacity-60"
            disabled={!shown.controls.includes('undo')}
            onclick={() => input.click('undo')}
          >
            {m.study_undo_answer()}
          </button>
          {#if shown.controls.includes('replay')}
            <button
              type="button"
              class="min-h-11 min-w-11 rounded-md border px-3 transition-colors duration-150"
              onclick={() => input.click('replay')}
            >
              {m.study_replay()}
            </button>
          {/if}
          <button
            type="button"
            class="min-h-11 min-w-11 rounded-md border px-3 transition-colors duration-150"
            onclick={() => input.click('bury')}
          >
            {m.study_bury()}
          </button>
          <button
            type="button"
            class="min-h-11 min-w-11 rounded-md border px-3 transition-colors duration-150 aria-pressed:border-foreground aria-pressed:bg-card aria-pressed:font-semibold"
            aria-pressed={face.view.flag === 1}
            onclick={() => input.click('flag')}
          >
            {m.study_flag()}
          </button>
        </div>
      {/if}
    {/if}
    {#if shown.phase === 'refused'}
      <button
        type="button"
        class="min-h-11 rounded-md bg-foreground px-4 font-medium text-background transition-colors duration-150"
        onclick={retry}
      >
        {m.study_retry()}
      </button>
    {/if}
  </section>

  <label class="mt-6 inline-flex min-h-11 items-center gap-2">
    <input
      type="checkbox"
      class="size-6"
      checked={input.characterKeys}
      onchange={(event) => (input.characterKeys = event.currentTarget.checked)}
    />
    {m.study_character_keys()}
  </label>
  <div class="flex flex-col">
    <VoicePicker {choices} languages={shown.languages} synthesis={globalThis.speechSynthesis} />
  </div>

  <nav class="mt-4">
    <a href="/study" class="inline-flex min-h-11 items-center rounded-md border px-4">{m.study_back_to_decks()}</a>
    <a href="/study/mapping" class="inline-flex min-h-11 items-center rounded-md border px-4">{m.study_mapping_title()}</a>
  </nav>
</main>
