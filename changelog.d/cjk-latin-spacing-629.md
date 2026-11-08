### Fixed

- The Japanese, Simplified Chinese and Traditional Chinese messages of the web app no longer carry
  a space typed between a CJK character and a Latin letter, a digit, a placeholder or a percent
  sign, so every such boundary renders the same narrow gap the stylesheet draws; the census of the
  web app's message catalogs now refuses such a space by name, with its count (#1053).
