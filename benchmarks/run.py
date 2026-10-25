"""Benchmark ggrust against lets-plot on identical work.

Fairness rules (see memory/letsplot_default_sampling.md):
- same polars frame, same plot (scatter), same size (600 x 400), same PNG scale (2)
- lets-plot runs with ``sampling="none"`` so it draws every point, like ggrust
- each measurement runs in a fresh process: ``cold`` is the first call (includes
  library start-up work), ``warm`` the median of the following calls
- memory is the peak resident set above the level reached after building the data

Usage::

    uv run python benchmarks/run.py                       # default matrix
    uv run python benchmarks/run.py --sizes 10000 100000 --outputs svg
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


def build_plot(library: str, size: int):
    """Return the data and a function building the benchmark plot with ``library``."""
    import numpy as np
    import polars as pl

    rng = np.random.default_rng(0)
    frame = pl.DataFrame({"x": rng.normal(size=size), "y": rng.normal(size=size)})
    if library == "ggrust":
        import ggrust as module

        layer = module.geom_point(size=1)
    else:
        warnings.filterwarnings("ignore")
        import lets_plot as module

        layer = module.geom_point(size=1, sampling="none")
    return frame, lambda: module.ggplot(frame, module.aes("x", "y")) + layer


def render_once(plot, output: str) -> int:
    """Render ``plot`` once and return the output size in bytes."""
    if output == "svg":
        return len(plot.to_svg())
    buffer = io.BytesIO()
    plot.to_png(buffer, scale=2)
    return len(buffer.getvalue())


def worker(library: str, output: str, size: int, repeats: int) -> dict:
    """Measure one configuration in this process."""
    _frame, make_plot = build_plot(library, size)
    baseline_kb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    times = []
    output_bytes = 0
    for _ in range(repeats + 1):
        start = time.perf_counter()
        output_bytes = render_once(make_plot(), output)
        times.append(time.perf_counter() - start)
    peak_kb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return {
        "library": library,
        "output": output,
        "points": size,
        "cold_seconds": times[0],
        "warm_seconds": statistics.median(times[1:]) if repeats else None,
        "repeats": repeats,
        "peak_memory_above_data_mb": (peak_kb - baseline_kb) / 1024,
        "output_bytes": output_bytes,
    }


def run_in_subprocess(library: str, output: str, size: int, repeats: int, timeout: float) -> dict:
    """Measure one configuration in a fresh process."""
    command = [sys.executable, __file__, "--worker", library, output, str(size), str(repeats)]
    try:
        completed = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return {"library": library, "output": output, "points": size, "timeout_seconds": timeout}
    if completed.returncode != 0:
        return {
            "library": library,
            "output": output,
            "points": size,
            "error": completed.stderr[-500:],
        }
    return json.loads(completed.stdout.strip().splitlines()[-1])


def format_number(result: dict, key: str, digits: int = 3) -> str:
    """Format ``result[key]`` or ``-`` when missing."""
    value = result.get(key)
    return "-" if value is None else f"{value:.{digits}f}"


def markdown(results: list[dict]) -> str:
    """Results as a markdown table."""
    lines = [
        "| output | points | library | cold (s) | warm (s) | peak memory above data (MB) |",
        "|---|---:|---|---:|---:|---:|",
    ]
    for result in results:
        if "timeout_seconds" in result:
            cells = ["timeout", f"> {result['timeout_seconds']:.0f}", "-"]
        elif "error" in result:
            cells = ["error", "-", "-"]
        else:
            cells = [
                format_number(result, "cold_seconds"),
                format_number(result, "warm_seconds"),
                format_number(result, "peak_memory_above_data_mb", 1),
            ]
        prefix = f"| {result['output']} | {result['points']:,} | {result['library']} | "
        lines.append(prefix + " | ".join(cells) + " |")
    return "\n".join(lines)


def main() -> None:
    """Run the benchmark matrix, or one worker measurement."""
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--worker", nargs=4, metavar=("LIBRARY", "OUTPUT", "POINTS", "REPEATS"))
    parser.add_argument("--libraries", nargs="+", default=["ggrust", "lets-plot"])
    parser.add_argument("--outputs", nargs="+", default=["svg", "png"])
    parser.add_argument("--sizes", nargs="+", type=int, default=[10_000, 100_000, 1_000_000])
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=600)
    arguments = parser.parse_args()
    if arguments.worker:
        library, output, size, repeats = arguments.worker
        print(json.dumps(worker(library, output, int(size), int(repeats))))
        return
    results = []
    for output in arguments.outputs:
        for size in arguments.sizes:
            for library in arguments.libraries:
                # Large lets-plot runs take minutes; one warm repeat is enough there.
                repeats = arguments.repeats if size <= 100_000 or library == "ggrust" else 1
                result = run_in_subprocess(library, output, size, repeats, arguments.timeout)
                results.append(result)
                print(markdown([result]).splitlines()[-1], flush=True)
    report = {
        "date": datetime.now(UTC).isoformat(timespec="seconds"),
        "machine": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "python": platform.python_version(),
        },
        "results": results,
    }
    stamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%S")
    RESULTS.mkdir(exist_ok=True)
    (RESULTS / f"scatter-{stamp}.json").write_text(json.dumps(report, indent=2))
    (RESULTS / f"scatter-{stamp}.md").write_text(markdown(results) + "\n")
    print(f"\nwrote {RESULTS}/scatter-{stamp}.json and .md")


if __name__ == "__main__":
    main()
