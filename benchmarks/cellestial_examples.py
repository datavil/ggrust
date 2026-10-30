"""Benchmark the cellestial docstring examples on ggrust against lets-plot.

Each case is copied from a cellestial docstring (``Examples`` section) and runs on the
pbmc3k dataset. The same cellestial code runs twice: once in an environment where cellestial
imports ``ggrust`` (the port worktree) and once where it imports ``lets_plot``. Measured per
case, in a fresh process each: ``build`` (the cellestial call, includes its frame building),
``svg`` and ``png`` (scale 2) render of an already built plot, and the peak resident set
above the level reached after loading the dataset (cumulative over the stages).

pbmc3k has about 2.7k cells, below every lets-plot default sampling limit, so both
libraries draw every row (see memory/letsplot_default_sampling.md).

Usage::

    python benchmarks/cellestial_examples.py --ggrust-python <port venv python> \
        --lets-plot-python <poetry env python>
"""

import argparse
import io
import json
import platform
import resource
import statistics
import subprocess
import sys
import time
import warnings
from datetime import UTC, datetime
from pathlib import Path

RESULTS = Path(__file__).parent / "results"
MARKERS = ["C1QA", "PSAP", "CD79A", "CD79B", "CST3", "LYZ"]


def cases(cl, m, data) -> dict:
    """Case name -> zero-argument builder, as written in the cellestial docstrings."""
    return {
        "dimensional_categorical": lambda: cl.dimensional(
            data, key="cell_type_lvl1", axis_type="arrow", legend_ondata=True
        ),
        "dimensional_continuous": lambda: cl.dimensional(
            data, key="CD14", axis_type="arrow", color_high="red"
        ),
        "dimensionals_grid": lambda: cl.dimensionals(
            data,
            keys=["cell_type_lvl1", "sample", "MALAT1", "YBX3"],
            axis_type="arrow",
            color_high="red",
            ncol=2,
        ),
        "scatter": lambda: cl.scatter(
            data,
            mapping=m.aes(x="n_genes_by_counts", y="pct_counts_in_top_50_genes"),
            color="grey",
            alpha=0.6,
        ),
        "violin": lambda: (
            cl.violin(
                data,
                "CD14",
                fill="cell_type_lvl1",
                scale="width",
                point_size=2,
                threshold=0.1,
                trim=False,
            )
            + m.ggsize(800, 400)
            + m.scale_fill_brewer(palette="Set2")
            + m.guides(fill=m.guide_legend(ncol=2))
        ),
        "dotplot": lambda: cl.dotplot(data, keys=MARKERS, group_by="cell_type_lvl1"),
        "stacked_violin": lambda: cl.stacked_violin(data, keys=MARKERS, group_by="cell_type_lvl1"),
        "heatmap": lambda: (
            cl.heatmap(data, group_by="cell_type_lvl1", keys=MARKERS, dendrogram=True)
            + m.scale_fill_viridis()
        ),
        "volcano": lambda: cl.volcano(data, group="B Cells", group_by="cell_type_lvl1"),
    }


def render(plot, output: str) -> int:
    """Render ``plot`` once and return the output size in bytes."""
    if output == "svg":
        return len(plot.to_svg())
    buffer = io.BytesIO()
    plot.to_png(buffer, scale=2)
    return len(buffer.getvalue())


def peak_mb(baseline: int) -> float:
    """Peak resident set above ``baseline`` in MB (macOS reports bytes, Linux kilobytes)."""
    unit = 1 if sys.platform == "darwin" else 1024
    return (resource.getrusage(resource.RUSAGE_SELF).ru_maxrss - baseline) * unit / 1024**2


def worker(library: str, case: str, repeats: int) -> dict:
    """Measure one case in this process; ``library`` is the one this cellestial imports."""
    warnings.filterwarnings("ignore")
    import cellestial as cl
    import cellestial.datasets

    if library == "ggrust":
        import ggrust as module
    else:
        import lets_plot as module
    data = cl.datasets.pbmc3k()
    data.raw = None  # as in the volcano docstring; harmless for the other cases
    build = cases(cl, module, data)[case]
    baseline = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    result = {"library": library, "case": case, "cells": data.n_obs}
    for stage in ("build", "svg", "png"):
        times = []
        size_bytes = 0
        try:
            for _ in range(repeats + 1):
                start = time.perf_counter()
                plot = build()
                if stage != "build":
                    start = time.perf_counter()
                    size_bytes = render(plot, stage)
                times.append(time.perf_counter() - start)
        except Exception as error:
            result[f"{stage}_error"] = f"{type(error).__name__}: {error}"[:300]
            continue
        result[f"{stage}_cold_seconds"] = times[0]
        result[f"{stage}_warm_seconds"] = statistics.median(times[1:])
        result[f"{stage}_bytes"] = size_bytes
        result[f"{stage}_peak_mb"] = peak_mb(baseline)
    return result


def run_in_subprocess(library: str, python: str, case: str, repeats: int, timeout: float) -> dict:
    """Measure one case in a fresh process of ``python``."""
    command = [python, __file__, "--worker", library, case, str(repeats)]
    try:
        done = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return {"library": library, "case": case, "timeout_seconds": timeout}
    if done.returncode != 0:
        return {"library": library, "case": case, "error": done.stderr[-500:]}
    return json.loads(done.stdout.strip().splitlines()[-1])


def seconds(result: dict, stage: str) -> str:
    """Warm seconds of ``stage``, or why it is missing."""
    if f"{stage}_error" in result or "error" in result:
        return "error"
    value = result.get(f"{stage}_warm_seconds")
    return "n/a" if value is None else f"{value:.4f}"


def speed_up(ours: dict, theirs: dict, stage: str) -> str:
    """lets-plot time over ggrust time for ``stage``, when both ran."""
    ours_time = ours.get(f"{stage}_warm_seconds")
    theirs_time = theirs.get(f"{stage}_warm_seconds")
    return f"{theirs_time / ours_time:.0f}x" if ours_time and theirs_time else ""


def render_mb(result: dict) -> str:
    """Peak memory of the SVG and PNG renders above the cellestial build (same on both sides)."""
    last = "png" if "png_peak_mb" in result else "svg"
    if f"{last}_peak_mb" not in result or "build_peak_mb" not in result:
        return "error"
    marker = "" if last == "png" else " (svg only)"
    return f"{result[f'{last}_peak_mb'] - result['build_peak_mb']:.0f}{marker}"


def table(results: list[dict]) -> str:
    """Markdown table: one row per case, ggrust next to lets-plot."""
    by_key = {(r.get("library"), r["case"]): r for r in results}
    lines = [
        "| case | build (s) | svg ggrust (s) | svg lets-plot (s) | svg speed-up "
        "| png ggrust (s) | png lets-plot (s) | png speed-up "
        "| render MB ggrust | render MB lets-plot |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for case in dict.fromkeys(r["case"] for r in results):
        ours = by_key.get(("ggrust", case), {})
        theirs = by_key.get(("lets-plot", case), {})
        cells = [case, seconds(ours, "build")]
        for stage in ("svg", "png"):
            cells += [seconds(ours, stage), seconds(theirs, stage), speed_up(ours, theirs, stage)]
        cells += [render_mb(ours), render_mb(theirs)]
        lines.append("| " + " | ".join(cells) + " |")
    return "\n".join(lines)


def main() -> None:
    """Run every case in both environments and write the results."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--worker", nargs=3, metavar=("LIBRARY", "CASE", "REPEATS"))
    parser.add_argument("--ggrust-python", help="python whose cellestial imports ggrust")
    parser.add_argument("--lets-plot-python", help="python whose cellestial imports lets_plot")
    parser.add_argument("--cases", nargs="*", help="subset of case names")
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--table", type=Path, help="print the table of a results json and exit")
    parser.add_argument("--timeout", type=float, default=900)
    arguments = parser.parse_args()
    if arguments.table:
        print(table(json.loads(arguments.table.read_text())["results"]))
        return
    if arguments.worker:
        library, case, repeats = arguments.worker
        print(json.dumps(worker(library, case, int(repeats))))
        return

    names = arguments.cases or list(cases(None, None, None))
    results = []
    for name in names:
        for library, python in (
            ("ggrust", arguments.ggrust_python),
            ("lets-plot", arguments.lets_plot_python),
        ):
            result = run_in_subprocess(library, python, name, arguments.repeats, arguments.timeout)
            print(json.dumps(result), flush=True)
            results.append(result)

    RESULTS.mkdir(exist_ok=True)
    stamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%S")
    meta = {"machine": platform.platform(), "processor": platform.processor()}
    (RESULTS / f"cellestial-{stamp}.json").write_text(
        json.dumps({"meta": meta, "results": results}, indent=2)
    )
    markdown = table(results)
    (RESULTS / f"cellestial-{stamp}.md").write_text(markdown + "\n")
    print(markdown)


if __name__ == "__main__":
    main()
