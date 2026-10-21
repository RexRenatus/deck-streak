<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { resolve, sideAfter, type Action, type Intent, type Side } from '$lib/remote/actions';
  import { GamepadReader, snapshotOf, type PadReading } from '$lib/remote/gamepad';
  import { readKey } from '$lib/remote/keys';
  import { append, type LogEntry, type Source } from '$lib/remote/log';
  import { WakeLockHolder } from '$lib/remote/wake-lock';

  // SPEC-343 R16; ADR-354. The remote harness: it reads each gamepad every animation frame and the
  // keys from the window through the remote modules, drives a simulated review, holds the screen
  // lock while a review is open, a gamepad is connected and the page is visible, and logs the last
  // 200 events. It makes no network request of its own.

  const reader = new GamepadReader();
  const holder = new WakeLockHolder(navigator.wakeLock, () => {
    lock = holder.state;
    refusal = holder.refusal;
    record('lock', lock);
  });

  let review = $state(false);
  let side = $state<Side>('question');
  let visibility = $state(document.visibilityState);
  let lock = $state(holder.state);
  let refusal = $state<string | null>(null);
  let keyShown = $state<string>(m.remote_none());
  let codeShown = $state<string>(m.remote_none());
  let pads = $state.raw<PadReading[]>([]);
  let log = $state.raw<LogEntry[]>([]);

  /** The raw input each gamepad last logged, so a frame that changed nothing logs nothing. */
  const logged = new Map<number, string>();

  /** Adds an event to the log, with the page's visibility and the lock's state at that moment. */
  function record(source: Source, raw: string, action: Action | null = null): void {
    log = append(log, {
      time: Math.round(performance.now()),
      source,
      raw,
      action,
      visibility,
      lock
    });
  }

  /** Fires `intent` on the review's side, moves the side, and logs it. */
  function fire(source: Source, raw: string, intent: Intent | null): void {
    const action = intent && resolve(intent, side);
    side = sideAfter(action, side);
    record(source, raw, action);
  }

  /** Tells the holder the condition: a review open, a gamepad connected, the page visible. */
  function sync(): void {
    holder.set({ review, gamepad: pads.length > 0, visible: visibility === 'visible' });
  }

  /** One animation frame: every gamepad read against its last frame. */
  function frame(): void {
    const snapshots = navigator
      .getGamepads()
      .flatMap((pad) => (pad === null ? [] : [snapshotOf(pad)]));
    const readings = reader.read(snapshots);
    for (const reading of readings) {
      const raw = `[${reading.pressed.join(' ')}]`;
      for (const intent of reading.fired) {
        fire('gamepad', raw, intent);
      }
      if (reading.fired.length === 0 && raw !== logged.get(reading.index)) {
        record('gamepad', raw);
      }
      logged.set(reading.index, raw);
    }
    pads = readings;
    sync();
  }

  function onkeydown(event: KeyboardEvent): void {
    keyShown = JSON.stringify(event.key);
    codeShown = event.code;
    fire('key', `${keyShown} ${event.code}`, readKey(event));
  }

  function connected(event: GamepadEvent): void {
    record('connection', `connected ${event.gamepad.index}`);
  }

  function disconnected(event: GamepadEvent): void {
    reader.forget(event.gamepad.index);
    pads = pads.filter((pad) => pad.index !== event.gamepad.index);
    record('connection', `disconnected ${event.gamepad.index}`);
    sync();
  }

  function visibilityChanged(): void {
    visibility = document.visibilityState;
    record('visibility', visibility);
    sync();
  }

  function toggleReview(): void {
    review = !review;
    record('review', String(review));
    sync();
  }

  // The log opens with the page's visibility.
  record('visibility', document.visibilityState);

  // The frame loop runs while the screen is open; closing the screen closes the review, which
  // releases the lock.
  $effect(() => {
    let id = requestAnimationFrame(function tick() {
      frame();
      id = requestAnimationFrame(tick);
    });
    return () => {
      cancelAnimationFrame(id);
      review = false;
      sync();
    };
  });
</script>

<svelte:window
  {onkeydown}
  ongamepadconnected={connected}
  ongamepaddisconnected={disconnected}
/>
<svelte:document onvisibilitychange={visibilityChanged} />

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">{m.remote_title()}</h1>
  <p class="mt-3">{m.remote_intro()}</p>

  <button
    type="button"
    class="mt-6 inline-flex min-h-11 items-center rounded-md border px-4"
    onclick={toggleReview}
  >
    {review ? m.remote_review_close() : m.remote_review_open()}
  </button>

  <section class="mt-8" aria-labelledby="remote-status">
    <h2 id="remote-status" class="text-sm font-medium">{m.remote_status()}</h2>
    <dl class="mt-2 grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
      <dt>{m.remote_side()}</dt>
      <dd>{side}</dd>
      <dt>{m.remote_visibility()}</dt>
      <dd>{visibility}</dd>
      <dt>{m.remote_lock()}</dt>
      <dd>{lock}</dd>
      <dt>{m.remote_refusal()}</dt>
      <dd>{refusal ?? m.remote_none()}</dd>
      <dt>{m.remote_key()}</dt>
      <dd>{keyShown}</dd>
      <dt>{m.remote_code()}</dt>
      <dd>{codeShown}</dd>
    </dl>
  </section>

  <section class="mt-8" aria-labelledby="remote-gamepads">
    <h2 id="remote-gamepads" class="text-sm font-medium">{m.remote_gamepads()}</h2>
    {#each pads as pad (pad.index)}
      <dl class="mt-2 grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
        <dt>{m.remote_gamepad()}</dt>
        <dd><code>{pad.id}</code></dd>
        <dt>{m.remote_mapping()}</dt>
        <dd>{pad.mapping}</dd>
        <dt>{m.remote_pressed()}</dt>
        <dd>{pad.pressed.join(' ')}</dd>
        <dt>{m.remote_axes()}</dt>
        <dd>{pad.axes.map((axis) => axis.toFixed(2)).join(' ')}</dd>
      </dl>
    {:else}
      <p class="mt-2 text-sm">{m.remote_no_gamepad()}</p>
    {/each}
  </section>

  <section class="mt-8" aria-labelledby="remote-log">
    <h2 id="remote-log" class="text-sm font-medium">{m.remote_log()}</h2>
    <table class="mt-2 w-full text-sm" aria-labelledby="remote-log">
      <caption class="text-left">{m.remote_log_caption()}</caption>
      <thead>
        <tr>
          <th scope="col" class="text-left">{m.remote_log_time()}</th>
          <th scope="col" class="text-left">{m.remote_log_source()}</th>
          <th scope="col" class="text-left">{m.remote_log_raw()}</th>
          <th scope="col" class="text-left">{m.remote_log_action()}</th>
          <th scope="col" class="text-left">{m.remote_log_visibility()}</th>
          <th scope="col" class="text-left">{m.remote_log_lock()}</th>
        </tr>
      </thead>
      <tbody>
        {#each log as entry (entry)}
          <tr>
            <td>{entry.time}</td>
            <td>{entry.source}</td>
            <td><code>{entry.raw}</code></td>
            <td>{entry.action}</td>
            <td>{entry.visibility}</td>
            <td>{entry.lock}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
