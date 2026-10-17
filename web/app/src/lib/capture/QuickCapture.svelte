<script lang="ts">
  import type { Answer } from '$lib/api';
  import { m } from '$lib/paraglide/messages.js';
  import {
    QUICK_TEXT_CHARS,
    fits,
    newCaptureId,
    type CaptureKind,
    type CaptureRequest,
    type Saved
  } from './capture';

  // The quick capture (SPEC-118 R11): one text field and a journal choice. Each capture carries a
  // fresh retry key, and pressing save again after a failure resends the same capture with the
  // same key, so a capture whose answer was lost is written once (R10, A17, A24). A capture whose
  // text or kind changed is a new capture, with a key of its own.
  let { send }: { send: (request: CaptureRequest) => Promise<Answer<Saved>> } = $props();

  let text = $state('');
  let journal = $state(false);
  let sending = $state(false);
  let outcome = $state<'failed' | 'reopen' | { saved: string } | null>(null);
  // the last capture sent and not yet saved: a retry of it reuses its key
  let pending: CaptureRequest | null = null;

  const ready = $derived(!sending && fits(text));

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!ready) return;
    const kind: CaptureKind = journal ? 'journal' : 'text';
    const request: CaptureRequest =
      pending !== null && pending.kind === kind && pending.text === text
        ? pending
        : { captureId: newCaptureId(), kind, text };
    pending = request;
    sending = true;
    const answer = await send(request);
    sending = false;
    if (answer.kind === 'ok') {
      outcome = { saved: answer.value.name };
      pending = null;
      text = '';
      journal = false;
    } else {
      outcome = answer.kind === 'reopen' ? 'reopen' : 'failed';
    }
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
    readonly={sending}
    bind:value={text}
  ></textarea>
  <p id="capture-hint" class="mt-1 text-sm">{m.capture_text_hint({ max: QUICK_TEXT_CHARS })}</p>
  <label class="mt-4 flex min-h-11 items-center gap-2">
    <input type="checkbox" disabled={sending} bind:checked={journal} />
    {m.capture_journal()}
  </label>
  <button
    type="submit"
    class="mt-4 min-h-11 rounded-md border px-4 font-medium disabled:opacity-60"
    disabled={!ready}
  >
    {sending ? m.capture_saving() : m.capture_save()}
  </button>
  <div role="status" class="mt-4">
    {#if outcome !== null && typeof outcome === 'object'}
      {m.capture_saved({ name: outcome.saved })}
    {/if}
  </div>
  {#if outcome === 'failed'}
    <p role="alert" class="mt-2">{m.capture_failed()}</p>
  {:else if outcome === 'reopen'}
    <p role="alert" class="mt-2">{m.reopen_from_telegram()}</p>
  {/if}
</form>
