"""Benchmark charts for the docs page, the README and REPORT-three-way.md, drawn by ggrust.

Linear axes on purpose: a log axis hides how large the gaps are.

uv run python benchmarks/three_way_charts.py results/three-way-A.json results/three-way-B.json
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Literal

import polars as pl
from ggrust import (
    aes,
    coord_flip,
    element_blank,
    element_line,
    element_rect,
    element_text,
    facet_wrap,
    geom_bar,
    geom_line,
    geom_point,
    geom_text,
    ggplot,
    ggsize,
    labs,
    scale_color_manual,
    scale_fill_manual,
    scale_x_continuous,
    scale_y_continuous,
    theme,
    theme_minimal,
)
from ggrust.plot.core import FeatureSpec, PlotSpec

HERE = Path(__file__).parent
OUTPUT = HERE.parent / "docs" / "images" / "benchmarks"
LIBRARIES = ["ggrust", "ggplot2", "lets-plot"]
SCENARIOS = ["scatter", "color_discrete", "histogram", "bar", "boxplot", "violin"]
SCENARIOS += ["density", "smooth", "line", "bin2d", "facet"]
TIMEOUT_NOTE = "lets-plot did not finish the 1M-row line PNG within 300 s; counted as 300 s."
Mode = Literal["light", "dark"]
PALETTES: dict[Mode, dict[str, str]] = {
    "light": {
        "ggrust": "#e4572e",
        "ggplot2": "#8c96a3",
        "lets-plot": "#c3cad3",
        "ink": "#1f2328",
        "muted": "#59636e",
        "grid": "#e6e9ed",
        "paper": "#ffffff",
    },
    "dark": {
        "ggrust": "#ff7a50",
        "ggplot2": "#7d8794",
        "lets-plot": "#4b535d",
        "ink": "#e6edf3",
        "muted": "#9198a1",
        "grid": "#2a3038",
    },
}


def load(paths: list[str]) -> pl.DataFrame:
    """Every measurement as one frame; a timeout counts as its time limit."""
    rows = []
    for path in paths:
        for result in json.loads(Path(path).read_text())["results"]:
            rows.append(
                {
                    "library": result["library"],
                    "scenario": result["scenario"],
                    "rows": result["rows"],
                    "format": result["format"].upper(),
                    "seconds": result.get("timeout_seconds", result.get("warm_seconds")),
                    "memory": result.get("memory_mb"),
                    "timed_out": "timeout_seconds" in result,
                }
            )
    return pl.DataFrame(rows).with_columns(pl.col("scenario").cast(pl.Enum(SCENARIOS)))


def look(mode: Mode, legend: bool = False) -> FeatureSpec:
    """Shared minimal theme: quiet text, no axis titles, no ticks."""
    colors = PALETTES[mode]
    # Dark charts are transparent, so they sit on GitHub's and the docs' own dark page.
    background = (
        element_rect(fill=colors["paper"], color=colors["paper"])
        if mode == "light"
        else element_blank()
    )
    return theme_minimal() + theme(
        plot_background=background,
        panel_grid_major_y=element_blank(),
        panel_grid_minor=element_blank(),
        panel_grid_major_x=element_blank(),
        axis_text=element_text(color=colors["muted"], size=11),
        # coord_flip: the theme follows the aesthetic, so x is the vertical library axis.
        axis_text_y=element_blank(),
        axis_text_x=element_text(color=colors["ink"], size=12),
        axis_title=element_blank(),
        axis_ticks=element_blank(),
        strip_text=element_text(color=colors["ink"], size=12, hjust=0),
        plot_title=element_text(color=colors["ink"], size=17, face="bold"),
        plot_subtitle=element_text(color=colors["muted"], size=12),
        plot_caption=element_text(color=colors["muted"], size=10),
        legend_position="top" if legend else "none",
        legend_text=element_text(color=colors["ink"], size=12),
        legend_title=element_blank(),
        legend_background=element_blank(),
        legend_key=element_blank(),
    )


def seconds_label(value: float, timed_out: bool) -> str:
    """Readable time with a unit."""
    prefix = "> " if timed_out else ""
    if value < 1:
        return f"  {prefix}{value * 1000:.0f} ms"
    return f"  {prefix}{value:.1f} s"


def memory_label(value: float) -> str:
    """Readable memory with a unit."""
    return f"  {value / 1024:.1f} GB" if value >= 1024 else f"  {value:.0f} MB"


def with_labels(frame: pl.DataFrame, metric: str) -> pl.DataFrame:
    """Add a ``label`` column for ``seconds`` or ``memory``; memory drops timed-out runs."""
    if metric == "seconds":
        labels = [
            seconds_label(value, timed_out)
            for value, timed_out in zip(frame["seconds"], frame["timed_out"], strict=True)
        ]
        return frame.with_columns(label=pl.Series(labels, dtype=pl.String))
    frame = frame.filter(~pl.col("timed_out"))
    return frame.with_columns(label=pl.Series([memory_label(v) for v in frame["memory"]]))


def bars(
    data: pl.DataFrame,
    value: str,
    mode: Mode,
    text_size: float,
    room: float,
    width: float = 0.72,
) -> PlotSpec:
    """Horizontal bars per library, ggrust on top, value printed after each bar."""
    order = list(reversed(LIBRARIES))
    data = data.with_columns(pl.col("library").cast(pl.Enum(order)))
    return (
        ggplot(data, aes("library", value))
        + geom_bar(aes(fill="library"), stat="identity", width=width, size=0)
        + geom_text(aes(label="label"), hjust=0, size=text_size, color=PALETTES[mode]["ink"])
        + scale_y_continuous(expand=[room, 0])
        + scale_fill_manual(values=[PALETTES[mode][name] for name in order])
        + coord_flip()
    )


def overview(frame: pl.DataFrame, mode: Mode) -> PlotSpec:
    """README hero: median time and median memory per plot, at 1M rows."""
    big = frame.filter(pl.col("rows") == 1_000_000)
    time = with_labels(
        big.group_by("library").agg(pl.col("seconds").median(), timed_out=pl.lit(False)),
        "seconds",
    ).select("library", value="seconds", label="label", panel=pl.lit("Render time per plot"))
    memory = with_labels(
        big.filter(~pl.col("timed_out"))
        .group_by("library")
        .agg(pl.col("memory").median(), timed_out=pl.lit(False)),
        "memory",
    ).select("library", value="memory", label="label", panel=pl.lit("Peak memory per plot"))
    data = pl.concat([time, memory])
    # Each competitor's value also says how many times ggrust's it is.
    data = data.with_columns(
        ratio=pl.col("value")
        / pl.col("value").filter(pl.col("library") == "ggrust").first().over("panel")
    ).with_columns(
        label=pl.when(pl.col("library") == "ggrust")
        .then(pl.col("label"))
        .otherwise(pl.col("label") + pl.format("   ({}x)", pl.col("ratio").round(1)))
    )
    return (
        bars(data, "value", mode, text_size=5.5, room=0.6)
        + facet_wrap("panel", ncol=2, scales="free_y", order=0)
        + labs(
            title="ggrust vs ggplot2 vs lets-plot at 1M rows",
            subtitle="11 common plots, each saved as SVG and PNG. Shorter is better.",
            caption="Medians over the 22 renders. Apple M4 Pro. " + TIMEOUT_NOTE,
        )
        + look(mode)
        + ggsize(860, 290)
    )


def per_scenario(frame: pl.DataFrame, output_format: str, metric: str, mode: Mode) -> PlotSpec:
    """One small panel per scenario at 1M rows, each with its own linear axis."""
    big = frame.filter((pl.col("rows") == 1_000_000) & (pl.col("format") == output_format))
    data = with_labels(big.sort("scenario"), metric)
    if metric == "seconds":
        title = f"Render time to {output_format}, 1M rows"
        subtitle = "From building the plot to the file on disk. Shorter is better."
    else:
        title = f"Peak memory while rendering to {output_format}, 1M rows"
        subtitle = "Resident memory above the loaded data. Shorter is better."
    caption = "Apple M4 Pro. Each panel has its own linear axis."
    if metric == "seconds" and output_format == "PNG":
        caption += " " + TIMEOUT_NOTE
    return (
        bars(data, metric, mode, text_size=4.2, room=0.7)
        + facet_wrap("scenario", ncol=4, scales="free_y", order=0)
        + labs(title=title, subtitle=subtitle, caption=caption)
        + look(mode)
        + theme(axis_text_x=element_text(color=PALETTES[mode]["ink"], size=10))
        + ggsize(980, 620)
    )


def scaling(frame: pl.DataFrame, mode: Mode) -> PlotSpec:
    """Scatter render time as rows grow, both formats, linear axes."""
    colors = PALETTES[mode]
    data = frame.filter(pl.col("scenario") == "scatter").with_columns(
        millions=pl.col("rows") / 1_000_000,
        library=pl.col("library").cast(pl.Enum(LIBRARIES)),
    )
    return (
        ggplot(data, aes("millions", "seconds", color="library"))
        + geom_line(size=1.4)
        + geom_point(size=3.2)
        + scale_color_manual(values=[colors[name] for name in LIBRARIES])
        + scale_x_continuous(
            breaks=[0.01, 0.1, 0.25, 0.5, 0.75, 1.0],
            labels=["", "100k", "250k", "500k", "750k", "1M"],
        )
        + facet_wrap("format", ncol=2, scales="free_y")
        + labs(
            title="Scatter plot render time as data grows",
            subtitle="Seconds, linear axes. ggrust stays near zero.",
            caption="Apple M4 Pro. Measured at 10k, 100k and 1M rows.",
        )
        + look(mode, legend=True)
        + theme(
            axis_text_x=element_text(color=colors["muted"], size=11),
            axis_text_y=element_text(color=colors["muted"], size=11),
            panel_grid_major_y=element_line(color=colors["grid"], size=0.6),
        )
        + ggsize(900, 380)
    )


def compact(frame: pl.DataFrame, mode: Mode) -> PlotSpec:
    """README headline in the spirit of uv's: thin bars, values, faint grid, no title."""
    colors = PALETTES[mode]
    data = (
        frame.filter(pl.col("rows") == 1_000_000)
        .group_by("library")
        .agg(pl.col("seconds").median())
    )
    data = data.with_columns(
        label=pl.Series([f"  {value:.2f}s" for value in data["seconds"]], dtype=pl.String)
    )
    limit = float(data["seconds"].max() or 1.0)
    step = 2.0 if limit > 4 else 1.0
    breaks = [step * index for index in range(int(limit * 1.13 // step) + 1)]
    return (
        bars(data, "seconds", mode, text_size=5, room=0.0, width=0.5)
        + scale_y_continuous(
            breaks=breaks,
            labels=[f"{value:g}s" for value in breaks],
            limits=[0, limit * 1.13],
            expand=[0, 0],
        )
        + look(mode)
        + theme(
            axis_text_y=element_text(color=colors["muted"], size=11),
            axis_text_x=element_text(color=colors["ink"], size=13),
            panel_grid_major_y=element_line(color=colors["grid"], size=0.8),
            plot_margin=[4, 10, 2, 4],
        )
        + ggsize(620, 140)
    )


def main() -> None:
    """Write every chart, light and dark, into docs/images/benchmarks."""
    frame = load(sys.argv[1:])
    OUTPUT.mkdir(parents=True, exist_ok=True)
    modes: tuple[Mode, ...] = ("light", "dark")
    for mode in modes:
        suffix = "" if mode == "light" else "-dark"
        charts: dict[str, PlotSpec] = {
            "overview": overview(frame, mode),
            "time-png": per_scenario(frame, "PNG", "seconds", mode),
            "time-svg": per_scenario(frame, "SVG", "seconds", mode),
            "memory-png": per_scenario(frame, "PNG", "memory", mode),
            "memory-svg": per_scenario(frame, "SVG", "memory", mode),
            "scaling": scaling(frame, mode),
            "headline": compact(frame, mode),
        }
        for name, plot in charts.items():
            path = OUTPUT / f"{name}{suffix}.png"
            plot.to_png(str(path), scale=2)
            print(path)


if __name__ == "__main__":
    main()
