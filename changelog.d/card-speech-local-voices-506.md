### Fixed

- Card speech on the web and in the Mini App now speaks only with a voice the device runs itself. A remote voice, which would send the card's text to a speech service, is never listed, chosen or spoken; a language with only remote voices, or none, is not spoken, and the review says "No voice on this device for" the language, in every locale. A remote voice chosen earlier is kept and ignored (SPEC-506, ADR-524).
