"""Writes velocity_grid_scvelo.json: scVelo's ``compute_velocity_on_grid`` results.

`case_inputs()` is shared with `tests/test_velocity_grid.py`. Each case stores the
grid axes in full and, per velocity component, the NaN positions (as runs) and a summary
(sum, sum of squares and a position-weighted sum of the finite values, plus a few
sample values) to keep the fixture small.

Run with an environment that has scvelo (not a ggrust dependency), e.g. cellestial's:
``python tests/fixtures/generate_velocity_grid_reference.py``.
"""

import json
from pathlib import Path

import numpy as np

SAMPLES = 25


def case_inputs() -> list[tuple[str, np.ndarray, np.ndarray, dict]]:
    """``(name, points, vectors, options)`` per case, float64 throughout."""
    rng = np.random.default_rng(11)
    cases = []
    # a noisy spiral flow
    angle = rng.uniform(0, 6 * np.pi, 1500)
    radius = 1 + angle / 3 + rng.normal(0, 0.15, angle.size)
    points = np.column_stack([radius * np.cos(angle), radius * np.sin(angle)])
    vectors = np.column_stack([-points[:, 1], points[:, 0]]) + rng.normal(0, 0.3, points.shape)
    cases.append(("spiral", points, vectors, {}))
    cases.append(
        ("spiral_dense_smooth", points, vectors, {"density": 1.4, "smooth": 0.8, "n_neighbors": 12})
    )
    cases.append(("spiral_cutoff", points, vectors, {"min_mass": 4.5, "cutoff_percentile": 30}))
    cases.append(("spiral_quiver", points, vectors, {"adjust_for_stream": False}))
    cases.append(
        ("spiral_quiver_mass", points, vectors, {"adjust_for_stream": False, "min_mass": 20})
    )
    # two clusters on different scales, with non-finite rows to drop
    first = rng.normal([0, 0], [1, 0.2], (700, 2))
    second = rng.normal([30, 5], [4, 3], (900, 2))
    points = np.vstack([first, second])
    vectors = np.vstack([rng.normal([1, 0], 0.1, (700, 2)), rng.normal([0, -2], 1.0, (900, 2))])
    points[3, 0] = np.nan
    vectors[10, 1] = np.inf
    points[42, 1] = -np.inf
    cases.append(("clusters", points, vectors, {"density": 0.8}))
    # few cells, explicit neighbours, coarse grid
    points = rng.uniform(-5, 5, (60, 2))
    vectors = rng.normal(0, 1, (60, 2))
    cases.append(("small", points, vectors, {"n_neighbors": 7, "density": 0.3}))
    return cases


def _scvelo_options(options: dict) -> dict:
    """ggrust option names to scVelo's."""
    names = {"cutoff_percentile": "cutoff_perc"}
    return {names.get(key, key): value for key, value in options.items()}


def scvelo_grid(points: np.ndarray, vectors: np.ndarray, options: dict) -> tuple:
    """``(x, y, u, v)`` from scVelo, laid out as ``velocity_grid()`` returns them."""
    from scvelo.plotting.velocity_embedding_grid import compute_velocity_on_grid

    options = _scvelo_options(options)
    stream = options.pop("adjust_for_stream", True)
    density = options.get("density", 1)
    if stream:
        grid, velocity = compute_velocity_on_grid(
            points, vectors, autoscale=False, adjust_for_stream=True, **options
        )
        return grid[0], grid[1], velocity[0], velocity[1]
    # quiver: scVelo returns the kept grid points; ggrust keeps the grid with NaN.
    kept_points, kept_vectors = compute_velocity_on_grid(
        points, vectors, autoscale=False, adjust_for_stream=False, **options
    )
    size = int(50 * density)
    finite = points[np.isfinite(points.sum(1) + vectors.sum(1))]
    axes = []
    for dimension in range(2):
        low, high = finite[:, dimension].min(), finite[:, dimension].max()
        low = low - 0.01 * np.abs(high - low)
        high = high + 0.01 * np.abs(high - low)
        axes.append(np.linspace(low, high, size))
    u = np.full((size, size), np.nan)
    v = np.full((size, size), np.nan)
    columns = np.searchsorted(axes[0], kept_points[:, 0])
    rows = np.searchsorted(axes[1], kept_points[:, 1])
    assert np.array_equal(axes[0][columns], kept_points[:, 0])
    assert np.array_equal(axes[1][rows], kept_points[:, 1])
    u[rows, columns] = kept_vectors[:, 0]
    v[rows, columns] = kept_vectors[:, 1]
    return axes[0], axes[1], u, v


def _runs(indices: np.ndarray) -> list[list[int]]:
    """Sorted indices as ``[start, stop)`` runs of consecutive values."""
    runs: list[list[int]] = []
    for index in indices.tolist():
        if runs and runs[-1][1] == index:
            runs[-1][1] = index + 1
        else:
            runs.append([index, index + 1])
    return runs


def summarize(component: np.ndarray) -> dict:
    """NaN positions (as runs) and a summary of the finite values of one component."""
    flat = np.asarray(component, dtype=np.float64).ravel()
    nan = np.flatnonzero(np.isnan(flat))
    finite = np.flatnonzero(~np.isnan(flat))
    values = flat[finite]
    samples = finite[np.linspace(0, finite.size - 1, min(SAMPLES, finite.size)).astype(int)]
    return {
        "nan_runs": _runs(nan),
        "sum": float(values.sum()),
        "sum_squares": float((values**2).sum()),
        "weighted_sum": float((values * (finite + 1)).sum()),
        "samples": [[int(index), float(flat[index])] for index in samples],
    }


def main() -> None:
    import scvelo

    cases = []
    for name, points, vectors, options in case_inputs():
        x, y, u, v = scvelo_grid(points, vectors, options)
        cases.append(
            {
                "name": name,
                "x": np.asarray(x).tolist(),
                "y": np.asarray(y).tolist(),
                "u": summarize(u),
                "v": summarize(v),
            }
        )
    reference = {"scvelo": scvelo.__version__, "numpy": np.__version__, "cases": cases}
    path = Path(__file__).with_name("velocity_grid_scvelo.json")
    path.write_text(json.dumps(reference) + "\n")
    print(f"wrote {path} ({path.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
