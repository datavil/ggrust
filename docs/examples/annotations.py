"""Bands and corner labels

Unemployment in the United States with recessions as bands: `geom_band` spans the
whole panel between two x values, and `annotate` with an infinite position pins a
note to a corner, whatever the data range.
"""

# --8<-- [start:code]
from datetime import date

from ggrust import *
from ggrust.data import economics

recessions = {
    "start": [date(1969, 12, 1), date(1973, 11, 1), date(1980, 1, 1), date(1981, 7, 1),
              date(1990, 7, 1), date(2001, 3, 1), date(2007, 12, 1)],
    "end": [date(1970, 11, 1), date(1975, 3, 1), date(1980, 7, 1), date(1982, 11, 1),
            date(1991, 3, 1), date(2001, 11, 1), date(2009, 6, 1)],
}  # fmt: skip
plot = (
    ggplot(economics, aes("date", "uempmed"))
    + geom_band(aes(xmin="start", xmax="end"), data=recessions, fill="#b7410e", alpha=0.15, size=0)
    + geom_line(color="#1f4e79", size=1)
    + annotate(
        "text",
        x=float("-inf"),
        y=float("inf"),
        label="shaded: recessions",
        hjust=0,
        vjust=1,
        nudge_x=8,
        nudge_y=-8,
        nudge_unit="px",
        color="#b7410e",
    )
    + labs(x="", y="median weeks unemployed")
    + theme_light()
    + ggsize(750, 400)
)
plot
# --8<-- [end:code]
