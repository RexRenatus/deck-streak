<script lang="ts">
  import { onMount } from 'svelte';
  import { m } from '#lib/paraglide/messages.js';
  import { COLOUR_MODES, deviceStorage, readColourMode, storeColourMode, type ColourMode } from './colour-mode';

  // The three-way colour mode (SPEC-593 R8; ADR-616 D1), kept on the device. The server renders no
  // stored choice, so the control reads it once it has mounted.
  let mode = $state<ColourMode>('system');

  onMount(() => {
    mode = readColourMode(deviceStorage());
  });

  /** The label of a choice, in the page's language. */
  function label(option: ColourMode): string {
    if (option === 'light') return m.colour_mode_light();
    if (option === 'dark') return m.colour_mode_dark();
    return m.colour_mode_system();
  }

  function choose(option: ColourMode): void {
    mode = option;
    storeColourMode(deviceStorage(), document.documentElement, option);
  }
</script>

<fieldset>
  <legend class="text-sm font-medium">{m.colour_mode()}</legend>
  <div class="mt-2 flex flex-wrap gap-x-6 gap-y-2">
    {#each COLOUR_MODES as option (option)}
      <label class="flex min-h-11 items-center gap-2">
        <input type="radio" name="colour-mode" value={option} checked={mode === option} onchange={() => choose(option)} />
        {label(option)}
      </label>
    {/each}
  </div>
</fieldset>
