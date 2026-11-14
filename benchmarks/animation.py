"""Benchmark ggrust animations against R gganimate + gifski on identical work.

Fairness rules:

- Identical data: one CSV per (scenario, size), written once and read by both sides
  before timing starts (reading is not timed).
- Same work: the same plot, transition and options on both sides: 100 frames at
  10 fps, 480 x 480 px GIF (R at ``res = 72``, ggrust at its default 96 dpi, both one
  pixel per pixel), the GIF written to a file. The written GIFs are checked for frame
  count and pixel size, so neither side does less.
- Every (library, scenario, size) runs in a fresh process wrapped in ``/usr/bin/time -l``.
  Time = cold (first render) and warm (median of the following renders).
  Memory = peak RSS of the process minus its RSS after imports and data load.

``rows`` counts every data row: ``states`` has ``size`` points in each of 3 states,
``time`` has ``size`` points spread over 50 time steps, ``reveal`` is 6 lines with
``size`` points in total.

Usage::

    uv run python benchmarks/animation.py                       # 1k and 10k
    uv run python benchmarks/animation.py --sizes 1000 --scenarios states
"""

from __future__ import annotations

import argparse
import json
import platform
import re
import resource
import statistics
import subprocess
import sys
import tempfile
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal

HERE = Path(__file__).parent
RESULTS = HERE / "results"
SCENARIOS = ("states", "states_fade", "time", "reveal")
LIBRARIES = ("ggrust", "gganimate")
Library = Literal["ggrust", "gganimate"]


def write_data(scenario: str, size: int, path: Path) -> int:
    """Write the scenario's data as CSV and return its row count."""
    import numpy as np
    import polars as pl

    generator = np.random.default_rng(7)
    groups = np.array(list("abcdef"))
    if scenario in ("states", "states_fade"):
        ids = np.arange(size)
        parts = []
        for state in ("one", "two", "three"):
            keep = ids if scenario == "states" else ids[generator.random(size) < 0.8]
            parts.append(
                pl.DataFrame(
                    {
                        "id": keep,
                        "x": generator.normal(size=keep.size),
                        "y": generator.normal(size=keep.size),
                        "group": groups[keep % 6],
                        "state": [state] * keep.size,
                    }
                )
            )
        frame = pl.concat(parts)
    elif scenario == "time":
        ids = np.arange(size) % max(size // 50, 1)
        frame = pl.DataFrame(
            {
                "id": ids,
                "time": np.arange(size) // max(size // 50, 1),
                "x": generator.normal(size=size),
                "y": generator.normal(size=size),
                "group": groups[ids % 6],
            }
        )
    else:
        per_line = size // 6
        frame = pl.DataFrame(
            {
                "time": np.tile(np.arange(per_line), 6),
                "y": np.cumsum(generator.normal(size=per_line * 6)),
                "group": np.repeat(groups, per_line),
            }
        )
    frame.write_csv(path)
    return frame.height


def run_ggrust(scenario: str, csv: Path, out: Path, repeats: int) -> None:
    """Child process: render the ggrust animation ``repeats`` times, print JSON."""
    import polars as pl
    from ggrust import (
        aes,
        anim_save,
        animate,
        enter_fade,
        exit_fade,
        geom_line,
        geom_point,
        ggplot,
        transition_reveal,
        transition_states,
        transition_time,
    )

    frame = pl.read_csv(csv)
    if scenario in ("states", "states_fade"):
        plot = (
            ggplot(frame, aes("x", "y", color="group", group="id"))
            + geom_point(size=1.5)
            + transition_states("state", transition_length=1, state_length=1)
        )
        if scenario == "states_fade":
            plot = plot + enter_fade() + exit_fade()
    elif scenario == "time":
        plot = (
            ggplot(frame, aes("x", "y", color="group", group="id"))
            + geom_point(size=1.5)
            + transition_time("time")
        )
    else:
        plot = (
            ggplot(frame, aes("time", "y", color="group")) + geom_line() + transition_reveal("time")
        )
    baseline_kb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024
    times = []
    for _ in range(repeats):
        start = time.perf_counter()
        anim_save(str(out), animate(plot, nframes=100, fps=10, width=480, height=480))
        times.append(time.perf_counter() - start)
    print(json.dumps({"baseline_kb": baseline_kb, "times": times}))


def gif_shape(path: Path) -> tuple[int, int, int]:
    """Frame count, width and height of a GIF."""
    from PIL import Image

    with Image.open(path) as image:
        return image.n_frames, image.width, image.height


def measure(
    library: Library, scenario: str, csv: Path, out: Path, repeats: int
) -> dict[str, object]:
    """Run one measurement in a fresh process under ``/usr/bin/time -l``."""
    if library == "ggrust":
        command = [sys.executable, __file__, "--child", scenario, str(csv), str(out), str(repeats)]
    else:
        command = ["Rscript", str(HERE / "animation.R"), scenario, str(csv), str(out), str(repeats)]
    result = subprocess.run(
        ["/usr/bin/time", "-l", *command], capture_output=True, text=True, check=True
    )
    child = json.loads(result.stdout.strip().splitlines()[-1])
    peak = re.search(r"(\d+)\s+maximum resident set size", result.stderr)
    peak_mb = int(peak.group(1)) / 1024 / 1024 if peak else float("nan")
    times = child["times"]
    frames, width, height = gif_shape(out)
    return {
        "cold_s": times[0],
        "warm_s": statistics.median(times[1:]) if len(times) > 1 else None,
        "memory_mb": peak_mb - child["baseline_kb"] / 1024,
        "gif_kb": out.stat().st_size / 1024,
        "frames": frames,
        "pixels": f"{width}x{height}",
    }


def main() -> None:
    """Run every (scenario, size, library) measurement and write the results."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sizes", type=int, nargs="+", default=[1000, 10000])
    parser.add_argument("--scenarios", nargs="+", default=list(SCENARIOS), choices=SCENARIOS)
    parser.add_argument("--repeats", type=int, default=3)
    arguments = parser.parse_args()

    stamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%S")
    work = Path(tempfile.mkdtemp(prefix="ggrust-animation-"))
    rows = []
    for scenario in arguments.scenarios:
        for size in arguments.sizes:
            csv = work / f"{scenario}-{size}.csv"
            row_count = write_data(scenario, size, csv)
            for library in LIBRARIES:
                out = work / f"{library}-{scenario}-{size}.gif"
                result = measure(library, scenario, csv, out, arguments.repeats)
                result |= {
                    "library": library,
                    "scenario": scenario,
                    "size": size,
                    "rows": row_count,
                }
                rows.append(result)
                print(json.dumps(result), flush=True)
            csv.unlink()

    lines = [
        f"# Animation benchmark {stamp}",
        "",
        f"{platform.platform()}, {platform.processor()}, Python {platform.python_version()}. "
        "100 frames, 10 fps, 480 x 480 GIF. Time = warm median (cold in brackets).",
        "",
        "| scenario | rows | ggrust | gganimate | speed-up | ggrust memory "
        "| gganimate memory | frames | pixels |",
        "|---|---:|---:|---:|---:|---:|---:|---|---|",
    ]
    for scenario in arguments.scenarios:
        for size in arguments.sizes:
            ours, theirs = (
                next(
                    r
                    for r in rows
                    if r["library"] == lib and r["scenario"] == scenario and r["size"] == size
                )
                for lib in LIBRARIES
            )
            ours_s = ours["warm_s"] or ours["cold_s"]
            theirs_s = theirs["warm_s"] or theirs["cold_s"]
            lines.append(
                f"| {scenario} | {ours['rows']:,} | {ours_s:.2f} s ({ours['cold_s']:.2f}) "
                f"| {theirs_s:.2f} s ({theirs['cold_s']:.2f}) | {theirs_s / ours_s:.0f}x "
                f"| {ours['memory_mb']:.0f} MB | {theirs['memory_mb']:.0f} MB "
                f"| {ours['frames']} / {theirs['frames']} | {ours['pixels']} / {theirs['pixels']} |"
            )
    (RESULTS / f"animation-{stamp}.json").write_text(json.dumps(rows, indent=2))
    (RESULTS / f"animation-{stamp}.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--child":
        run_ggrust(sys.argv[2], Path(sys.argv[3]), Path(sys.argv[4]), int(sys.argv[5]))
    else:
        main()
