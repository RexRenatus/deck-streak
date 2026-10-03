### Fixed

- The notification-policy check no longer reads a command reply as a notification. The policy file
  declares the replies' duty in a new top-level `replies` list, the router's loader types it and
  refuses at start an entry that names a declared kind, and a test holds every committed golden
  message to being a notification or a declared reply.
