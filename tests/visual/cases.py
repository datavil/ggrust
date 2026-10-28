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


def boxplot_dodged(m):
    random = np.random.default_rng(31)
    frame = pl.DataFrame(
        {
            "cluster": np.repeat(["C0", "C1", "C2", "C3"], 60),
            "condition": np.tile(np.repeat(["control", "treated"], 30), 4),
            "expression": np.concatenate(
                [
                    random.lognormal(mean, 0.5, 30)
                    for mean in [0.5, 0.9, 1.2, 1.1, 0.2, 0.3, 0.8, 1.6]
                ]
            ),
        }
    )
    return (
        m.ggplot(frame, m.aes("cluster", "expression", fill="condition"))
        + m.geom_boxplot(outlier_shape=21, outlier_fill="white")
        + m.scale_fill_brewer(palette="Pastel1")
        + m.theme_classic()
        + m.ggsize(520, 340)
    )


def violins_quantiles(m):
    random = np.random.default_rng(41)
    frame = pl.DataFrame(
        {
            "species": np.repeat(["setosa", "versicolor", "virginica"], 80),
            "petal_length": np.concatenate(
                [
                    random.normal(1.5, 0.2, 80),
                    random.normal(4.3, 0.5, 80),
                    random.normal(5.6, 0.55, 80),
                ]
            ),
        }
    )
    return (
        m.ggplot(frame, m.aes("species", "petal_length", fill="species"))
        + m.geom_violin(quantile_lines=True, alpha=0.7, show_legend=False)
        + m.scale_fill_brewer(palette="Set2")
        + m.ggsize(480, 340)
    )


def violin_jitterdodge(m):
    random = np.random.default_rng(51)
    frame = pl.DataFrame(
        {
            "cell_type": np.repeat(["B", "NK", "T"], 60),
            "sample": np.tile(np.repeat(["day 0", "day 7"], 30), 3),
            "score": np.concatenate(
                [random.normal(mean, 0.6, 30) for mean in [1.0, 1.6, 2.4, 2.0, 0.6, 1.4]]
            ),
        }
    )
    return (
        m.ggplot(frame, m.aes("cell_type", "score", fill="sample"))
        + m.geom_violin(alpha=0.4)
        + m.geom_point(
            m.aes(color="sample"),
            position=m.position_jitterdodge(jitter_width=0.15, seed=3),
            size=1.5,
            alpha=0.7,
        )
        + m.scale_fill_brewer(palette="Dark2")
        + m.scale_color_brewer(palette="Dark2")
        + m.theme_classic()
        + m.ggsize(520, 340)
    )


def ridges_months(m):
    random = np.random.default_rng(61)
    months = ["Jan", "Mar", "May", "Jul", "Sep", "Nov"]
    means = [2.0, 8.0, 16.0, 23.0, 17.0, 6.0]
    frame = pl.DataFrame(
        {
            "month": np.repeat(months, 90),
            "temperature": np.concatenate([random.normal(mean, 3.5, 90) for mean in means]),
        }
    )
    return (
        m.ggplot(frame, m.aes("temperature", "month", fill="month"))
        + m.geom_area_ridges(scale=1.4, alpha=0.7, quantile_lines=True, show_legend=False)
        + m.scale_fill_brewer(palette="Spectral")
        + m.ggtitle("Daily temperature by month")
        + m.ggsize(480, 380)
    )


def boxplot_brackets(m):
    random = np.random.default_rng(73)
    fertilisers = ["none", "compost", "nitrogen"]
    means = [21.0, 25.0, 31.0]
    frame = pl.DataFrame(
        {
            "fertiliser": np.repeat(fertilisers, 30),
            "height_cm": np.concatenate([random.normal(mean, 3.0, 30) for mean in means]),
        }
    )
    comparisons = {
        "xmin": ["none", "compost", "none"],
        "xmax": ["compost", "nitrogen", "nitrogen"],
        "y": [38.5, 41.0, 44.0],
        "label": ["**", "***", "p < 0.0001"],
    }
    return (
        m.ggplot(frame, m.aes("fertiliser", "height_cm", fill="fertiliser"))
        + m.geom_boxplot(alpha=0.6, show_legend=False)
        + m.geom_bracket(
            m.aes(xmin="xmin", xmax="xmax", y="y", label="label"),
            data=comparisons,
            size=6,
            segment_size=0.5,
            bracket_shorten=0.05,
        )
        + m.scale_fill_brewer(palette="Set2")
        + m.ggtitle("Plant height after six weeks")
        + m.ggsize(420, 380)
    )


def bars_flipped(m):
    frame = pl.DataFrame(
        {
            "gene": ["CD3E", "CD3E", "MS4A1", "MS4A1", "NKG7", "NKG7", "LYZ", "LYZ"],
            "condition": ["control", "treated"] * 4,
            "mean_expression": [2.4, 1.8, 3.1, 3.3, 0.9, 2.2, 4.0, 3.2],
        }
    )
    return (
        m.ggplot(frame, m.aes("gene", "mean_expression", fill="condition"))
        + m.geom_bar(stat="identity", position="dodge", width=0.7)
        + m.coord_flip()
        + m.theme_classic()
        + m.ggsize(460, 300)
    )


def facet_samples(m):
    random = np.random.default_rng(32)
    frame = pl.DataFrame(
        {
            "UMAP1": random.normal(0, 3, 500),
            "UMAP2": random.normal(0, 2, 500),
            "sample": random.choice(["donor 1", "donor 2", "donor 3", "donor 4", "donor 5"], 500),
            "condition": random.choice(["control", "treated"], 500),
        }
    )
    return (
        m.ggplot(frame, m.aes("UMAP1", "UMAP2", color="condition"))
        + m.geom_point(size=1.5)
        + m.facet_wrap("sample", ncol=3)
        + m.theme_bw()
        + m.ggtitle("Cells per donor")
    )


def facet_free_temperatures(m):
    months = list(range(1, 13))
    frame = pl.DataFrame(
        {
            "month": months * 3,
            "temperature": [
                *[-8.0, -6, -1, 6, 13, 17, 19, 17, 12, 6, 0, -5],
                *[24.0, 25, 26, 27, 28, 28, 28, 28, 28, 27, 26, 25],
                *[3.0, 4, 7, 10, 14, 17, 19, 19, 16, 12, 7, 4],
            ],
            "city": ["Moscow"] * 12 + ["Singapore"] * 12 + ["London"] * 12,
        }
    )
    return (
        m.ggplot(frame, m.aes("month", "temperature"))
        + m.geom_line(color="#377eb8", size=1)
        + m.geom_point()
        + m.facet_wrap("city", ncol=3, scales="free_y", order=0)
        + m.theme_grey()
        + m.ggsize(700, 280)
    )


def grid_sales_dashboard(m):
    random = np.random.default_rng(41)
    days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    hours = [f"{hour}:00" for hour in range(7, 19)]
    frame = pl.DataFrame(
        {
            "day": [day for day in days for _ in hours],
            "hour": hours * len(days),
            "sales": random.gamma(4, 30, len(days) * len(hours)).round(),
        }
    )
    totals = frame.group_by("day", maintain_order=True).agg(pl.col("sales").sum())
    heatmap = (
        m.ggplot(frame, m.aes("hour", "day", fill="sales"))
        + m.geom_tile()
        + m.theme(axis_text_x=m.element_text(angle=90))
    )
    bars = (
        m.ggplot(totals, m.aes("day", "sales"))
        + m.geom_bar(stat="identity", fill="#377eb8")
        + m.coord_flip()
        + m.theme(axis_title_y="blank", axis_title_x="blank", axis_text_x="blank")
    )
    scatter = (
        m.ggplot(frame, m.aes("hour", "sales", color="day"))
        + m.geom_point()
        + m.theme(axis_text_x=m.element_text(angle=90))
    )
    return (
        m.gggrid(
            [heatmap, bars, scatter, None], ncol=2, widths=[3, 1], align=True, guides="collect"
        )
        + m.ggtitle("Coffee sales")
        + m.ggsize(800, 600)
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
        boxplot_dodged,
        violins_quantiles,
        violin_jitterdodge,
        ridges_months,
        bars_flipped,
        boxplot_brackets,
        facet_samples,
        facet_free_temperatures,
        grid_sales_dashboard,
    ]
}
