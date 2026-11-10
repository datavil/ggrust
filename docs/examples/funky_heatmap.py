"""Funky heatmap

Integration benchmark of single-cell methods (scIB, human immune cells), after panel
a of Luecken et al. 2022: the seven best and seven worst of 69 runs. Bars show the
overall scores, circles the metric scores (size) and their ranks (color).
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *
from ggrust.bistro import funky_heatmap, position_arguments

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
runs = pl.read_csv(DATA + "scib_immune.csv")

batch = {
    "PCR_batch": "PCR batch",
    "ASW_label/batch": "Batch ASW",
    "iLISI": "Graph iLISI",
    "graph_conn": "Graph connectivity",
    "kBET": "kBET",
}
bio = {
    "NMI_cluster/label": "NMI cluster/label",
    "ARI_cluster/label": "ARI cluster/label",
    "ASW_label": "Cell type ASW",
    "isolated_label_F1": "Isolated label F1",
    "isolated_label_silhouette": "Isolated label silhouette",
    "cLISI": "Graph cLISI",
    "cell_cycle_conservation": "Cell cycle conservation",
    "hvg_overlap": "HVG conservation",
    "trajectory": "Trajectory conservation",
}
methods = {
    "scanorama": "Scanorama",
    "fastmnn": "fastMNN",
    "scanvi": "scANVI*",
    "harmony": "Harmony",
    "scgen": "scGen*",
    "saucie": "SAUCIE",
}
icons = {"full": "genes", "embed": "embedding", "knn": "graph"}


def rank_color(column: str) -> pl.Expr:
    """Rank 1 (best) -> 1.0 (lightest color), last -> 0.0."""
    rank = pl.col(column).rank("min", descending=True)
    return 1 - (rank - 1) / (rank.max() - 1)


scores = (
    runs.with_columns(
        (pl.col(name) - pl.col(name).min()) / (pl.col(name).max() - pl.col(name).min())
        for name in [*batch, *bio]
    )
    .with_columns(batch=pl.mean_horizontal(*batch), bio=pl.mean_horizontal(*bio))
    .with_columns(overall=0.4 * pl.col("batch") + 0.6 * pl.col("bio"))
    .with_columns(
        rank_color(name).alias(f"{name}_rank") for name in ["overall", "batch", "bio", *batch, *bio]
    )
    .sort("overall", descending=True)
    .with_columns(
        id=pl.int_range(pl.len()).cast(pl.String),
        name=pl.col("method").replace(methods),
        icon=DATA + "scib_icons/" + pl.col("output").replace_strict(icons) + ".png",
        features=pl.col("features").replace({"full_feature": "FULL", "hvg": "HVG"}),
        scaling=pl.col("scaling").replace({"scaled": "+", "unscaled": "\u2013"}),
    )
)
shown = pl.concat([scores.head(7), scores.tail(7)])
separator = pl.DataFrame({"id": ["hidden"]})
table = pl.concat([shown.head(7), separator, shown.tail(7)], how="diagonal_relaxed")


def score(column: str, name: str, geom: str, group: str) -> dict:
    """Describe a score column: its size from ``column``, its color from the rank."""
    width = 2.5 if group == "overall" else 1.5 if geom == "bar" else 1
    return {
        "id": column,
        "id_color": f"{column}_rank",
        "name": name,
        "geom": geom,
        "group": group,
        "palette": group,
        "draw_outline": False,
        "options": {"width": width},
    }


def method(column: str, name: str, geom: str, width: float, **options: object) -> dict:
    """Describe a method column."""
    return {
        "id": column,
        "name": name,
        "geom": geom,
        "group": "method",
        "options": {"width": width, **options},
    }


column_info = [
    method("name", "Name", "text", 4.5, hjust=0),
    method("icon", "Output", "image", 1.5),
    {**method("features", "Features", "text", 2), "palette": "features", "id_color": "features"},
    method("scaling", "Scaling", "text", 1.5),
    score("overall", "Overall score", "bar", "overall"),
    score("batch", "Overall score", "bar", "batch"),
    *[score(column, name, "circle", "batch") for column, name in batch.items()],
    score("bio", "Overall score", "bar", "bio"),
    *[score(column, name, "circle", "bio") for column, name in bio.items()],
]
column_groups = [
    {"header": "Method", "group": "method", "palette": "black"},
    {"header": "Overall", "group": "overall", "palette": "overall"},
    {"header": "Batch correction", "group": "batch", "palette": "batch"},
    {"header": "Bio conservation", "group": "bio", "palette": "bio"},
]


def brewer(codes: str) -> list[str]:
    """Colors from dark (rank 69) to light (rank 1)."""
    return ["#" + code for code in reversed(codes.split())]


palettes = {
    "black": ["black", "black"],
    "features": {"FULL": "#808080", "HVG": "#008000"},
    "overall": brewer("FFFFD9 EDF8B1 C7E9B4 7FCDBB 41B6C4 1D91C0 225EA8 253494 081D58"),
    "batch": brewer("F7FCFD E0ECF4 BFD3E6 9EBCDA 8C96C6 8C6BB1 88419D 810F7C 4D004B"),
    "bio": brewer("FFF7F3 FDE0DD FCC5C0 FA9FB5 F768A1 DD3497 AE017E 7A0177 49006A"),
}
ranks = ["69", "", "", "", "1"]
legends = [
    {
        "title": "Output",
        "geom": "image",
        "labels": [DATA + f"scib_icons/{icon}.png" for icon in icons.values()],
        "values": ["Genes", "Embedding", "Graph"],
    },
    {
        "title": "Scaling",
        "geom": "text",
        "labels": ["+", "\u2013"],
        "values": ["Scaled", "Unscaled"],
    },
    {"title": "Overall rank", "palette": "overall", "geom": "rect", "labels": ranks, "size": 1},
    {
        "title": "Batch correction rank",
        "palette": "batch",
        "geom": "rect",
        "labels": ranks,
        "size": 1,
    },
    {
        "title": "Bio conservation rank",
        "palette": "bio",
        "geom": "rect",
        "labels": ranks,
        "size": 1,
    },
    {"title": "Score", "geom": "circle", "labels": ["0%", "", "", "", "100%"], "color": "white"},
]

heatmap = funky_heatmap(
    table,
    column_info=column_info,
    column_groups=column_groups,
    palettes=palettes,
    legends=legends,
    position_args=position_arguments(
        col_annot_position="bottom",
        col_annot_angle=90,
        col_annot_offset=6,
        col_group_radius=0.5,
        legend_position="right",
        expand_xmax=1,
    ),
    scale_column=False,
    add_abc=False,
)

# The hidden runs: a dark band on the separator row (row 8 is centred at y = -8 x 1.1).
# Its x range is the row background: 0.25 past the first and last column (0.1, 32.3).
hidden = len(scores) - len(shown)
band = pl.DataFrame(
    {
        "xmin": [-0.15],
        "xmax": [32.55],
        "ymin": [-9.35],
        "ymax": [-8.25],
        "x": [0.2],
        "y": [-8.8],
        "label": [f"{hidden} additional rows not shown"],
    }
)
plot = (
    heatmap
    + geom_rect(
        aes(xmin="xmin", xmax="xmax", ymin="ymin", ymax="ymax"),
        data=band,
        fill="#555555",
        color=None,
    )
    + geom_text(aes(x="x", y="y", label="label"), data=band, color="white", hjust=0, size=7.6)
)
plot
# --8<-- [end:code]
