# Speeding up exact lets-plot ports: skip work, never change arithmetic

Steps 6.1d (repel) and 6.1h (KDE). Parity ports must stay bit for bit.

- **What works:** cache derived values and refresh them where state changes (repel:
  each label's expanded box and leader segment in `update_position`), and skip tests
  that provably return false (bounding boxes more than 1e-6 px apart; Gaussian terms
  past 39 bandwidths, where `exp` is exactly 0). Repel 12x faster, outputs identical.
- **Trap 1: ported predicates are not geometric.** lets-plot `pointInRectangle` treats
  a zero-width/height box as an unbounded strip; a bbox skip changed layouts with empty
  labels (found by fuzzing old vs new over 6000 seeds). Degenerate boxes get NaN bounds.
- **Trap 2: ties.** A 1e-13 change (KDE via an exp recurrence, 1.6x faster) flipped
  violin quantile bands exactly at ties (`test_violin_parity[quantile_tie]`). Reverted.
- Check: byte-compare outputs against a clean worktree of HEAD, and fuzz old vs new.
