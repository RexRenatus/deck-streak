### Fixed

- The dynamic-site census reason for the path gate loader's namespace read now names all that
  the namespace holds: what the script defines, the `__file__` the loader sets, the
  `__builtins__` that `exec` adds and the five entries every new module starts with.
