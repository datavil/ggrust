"""Benchmark ggrust against lets-plot and R ggplot2 on common plot scenarios.

Fairness rules:

- Identical data: one CSV per size, written once, read by every library before timing
  starts (reading is not timed). lets-plot runs with ``sampling="none"``.
- Same work: the same 11 scenarios, the same stats, the same pixel size (600 x 400 px,
  PNG at scale 2 = 1200 x 800). Every library writes its output to a file, as a user
  saving a plot would. R uses the devices ``ggsave`` picks (svglite, ragg).
- Every (library, scenario, size, format) runs in a fresh process wrapped in
  ``/usr/bin/time -l``. Memory = peak RSS of that process minus its RSS after imports
  and data load. Time = cold (first render) and warm (median of the following renders).

Usage::

    uv run python benchmarks/three_way.py                       # 10k, 100k, 1M
    uv run python benchmarks/three_way.py --sizes 10000 --scenarios scatter bar
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import statistics
import subprocess
import sys
import time
import warnings
from collections.abc import Callable
from datetime import UTC, datetime
from pathlib import Path
from types import ModuleType
from typing import Any, Literal

HERE = Path(__file__).parent
RESULTS = HERE / "results"
LIBRARIES = ("ggrust", "lets-plot", "ggplot2")
FORMATS = ("svg", "png")
SCENARIOS = [
    "scatter",
    "color_discrete",
    "histogram",
    "bar",
    "boxplot",
    "violin",
    "density",
    "smooth",
    "line",
    "bin2d",
    "facet",
]
Format = Literal["build", "svg", "png"]


def write_frame(size: int, path: Path) -> None:
    """Everyday-style data: two numeric axes and a 6-level category, as CSV."""
    import numpy as np
    import polars as pl

    rng = np.random.default_rng(0)
    group = rng.integers(0, 6, size)
    pl.DataFrame(
        {
            "x": rng.normal(size=size) + group,
            "y": rng.normal(size=size) * (1 + group / 3),
            "group": np.char.add("g", group.astype(str)),
        }
    ).write_csv(path)


def builders(module: ModuleType, frame: Any, sample: dict[str, str]) -> dict[str, Callable]:
    """Scenario name -> zero-argument plot builder, the same specs as ``three_way.R``."""
    m = module
    point = lambda **kw: m.geom_point(**kw, **sample)  # noqa: E731
    return {
        "scatter": lambda: m.ggplot(frame, m.aes("x", "y")) + point(size=1),
        "color_discrete": lambda: m.ggplot(frame, m.aes("x", "y", color="group")) + point(size=1),
        "histogram": lambda: m.ggplot(frame, m.aes("x")) + m.geom_histogram(bins=30, **sample),
        "bar": lambda: m.ggplot(frame, m.aes("group")) + m.geom_bar(**sample),
        "boxplot": lambda: m.ggplot(frame, m.aes("group", "y")) + m.geom_boxplot(),
        "violin": lambda: m.ggplot(frame, m.aes("group", "y")) + m.geom_violin(**sample),
        "density": lambda: (
            m.ggplot(frame, m.aes("x", fill="group")) + m.geom_density(alpha=0.3, **sample)
        ),
        "smooth": lambda: (
            m.ggplot(frame, m.aes("x", "y")) + point(size=1) + m.geom_smooth(method="lm", **sample)
        ),
        "line": lambda: m.ggplot(frame, m.aes("x", "y", color="group")) + m.geom_line(**sample),
        "bin2d": lambda: m.ggplot(frame, m.aes("x", "y")) + m.geom_bin2d(bins=[30, 30], **sample),
        "facet": lambda: (
            m.ggplot(frame, m.aes("x", "y")) + point(size=1) + m.facet_wrap("group", ncol=3)
        ),
    }


def rss_kb() -> float:
    """Return this process's resident set size in KB (same probe as the R worker)."""
    output = subprocess.run(
        ["ps", "-o", "rss=", "-p", str(os.getpid())],
        capture_output=True,
        text=True,
        check=True,
    )
    return float(output.stdout.strip())


def python_worker(
    library: str, scenario: str, csv: str, output_format: Format, repeats: int, output: str
) -> dict[str, Any]:
    """Measure one Python configuration in this process."""
    import polars as pl

    frame = pl.read_csv(csv)
    if library == "ggrust":
        import ggrust as module

        sample: dict[str, str] = {}
    else:
        warnings.filterwarnings("ignore")
        import lets_plot as module

        module.LetsPlot.setup_html()
        sample = {"sampling": "none"}
    build = builders(module, frame, sample)[scenario]
    baseline = rss_kb()
    times: list[float] = []
    for _ in range(repeats + 1):
        start = time.perf_counter()
        plot = build()
        if output_format == "svg":
            plot.to_svg(output)
        elif output_format == "png":
            plot.to_png(output, scale=2)
        times.append(time.perf_counter() - start)
        if sum(times) > 30:
            break
    warm = statistics.median(times[1:]) if len(times) > 1 else times[0]
    size = 0 if output_format == "build" else Path(output).stat().st_size
    return {
        "cold_seconds": times[0],
        "warm_seconds": warm,
        "bytes": size,
        "baseline_rss_kb": baseline,
        "runs": len(times),
    }


def run_one(
    library: str,
    scenario: str,
    csv: Path,
    output_format: Format,
    repeats: int,
    output: Path,
    timeout: float,
) -> dict[str, Any]:
    """Measure one configuration in a fresh process under ``/usr/bin/time -l``."""
    if library == "ggplot2":
        inner = ["Rscript", str(HERE / "three_way.R")]
        inner += [scenario, str(csv), output_format, str(repeats), str(output)]
    else:
        inner = [sys.executable, __file__, "--worker", library, scenario, str(csv)]
        inner += [output_format, str(repeats), str(output)]
    base = {"library": library, "scenario": scenario, "format": output_format}
    try:
        done = subprocess.run(
            ["/usr/bin/time", "-l", *inner], capture_output=True, text=True, timeout=timeout
        )
    except subprocess.TimeoutExpired:
        return base | {"timeout_seconds": timeout}
    if done.returncode != 0:
        return base | {"error": done.stderr[-800:]}
    lines = [line for line in done.stdout.splitlines() if line.startswith("{")]
    result = base | json.loads(lines[-1])
    match = re.search(r"(\d+)\s+maximum resident set size", done.stderr)
    if match:
        peak_kb = int(match.group(1)) / 1024  # bytes on macOS
        result["peak_rss_mb"] = peak_kb / 1024
        result["memory_mb"] = (peak_kb - result["baseline_rss_kb"]) / 1024
    return result


def table(results: list[dict[str, Any]]) -> str:
    """Markdown table: one row per scenario, size and format, libraries side by side."""
    header = (
        "| scenario | rows | format | ggrust s | lets-plot s | ggplot2 s "
        "| vs lets-plot | vs ggplot2 | ggrust MB | lets-plot MB | ggplot2 MB |"
    )
    lines = [header, "|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|"]
    index = {(r["library"], r["scenario"], r["rows"], r["format"]): r for r in results}
    keys = sorted({(r["rows"], SCENARIOS.index(r["scenario"]), r["format"]) for r in results})

    def cell(result: dict[str, Any], key: str, pattern: str) -> str:
        if "timeout_seconds" in result:
            return f">{result['timeout_seconds']:.0f}"
        return pattern.format(result[key]) if key in result else "err"

    for rows, scenario_index, output_format in keys:
        scenario = SCENARIOS[scenario_index]
        row = [index.get((lib, scenario, rows, output_format), {}) for lib in LIBRARIES]
        mine = row[0].get("warm_seconds")
        ratios = []
        for other in row[1:]:
            theirs = other.get("warm_seconds")
            ratios.append(f"{theirs / mine:.1f}x" if mine and theirs else "-")
        lines.append(
            f"| {scenario} | {rows:,} | {output_format} | "
            + " | ".join(cell(r, "warm_seconds", "{:.3f}") for r in row)
            + " | "
            + " | ".join(ratios)
            + " | "
            + " | ".join(cell(r, "memory_mb", "{:.0f}") for r in row)
            + " |"
        )
    return "\n".join(lines)


def main() -> None:
    """Run the matrix, or one Python worker measurement."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", nargs=6)
    parser.add_argument("--sizes", nargs="+", type=int, default=[10_000, 100_000, 1_000_000])
    parser.add_argument("--scenarios", nargs="+", default=SCENARIOS, choices=SCENARIOS)
    parser.add_argument("--libraries", nargs="+", default=list(LIBRARIES), choices=LIBRARIES)
    parser.add_argument("--formats", nargs="+", default=list(FORMATS), choices=FORMATS)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=300)
    parser.add_argument("--work", type=Path, default=RESULTS / "three-way-work")
    arguments = parser.parse_args()
    if arguments.worker:
        library, scenario, csv, output_format, repeats, output = arguments.worker
        result = python_worker(library, scenario, csv, output_format, int(repeats), output)
        print(json.dumps(result))
        return
    stamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%S")
    work = arguments.work
    work.mkdir(parents=True, exist_ok=True)
    results = []
    for size in arguments.sizes:
        csv = work / f"data-{size}.csv"
        if not csv.exists():
            write_frame(size, csv)
        for scenario in arguments.scenarios:
            for output_format in arguments.formats:
                for library in arguments.libraries:
                    output = work / f"{scenario}-{size}-{library}.{output_format}"
                    result = run_one(
                        library,
                        scenario,
                        csv,
                        output_format,
                        arguments.repeats,
                        output,
                        arguments.timeout,
                    )
                    result["rows"] = size
                    results.append(result)
                    summary = (
                        result.get("error")
                        or result.get("timeout_seconds")
                        or (f"{result['warm_seconds']:.3f}s {result.get('memory_mb', 0):.0f}MB")
                    )
                    print(f"{library} {scenario} {size:,} {output_format}: {summary}", flush=True)
                    # Keep the small outputs for the visual check, drop the large ones.
                    if size > 10_000:
                        output.unlink(missing_ok=True)
    versions = subprocess.run(
        [
            "Rscript",
            "-e",
            'cat(R.version.string, "ggplot2", format(packageVersion("ggplot2")),'
            ' "svglite", format(packageVersion("svglite")),'
            ' "ragg", format(packageVersion("ragg")))',
        ],
        capture_output=True,
        text=True,
    ).stdout
    import ggrust
    import lets_plot

    report = {
        "date": datetime.now(UTC).isoformat(timespec="seconds"),
        "machine": {"platform": platform.platform(), "python": platform.python_version()},
        "versions": {
            "ggrust": ggrust.__version__,
            "lets_plot": lets_plot.__version__,
            "r": versions,
        },
        "results": results,
    }
    RESULTS.mkdir(exist_ok=True)
    (RESULTS / f"three-way-{stamp}.json").write_text(json.dumps(report, indent=2))
    (RESULTS / f"three-way-{stamp}.md").write_text(table(results) + "\n")
    print(table(results))
    print(f"\nstamp {stamp}")


if __name__ == "__main__":
    main()
