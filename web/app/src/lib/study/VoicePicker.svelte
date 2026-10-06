<script lang="ts">
  // SPEC-350 R16, A27; ADR-361 D8, D13: the voice picker.
  import type { Voice, VoiceChoices } from './voice';

  /** Where the picker reads the device's voices: `speechSynthesis` is one. */
  interface VoiceList {
    getVoices(): Voice[];
    addEventListener(type: 'voiceschanged', listener: () => void): void;
    removeEventListener(type: 'voiceschanged', listener: () => void): void;
  }

  let {}: { choices: VoiceChoices; languages: readonly string[]; synthesis: VoiceList | undefined } = $props();
</script>
