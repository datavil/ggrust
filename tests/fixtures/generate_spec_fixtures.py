"""Generate plot spec fixtures from real lets_plot for the Rust spec tests.

Each fixture is the JSON of `as_dict()` for a cellestial-style plot, with inline
data removed (ggrust passes frames separately as Arrow). Run from the repo root:

    uv run python tests/fixtures/generate_spec_fixtures.py
"""

import json
import warnings
from pathlib import Path

import numpy as np
import polars as pl

warnings.filterwarnings("ignore", category=SyntaxWarning)

from lets_plot import (  # noqa: E402
    aes,
    arrow,
    as_discrete,
    coord_cartesian,
    coord_fixed,
    coord_flip,
    element_blank,
    element_line,
    element_rect,
    element_text,
    facet_wrap,
    geom_area_ridges,
    geom_bar,
    geom_blank,
    geom_boxplot,
    geom_bracket,
    geom_function,
    geom_histogram,
    geom_hline,
    geom_imshow,
    geom_label,
    geom_label_repel,
    geom_line,
    geom_path,
    geom_point,
    geom_polygon,
    geom_raster,
    geom_rect,
    geom_segment,
    geom_text,
    geom_text_repel,
    geom_tile,
    geom_violin,
    geom_vline,
    gggrid,
    ggplot,
    ggsize,
    ggtb,
    ggtitle,
    guide_colorbar,
    guide_legend,
    guides,
    labs,
    layer_tooltips,
    position_dodge,
    position_jitterdodge,
    scale_color_brewer,
    scale_color_continuous,
    scale_color_gradient,
    scale_color_gradient2,
    scale_color_hue,
    scale_color_manual,
    scale_color_viridis,
    scale_fill_brewer,
    scale_fill_continuous,
    scale_fill_gradient,
    scale_fill_gradient2,
    scale_fill_gradientn,
    scale_fill_hue,
    scale_fill_manual,
    scale_fill_viridis,
    scale_size,
    scale_x_continuous,
    scale_x_discrete,
    scale_y_continuous,
    scale_y_discrete,
    scale_y_log2,
    scale_y_reverse,
    theme,
    theme_bw,
    theme_classic,
    theme_void,
    xlab,
    ylab,
)

OUTPUT_DIRECTORY = Path(__file__).parents[2] / "crates/ggrust-core/tests/fixtures/specs"

CELLS = pl.DataFrame(
    {
        "UMAP1": [0.1, 0.5, -0.3, 1.2],
        "UMAP2": [1.0, -0.2, 0.4, 0.9],
        "cluster": pl.Series(["B", "T", "NK", "T"], dtype=pl.Categorical),
        "CD14": [0.0, 2.5, 1.1, 0.3],
        "sample": ["s1", "s1", "s2", "s2"],
    }
)


def strip_data(value):
    """Remove inline `data` entries recursively."""
    if isinstance(value, dict):
        return {key: strip_data(item) for key, item in value.items() if key != "data"}
    if isinstance(value, list):
        return [strip_data(item) for item in value]
    return value


def umap_categorical():
    return (
        ggplot(CELLS, aes("UMAP1", "UMAP2", color="cluster"))
        + geom_point(
            size=1.5,
            alpha=0.8,
            shape=16,
            tooltips=layer_tooltips(["sample"]).line("@cluster").format("@CD14", ".2f"),
        )
        + scale_color_brewer(palette="Set2")
        + guides(color=guide_legend(ncol=2, override_aes={"size": 4}))
        + coord_fixed(ratio=1)
        + theme_classic()
        + theme(
            axis_text=element_blank(),
            axis_ticks=element_blank(),
            legend_position="right",
        )
        + labs(x="UMAP1", y="UMAP2", color="Cluster")
        + ggsize(500, 400)
        + ggtb(size_zoomin=-1)
    )


def umap_continuous():
    return (
        ggplot(CELLS, aes("UMAP1", "UMAP2", color="CD14"))
        + geom_point(size=1)
        + scale_color_viridis(option="magma", direction=-1)
        + scale_color_gradient(low="grey", high="red")
        + scale_color_continuous(cmap="viridis")
        + scale_color_gradient2(low="blue", mid="white", high="red", midpoint=0)
        + guides(color=guide_colorbar(barwidth=8, barheight=120))
        + theme(legend_position="bottom", axis_line=element_line(color="black", size=1))
        + ggtitle("CD14", subtitle="expression")
    )


def violin_box():
    return (
        ggplot(CELLS, aes("cluster", "CD14", fill="cluster"))
        + geom_violin(quantiles=[0.25, 0.5, 0.75], scale="width", trim=True)
        + geom_boxplot(width=0.2, outlier_shape=None, position=position_dodge(width=0.9))
        + geom_point(position=position_jitterdodge(jitter_width=0.1, seed=42), size=0.5)
        + scale_fill_hue()
        + scale_color_hue()
        + scale_y_log2()
        + ylab("Expression")
        + theme_bw()
        + theme(legend_position="none", axis_text_x=element_text(angle=45, hjust=1))
    )


def ridge():
    return (
        ggplot(CELLS, aes("CD14", "cluster", fill="cluster"))
        + geom_area_ridges(scale=1.5, alpha=0.7, tooltips=layer_tooltips().line("@cluster"))
        + scale_fill_manual(values=["#e41a1c", "#377eb8", "#4daf4a"])
        + scale_y_discrete(limits=["NK", "T", "B"])
        + coord_flip()
    )


def heatmap():
    return (
        ggplot(CELLS, aes("sample", "cluster", fill="CD14"))
        + geom_tile(color="white", size=0.2)
        + geom_raster()
        + scale_fill_gradient2(low="blue", mid="white", high="red", midpoint=1)
        + scale_fill_gradientn(colors=["#000004", "#fcfdbf"])
        + scale_fill_gradient(low="white", high="black")
        + scale_x_discrete(expand=[0, 0])
        + scale_y_reverse()
        + theme(
            axis_text_x=element_text(angle=90, size=8, vjust=0.5),
            panel_background=element_rect(fill="white", color="black"),
            panel_grid=element_blank(),
        )
    )


def dotplot():
    return (
        ggplot(CELLS, aes("sample", "cluster", size="CD14", fill="CD14"))
        + geom_point(shape=21, color="black")
        + scale_size(range=[1, 6], name="Fraction")
        + scale_fill_continuous(low="white", high="red")
        + scale_fill_viridis()
        + guides(size=guide_legend(nrow=1, byrow=True), fill="none")
    )


def volcano():
    return (
        ggplot(CELLS, aes("UMAP1", "UMAP2", color="sample"))
        + geom_point()
        + geom_text_repel(aes(label="cluster"), seed=1, max_overlaps=10, size=8)
        + geom_label_repel(aes(label="cluster"), seed=1)
        + geom_vline(xintercept=[-1, 1], linetype="dashed")
        + geom_hline(yintercept=1.3, linetype="dashed")
        + scale_color_manual(values={"s1": "red", "s2": "blue"}, na_value="grey")
        + scale_x_continuous(limits=[-2, None], breaks=[-1, 0, 1], format=".1f")
        + scale_y_continuous(expand=[0, 0.05])
    )


def distributions():
    return (
        ggplot(CELLS)
        + geom_bar(aes("cluster", fill="sample"), position="dodge")
        + geom_histogram(aes("CD14"), bins=30)
        + geom_boxplot(aes("cluster", "CD14"))
        + geom_bracket(aes(xmin="cluster", xmax="sample", y="CD14", label="cluster"))
        + geom_function(fun=lambda x: x * 2, xlim=[0, 1], n=5)
        + scale_fill_brewer(palette="Paired")
    )


def primitives():
    return (
        ggplot(CELLS, aes("UMAP1", "UMAP2"))
        + geom_path(aes(group="cluster"), size=0.5)
        + geom_line()
        + geom_segment(aes(xend="CD14", yend="CD14"), arrow=arrow(length=5, type="closed"))
        + geom_polygon(aes(group="cluster"), fill=None, color="black")
        + geom_rect(aes(xmin="UMAP1", xmax="CD14", ymin="UMAP2", ymax="CD14"), alpha=0.2)
        + geom_text(aes(label="cluster"), size_unit="x", nudge_y=0.1)
        + geom_label(aes(label="cluster"), fill="white")
        + geom_blank()
        + xlab("x")
        + ylab("y")
    )


def faceted():
    return (
        ggplot(CELLS, aes("UMAP1", "UMAP2"))
        + geom_point()
        + facet_wrap("sample", ncol=2, scales="free_y", order=-1)
        + coord_cartesian(xlim=[-1, 2], ylim=[None, 1.5])
        + ggtitle("Faceted")
        + labs(caption="caption text")
    )


def void_theme():
    return (
        ggplot(CELLS, aes("UMAP1", "UMAP2", color=as_discrete("cluster", order=-1)))
        + geom_point()
        + theme_void()
        + theme(
            legend_position=[1, 1],
            legend_direction="horizontal",
            plot_background=element_rect(fill="white"),
            panel_grid=element_line(size=0.5, linetype="dashed"),
            axis_title=element_text(face="bold", family="Arial", margin=[1, 2, 3, 4]),
        )
    )


def discrete_ordering():
    return ggplot(CELLS, aes(color=as_discrete("cluster", order_by="CD14", order=1))) + geom_point(
        aes("UMAP1", "UMAP2", fill=as_discrete("sample", label="Sample"))
    )


def pandas_categorical():
    """Ordered pandas categoricals carry `factor_levels` (unordered ones do not)."""
    frame = CELLS.to_pandas()
    frame["cluster"] = frame["cluster"].cat.reorder_categories(["T", "NK", "B"], ordered=True)
    return ggplot(frame, aes("UMAP1", "UMAP2", color="cluster")) + geom_point()


def polars_enum():
    """polars Enum carries `factor_levels`; polars Categorical does not."""
    frame = CELLS.with_columns(pl.col("cluster").cast(pl.String).cast(pl.Enum(["T", "NK", "B"])))
    return ggplot(frame, aes("UMAP1", "UMAP2", color="cluster")) + geom_point()


def image():
    pixels = np.zeros((2, 3, 3), dtype=np.uint8)
    return ggplot() + geom_imshow(pixels, extent=[0, 3, 0, 2])


def grid():
    inner = gggrid([dotplot(), ridge()], ncol=2)
    plots = [umap_categorical(), heatmap(), None, violin_box(), inner]
    return (
        gggrid(plots, ncol=2, widths=[1, 2], hspace=4, vspace=2, align=True, guides="collect")
        + ggsize(1000, 800)
        + theme(legend_position="bottom")
    )


def with_inline_data():
    """Kept with its `data` so the Rust side proves inline data is skipped."""
    return ggplot(CELLS, aes("UMAP1", "UMAP2")) + geom_point(data=CELLS)


FIXTURES = [
    umap_categorical,
    umap_continuous,
    violin_box,
    ridge,
    heatmap,
    dotplot,
    volcano,
    distributions,
    primitives,
    faceted,
    void_theme,
    discrete_ordering,
    pandas_categorical,
    polars_enum,
    image,
    grid,
]


def main():
    OUTPUT_DIRECTORY.mkdir(parents=True, exist_ok=True)
    for build in FIXTURES:
        spec = strip_data(build().as_dict())
        path = OUTPUT_DIRECTORY / f"{build.__name__}.json"
        path.write_text(json.dumps(spec, indent=1, sort_keys=True) + "\n")
    inline = with_inline_data().as_dict()
    (OUTPUT_DIRECTORY / "with_inline_data.json").write_text(
        json.dumps(inline, indent=1, sort_keys=True, default=str) + "\n"
    )
    print(f"wrote {len(FIXTURES) + 1} fixtures to {OUTPUT_DIRECTORY}")


if __name__ == "__main__":
    main()
