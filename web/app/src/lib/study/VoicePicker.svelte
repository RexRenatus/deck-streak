<script lang="ts">
  // SPEC-350 R16, A27; ADR-361 D8, D13. The voice picker: for each language the shown card speaks,
  // the language's default voice, then the device's voices for it. A choice is stored per language;
  // the list follows the device's voices, which a browser may load after the page.
  import { m } from '$lib/paraglide/messages.js';
  import type { Voice, VoiceChoices } from './voice';

  /** Where the picker reads the device's voices: `speechSynthesis` is one. */
  interface VoiceList {
    getVoices(): Voice[];
    addEventListener(type: 'voiceschanged', listener: () => void): void;
    removeEventListener(type: 'voiceschanged', listener: () => void): void;
  }

  let {
    choices,
    languages,
    synthesis
  }: { choices: VoiceChoices; languages: readonly string[]; synthesis: VoiceList | undefined } = $props();

  let voices: readonly Voice[] = $state.raw([]);

  $effect(() => {
    if (synthesis === undefined) return;
    const list = synthesis;
    const update = () => (voices = list.getVoices());
    update();
    list.addEventListener('voiceschanged', update);
    return () => list.removeEventListener('voiceschanged', update);
  });
</script>

{#each languages as language (language)}
  <label class="mt-2 inline-flex min-h-11 items-center gap-2">
    {m.study_voice({ language })}
    <select
      class="min-h-11 min-w-11 rounded-md border px-3"
      value={choices.voiceFor(voices, language)?.voiceURI ?? ''}
      onchange={(event) => choices.choose(language, event.currentTarget.value)}
    >
      <option value="">{m.study_voice_default()}</option>
      {#each choices.voices(voices, language) as voice (voice.voiceURI)}
        <option value={voice.voiceURI}>{voice.name}</option>
      {/each}
    </select>
  </label>
{/each}
