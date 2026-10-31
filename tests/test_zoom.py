"""Zoom and pan of HTML output (``ggtb()``, plan.md 4.3).

The engine writes a zoom payload (axis domains, break rules, SVG templates) and
the runtime (``js/ggrust.js``) redraws zoomed axes with breaks it computes itself.
These tests pin the payload and check, with node, that the runtime's breaks and
labels equal the engine's own for the same limits (lets-plot zooms by setting
coord limits, so a zoomed axis must read like ``coord_cartesian(xlim=...)``).
Pointer interactions were checked in a browser.
"""

import json
import math
import re
import shutil
import subprocess
from pathlib import Path

import ggrust as gg
import polars as pl
import pytest

RUNTIME = Path(__file__).resolve().parents[1] / "js" / "ggrust.js"

FRAME = pl.DataFrame(
    {
        "x": [0.5, 3.0, 12.0, 45.0, 160.0, 900.0, 2500.0],
        "y": [-3.0, 1.0, 4.5, 2.0, 8.0, 11.0, 6.5],
        "g": ["a", "b", "c", "a", "b", "c", "a"],
    }
)


def zoom_payload(plot) -> dict | None:
    """The zoom payload of ``plot``'s notebook HTML (``None`` without one)."""
    found = re.search(r'class="ggrust-zoom">(.*?)</script>', plot._repr_html_())
    return json.loads(found.group(1)) if found else None


def svg_of(html: str) -> str:
    """The SVG element of an HTML output (without the runtime script)."""
    return html[html.index("<svg") : html.index("</svg>")]


def axis_labels(svg: str, axis: str) -> list[str]:
    """Label texts of the first ``axis-x`` / ``axis-y`` group of an SVG."""
    group = re.search(rf'<g class="axis-{axis}">(.*?)</g>\n(?!<text)', svg, re.S)
    assert group, f"no axis-{axis} group"
    texts = re.findall(r"<text[^>]*>(.*?)</text>", group.group(1))
    return [re.sub(r"<tspan[^>]*>", "^", text).replace("</tspan>", "") for text in texts]


def test_ggtb_writes_a_zoom_payload_and_room_for_the_toolbar():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point() + gg.ggtb()
    payload = zoom_payload(plot)
    assert payload["size_basis"] == "max" and payload["size_zoomin"] == 0
    (tile,) = payload["plots"][0]["tiles"]
    horizontal = tile["horizontal"]
    assert horizontal["transform"] == "identity" and horizontal["fixed"] is None
    assert horizontal["count"] > 0
    # Pixels of the domain ends lie on the panel edges.
    left, top, width, height = tile["panel"]
    assert horizontal["pixels"] == pytest.approx([left, left + width])
    assert tile["vertical"]["pixels"] == pytest.approx([top + height, top])
    assert ">@<" in horizontal["label"] and ">^<" in horizontal["label_superscript"]
    # lets-plot's toolbar is 33 px above the plot.
    iframe = plot.to_html(iframe=True)
    assert 'height="433"' in iframe
    html = plot._repr_html_()
    assert 'class="ggrust-points"' in html and 'class="tile"' in html


def test_without_ggtb_nothing_changes():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point() + gg.geom_text(gg.aes(label="g"))
    assert zoom_payload(plot) is None
    html = svg_of(plot._repr_html_())
    assert "data-anchor" not in html and "ggrust-points" not in html
    assert 'height="400"' in plot.to_html(iframe=True)
    # Static SVG is the same with or without the toolbar.
    assert (plot + gg.ggtb()).to_svg() == plot.to_svg()


def test_texts_and_markers_are_anchored_in_html():
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_point(gg.aes(shape="g"))
        + gg.geom_label(gg.aes(label="g"))
        + gg.ggtb()
    )
    html = svg_of(plot._repr_html_())
    # Seven labels (box + text), four triangles / squares of the "b" and "c"
    # points, and their two legend keys (point batches too; the runtime only
    # moves what is inside a panel).
    assert html.count("<g data-anchor=") == 7 + 4 + 2


def test_discrete_axes_carry_their_drawn_labels():
    plot = gg.ggplot(FRAME, gg.aes("g", "y")) + gg.geom_boxplot() + gg.ggtb()
    horizontal = zoom_payload(plot)["plots"][0]["tiles"][0]["horizontal"]
    assert [item[0] for item in horizontal["fixed"]] == [0, 1, 2]
    assert all(
        ">a<" in item[2] or ">b<" in item[2] or ">c<" in item[2] for item in horizontal["fixed"]
    )


def test_facets_share_fixed_scales_and_grids_list_every_plot():
    faceted = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point() + gg.facet_wrap("g")
    payload = zoom_payload(faceted + gg.ggtb())
    assert len(payload["plots"][0]["tiles"]) == 3 and payload["plots"][0]["shared"] == [True, True]
    free = zoom_payload(
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.facet_wrap("g", scales="free_y")
        + gg.ggtb()
    )
    assert free["plots"][0]["shared"] == [True, False]
    grid = (
        gg.gggrid([faceted, gg.ggplot(FRAME, gg.aes("x")) + gg.geom_histogram()], ncol=2)
        + gg.ggtb()
    )
    assert [len(plot["tiles"]) for plot in zoom_payload(grid)["plots"]] == [3, 1]
    assert grid._repr_html_().count('class="tile"') == 4


def test_size_options_are_checked():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point()
    assert zoom_payload(plot + gg.ggtb(size_basis="y", size_zoomin=-1))["size_basis"] == "y"
    with pytest.raises(ValueError, match=r"size_zoomin.*-1"):
        gg.ggtb(size_zoomin=-2)
    for bad in [float("inf"), True, "3"]:
        with pytest.raises(ValueError, match=r"size_zoomin.*finite number"):
            gg.ggtb(size_zoomin=bad)
    with pytest.raises(ValueError, match=r"size_basis.*'x', 'y', 'min', 'max'"):
        gg.ggtb(size_basis="z")


def run_breaks(queries: list[tuple[str, float, float, int]]) -> list[dict]:
    """The runtime's breaks for each (transform, low, high, count), via node."""
    script = (
        "global.window = {}; global.document = {};"
        f"eval(require('fs').readFileSync({json.dumps(str(RUNTIME))}, 'utf8') + '(\"test\")');"
        f"const queries = {json.dumps(queries)};"
        "const breaks = queries.map(q => window.ggrust.breaks(q[0], q[1], q[2], q[3]));"
        "console.log(JSON.stringify(breaks));"
    )
    output = subprocess.run(["node", "-e", script], capture_output=True, text=True, check=True)
    return json.loads(output.stdout)


def transformed(scale: str, value: float) -> float:
    functions = {
        "identity": lambda v: v,
        "log10": math.log10,
        "sqrt": math.sqrt,
        "reverse": lambda v: -v,
    }
    return functions[scale](value)


ZOOM_CASES = [
    # (scale, column, data limits of the zoomed view)
    ("identity", "y", (1.2, 7.9)),
    ("identity", "y", (2.001, 2.0035)),
    ("identity", "x", (1000.0, 2500000.0)),
    ("identity", "x", (0.0000012, 0.0000097)),
    ("identity", "y", (-0.37, 0.41)),
    ("log10", "x", (2.0, 5000.0)),
    ("log10", "x", (20.0, 90.0)),
    ("sqrt", "x", (4.0, 400.0)),
    ("reverse", "y", (1.5, 9.5)),
]


@pytest.mark.skipif(shutil.which("node") is None, reason="needs node")
@pytest.mark.parametrize(("scale", "column", "limits"), ZOOM_CASES)
def test_runtime_breaks_equal_the_engine_at_the_same_limits(scale, column, limits):
    # The y axis: its break count depends on the font size only, so a static
    # plot with coord limits asks for as many breaks as the zoomed panel.
    scale_y = {
        "identity": [],
        "log10": [gg.scale_y_log10()],
        "sqrt": [gg.scale_y_continuous(trans="sqrt")],
        "reverse": [gg.scale_y_reverse()],
    }
    plot = gg.ggplot(FRAME, gg.aes("g", column)) + gg.geom_point() + gg.ggtb()
    for layer in scale_y[scale]:
        plot += layer
    vertical = zoom_payload(plot)["plots"][0]["tiles"][0]["vertical"]
    low, high = sorted(transformed(scale, value) for value in limits)
    (breaks,) = run_breaks([(scale, low, high, vertical["count"])])
    runtime = [
        label["text"] + ("^" + label["superscript"] if label["superscript"] else "")
        for label in breaks["labels"]
    ]
    engine = axis_labels((plot + gg.coord_cartesian(ylim=list(limits))).to_svg(), "y")
    assert runtime == engine
