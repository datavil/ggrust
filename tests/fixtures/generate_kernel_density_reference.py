"""Writes kernel_density_scipy.json: 1D densities by scipy's ``gaussian_kde``.

`case_inputs()` is shared with `tests/test_kernel_density.py`. Each case stores
the density at every evaluation point (64 or fewer per case, to keep the fixture
small).

The recipe (`reference_density()`): ``gaussian_kde(values[np.isfinite(values)],
bw_method=bandwidth)(at)``. A float32 sample is passed to scipy as float32 (scipy
promotes it to float64 exactly, so ggrust gets the same numbers as float64).

Run with an environment that has scipy (not a ggrust dependency), e.g. cellestial's:
``python tests/fixtures/generate_kernel_density_reference.py``.
"""

import json
from pathlib import Path

import numpy as np

POINTS = 64


def case_inputs() -> list[tuple[str, np.ndarray, np.ndarray, str | float]]:
    """``(name, values, at, bandwidth)`` per case."""
    rng = np.random.default_rng(11)
    cases = []

    def grid(values: np.ndarray, size: int = POINTS) -> np.ndarray:
        finite = values[np.isfinite(values)]
        return np.linspace(float(finite.min()), float(finite.max()), size)

    normal = rng.normal(3.0, 1.2, 500)
    cases.append(("normal_scott", normal, grid(normal), "scott"))
    bimodal = np.concatenate([rng.normal(0, 0.5, 300), rng.normal(4, 1.0, 200)])
    cases.append(("bimodal_silverman", bimodal, grid(bimodal), "silverman"))
    cases.append(("bimodal_factor", bimodal, grid(bimodal), 0.15))
    cases.append(("bimodal_integer_factor", bimodal, grid(bimodal), 2))
    # Expression-like counts: many zeros and ties (log1p of Poisson counts).
    counts = np.log1p(rng.poisson(0.7, 2000).astype(np.float64))
    cases.append(("zero_heavy", counts, grid(counts, 50), "scott"))
    # The smallest samples.
    cases.append(("two_values", np.array([0.5, 2.0]), np.linspace(-1, 3.5, 19), "scott"))
    three = np.array([1.0, 1.5, 4.0])
    cases.append(("three_values", three, np.linspace(-2, 7, 31), "silverman"))
    # Large offset and scale, tiny scale.
    heavy = rng.gamma(2.0, 1.5, 800) * 1e4 + 1e6
    cases.append(("large_offset", heavy, grid(heavy), "scott"))
    tiny = rng.normal(0, 1e-6, 300)
    cases.append(("tiny_scale", tiny, grid(tiny), "scott"))
    # Non-finite values to drop; unsorted points outside the range.
    with_gaps = rng.normal(0, 1, 120)
    with_gaps[[3, 50]] = np.nan
    with_gaps[77] = np.inf
    outside = rng.uniform(-8, 8, 40)
    cases.append(("non_finite_unsorted", with_gaps, outside, "scott"))
    # A float32 sample (as anndata layers often are).
    single = rng.lognormal(0.0, 0.6, 1000).astype(np.float32)
    cases.append(("float32", single, grid(single), "scott"))
    # Larger than the parallel threshold, few points.
    large = rng.standard_t(4, 30000)
    cases.append(("large_sample", large, grid(large, 40), "scott"))
    return cases


def reference_density(values: np.ndarray, at: np.ndarray, bandwidth: str | float) -> np.ndarray:
    """The density at ``at`` by scipy."""
    from scipy.stats import gaussian_kde

    return gaussian_kde(values[np.isfinite(values)], bw_method=bandwidth)(at)


def main() -> None:
    import scipy

    cases = []
    for name, values, at, bandwidth in case_inputs():
        density = reference_density(values, at, bandwidth)
        cases.append({"name": name, "density": density.tolist()})
    reference = {"scipy": scipy.__version__, "numpy": np.__version__, "cases": cases}
    path = Path(__file__).with_name("kernel_density_scipy.json")
    path.write_text(json.dumps(reference) + "\n")
    print(f"wrote {path} ({path.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
