"""Draw the charts of benchmarks/REPORT.md from the raw results, with ggrust itself.

Reads the ``scatter-*``, ``scenarios-*`` and ``cellestial-*`` JSON files of the newest run
day in ``benchmarks/results/`` (or the files given) and writes three PNGs next to the report:

- ``report-speedup.png``: lets-plot time / ggrust time per case and output (log scale)
- ``report-memory.png``: lets-plot / ggrust peak memory per case (log scale)
- ``report-scaling.png``: warm render time against points for the plain scatter

Usage::

    uv run python benchmarks/report_charts.py
    uv run python benchmarks/report_charts.py --results a.json b.json ...
"""

import argparse
import json
from pathlib import Path

import ggrust as gg
import polars as pl

HERE = Path(__file__).parent
RESULTS = HERE / "results"
OUTPUT_COLORS = {"svg": "#377eb8", "png": "#e41a1c"}
LIBRARY_COLORS = {"ggrust": "#e41a1c", "lets-plot": "#377eb8", "lets-plot-sampled": "#999999"}


def newest_day(prefix: str) -> list[Path]:
    """Results JSONs named ``prefix-<day>-<time>.json`` from the newest day (one run's parts)."""
    files = sorted(RESULTS.glob(f"{prefix}-*.json"))
    if not files:
        return []
    day = files[-1].name.split("-")[1]
    return [path for path in files if path.name.split("-")[1] == day]


def load_results(paths: list[Path]) -> dict:
    """Concatenate the ``results`` lists of the given JSON files."""
    if not paths:
        raise SystemExit("no results files found; run the benchmarks first or pass paths")
    return {"results": [r for path in paths for r in json.loads(path.read_text())["results"]]}


def pair_rows(results: list[dict], key: str) -> dict[tuple, dict[str, dict]]:
    """Group results by ``(key value, points)`` into ``{library: result}``."""
    pairs: dict[tuple, dict[str, dict]] = {}
    for result in results:
        identity = (result[key], result.get("points", result.get("cells")))
        pairs.setdefault(identity, {})[result["library"]] = result
    return pairs


def comparison_frame(scatter: dict, scenarios: dict, cellestial: dict) -> pl.DataFrame:
    """One row per (suite, case, output) with both libraries' warm time and memory."""
    rows: list[dict] = []
    for (output, points), pair in pair_rows(scatter["results"], "output").items():
        mine, theirs = pair.get("ggrust", {}), pair.get("lets-plot", {})
        if points != max(r["points"] for r in scatter["results"]):
            continue
        rows.append(
            {
                "case": f"scatter {points:,}",
                "suite": "scatter",
                "output": output,
                "ggrust_seconds": mine.get("warm_seconds"),
                "lets_plot_seconds": theirs.get("warm_seconds"),
                "ggrust_mb": mine.get("peak_memory_above_data_mb"),
                "lets_plot_mb": theirs.get("peak_memory_above_data_mb"),
            }
        )
    largest: dict[str, int] = {}
    for result in scenarios["results"]:
        if "svg_warm_seconds" in result:
            largest[result["scenario"]] = max(largest.get(result["scenario"], 0), result["points"])
    for (scenario, points), pair in pair_rows(scenarios["results"], "scenario").items():
        if points != largest.get(scenario):
            continue
        mine, theirs = pair.get("ggrust", {}), pair.get("lets-plot", {})
        for output in ("svg", "png"):
            rows.append(
                {
                    "case": f"{scenario} {points:,}",
                    "suite": "scenarios",
                    "output": output,
                    "ggrust_seconds": mine.get(f"{output}_warm_seconds"),
                    "lets_plot_seconds": theirs.get(f"{output}_warm_seconds"),
                    # Peak over the whole worker: the PNG stage includes the SVG stage.
                    "ggrust_mb": mine.get("png_peak_mb"),
                    "lets_plot_mb": theirs.get("png_peak_mb"),
                }
            )
    for (case, _cells), pair in pair_rows(cellestial["results"], "case").items():
        mine, theirs = pair.get("ggrust", {}), pair.get("lets-plot", {})
        for output in ("svg", "png"):
            rows.append(
                {
                    "case": f"cellestial {case}",
                    "suite": "cellestial",
                    "output": output,
                    "ggrust_seconds": mine.get(f"{output}_warm_seconds"),
                    "lets_plot_seconds": theirs.get(f"{output}_warm_seconds"),
                    "ggrust_mb": render_mb(mine),
                    "lets_plot_mb": render_mb(theirs),
                }
            )
    return pl.DataFrame(rows, infer_schema_length=None).with_columns(
        speed_up=pl.col("lets_plot_seconds") / pl.col("ggrust_seconds"),
        memory_ratio=pl.col("lets_plot_mb") / pl.col("ggrust_mb").clip(lower_bound=1),
    )


def render_mb(result: dict) -> float | None:
    """Peak memory of the cellestial renders above the build, or None when one failed."""
    if "png_peak_mb" not in result or "build_peak_mb" not in result:
        return None
    return result["png_peak_mb"] - result["build_peak_mb"]


def log_limits(values: pl.Series) -> list[float]:
    """Log-axis limits padding the range of ``values`` by 25 percent each way."""
    return [float(values.min()) / 1.25, float(values.max()) * 1.25]


def ratio_chart(
    frame: pl.DataFrame, column: str, title: str, x_label: str, by_output: bool
) -> gg.PlotSpec:
    """Dot chart of a lets-plot / ggrust ratio per case, cases ordered by their median ratio.

    Log axes get explicit limits: lets-plot (and ggrust, for parity) expand them in data
    space and drop the lower expansion, which puts the extreme points on the panel border.
    """
    order = (
        frame.group_by("case")
        .agg(pl.col(column).median())
        .sort(column, descending=True)["case"]
        .to_list()
    )
    data = frame.with_columns(pl.col("case").cast(pl.Enum(order)))
    points = gg.geom_point(size=4) if by_output else gg.geom_point(size=4, color="#4daf4a")
    plot = (
        gg.ggplot(
            data, gg.aes(column, "case", color="output") if by_output else gg.aes(column, "case")
        )
        + gg.geom_vline(xintercept=1, color="#999999", linetype="dashed")
        + points
        + gg.scale_x_log10(x_label, limits=log_limits(data[column].append(pl.Series([1.0]))))
        + gg.labs(y="", title=title)
        + gg.theme_bw()
        + gg.ggsize(760, 28 * len(order) + 120)
    )
    return plot + gg.scale_color_manual(values=OUTPUT_COLORS) if by_output else plot


def scaling_chart(scatter: dict) -> gg.PlotSpec:
    """Warm time against points, both axes log, one panel per output."""
    frame = pl.DataFrame(
        [r for r in scatter["results"] if r.get("warm_seconds") is not None],
        infer_schema_length=None,
    ).select("library", "output", "points", "warm_seconds")
    return (
        gg.ggplot(frame, gg.aes("points", "warm_seconds", color="library"))
        + gg.geom_line(size=1)
        + gg.geom_point(size=3)
        + gg.scale_x_log10("points drawn", limits=log_limits(frame["points"]))
        + gg.scale_y_log10("warm render time (s)", limits=log_limits(frame["warm_seconds"]))
        + gg.scale_color_manual(values=LIBRARY_COLORS)
        + gg.facet_wrap("output")
        + gg.labs(title="Plain scatter: render time against points")
        + gg.theme_bw()
        + gg.ggsize(760, 360)
    )


def main() -> None:
    """Load the results and write the three report charts."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--scatter", type=Path, nargs="+", default=newest_day("scatter"))
    parser.add_argument("--scenarios", type=Path, nargs="+", default=newest_day("scenarios"))
    parser.add_argument("--cellestial", type=Path, nargs="+", default=newest_day("cellestial")[-1:])
    arguments = parser.parse_args()
    scatter = load_results(arguments.scatter)
    scenarios = load_results(arguments.scenarios)
    cellestial = load_results(arguments.cellestial)

    frame = comparison_frame(scatter, scenarios, cellestial)
    speed = frame.drop_nulls("speed_up")
    memory = frame.filter(pl.col("output") == "png").drop_nulls("memory_ratio")
    charts = {
        "report-speedup.png": ratio_chart(
            speed,
            "speed_up",
            "ggrust speed-up over lets-plot",
            "lets-plot time / ggrust time",
            by_output=True,
        ),
        "report-memory.png": ratio_chart(
            memory,
            "memory_ratio",
            "lets-plot peak memory / ggrust peak memory (svg + png renders)",
            "memory ratio",
            by_output=False,
        ),
        "report-scaling.png": scaling_chart(scatter),
    }
    for name, chart in charts.items():
        chart.to_png(str(HERE / name), scale=2)
        print(HERE / name)


if __name__ == "__main__":
    main()
