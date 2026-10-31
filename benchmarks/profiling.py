r"""Profile ggrust renders: stage times, memory and flamegraphs of the Rust engine.

For every (case, output) pair, in a fresh process each:

- **stages**: wall time of one warm render split into ``as_dict`` (Python spec), ``to_json``,
  ``export`` (frames to Arrow), ``engine`` (the Rust call) and the remaining glue (Python plus
  two small Rust spec scans). ``rss_mb`` is how much the renders raise the process's peak
  resident set over the peak reached while building the plot (a lower bound when the build
  peaked higher); ``python_mb`` is the ``tracemalloc`` peak of one render (Python only).
- **flamegraph**: the case renders in a loop while macOS ``sample`` records its stacks;
  sleeping threads and polars' thread pool are left out. ``inferno`` folds them
  (``llvm-cxxfilt`` demangles) into ``<case>-<output>.svg`` and the top frames are listed.
  ``cargo flamegraph`` and ``py-spy`` need root on macOS (dtrace / task_for_pid); ``sample``
  does not. Build the extension with line tables so frames are named::

      CARGO_PROFILE_RELEASE_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_STRIP=none \
          uv run maturin build --release -o <dir>

Cases are the cellestial docstring examples of ``cellestial_examples.py`` (needs a python
whose cellestial imports ggrust) and ``scenarios.py`` scatter variants (``<scenario>@<points>``).

Usage::

    python benchmarks/profiling.py --python <port venv python> --outputs svg png html \
        --cases violin scatter@1000000
"""

import argparse
import json
import re
import resource
import subprocess
import sys
import time
import tracemalloc
import warnings
from collections import Counter
from datetime import UTC, datetime
from pathlib import Path

RESULTS = Path(__file__).parent / "results"
OUTPUTS = {"svg": "svg", "png": "png", "html": "html_page"}
STAGES = ("as_dict", "to_json", "export", "engine")
# Leaf frames of threads that sleep; their samples are not CPU time. ``swtch_pri`` (rayon
# workers yielding while they look for work) burns CPU and stays in.
BLOCKING = {
    "__psynch_cvwait",
    "__workq_kernreturn",
    "__semwait_signal",
    "mach_msg2_trap",
    "kevent",
    "kevent64",
    "__select",
    "__ulock_wait2",
}


def build_plot(case: str):
    """Zero-argument builder for ``case`` (cellestial example or ``scenario@points``)."""
    import ggrust

    sys.path.insert(0, str(Path(__file__).parent))
    if "@" in case:
        import scenarios

        name, points = case.split("@")
        return scenarios.scenarios(ggrust, scenarios.make_frame(int(points)), {})[name]
    import cellestial as cl
    import cellestial.datasets
    import cellestial_examples

    data = cl.datasets.pbmc3k()
    data.raw = None
    return cellestial_examples.cases(cl, ggrust, data)[case]


def render_once(plot, output: str) -> int:
    """Render ``plot`` once (PNG at scale 2, as the benchmarks) and return the size in bytes."""
    from ggrust._render import render_bytes

    return len(render_bytes(plot, OUTPUTS[output], scale=2.0 if output == "png" else 1.0))


def timed(timers: dict, name: str, function):
    """Wrap ``function`` so its wall time accumulates in ``timers[name]``."""

    def wrapper(*arguments, **keywords):
        start = time.perf_counter()
        try:
            return function(*arguments, **keywords)
        finally:
            timers[name] = timers.get(name, 0.0) + time.perf_counter() - start

    return wrapper


def stages_worker(case: str, output: str, repeats: int) -> dict:
    """Median stage times of ``repeats`` warm renders, plus memory, in this process."""
    from ggrust import _render

    warnings.filterwarnings("ignore")
    plot = build_plot(case)()
    baseline = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    render_once(plot, output)  # warm: font loading, lazy statics
    timers: dict[str, float] = {}
    _render.to_json = timed(timers, "to_json", _render.to_json)
    _render._export_with_tooltips = timed(timers, "export", _render._export_with_tooltips)
    _render.render = timed(timers, "engine", _render.render)
    plot.as_dict = timed(timers, "as_dict", plot.as_dict)
    runs = []
    for _ in range(repeats):
        timers.clear()
        start = time.perf_counter()
        size_bytes = render_once(plot, output)
        total = time.perf_counter() - start
        run = {stage: timers.get(stage, 0.0) for stage in STAGES}
        run["glue"] = total - sum(run.values())
        run["total"] = total
        runs.append(run)
    runs.sort(key=lambda run: run["total"])
    result = {"case": case, "output": output, "bytes": size_bytes, **runs[len(runs) // 2]}
    unit = 1 if sys.platform == "darwin" else 1024
    rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    result["rss_mb"] = (rss - baseline) * unit / 1024**2
    tracemalloc.start()
    render_once(plot, output)
    result["python_mb"] = tracemalloc.get_traced_memory()[1] / 1024**2
    tracemalloc.stop()
    return result


def loop_worker(case: str, output: str, seconds: float) -> None:
    """Render in a loop for ``seconds`` after printing ``ready`` (the parent samples us)."""
    warnings.filterwarnings("ignore")
    plot = build_plot(case)()
    render_once(plot, output)
    print("ready", flush=True)
    end = time.perf_counter() + seconds
    while time.perf_counter() < end:
        render_once(plot, output)


def demangle(frames: set[str]) -> dict[str, str]:
    """Mangled frame -> readable name without ``;`` (the folded-stack separator).

    Rust symbols are v0-mangled; Xcode's ``llvm-cxxfilt`` demangles them (rustfilt does not).
    """
    ordered = sorted(frames)
    demangler = subprocess.run(
        ["xcrun", "--find", "llvm-cxxfilt"], capture_output=True, text=True, check=True
    ).stdout.strip()
    done = subprocess.run(
        [demangler], input="\n".join(ordered), capture_output=True, text=True, check=True
    )
    readable = [name.replace(";", ",") for name in done.stdout.splitlines()]
    return dict(zip(ordered, readable, strict=True))


def is_blocked(stack: str) -> bool:
    """Whether a stack ends in a blocking wait (``sample`` records idle threads too)."""
    return stack.rpartition(";")[2] in BLOCKING


def is_foreign(stack: str) -> bool:
    """Whether a stack runs on another library's thread pool (polars, loaded by cellestial)."""
    return "polars-" in stack.partition(";")[0]


def clean_frame(frame: str) -> str:
    """Drop generic arguments that follow a name; keep ``<Type as Trait>::`` wrappers."""
    while True:
        shorter = re.sub(r"(?<=[\w:])<[^<>]*>", "", frame)
        if shorter == frame:
            return frame.strip().removesuffix("::")
        frame = shorter


def top_frames(folded: Path, count: int) -> dict:
    """Top self-time and inclusive-time frames (share of on-CPU samples) of a folded file."""
    own: Counter = Counter()
    inclusive: Counter = Counter()
    total = 0
    for line in folded.read_text().splitlines():
        stack, _, number = line.rpartition(" ")
        if not stack or not number.isdigit():
            continue
        frames = [clean_frame(frame) for frame in stack.split(";")]
        samples = int(number)
        total += samples
        own[frames[-1]] += samples
        for frame in set(frames):
            inclusive[frame] += samples
    share = lambda counter: [  # noqa: E731
        [name, round(samples / total, 4)] for name, samples in counter.most_common(count)
    ]
    ggrust = Counter({name: n for name, n in inclusive.items() if "ggrust_core" in name})
    return {"samples": total, "self": share(own), "ggrust_inclusive": share(ggrust)}


def flamegraph(python: str, case: str, output: str, seconds: float, directory: Path) -> dict:
    """Sample a render loop of ``case`` and write its folded stacks and flamegraph SVG."""
    seconds = max(1, round(seconds))
    command = [python, __file__, "--loop", case, output, str(seconds + 3)]
    name = f"{case.replace('@', '-')}-{output}"
    raw = directory / f"{name}.sample.txt"
    with subprocess.Popen(command, stdout=subprocess.PIPE, text=True) as process:
        try:
            assert process.stdout is not None
            if process.stdout.readline().strip() != "ready":  # data built, first render done
                raise RuntimeError(f"the render loop of {case} {output} exited before sampling")
            sample = ["/usr/bin/sample", str(process.pid), str(seconds), "1", "-mayDie"]
            subprocess.run([*sample, "-file", raw], capture_output=True, check=True)
        finally:
            process.kill()
    folded = directory / f"{name}.folded"
    collapsed = subprocess.run(
        ["inferno-collapse-sample", "--no-modules", raw], capture_output=True, text=True, check=True
    ).stdout.splitlines()
    on_cpu = []
    for line in collapsed:
        stack = line.rpartition(" ")[0]
        if not (is_blocked(stack) or is_foreign(stack)):
            on_cpu.append(line)
    names = demangle({frame for line in on_cpu for frame in line.rpartition(" ")[0].split(";")})
    lines = []
    for line in on_cpu:
        stack, _, samples = line.rpartition(" ")
        lines.append(";".join(names[frame] for frame in stack.split(";")) + " " + samples)
    folded.write_text("\n".join(lines) + "\n")
    with (directory / f"{name}.svg").open("w") as handle:
        title = f"ggrust {case} {output}"
        subprocess.run(["inferno-flamegraph", "--title", title, folded], stdout=handle, check=True)
    raw.unlink()
    return top_frames(folded, 12)


def run_json(command: list[str]) -> dict:
    """Run a worker and parse the JSON on its last stdout line."""
    done = subprocess.run(command, capture_output=True, text=True)
    if done.returncode != 0:
        return {"error": done.stderr[-500:]}
    return json.loads(done.stdout.strip().splitlines()[-1])


def table(results: list[dict]) -> str:
    """Markdown table of the stage times (milliseconds) and memory."""
    lines = [
        "| case | output | total ms | as_dict | to_json | export | engine | glue "
        "| rss MB | python MB | top self frames |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for result in results:
        if "error" in result:
            lines.append(f"| {result['case']} | {result['output']} | error |" + " |" * 8)
            continue
        cells = [result["case"], result["output"], f"{result['total'] * 1e3:.1f}"]
        cells += [f"{result[stage] * 1e3:.1f}" for stage in (*STAGES, "glue")]
        cells += [f"{result['rss_mb']:.0f}", f"{result['python_mb']:.1f}"]
        frames = result.get("profile", {}).get("self", [])[:3]
        cells.append("; ".join(f"`{name[-60:]}` {share:.0%}" for name, share in frames))
        lines.append("| " + " | ".join(cells) + " |")
    return "\n".join(lines)


def main() -> None:
    """Profile every requested case and output; write JSON, a table and flamegraphs."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--python", default=sys.executable, help="python that imports ggrust")
    parser.add_argument("--cases", nargs="+", required=False)
    parser.add_argument("--outputs", nargs="+", default=["svg", "png", "html"])
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--seconds", type=float, default=4, help="sampling time per flamegraph")
    parser.add_argument("--no-flamegraph", action="store_true")
    parser.add_argument("--stages", nargs=3, metavar=("CASE", "OUTPUT", "REPEATS"))
    parser.add_argument("--loop", nargs=3, metavar=("CASE", "OUTPUT", "SECONDS"))
    arguments = parser.parse_args()
    if arguments.stages:
        case, output, repeats = arguments.stages
        print(json.dumps(stages_worker(case, output, int(repeats))))
        return
    if arguments.loop:
        case, output, seconds = arguments.loop
        loop_worker(case, output, float(seconds))
        return

    stamp = datetime.now(UTC).strftime("%Y%m%d-%H%M%S")
    directory = RESULTS / f"profile-{stamp}"
    directory.mkdir(parents=True)
    results = []
    for case in arguments.cases:
        for output in arguments.outputs:
            command = [arguments.python, __file__, "--stages", case, output]
            result = {"case": case, "output": output}
            result |= run_json([*command, str(arguments.repeats)])
            if "error" not in result and not arguments.no_flamegraph:
                try:
                    seconds = arguments.seconds
                    result["profile"] = flamegraph(
                        arguments.python, case, output, seconds, directory
                    )
                except (subprocess.CalledProcessError, OSError, RuntimeError) as error:
                    result["profile_error"] = f"{type(error).__name__}: {error}"[:300]
            print(json.dumps({key: value for key, value in result.items() if key != "profile"}))
            results.append(result)
            (directory / "profile.json").write_text(json.dumps(results, indent=2))
    markdown = table(results)
    (directory / "profile.md").write_text(markdown + "\n")
    print(markdown)


if __name__ == "__main__":
    main()
