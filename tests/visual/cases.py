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


def legends_hue_size(m):
    random = np.random.default_rng(23)
    frame = pl.DataFrame(
        {
            "UMAP1": random.normal(0, 3, 400),
            "UMAP2": random.normal(0, 3, 400),
            "cell_type": random.choice(["T cell", "B cell", "NK", "Monocyte"], 400),
            "sample": random.choice(["donor A", "donor B"], 400),
        }
    )
    return (
        m.ggplot(frame, m.aes("UMAP1", "UMAP2", color="cell_type", size="sample"))
        + m.geom_point(alpha=0.6)
        + m.scale_color_hue(name="Cell type")
    )


def colorbar_viridis_size(m):
    random = np.random.default_rng(24)
    frame = pl.DataFrame(
        {
            "gene": np.repeat(["CD3E", "MS4A1", "NKG7", "LYZ", "FCGR3A"], 6),
            "cluster": np.tile([f"c{index}" for index in range(6)], 5),
            "expression": random.uniform(0, 3, 30),
            "fraction": random.uniform(0, 100, 30),
        }
    )
    return (
        m.ggplot(frame, m.aes("cluster", "gene", color="expression", size="fraction"))
        + m.geom_point()
        + m.scale_color_viridis(option="magma", direction=-1)
    )


def brewer_clusters(m):
    random = np.random.default_rng(25)
    labels = random.integers(0, 10, 800)
    centers = random.uniform(-8, 8, (10, 2))
    frame = pl.DataFrame(
        {
            "UMAP1": centers[labels, 0] + random.normal(0, 0.8, 800),
            "UMAP2": centers[labels, 1] + random.normal(0, 0.8, 800),
            "leiden": [str(label) for label in labels],
        }
    )
    return (
        m.ggplot(frame, m.aes("UMAP1", "UMAP2", color="leiden"))
        + m.geom_point(size=1)
        + m.scale_color_brewer(palette="Paired")
    )


def legends_bottom_box(m):
    return colorbar_viridis_size(m) + m.theme(
        legend_position="bottom", legend_box="horizontal", legend_direction="horizontal"
    )


def legend_two_columns(m):
    return umap_clusters(m) + m.guides(color=m.guide_legend(ncol=2))


def legend_inside_corner(m):
    return legends_hue_size(m) + m.theme(
        legend_position=[1, 1], legend_justification=[1, 1], legend_box="horizontal"
    )


def labelled_axes(m):
    random = np.random.default_rng(26)
    genes = ["CD3E", "MS4A1", "NKG7", "LYZ", "FCGR3A", "PPBP"]
    frame = pl.DataFrame(
        {
            "x": np.tile(np.arange(6.0), 5),
            "y": np.repeat(np.arange(5.0), 6),
            "mean": random.uniform(0, 3, 30),
        }
    )
    return (
        m.ggplot(frame, m.aes("x", "y", color="mean"))
        + m.geom_point(size=7)
        + m.scale_x_continuous(
            breaks=list(range(6)), labels=genes, limits=[-0.5, 5.5], expand=[0, 0]
        )
        + m.scale_y_continuous(breaks=list(range(5)), labels=[f"cluster {i}" for i in range(5)])
        + m.scale_color_viridis()
    )


def point_shapes(m):
    frame = pl.DataFrame(
        {
            "x": [float(code % 9) for code in range(26)],
            "y": [float(code // 9) for code in range(26)],
            "code": [str(code) for code in range(26)],
        }
    )
    return (
        m.ggplot(frame, m.aes("x", "y", shape="code"))
        + m.geom_point(size=5, fill="#fdb462", color="#3f007d", stroke=1)
        + m.scale_shape_manual(values=list(range(26)))
        + m.guides(shape="none")
        + m.ggsize(500, 300)
    )


def size_scales(m):
    random = np.random.default_rng(27)
    frame = pl.DataFrame(
        {
            "UMAP1": random.normal(0, 3, 300),
            "UMAP2": random.normal(0, 3, 300),
            "pct": random.uniform(0, 100, 300),
        }
    )
    return (
        m.ggplot(frame, m.aes("UMAP1", "UMAP2", size="pct", alpha="pct"))
        + m.geom_point(color="#3f007d")
        + m.scale_size(trans="sqrt", breaks=[25, 50, 75, 100], range=[0.5, 6])
        + m.scale_alpha(range=[0.3, 0.9])
    )


def fixed_spatial(m):
    random = np.random.default_rng(28)
    frame = pl.DataFrame(
        {
            "x": random.uniform(0, 1000, 500),
            "y": random.uniform(0, 600, 500),
            "cluster": [f"c{index}" for index in random.integers(0, 4, 500)],
        }
    )
    return (
        m.ggplot(frame, m.aes("x", "y", color="cluster"))
        + m.geom_point(size=1.5)
        + m.coord_fixed(xlim=[0, 1000], ylim=[0, 600])
        + m.ggtitle("Spatial spots")
    )


def lines_by_group(m):
    random = np.random.default_rng(29)
    days = np.arange(30, dtype=float)
    frame = pl.DataFrame(
        {
            "day": np.concatenate([days, days, days]),
            "temperature": np.concatenate(
                [
                    12 + 6 * np.sin(days / 5) + random.normal(0, 0.8, 30),
                    18 + 4 * np.sin(days / 6) + random.normal(0, 0.8, 30),
                    8 + 3 * np.cos(days / 4) + random.normal(0, 0.8, 30),
                ]
            ),
            "city": ["Oslo"] * 30 + ["Rome"] * 30 + ["Reykjavik"] * 30,
        }
    )
    return (
        m.ggplot(frame, m.aes("day", "temperature", color="city"))
        + m.geom_line(size=1)
        + m.geom_hline(yintercept=10, linetype="dashed", color="grey")
        + m.geom_vline(xintercept=15, linetype="dotted")
        + m.ggtitle("Daily temperature")
    )


def annotations(m):
    shapes = pl.DataFrame(
        {
            "x": [1.0, 3.0, 2.0, 4.0, 6.0, 5.0],
            "y": [1.0, 1.0, 3.0, 2.0, 2.0, 4.5],
            "shape": ["left", "left", "left", "right", "right", "right"],
        }
    )
    notes = pl.DataFrame({"x": [2.0, 5.0], "y": [3.4, 4.9], "note": ["peak A", "peak B"]})
    return (
        m.ggplot(shapes, m.aes("x", "y"))
        + m.geom_polygon(m.aes(fill="shape"), alpha=0.6, color="black")
        + m.geom_segment(x=3.5, y=4.5, xend=2.2, yend=3.2, arrow=m.arrow(type="closed"), size=0.8)
        + m.geom_label(m.aes(label="note"), data=notes, size=6)
        + m.geom_text(x=3.5, y=0.6, label="baseline", fontface="italic", hjust=0)
        + m.geom_path(m.aes(x="x", y="y"), data=notes, linetype="longdash")
        + m.geom_blank(m.aes(x="x", y="y"), data=pl.DataFrame({"x": [0.0, 7.0], "y": [0.0, 5.5]}))
    )


def bars_stacked(m):
    random = np.random.default_rng(21)
    frame = pl.DataFrame(
        {
            "cell_type": random.choice(
                ["T cell", "B cell", "NK", "Monocyte"], 400, p=[0.4, 0.2, 0.1, 0.3]
            ),
            "sample": random.choice(["donor 1", "donor 2", "donor 3"], 400),
        }
    )
    return (
        m.ggplot(frame, m.aes("cell_type", fill="sample"))
        + m.geom_bar()
        + m.scale_fill_brewer(palette="Set2")
        + m.theme_classic()
        + m.ggsize(500, 350)
    )


def bars_dodged(m):
    frame = pl.DataFrame(
        {
            "month": ["Jan", "Feb", "Mar", "Jan", "Feb", "Mar", "Jan", "Mar"],
            "fruit": ["apple", "apple", "apple", "pear", "pear", "pear", "plum", "plum"],
            "sold": [12.0, 18.0, 9.0, 7.0, 11.0, 15.0, 4.0, 6.0],
        }
    )
    return (
        m.ggplot(frame, m.aes("month", "sold", fill="fruit"))
        + m.geom_bar(stat="identity", position="dodge", color="black", width=0.8)
        + m.ggtitle("Fruit sold per month")
    )


def histogram_overlap(m):
    random = np.random.default_rng(5)
    frame = pl.DataFrame(
        {
            "height": np.concatenate([random.normal(165, 7, 300), random.normal(178, 8, 300)]),
            "group": ["A"] * 300 + ["B"] * 300,
        }
    )
    return (
        m.ggplot(frame, m.aes("height", fill="group"))
        + m.geom_histogram(binwidth=2.5, position="identity", alpha=0.5, color="white")
        + m.ggsize(500, 320)
    )


def bars_signed(m):
    frame = pl.DataFrame(
        {
            "year": [2018.0, 2019, 2020, 2021, 2022, 2023],
            "anomaly": [0.4, -0.2, 0.7, -0.5, 0.1, 0.9],
        }
    )
    return (
        m.ggplot(frame, m.aes("year", "anomaly"))
        + m.geom_bar(stat="identity", fill="#d95f02")
        + m.geom_hline(yintercept=0)
        + m.ggsize(400, 300)
    )


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
        legends_hue_size,
        colorbar_viridis_size,
        brewer_clusters,
        legends_bottom_box,
        legend_two_columns,
        legend_inside_corner,
        labelled_axes,
        point_shapes,
        size_scales,
        fixed_spatial,
        lines_by_group,
        annotations,
        bars_stacked,
        bars_dodged,
        histogram_overlap,
        bars_signed,
    ]
}
