### Changed

- A pull request into dev also enters the Mergify merge queue when the `deckstreak-ready` GitHub
  App posts the status `queue-ready` with the state `success` on its head, read only in that App's
  qualified form; the label `queue` still queues it as before. A queue command admits only a pull
  request one of the two would admit, and a pull request whose status from that App reads
  `failure` stays out of the queue or leaves it.
