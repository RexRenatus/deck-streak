<script lang="ts">
  import type { Answer } from '$lib/api';
  import { m } from '$lib/paraglide/messages.js';
  import { QUICK_TEXT_CHARS, fits, type CaptureRequest, type Saved } from './capture';

  // The quick capture (SPEC-118 R11): one text field and a journal choice. A stub until its tests
  // are red.
  let { send }: { send: (request: CaptureRequest) => Promise<Answer<Saved>> } = $props();

  let text = $state('');
  let journal = $state(false);
  const ready = $derived(fits(text) && send !== undefined);

  function save(event: SubmitEvent): void {
    event.preventDefault();
  }
</script>

<form onsubmit={save} aria-labelledby="capture-title">
  <h2 id="capture-title" class="text-sm font-medium">{m.capture_title()}</h2>
  <label for="capture-text" class="mt-4 block">{m.capture_text_label()}</label>
  <textarea
    id="capture-text"
    class="mt-2 block w-full rounded-md border bg-background p-2 text-foreground"
    rows="6"
    maxlength={QUICK_TEXT_CHARS}
    aria-describedby="capture-hint"
    bind:value={text}
  ></textarea>
  <p id="capture-hint" class="mt-1 text-sm">{m.capture_text_hint({ max: QUICK_TEXT_CHARS })}</p>
  <label class="mt-4 flex min-h-11 items-center gap-2">
    <input type="checkbox" bind:checked={journal} />
    {m.capture_journal()}
  </label>
  <button
    type="submit"
    class="mt-4 min-h-11 rounded-md border px-4 font-medium disabled:opacity-60"
    disabled={!ready}
  >
    {m.capture_save()}
  </button>
  <div role="status" class="mt-4"></div>
</form>
