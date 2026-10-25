# Visual cases must not share a random generator at call time

A case function that draws from the module-level `RANDOM` *when called* gets data that
depends on which cases ran before it. Blessing with `-k <case>` then produced a golden
that failed in the full run (Phase 0.10.2b, `discrete_strip`).

**Rule:** module-level data may use `RANDOM` (drawn once at import). Inside a case,
use `np.random.default_rng(<own seed>)`. After blessing, run `pytest tests/visual`
both whole and with `-k` to confirm.
