<script lang="ts">
  // SPEC-377 R9, R11; ADR-388 D5, D13. The choice screen.
  import { m } from '$lib/paraglide/messages.js';
  import type { ChoiceClient } from './status';

  let {
    client,
    lost,
    ondone
  }: { client: ChoiceClient; lost: boolean; ondone: (outcome: 'written' | 'cancelled') => void } = $props();

</script>

<section class="flex flex-col gap-3">
  <h2 class="text-lg font-medium">{m.sync_choose()}</h2>
  <button
    type="button"
    class="min-h-11 rounded-md border px-4"
    onclick={() => void client.choiceConfirm('upload').then(() => ondone('written'))}>{m.sync_choice_upload()}</button
  >
  <button
    type="button"
    class="min-h-11 rounded-md border px-4"
    onclick={() => void client.choiceConfirm('download').then(() => ondone('written'))}
    >{lost ? m.sync_choice_restore() : m.sync_choice_download()}</button
  >
</section>
