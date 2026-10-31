"""Time renders that parallel subplots, panels, layers and groups speed up (step 6.2).

ggrust only (no lets-plot): the point is old build vs new build on the same machine. Each case
is built once, rendered once to warm up, then ``--repeats`` times; the median is reported in
milliseconds. Run it with one build installed, then the other, and compare the JSON files.

Usage::

    uv run python benchmarks/parallel.py --outputs svg html --json before.json
    uv run python benchmarks/parallel.py --cases grid4_50k facet9_300k --repeats 15
"""

import argparse
import io
import json
import statistics
import time
from collections.abc import Callable
from pathlib import Path
from typing import Literal

import ggrust as g
import numpy as np
import polars as pl

Output = Literal["svg", "png", "html"]


def make_frame(size: int, groups: int = 9, seed: int = 0) -> pl.DataFrame:
    """Two normal axes, a uniform value and ``groups`` string levels."""
    generator = np.random.default_rng(seed)
    return pl.DataFrame(
        {
            "x": generator.normal(size=size),
            "y": generator.normal(size=size),
            "value": generator.uniform(size=size),
            "group": generator.integers(0, groups, size=size).astype(str),
        }
    )


def cases() -> dict[str, Callable[[], g.PlotSpec | g.SupPlotsSpec]]:
    """Builders of every case, by name."""
    small = make_frame(3_000)
    medium = make_frame(50_000)
    large = make_frame(300_000)
    many_groups = make_frame(200_000, groups=30)

    def scatter(frame: pl.DataFrame) -> g.PlotSpec:
        return g.ggplot(frame, g.aes("x", "y", color="value")) + g.geom_point(size=1)

    return {
        "small_scatter": lambda: scatter(small),
        "grid4_50k": lambda: g.gggrid([scatter(medium) for _ in range(4)], ncol=2),
        "grid9_3k": lambda: g.gggrid([scatter(small) for _ in range(9)], ncol=3),
        "facet9_300k": lambda: scatter(large) + g.facet_wrap("group"),
        "layers4_50k": lambda: (
            g.ggplot(medium, g.aes("x", "y"))
            + g.geom_point(color="red", size=1)
            + g.geom_point(g.aes(y="value"), size=1)
            + g.geom_line(g.aes(color="group"))
            + g.geom_point(g.aes(x="value"), size=1)
        ),
        "violin30_200k": lambda: g.ggplot(many_groups, g.aes("group", "x")) + g.geom_violin(),
        "boxplot30_200k": lambda: g.ggplot(many_groups, g.aes("group", "x")) + g.geom_boxplot(),
    }


def render(plot: g.PlotSpec | g.SupPlotsSpec, output: Output) -> None:
    """Render ``plot`` to ``output`` in memory."""
    getattr(plot, f"to_{output}")(io.BytesIO())


def main() -> None:
    """Time every selected case and output; print and optionally save the medians."""
    builders = cases()
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--cases", nargs="*", choices=list(builders), default=list(builders))
    parser.add_argument(
        "--outputs", nargs="*", choices=["svg", "png", "html"], default=["svg", "png", "html"]
    )
    parser.add_argument("--repeats", type=int, default=7)
    parser.add_argument("--json", type=Path, help="also write {case/output: ms} here")
    arguments = parser.parse_args()
    results: dict[str, float] = {}
    for name in arguments.cases:
        plot = builders[name]()
        for output in arguments.outputs:
            render(plot, output)
            times = []
            for _ in range(arguments.repeats):
                start = time.perf_counter()
                render(plot, output)
                times.append(time.perf_counter() - start)
            milliseconds = statistics.median(times) * 1000
            results[f"{name}/{output}"] = milliseconds
            print(f"{name:16s} {output:4s} {milliseconds:9.1f} ms", flush=True)
    if arguments.json:
        arguments.json.write_text(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
