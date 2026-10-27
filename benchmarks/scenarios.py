"""Benchmark ggrust against lets-plot on common plot scenarios, plus side-by-side images.

Same fairness rules as ``run.py`` (identical data, ``sampling="none"`` for lets-plot, every
measurement in a fresh process). Each scenario is built once per library from the same
polars frame. Measured per scenario: spec build, SVG, PNG (scale 2), peak memory above data.

Usage::

    uv run python benchmarks/scenarios.py                  # default sizes
    uv run python benchmarks/scenarios.py --sizes 10000 100000
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


def make_frame(size: int):
    """Everyday-style data: two numeric axes, a 6-level category, a continuous value."""
    import numpy as np
    import polars as pl

    rng = np.random.default_rng(0)
    group = rng.integers(0, 6, size)
    x = rng.normal(size=size) + group
    y = rng.normal(size=size) * (1 + group / 3)
    return pl.DataFrame(
        {
            "x": x,
            "y": y,
            "group": np.char.add("g", group.astype(str)),
            "value": rng.uniform(0, 100, size),
            "positive": 10.0 ** rng.uniform(0, 4, size),
        }
    )


def scenarios(m, frame, sample: dict) -> dict:
    """Scenario name -> zero-argument plot builder. ``sample`` holds library-specific kwargs."""
    point = lambda **kw: m.geom_point(**kw, **sample)  # noqa: E731
    return {
        "scatter": lambda: m.ggplot(frame, m.aes("x", "y")) + point(size=1),
        "color_discrete": lambda: m.ggplot(frame, m.aes("x", "y", color="group")) + point(size=1),
        "color_continuous": lambda: m.ggplot(frame, m.aes("x", "y", color="value")) + point(size=1),
        "size_alpha": lambda: (
            m.ggplot(frame, m.aes("x", "y", size="value")) + point(alpha=0.3, color="#377eb8")
        ),
        "log_scales": lambda: (
            m.ggplot(frame, m.aes("positive", "value"))
            + point(size=1)
            + m.scale_x_log10()
            + m.scale_y_log10()
        ),
        "titled_themed": lambda: (
            m.ggplot(frame, m.aes("x", "y", color="group"))
            + point(size=1.5, alpha=0.6)
            + m.ggtitle("Groups", subtitle="shifted normals")
            + m.labs(x="X axis", y="Y axis")
            + m.theme(legend_position="bottom")
        ),
    }


def load(library: str):
    """Import the library module and its per-layer sampling kwargs."""
    if library == "ggrust":
        import ggrust as module

        return module, {}
    warnings.filterwarnings("ignore")
    import lets_plot as module

    return module, {"sampling": "none"}


def worker(library: str, scenario: str, size: int, repeats: int) -> dict:
    """Measure one (library, scenario, size) in this process."""
    frame = make_frame(size)
    module, sample = load(library)
    build = scenarios(module, frame, sample)[scenario]
    baseline_kb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    result = {"library": library, "scenario": scenario, "points": size}
    for output in ("build", "svg", "png"):
        times = []
        for _ in range(repeats + 1):
            start = time.perf_counter()
            plot = build()
            if output == "svg":
                size_bytes = len(plot.to_svg())
            elif output == "png":
                buffer = io.BytesIO()
                plot.to_png(buffer, scale=2)
                size_bytes = len(buffer.getvalue())
            else:
                size_bytes = 0
            times.append(time.perf_counter() - start)
        result[f"{output}_cold_seconds"] = times[0]
        result[f"{output}_warm_seconds"] = statistics.median(times[1:])
        result[f"{output}_bytes"] = size_bytes
        result[f"{output}_peak_mb"] = (
            resource.getrusage(resource.RUSAGE_SELF).ru_maxrss - baseline_kb
        ) / (1024 * 1024)  # macOS reports bytes
    return result


def run_in_subprocess(library, scenario, size, repeats, timeout) -> dict:
    """Measure one configuration in a fresh process."""
    command = [sys.executable, __file__, "--worker", library, scenario, str(size), str(repeats)]
    base = {"library": library, "scenario": scenario, "points": size}
    try:
        done = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return base | {"timeout_seconds": timeout}
    if done.returncode != 0:
        return base | {"error": done.stderr[-500:]}
    return json.loads(done.stdout.strip().splitlines()[-1])


def render_images(size: int, directory: Path) -> None:
    """Write each scenario as PNG for both libraries, for the side-by-side figure."""
    frame = make_frame(size)
    directory.mkdir(parents=True, exist_ok=True)
    for library in ("ggrust", "lets-plot"):
        module, sample = load(library)
        for name, build in scenarios(module, frame, sample).items():
            build().to_png(str(directory / f"{name}-{library}.png"), scale=2)


def table(results: list[dict]) -> str:
    """Markdown table, one row per scenario and size, libraries side by side."""
    lines = [
        "| scenario | points | stage | ggrust (s) | lets-plot (s) | speed-up "
        "| ggrust MB | lets-plot MB | memory ratio |",
        "|---|---:|---|---:|---:|---:|---:|---:|---:|",
    ]
    keys = sorted({(r["scenario"], r["points"]) for r in results}, key=lambda k: (k[1], k[0]))
    for scenario, points in keys:
        mine = next(
            (
                r
                for r in results
                if (r["scenario"], r["points"], r["library"]) == (scenario, points, "ggrust")
            ),
            {},
        )
        theirs = next(
            (
                r
                for r in results
                if (r["scenario"], r["points"], r["library"]) == (scenario, points, "lets-plot")
            ),
            {},
        )
        for stage in ("build", "svg", "png"):
            a = mine.get(f"{stage}_warm_seconds")
            b = theirs.get(f"{stage}_warm_seconds")
            ma = mine.get(f"{stage}_peak_mb")
            mb = theirs.get(f"{stage}_peak_mb")
            if a is None or b is None:
                continue
            lines.append(
                f"| {scenario} | {points:,} | {stage} | {a:.4f} | {b:.4f} | {b / a:.0f}x "
                f"| {ma:.0f} | {mb:.0f} | {mb / max(ma, 1):.1f}x |"
            )
    return "\n".join(lines)


def main() -> None:
    """Run the matrix, or one worker measurement."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", nargs=4, metavar=("LIBRARY", "SCENARIO", "POINTS", "REPEATS"))
    parser.add_argument("--sizes", nargs="+", type=int, default=[10_000, 100_000])
    parser.add_argument("--repeats", type=int, default=2)
    parser.add_argument("--timeout", type=float, default=900)
    parser.add_argument("--images", type=int, default=2_000, help="points in side-by-side images")
    arguments = parser.parse_args()
    if arguments.worker:
        library, scenario, size, repeats = arguments.worker
        print(json.dumps(worker(library, scenario, int(size), int(repeats))))
        return
    names = [
        "scatter",
        "color_discrete",
        "color_continuous",
        "size_alpha",
        "log_scales",
        "titled_themed",
    ]
    results = []
    for size in arguments.sizes:
        for scenario in names:
            for library in ("ggrust", "lets-plot"):
                result = run_in_subprocess(
                    library, scenario, size, arguments.repeats, arguments.timeout
                )
                results.append(result)
                print(result.get("error") or f"{library} {scenario} {size:,} ok", flush=True)
    stamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%S")
    RESULTS.mkdir(exist_ok=True)
    report = {
        "date": datetime.now(UTC).isoformat(timespec="seconds"),
        "machine": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "python": platform.python_version(),
        },
        "results": results,
    }
    (RESULTS / f"scenarios-{stamp}.json").write_text(json.dumps(report, indent=2))
    (RESULTS / f"scenarios-{stamp}.md").write_text(table(results) + "\n")
    render_images(arguments.images, RESULTS / f"scenarios-{stamp}-images")
    print(table(results))
    print(f"\nstamp {stamp}")


if __name__ == "__main__":
    main()
