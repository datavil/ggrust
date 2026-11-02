"""Periodic table

The periodic table of chemical elements, after the lets-plot demo: tiles placed by
group and period, colored by element type, with a larger key tile. Hover a tile for
details.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
elements = pl.read_csv(DATA + "elements.csv")

side = 0.95  # tile side, in table cells
key_ratio = 1.5  # the key tile is this much larger
key_x, key_y = 10.25, 1.25

# Lanthanides and actinides go in two rows below the table.
f_block = pl.col("type").is_in(["Lanthanide", "Actinide"])
main = elements.filter(~f_block).with_columns(x=pl.col("group"), y=pl.col("period"))
below = elements.filter(f_block).with_columns(
    x=pl.int_range(pl.len()) % 15 + 3, y=pl.col("period") + 2.5
)
table = pl.concat([main, below], how="vertical_relaxed")
placeholders = pl.DataFrame(
    {"x": [3, 3], "y": [6, 7], "type": ["Lanthanide", "Actinide"], "range": ["57-71", "89-103"]}
)
key = elements.filter(pl.col("atomic_number") == 78).with_columns(x=key_x, y=key_y)
groups = main.group_by("group").agg(y=pl.col("period").min())
periods = pl.DataFrame({"x": [0] * 7, "period": range(1, 8)})
notes = pl.DataFrame(
    {
        "x": [key_x + 0.8, key_x + 1, key_x + 0.4],
        "y": [key_y - 0.9, key_y + 0.1, key_y + 1],
        "label": ["Atomic Number", "Symbol", "Atomic Mass"],
    }
)
hover = (
    layer_tooltips()
    .title("@element\n(@type)")
    .line("Atomic number|@atomic_number")
    .line("Atomic mass|@atomic_mass")
    .line("Electrons|@electron_configuration")
)
head = arrow(angle=30, length=4, type="closed")

plot = (
    ggplot(mapping=aes("x", "y"))
    # The table: atomic number, symbol and mass on each tile.
    + geom_tile(
        aes(fill="type"),
        data=table,
        color="black",
        size=0.25,
        width=side,
        height=side,
        tooltips=hover,
    )
    + geom_text(
        aes(label="atomic_number"),
        data=table,
        nudge_x=-0.37,
        nudge_y=0.37,
        hjust="left",
        vjust="top",
        size=5,
    )
    + geom_text(aes(label="symbol"), data=table, nudge_y=0.05, size=7, fontface="bold")
    + geom_text(aes(label="atomic_mass"), data=table, nudge_y=-0.2, size=4)
    # Where the lanthanides and actinides belong.
    + geom_tile(
        aes(fill="type"),
        data=placeholders,
        color="black",
        size=0.25,
        width=side,
        height=side,
        tooltips="none",
    )
    + geom_text(aes(label="range"), data=placeholders, nudge_y=0.05, size=5, fontface="bold")
    + geom_text(aes(label="type"), data=placeholders, nudge_y=-0.2, size=4)
    # The key: one larger tile with notes and arrows.
    + geom_tile(
        aes(fill="type"),
        data=key,
        color="black",
        size=0.25,
        width=key_ratio * side,
        height=key_ratio * side,
        tooltips="none",
    )
    + geom_text(
        aes(label="atomic_number"),
        data=key,
        nudge_x=-0.37 * key_ratio,
        nudge_y=0.37 * key_ratio,
        hjust="left",
        vjust="top",
        size=5 * key_ratio,
    )
    + geom_text(
        aes(label="symbol"), data=key, nudge_y=0.05 * key_ratio, size=7 * key_ratio, fontface="bold"
    )
    + geom_text(aes(label="atomic_mass"), data=key, nudge_y=-0.2 * key_ratio, size=4 * key_ratio)
    # Group and period numbers.
    + geom_text(
        aes("group", "y", label="group"),
        data=groups,
        color="gray",
        nudge_y=0.525,
        vjust="bottom",
        size=6,
    )
    + geom_text(
        aes("x", "period", label="period"), data=periods, color="gray", nudge_x=0.375, size=6
    )
    # Notes on the key.
    + geom_text(aes(label="label"), data=notes, hjust=0)
    + geom_segment(x=key_x + 0.7, y=key_y - 0.9, xend=key_x - 0.3, yend=key_y - 0.6, arrow=head)
    + geom_segment(x=key_x + 0.9, y=key_y + 0.1, xend=key_x + 0.3, yend=key_y - 0.1, arrow=head)
    + geom_segment(x=key_x + 0.3, y=key_y + 1, xend=key_x, yend=key_y + 0.5, arrow=head)
    + scale_y_reverse()
    + scale_fill_brewer(name="", type="qual", palette="Set2", guide=guide_legend(ncol=2))
    + coord_fixed(ratio=1.2)
    + labs(title="Periodic Table of Chemical Elements", caption="© 1869, Dmitri Mendeleev")
    + theme_void()
    + theme(
        plot_title=element_text(size=26, face="bold", hjust=0.5, margin=[30, 0, 5, 0]),
        plot_caption=element_text(size=18),
        plot_background=element_rect(color="black", size=3),
        legend_position=[0.36, 0.85],
        legend_background="blank",
    )
    + ggsize(1000, 700)
)
plot
# --8<-- [end:code]
