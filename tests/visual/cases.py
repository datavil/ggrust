"""Visual test cases, written once and built with either library.

Each case is a function ``(module) -> plot`` where ``module`` is ``ggrust`` or
``lets_plot``, so the same code feeds the golden tests and the side-by-side report.
Add a case here when a phase adds a visible feature. Cases that need random data use
their own seeded generator, so a case renders the same whichever cases run first.
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
    random = np.random.default_rng(11)  # own generator: data must not depend on case order
    frame = pl.DataFrame(
        {"total_counts": 10.0 ** random.uniform(2.5, 5, 400), "n_genes": random.uniform(1, 9, 400)}
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


def discrete_strip(m):
    clusters = ["T cells", "B cells", "NK", "Monocytes", "DC"]
    frame = pl.DataFrame(
        {
            "cluster": [clusters[index % 5] for index in range(300)],
            "n_genes": np.random.default_rng(12).lognormal(7.5, 0.4, 300),
        }
    )
    return (
        m.ggplot(frame, m.aes("cluster", "n_genes"))
        + m.geom_point(alpha=0.4, color="#984ea3")
        + m.scale_x_discrete(limits=["B cells", "T cells", "NK", "Monocytes"])
        + m.ggsize(500, 320)
    )


def discrete_genes(m):
    genes = ["CD3E", "CD14", "MS4A1", "NKG7", "FCGR3A", "LYZ", "PPBP", "GNLY", "CST3", "IL7R"]
    frame = pl.DataFrame(
        {
            "gene": [genes[index % 10] for index in range(200)],
            "cluster": [f"cluster {index % 4}" for index in range(200)],
        }
    )
    return (
        m.ggplot(frame, m.aes("gene", "cluster"))
        + m.geom_point(size=4, color="#e41a1c", alpha=0.3)
        + m.ggsize(420, 300)
    )


def umap_clusters(m):
    random = np.random.default_rng(21)
    centers = random.uniform(-8, 8, (7, 2))
    labels = random.integers(0, 7, 1500)
    frame = pl.DataFrame(
        {
            "UMAP1": centers[labels, 0] + random.normal(0, 1.0, 1500),
            "UMAP2": centers[labels, 1] + random.normal(0, 1.0, 1500),
            "cluster": [f"cluster {label}" for label in labels],
        }
    )
    return m.ggplot(frame, m.aes("UMAP1", "UMAP2", color="cluster")) + m.geom_point(size=1.5)


def umap_expression(m):
    random = np.random.default_rng(22)
    umap1, umap2 = random.normal(0, 3, 1500), random.normal(0, 3, 1500)
    expression = np.clip(
        np.exp(-((umap1 - 2) ** 2 + umap2**2) / 8) * 5 + random.normal(0, 0.3, 1500), 0, None
    )
    frame = pl.DataFrame({"UMAP1": umap1, "UMAP2": umap2, "CD14": expression})
    return m.ggplot(frame, m.aes("UMAP1", "UMAP2", color="CD14")) + m.geom_point(size=1.5)


CASES = {
    case.__name__: case
    for case in [
        scatter_basic,
        umap_titles,
        qc_commas,
        tiny_plot,
        log_scales,
        reversed_limits,
        discrete_strip,
        discrete_genes,
        umap_clusters,
        umap_expression,
    ]
}
