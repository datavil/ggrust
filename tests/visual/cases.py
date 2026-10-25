"""Visual test cases, written once and built with either library.

Each case is a function ``(module) -> plot`` where ``module`` is ``ggrust`` or
``lets_plot``, so the same code feeds the golden tests and the side-by-side report.
Add a case here when a phase adds a visible feature.
"""

import numpy as np
import polars as pl

RANDOM = np.random.default_rng(7)

CELLS = pl.DataFrame(
    {
        "UMAP1": RANDOM.normal(0, 3, 600),
        "UMAP2": RANDOM.normal(0, 2, 600),
        "n_genes": RANDOM.uniform(200, 12_000, 600),
        "fraction_mt": RANDOM.uniform(-0.002, 0.01, 600),
    }
)


def scatter_basic(m):
    frame = pl.DataFrame({"x": [1.0, 2.0, 3.0, 4.5], "y": [2.0, 1.0, 4.0, 3.0]})
    return m.ggplot(frame, m.aes("x", "y")) + m.geom_point() + m.ggtitle("Title")


def umap_titles(m):
    return (
        m.ggplot(CELLS, m.aes("UMAP1", "UMAP2"))
        + m.geom_point(size=1.2, color="#377eb8", alpha=0.6)
        + m.ggtitle("PBMC cells", subtitle="600 random cells")
        + m.labs(x="UMAP 1", y="UMAP 2")
    )


def qc_commas(m):
    return m.ggplot(CELLS, m.aes("n_genes", "fraction_mt")) + m.geom_point() + m.ggsize(500, 350)


def tiny_plot(m):
    frame = pl.DataFrame({"x": [0.0, 1e6], "y": [-5.0, 5.0]})
    return m.ggplot(frame, m.aes("x", "y")) + m.geom_point(color="red", size=4) + m.ggsize(220, 160)


def log_scales(m):
    frame = pl.DataFrame(
        {"total_counts": 10.0 ** RANDOM.uniform(2.5, 5, 400), "n_genes": RANDOM.uniform(1, 9, 400)}
    ).with_columns(n_genes=pl.col("total_counts") ** 0.8 / pl.col("n_genes"))
    return (
        m.ggplot(frame, m.aes("total_counts", "n_genes"))
        + m.geom_point(size=1.5, alpha=0.5, color="#4daf4a")
        + m.scale_x_log10("total counts")
        + m.scale_y_log2()
        + m.ggsize(500, 350)
    )


def reversed_limits(m):
    return (
        m.ggplot(CELLS, m.aes("UMAP1", "UMAP2"))
        + m.geom_point(size=1)
        + m.scale_y_reverse()
        + m.xlim(-5, 15)
        + m.ggsize(400, 300)
    )


CASES = {
    case.__name__: case
    for case in [scatter_basic, umap_titles, qc_commas, tiny_plot, log_scales, reversed_limits]
}
