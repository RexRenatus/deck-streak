# Schematic: the kernel's CPython numbers and how a golden proves them

Kind: component and data flow. Decided by ADR-090; built by SPEC-302.

## 1. The components

```mermaid
flowchart LR
  curriculum[curriculum, insights: later ports] -->|calls| pynum[kernel::pynum]
  analytics[analytics::metrics::python_sum] -->|delegates| pynum
  pynum --> sum[sum, median, mean, round]
  pynum --> pct[nearest_rank_percentile]
  pynum --> mt[PyRandom: Mersenne Twister, random, choices]
  pynum --> lg[lgamma]
```

The kernel depends on no internal crate (ADR-002), so every context reads one copy.

## 2. The proof

```mermaid
flowchart LR
  cpython[CPython 3.12: sum, statistics, round, random, math.lgamma] -->|adapter| gen[parity-oracle generate.py]
  pred[predecessor adaptive.percentile] -->|function| gen
  gen --> goldens[(goldens/*.json)]
  goldens --> tests[kernel tests: pynum_goldens, pynum_random]
  tests -->|bit-for-bit| pynum
```

Floats compare by bit pattern, so a value one unit in the last place away fails its golden.
