# Jitter follows lets-plot bit for bit (Phase 2.4)

Parity tests: `tests/test_jitter_parity.py` (seeded renders, point centres).

- lets-plot's Python runs Kotlin compiled to JS; `Random(seed)` is Kotlin's XorWow
  (`position/random.rs`). A seeded lets-plot render IS the RNG reference: no JVM or
  Kotlin stdlib is installed here (Java 11 only).
- Offsets: `(2u - 1) x width x resolution(x)`, then `(2v - 1) x height x resolution(y)`,
  drawn x then y per row in row order; width/height default 0.4. y resolution is the
  smallest gap between y values, so continuous y barely moves (as lets-plot).
- Jitterdodge = jitter, then dodge the jittered x around the UNJITTERED x
  (`dodge(data, width, jitter)` in `position/dodge.rs`).
- `geom_jitter` = point geom whose default position takes the layer's `width`,
  `height`, `seed` (`DefaultPosition::from_layer`).
- **Deliberate difference:** no seed -> `DEFAULT_SEED` (0), so renders and goldens are
  reproducible; lets-plot reseeds every render.

**How to apply:** any random position (sina, repel) should reuse `KotlinRandom` and
check a seeded lets-plot render first.
