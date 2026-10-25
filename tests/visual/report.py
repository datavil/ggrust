"""Side-by-side report: lets-plot's own rendering next to ggrust's, for every case.

uv run python tests/visual/report.py   # writes tests/visual/output/report.html
"""

import sys
import time
import warnings
from html import escape
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
warnings.filterwarnings("ignore")

import ggrust  # noqa: E402
import lets_plot  # noqa: E402
from cases import CASES  # noqa: E402

OUTPUT = Path(__file__).parent / "output"


def timed_svg(plot) -> tuple[str, float]:
    start = time.perf_counter()
    svg = plot.to_svg()
    return svg, time.perf_counter() - start


def main() -> None:
    rows = []
    for name, case in CASES.items():
        theirs, their_seconds = timed_svg(case(lets_plot))
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            ours, our_seconds = timed_svg(case(ggrust))
        notes = "".join(f"<li>{escape(str(warning.message))}</li>" for warning in caught)
        rows.append(
            f"<h2>{escape(name)}</h2><div class='pair'>"
            f"<figure>{theirs}<figcaption>lets-plot ({their_seconds * 1000:.0f} ms)"
            "</figcaption></figure>"
            f"<figure>{ours}<figcaption>ggrust ({our_seconds * 1000:.0f} ms)</figcaption></figure>"
            f"</div><ul>{notes}</ul>"
        )
    page = (
        "<!DOCTYPE html><html><head><meta charset='utf-8'><title>ggrust vs lets-plot</title>"
        "<style>body{font-family:sans-serif;margin:16px}.pair{display:flex;gap:16px;flex-wrap:wrap}"
        "figure{margin:0;border:1px solid #ddd}figcaption{padding:4px;color:#555}</style></head>"
        f"<body><h1>ggrust vs lets-plot</h1>{''.join(rows)}</body></html>"
    )
    OUTPUT.mkdir(exist_ok=True)
    path = OUTPUT / "report.html"
    path.write_text(page, encoding="utf-8")
    print(f"wrote {path}")


if __name__ == "__main__":
    main()
