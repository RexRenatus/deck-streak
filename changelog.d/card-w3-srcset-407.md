### Fixed

- A card's image-set candidates (`srcset` and `picture source`) are held in every browser engine, Firefox included: the card document now inherits its content policy from a host document at its creation, so a candidate the engine fetches ahead of the document's own policy is refused.
