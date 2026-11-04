"""Reference dendrogram layouts from SciPy, for tests/test_dendrogram.py.

Run from the repository root (SciPy is not a ggrust dependency):
    uv run --with scipy python tests/fixtures/generate_scipy_dendrogram_reference.py
Writes tests/fixtures/scipy_dendrogram_reference.json: per case the linkage matrix
``Z``, the labels, and ``scipy.cluster.hierarchy.dendrogram``'s ``leaves``, ``ivl``,
``icoord`` and ``dcoord``. One case clusters like ``scanpy.tl.dendrogram``
(correlation of group means, complete linkage).
"""

import json
from pathlib import Path

import numpy as np
from scipy.cluster.hierarchy import dendrogram, linkage
from scipy.spatial.distance import pdist

generator = np.random.default_rng(11)
cases = []
for name, points, method, metric in [
    ("normal_complete", generator.normal(size=(12, 4)), "complete", "euclidean"),
    ("ties_average", generator.integers(0, 3, size=(15, 3)).astype(float), "average", "euclidean"),
    ("ward", generator.normal(size=(9, 2)), "ward", "euclidean"),
    ("single", generator.normal(size=(7, 3)), "single", "euclidean"),
    # scanpy.tl.dendrogram: Pearson correlation of per-group means, complete linkage.
    ("scanpy_like", generator.normal(size=(10, 20)), "complete", "correlation"),
    ("two", np.array([[0.0], [1.0]]), "complete", "euclidean"),
]:
    if metric == "correlation":
        z = linkage(pdist(points, "correlation"), method)
    else:
        z = linkage(points, method)
    labels = [f"{name}_{index}" for index in range(len(points))]
    info = dendrogram(z, labels=labels, no_plot=True)
    cases.append(
        {
            "name": name,
            "linkage": z.tolist(),
            "labels": labels,
            "leaves": info["leaves"],
            "ivl": info["ivl"],
            "icoord": info["icoord"],
            "dcoord": info["dcoord"],
        }
    )
path = Path(__file__).parent / "scipy_dendrogram_reference.json"
path.write_text(json.dumps(cases, indent=1) + "\n")
