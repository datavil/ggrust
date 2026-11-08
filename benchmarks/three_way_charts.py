"""Charts for REPORT-three-way.md, drawn by ggrust from three_way.py results.

uv run python benchmarks/three_way_charts.py results/three-way-A.json results/three-way-B.json
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import polars as pl
from ggrust import (
    aes,
    coord_flip,
    facet_wrap,
    geom_bar,
    ggplot,
    ggsize,
    labs,
    position_dodge,
    scale_fill_manual,
    scale_y_log10,
    theme,
)

HERE = Path(__file__).parent
COLORS = {"ggrust": "#e4572e", "lets-plot": "#29335c", "ggplot2": "#8fb339"}
SCENARIOS = ["scatter", "color_discrete", "histogram", "bar", "boxplot", "violin"]
SCENARIOS += ["density", "smooth", "line", "bin2d", "facet"]


def load(paths: list[str]) -> pl.DataFrame:
    """All measured rows (timeouts dropped) as one frame."""
    rows = []
    for path in paths:
        for result in json.loads(Path(path).read_text())["results"]:
            if "warm_seconds" in result:
                rows.append(
                    {
                        "library": result["library"],
                        "scenario": result["scenario"],
                        "rows": result["rows"],
                        "format": result["format"].upper(),
                        "milliseconds": result["warm_seconds"] * 1000,
                        "memory": max(result["memory_mb"], 1.0),
                    }
                )
    order = pl.Enum(list(reversed(SCENARIOS)))
    return pl.DataFrame(rows).with_columns(
        pl.col("scenario").cast(order), pl.col("library").cast(pl.Enum(list(COLORS)))
    )


def chart(frame: pl.DataFrame, value: str, title: str, path: Path) -> None:
    """Dodged bars per scenario, log axis, one panel per format."""
    plot = (
        ggplot(frame, aes("scenario", value, fill="library"))
        + geom_bar(stat="identity", position=position_dodge(width=0.8), width=0.75)
        + scale_y_log10()
        + scale_fill_manual(values=list(COLORS.values()))
        + coord_flip()
        + facet_wrap("format", ncol=2)
        + labs(title=title, x="", y="")
        + theme(legend_position="top")
        + ggsize(900, 620)
    )
    plot.to_png(str(path), scale=2)
    print(path)


def main() -> None:
    """Write the speed and memory charts for the 1M-row runs."""
    frame = load(sys.argv[1:]).filter(pl.col("rows") == 1_000_000)
    chart(
        frame,
        "milliseconds",
        "Render time at 1M rows (ms, log scale, lower is better)",
        HERE / "report-three-way-time.png",
    )
    chart(
        frame,
        "memory",
        "Peak memory above data at 1M rows (MB, log scale, lower is better)",
        HERE / "report-three-way-memory.png",
    )


if __name__ == "__main__":
    main()
