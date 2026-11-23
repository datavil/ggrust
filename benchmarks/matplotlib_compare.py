"""Benchmark ggrust against matplotlib on common plot scenarios, across data sizes.

Fairness rules:

- same polars frame for both libraries; matplotlib's conversion to numpy, group splitting
  and sorting (``geom_line`` sorts by x) happen inside the timed region, as ggrust's do
- same picture size: 600 x 400 px figure, PNG at 2x (1200 x 800), SVG at 1x
- matplotlib uses the Agg backend and its idiomatic calls (``ax.scatter``, ``ax.hist``,
  ``ax.boxplot``, ``ax.violinplot``, ``plt.subplots`` for facets); one extra row,
  ``scatter_plot``, gives matplotlib its faster ``ax.plot(x, y, "o")`` path for plain scatter
- each (library, scenario, size, output) runs in a fresh process: **cold** = first call,
  **warm** = median of the following calls, each call is data frame -> output bytes
- **memory** = peak resident set above the level reached after importing and building data

Usage::

    uv run --with matplotlib python benchmarks/matplotlib_compare.py
    uv run --with matplotlib python benchmarks/matplotlib_compare.py --sizes 1000 100000
"""

from __future__ import annotations

import argparse
import io
import json
import platform
import resource
import statistics
import subprocess
import sys
import time
from collections.abc import Callable
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING, Literal

if TYPE_CHECKING:
    import polars as pl

sys.path.insert(0, str(Path(__file__).parent))
from scenarios import make_frame

RESULTS = Path(__file__).parent / "results"
# ru_maxrss is in kilobytes on Linux and in bytes on macOS.
BYTES_PER_UNIT = 1024 if sys.platform == "darwin" else 1
Library = Literal["ggrust", "matplotlib"]
Output = Literal["svg", "png"]
LIBRARIES: tuple[Library, ...] = ("ggrust", "matplotlib")
SCENARIOS = [
    "scatter",
    "scatter_plot",
    "color_discrete",
    "color_continuous",
    "log_scales",
    "histogram",
    "bar",
    "boxplot",
    "violin",
    "line",
    "facet",
]


def ggrust_writer(frame: pl.DataFrame) -> dict[str, Callable[[Output], bytes]]:
    """Scenario name -> function writing the ggrust plot to SVG or PNG bytes."""
    import ggrust as m

    def write(build: Callable[[], m.PlotSpec]) -> Callable[[Output], bytes]:
        def run(output: Output) -> bytes:
            plot = build() + m.ggsize(600, 400)
            if output == "svg":
                return plot.to_svg().encode()
            buffer = io.BytesIO()
            plot.to_png(buffer, scale=2)
            return buffer.getvalue()

        return run

    scatter = lambda: m.ggplot(frame, m.aes("x", "y")) + m.geom_point(size=1)  # noqa: E731
    return {
        "scatter": write(scatter),
        "scatter_plot": write(scatter),
        "color_discrete": write(
            lambda: m.ggplot(frame, m.aes("x", "y", color="group")) + m.geom_point(size=1)
        ),
        "color_continuous": write(
            lambda: m.ggplot(frame, m.aes("x", "y", color="value")) + m.geom_point(size=1)
        ),
        "log_scales": write(
            lambda: (
                m.ggplot(frame, m.aes("positive", "value"))
                + m.geom_point(size=1)
                + m.scale_x_log10()
                + m.scale_y_log10()
            )
        ),
        "histogram": write(lambda: m.ggplot(frame, m.aes("x")) + m.geom_histogram(bins=30)),
        "bar": write(lambda: m.ggplot(frame, m.aes("group")) + m.geom_bar()),
        "boxplot": write(lambda: m.ggplot(frame, m.aes("group", "y")) + m.geom_boxplot()),
        "violin": write(lambda: m.ggplot(frame, m.aes("group", "y")) + m.geom_violin()),
        "line": write(lambda: m.ggplot(frame, m.aes("x", "y", color="group")) + m.geom_line()),
        "facet": write(
            lambda: (
                m.ggplot(frame, m.aes("x", "y"))
                + m.geom_point(size=1)
                + m.facet_wrap("group", ncol=3)
            )
        ),
    }


def matplotlib_writer(frame: pl.DataFrame) -> dict[str, Callable[[Output], bytes]]:
    """Scenario name -> function writing the equivalent matplotlib figure to SVG or PNG bytes."""
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    # geom_point(size=1) is a 2.2 px dot = 1.58 pt at 100 dpi; matplotlib's s is diameter^2 in pt^2.
    # Edge strokes are off so marker sizes match.
    marker_area = 2.5
    marker_size = 1.58

    def groups() -> list[tuple[str, pl.DataFrame]]:
        parts = frame.partition_by("group", as_dict=True, maintain_order=True)
        return sorted((str(key[0]), part) for key, part in parts.items())

    def write(draw: Callable[[], plt.Figure]) -> Callable[[Output], bytes]:
        def run(output: Output) -> bytes:
            figure = draw()
            buffer = io.BytesIO()
            figure.savefig(buffer, format=output, dpi=200 if output == "png" else 100)
            plt.close(figure)
            return buffer.getvalue()

        return run

    def axes() -> tuple[plt.Figure, plt.Axes]:
        return plt.subplots(figsize=(6, 4), dpi=100)

    def scatter() -> plt.Figure:
        figure, ax = axes()
        ax.scatter(frame["x"].to_numpy(), frame["y"].to_numpy(), s=marker_area, linewidths=0)
        ax.set(xlabel="x", ylabel="y")
        return figure

    def scatter_plot() -> plt.Figure:
        figure, ax = axes()
        ax.plot(
            frame["x"].to_numpy(),
            frame["y"].to_numpy(),
            "o",
            markersize=marker_size,
            markeredgewidth=0,
        )
        ax.set(xlabel="x", ylabel="y")
        return figure

    def color_discrete() -> plt.Figure:
        figure, ax = axes()
        for name, part in groups():
            ax.scatter(
                part["x"].to_numpy(), part["y"].to_numpy(), s=marker_area, label=name, linewidths=0
            )
        ax.legend(title="group", loc="center left", bbox_to_anchor=(1, 0.5))
        ax.set(xlabel="x", ylabel="y")
        figure.tight_layout()
        return figure

    def color_continuous() -> plt.Figure:
        figure, ax = axes()
        mappable = ax.scatter(
            frame["x"].to_numpy(),
            frame["y"].to_numpy(),
            c=frame["value"].to_numpy(),
            s=marker_area,
            linewidths=0,
        )
        figure.colorbar(mappable, ax=ax, label="value")
        ax.set(xlabel="x", ylabel="y")
        return figure

    def log_scales() -> plt.Figure:
        figure, ax = axes()
        ax.scatter(
            frame["positive"].to_numpy(), frame["value"].to_numpy(), s=marker_area, linewidths=0
        )
        ax.set(xscale="log", yscale="log", xlabel="positive", ylabel="value")
        return figure

    def histogram() -> plt.Figure:
        figure, ax = axes()
        ax.hist(frame["x"].to_numpy(), bins=30)
        ax.set(xlabel="x", ylabel="count")
        return figure

    def bar() -> plt.Figure:
        figure, ax = axes()
        counts = frame["group"].value_counts().sort("group")
        ax.bar(counts["group"].to_list(), counts["count"].to_numpy())
        ax.set(xlabel="group", ylabel="count")
        return figure

    def boxplot() -> plt.Figure:
        figure, ax = axes()
        parts = groups()
        ax.boxplot([part["y"].to_numpy() for _, part in parts], tick_labels=[n for n, _ in parts])
        ax.set(xlabel="group", ylabel="y")
        return figure

    def violin() -> plt.Figure:
        figure, ax = axes()
        parts = groups()
        ax.violinplot([part["y"].to_numpy() for _, part in parts])
        ax.set_xticks(range(1, len(parts) + 1), [n for n, _ in parts])
        ax.set(xlabel="group", ylabel="y")
        return figure

    def line() -> plt.Figure:
        figure, ax = axes()
        for name, part in groups():
            ordered = part.sort("x")
            ax.plot(ordered["x"].to_numpy(), ordered["y"].to_numpy(), label=name, linewidth=1)
        ax.legend(title="group", loc="center left", bbox_to_anchor=(1, 0.5))
        ax.set(xlabel="x", ylabel="y")
        figure.tight_layout()
        return figure

    def facet() -> plt.Figure:
        figure, grid = plt.subplots(2, 3, figsize=(6, 4), dpi=100, sharex=True, sharey=True)
        for ax, (name, part) in zip(grid.flat, groups(), strict=True):
            ax.scatter(part["x"].to_numpy(), part["y"].to_numpy(), s=marker_area, linewidths=0)
            ax.set_title(name)
        figure.tight_layout()
        return figure

    return {
        "scatter": write(scatter),
        "scatter_plot": write(scatter_plot),
        "color_discrete": write(color_discrete),
        "color_continuous": write(color_continuous),
        "log_scales": write(log_scales),
        "histogram": write(histogram),
        "bar": write(bar),
        "boxplot": write(boxplot),
        "violin": write(violin),
        "line": write(line),
        "facet": write(facet),
    }


def writers(library: Library, frame: pl.DataFrame) -> dict[str, Callable[[Output], bytes]]:
    """Scenario writers for one library."""
    return ggrust_writer(frame) if library == "ggrust" else matplotlib_writer(frame)


def worker(library: Library, scenario: str, size: int, output: Output, repeats: int) -> dict:
    """Measure one (library, scenario, size, output) in this process."""
    frame = make_frame(size)
    write = writers(library, frame)[scenario]
    baseline_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    times = []
    size_bytes = 0
    for _ in range(repeats + 1):
        start = time.perf_counter()
        size_bytes = len(write(output))
        times.append(time.perf_counter() - start)
    peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss - baseline_rss
    return {
        "library": library,
        "scenario": scenario,
        "points": size,
        "output": output,
        "cold_seconds": times[0],
        "warm_seconds": statistics.median(times[1:]) if repeats else times[0],
        "bytes": size_bytes,
        "peak_mb": peak / (1024 * BYTES_PER_UNIT),
    }


def import_seconds(library: Library) -> float:
    """Median fresh-process import time of the library's plotting entry point."""
    statement = (
        "import ggrust"
        if library == "ggrust"
        else "import matplotlib; matplotlib.use('Agg'); import matplotlib.pyplot"
    )
    code = f"import time; s = time.perf_counter(); {statement}; print(time.perf_counter() - s)"
    runs = [
        float(subprocess.run([sys.executable, "-c", code], capture_output=True, text=True).stdout)
        for _ in range(5)
    ]
    return statistics.median(runs)


def run_in_subprocess(
    library: Library, scenario: str, size: int, output: Output, repeats: int, timeout: float
) -> dict:
    """Measure one configuration in a fresh process."""
    command = [
        sys.executable,
        __file__,
        "--worker",
        library,
        scenario,
        str(size),
        output,
        str(repeats),
    ]
    base = {"library": library, "scenario": scenario, "points": size, "output": output}
    try:
        done = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return base | {"timeout_seconds": timeout}
    if done.returncode != 0:
        return base | {"error": done.stderr[-500:]}
    return json.loads(done.stdout.strip().splitlines()[-1])


def render_images(size: int, directory: Path) -> None:
    """Write every scenario as PNG for both libraries, to check the pictures match."""
    frame = make_frame(size)
    directory.mkdir(parents=True, exist_ok=True)
    for library in LIBRARIES:
        for name, write in writers(library, frame).items():
            (directory / f"{name}-{library}.png").write_bytes(write("png"))


def table(results: list[dict]) -> str:
    """Markdown table, one row per scenario, size and output, libraries side by side."""
    lines = [
        "| scenario | points | output | ggrust (s) | matplotlib (s) | speed-up "
        "| ggrust MB | matplotlib MB | ggrust bytes | matplotlib bytes |",
        "|---|---:|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    index = {(r["library"], r["scenario"], r["points"], r["output"]): r for r in results}
    keys = sorted(
        {(r["scenario"], r["points"], r["output"]) for r in results},
        key=lambda k: (SCENARIOS.index(k[0]), k[2], k[1]),
    )
    for scenario, points, output in keys:
        mine = index.get(("ggrust", scenario, points, output), {})
        theirs = index.get(("matplotlib", scenario, points, output), {})

        def cell(result: dict, key: str, pattern: str) -> str:
            if key in result:
                return pattern.format(result[key])
            if "timeout_seconds" in result:
                return f"> {result['timeout_seconds']:.0f} s"
            return "error" if "error" in result else "-"

        a, b = mine.get("warm_seconds"), theirs.get("warm_seconds")
        speed = f"{b / a:.1f}x" if a and b else "-"
        lines.append(
            f"| {scenario} | {points:,} | {output} | {cell(mine, 'warm_seconds', '{:.4f}')} "
            f"| {cell(theirs, 'warm_seconds', '{:.4f}')} | {speed} "
            f"| {cell(mine, 'peak_mb', '{:.0f}')} | {cell(theirs, 'peak_mb', '{:.0f}')} "
            f"| {cell(mine, 'bytes', '{:,}')} | {cell(theirs, 'bytes', '{:,}')} |"
        )
    return "\n".join(lines)


def main() -> None:
    """Run the matrix, or one worker measurement."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--worker", nargs=5, metavar=("LIBRARY", "SCENARIO", "POINTS", "OUTPUT", "REPEATS")
    )
    parser.add_argument("--sizes", nargs="+", type=int, default=[1_000, 10_000, 100_000, 1_000_000])
    parser.add_argument("--outputs", nargs="+", default=["png", "svg"], choices=["png", "svg"])
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=600)
    parser.add_argument("--images", type=int, default=5_000, help="points in comparison images")
    parser.add_argument("--scenarios", nargs="+", default=SCENARIOS, choices=SCENARIOS)
    arguments = parser.parse_args()
    if arguments.worker:
        library, scenario, size, output, repeats = arguments.worker
        print(json.dumps(worker(library, scenario, int(size), output, int(repeats))))
        return
    stamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%S")
    render_images(arguments.images, RESULTS / f"matplotlib-{stamp}-images")
    imports = {library: import_seconds(library) for library in LIBRARIES}
    print(f"import seconds: {imports}", flush=True)
    results = []
    for size in arguments.sizes:
        for scenario in arguments.scenarios:
            for output in arguments.outputs:
                for library in LIBRARIES:
                    result = run_in_subprocess(
                        library, scenario, size, output, arguments.repeats, arguments.timeout
                    )
                    results.append(result)
                    status = result.get("error") or (
                        f"{result['warm_seconds']:.4f} s" if "warm_seconds" in result else "timeout"
                    )
                    print(f"{library} {scenario} {size:,} {output}: {status}", flush=True)
    import ggrust
    import matplotlib

    report = {
        "date": datetime.now(UTC).isoformat(timespec="seconds"),
        "machine": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "python": platform.python_version(),
            "ggrust": ggrust.__version__,
            "matplotlib": matplotlib.__version__,
        },
        "import_seconds": imports,
        "results": results,
    }
    RESULTS.mkdir(exist_ok=True)
    (RESULTS / f"matplotlib-{stamp}.json").write_text(json.dumps(report, indent=2))
    (RESULTS / f"matplotlib-{stamp}.md").write_text(table(results) + "\n")
    print(table(results))
    print(f"\nstamp {stamp}")


if __name__ == "__main__":
    main()
