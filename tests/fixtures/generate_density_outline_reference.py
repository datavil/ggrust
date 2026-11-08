"""Writes density_outline_scipy.json: outlines by scipy's gaussian_kde + skimage's find_contours.

`case_inputs()` and `summarize()` are shared with `tests/test_density_outline.py`. Each
case stores, per contour, its vertex count, first and last vertex, and a summary of
its x and y (sum, sum of squares, a position-weighted sum and a few sample values),
to keep the fixture small.

The recipe (`reference_outline()`): KDE of the finite points (column-major, as
polars' `to_numpy()` gives them; numpy's sums depend on the memory layout), `np.linspace` axes over
the bounding box widened by `padding`, density on `np.meshgrid`, contours at
`level * max`, grid indices mapped back with `np.interp`.

Run with an environment that has scipy and scikit-image (not ggrust dependencies),
e.g. cellestial's: ``python tests/fixtures/generate_density_outline_reference.py``.
"""

import json
from pathlib import Path

import numpy as np

SAMPLES = 12


def case_inputs() -> list[tuple[str, np.ndarray, dict]]:
    """``(name, points, options)`` per case, float64 throughout."""
    rng = np.random.default_rng(7)
    cases = []
    # one correlated blob, the defaults
    blob = rng.normal(size=(400, 2)) @ np.array([[1.0, 0.6], [0.0, 0.5]]) + [2.0, -1.0]
    cases.append(("blob", blob, {}))
    # two clusters as one point set (two outlines), cellestial's defaults
    first = rng.normal([0, 0], [1, 0.4], (300, 2))
    second = rng.normal([9, 4], [0.6, 1.2], (250, 2))
    clusters = np.vstack([first, second])
    cases.append(("clusters", clusters, {"level": 0.04, "padding": 1.5}))
    # a ring: an outer and an inner outline
    angle = rng.uniform(0, 2 * np.pi, 700)
    radius = 5 + rng.normal(0, 0.3, angle.size)
    ring = np.column_stack([radius * np.cos(angle), radius * np.sin(angle)])
    cases.append(("ring", ring, {"level": 0.5, "grid_size": 150}))
    # few points, coarse grid
    cases.append(("five_points", rng.uniform(-3, 3, (5, 2)), {"grid_size": 40}))
    # no padding: outlines cut open at the grid edge
    cases.append(("no_padding", blob, {"padding": 0.0, "level": 0.02, "grid_size": 90}))
    # other bandwidths
    cases.append(("silverman", clusters, {"bandwidth": "silverman", "grid_size": 120}))
    cases.append(("factor", clusters, {"bandwidth": 0.15, "level": 0.2, "grid_size": 120}))
    # large offsets and scales, with non-finite rows to drop
    scaled = blob * [300.0, 40.0] + [1e4, -50.0]
    scaled[5, 0] = np.nan
    scaled[17, 1] = np.inf
    cases.append(("scaled", scaled, {"padding": 150.0, "grid_size": 100}))
    return cases


def reference_outline(points: np.ndarray, options: dict) -> list[np.ndarray]:
    """Outlines as ``(n, 2)`` x/y arrays, computed with scipy and scikit-image."""
    from scipy.stats import gaussian_kde
    from skimage import measure

    level = options.get("level", 0.1)
    padding = options.get("padding", 1.0)
    grid_size = options.get("grid_size", 200)
    bandwidth = options.get("bandwidth", "scott")
    # Column-major, as polars' `to_numpy()` returns it (cellestial's input): numpy's
    # reductions over `points.T` then run in the order ggrust follows (contiguous).
    points = np.asfortranarray(points[np.isfinite(points).all(axis=1)])
    kde = gaussian_kde(points.T, bw_method=bandwidth)
    x_min, y_min = points.min(axis=0) - padding
    x_max, y_max = points.max(axis=0) + padding
    x_range = np.linspace(x_min, x_max, grid_size)
    y_range = np.linspace(y_min, y_max, grid_size)
    xi, yi = np.meshgrid(x_range, y_range)
    zi = kde(np.vstack([xi.flatten(), yi.flatten()])).reshape(xi.shape)
    contours = measure.find_contours(zi, zi.max() * level)
    return [
        np.column_stack(
            [
                np.interp(contour[:, 1], np.arange(grid_size), x_range),
                np.interp(contour[:, 0], np.arange(grid_size), y_range),
            ]
        )
        for contour in contours
    ]


def summarize(values: np.ndarray) -> dict:
    """Sum, sum of squares, position-weighted sum and sample values of one coordinate."""
    values = np.asarray(values, dtype=np.float64)
    positions = np.linspace(0, values.size - 1, min(SAMPLES, values.size)).astype(int)
    return {
        "sum": float(values.sum()),
        "sum_squares": float((values**2).sum()),
        "weighted_sum": float((values * np.arange(1, values.size + 1)).sum()),
        "samples": [[int(index), float(values[index])] for index in positions],
    }


def summarize_contour(contour: np.ndarray) -> dict:
    """Vertex count, end vertices and per-coordinate summaries of one outline."""
    return {
        "n": len(contour),
        "first": contour[0].tolist(),
        "last": contour[-1].tolist(),
        "x": summarize(contour[:, 0]),
        "y": summarize(contour[:, 1]),
    }


def main() -> None:
    import scipy
    import skimage

    cases = []
    for name, points, options in case_inputs():
        contours = reference_outline(points, options)
        cases.append({"name": name, "contours": [summarize_contour(c) for c in contours]})
    reference = {
        "scipy": scipy.__version__,
        "scikit-image": skimage.__version__,
        "numpy": np.__version__,
        "cases": cases,
    }
    path = Path(__file__).with_name("density_outline_scipy.json")
    path.write_text(json.dumps(reference) + "\n")
    print(f"wrote {path} ({path.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
