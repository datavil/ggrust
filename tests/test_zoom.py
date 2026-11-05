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
import numpy as np
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


def test_layer_toggles_are_on_by_default():
    """``ggtb()`` lists each plot's layers; only ``layers=False`` removes the list."""
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point() + gg.geom_text(gg.aes(label="g"))
    assert zoom_payload(plot + gg.ggtb())["plots"][0]["layers"] == ["point", "text"]
    assert "layers" not in (plot + gg.ggtb()).as_dict()["ggtoolbar"]
    assert "layers" not in zoom_payload(plot + gg.ggtb(layers=False))["plots"][0]
    layered = plot + gg.geom_point(color="red") + gg.ggtb(layers=True)
    assert zoom_payload(layered)["plots"][0]["layers"] == ["point", "text", "point 2"]
    named = plot + gg.geom_point() + gg.ggtb(layers={"spots": 2, "names": 1})
    assert zoom_payload(named)["plots"][0]["layers"] == ["point", "names", "spots"]
    assert named.as_dict()["ggtoolbar"]["layers"] == {"spots": 2, "names": 1}
    relabelled = plot + gg.geom_point() + gg.ggtb(layers={"spots": 0})
    assert zoom_payload(relabelled)["plots"][0]["layers"] == ["spots", "text", "point"]
    # One group per layer, in layer order, in zoomable HTML only.
    svg = svg_of(layered._repr_html_())
    assert svg.count('<g class="layer">') == 3
    assert '<g class="layer">' not in layered.to_svg()


def test_layer_toggles_in_a_grid_list_each_plot():
    image = np.zeros((2, 3, 3), dtype=np.uint8)
    spatial = (
        gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_imshow(image_data=image) + gg.geom_point()
    )
    scatter = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point()
    grid = gg.gggrid([spatial, scatter], ncol=2) + gg.ggtb(layers={"tissue": 0})
    assert [plot["layers"] for plot in zoom_payload(grid)["plots"]] == [
        ["tissue", "point"],
        ["tissue"],
    ]


def test_boxplot_outliers_share_their_layer_toggle():
    """``geom_boxplot`` adds an outlier point layer; it is part of the boxplot's toggle."""
    frame = pl.DataFrame({"g": ["a"] * 5 + ["b"] * 5, "y": [1, 2, 3, 4, 40, 2, 3, 4, 5, -30]})
    plot = (
        gg.ggplot(frame, gg.aes("g", "y"))
        + gg.geom_boxplot()
        + gg.geom_point(color="red")
        + gg.ggtb(layers={"trend": 1})
    )
    assert len(plot.as_dict()["layers"]) == 3
    assert zoom_payload(plot)["plots"][0]["layers"] == ["boxplot", "trend"]
    assert svg_of(plot._repr_html_()).count('<g class="layer">') == 2
    found = re.search(r'class="ggrust-tooltips">(.*?)</script>', plot._repr_html_())
    assert {layer["layer"] for layer in json.loads(found.group(1))["layers"]} <= {0, 1}
    with pytest.raises(gg.errors.SpecError, match=r"index 2 does not exist"):
        (
            gg.ggplot(frame, gg.aes("g", "y")) + gg.geom_boxplot() + gg.ggtb(layers={"x": 2})
        ).to_html()


def test_tooltip_targets_know_their_layer():
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_line() + gg.geom_point() + gg.ggtb(layers=True)
    )
    found = re.search(r'class="ggrust-tooltips">(.*?)</script>', plot._repr_html_())
    assert [layer["layer"] for layer in json.loads(found.group(1))["layers"]] == [0, 1]


def test_layer_options_are_checked():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point()
    for bad, message in [
        ("yes", r"str is not accepted; use True, False, or a dict"),
        ({"spots": -1}, r"'spots' maps to -1, not a layer index"),
        ({"spots": True}, r"'spots' maps to True, not a layer index"),
        ({1: 0}, r"label 1 is not a string"),
        ({"a": 0, "b": 0}, r"layer 0 is labelled both 'a' and 'b'"),
    ]:
        with pytest.raises(gg.errors.SpecError, match=message):
            gg.ggtb(layers=bad)
    with pytest.raises(
        gg.errors.SpecError, match=r"ggtoolbar.layers.spots.*index 3 does not exist"
    ):
        (plot + gg.ggtb(layers={"spots": 3})).to_html()


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


def test_fixed_aspect_is_in_the_payload():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point() + gg.ggtb()
    assert zoom_payload(plot)["plots"][0]["fixed_aspect"] is False
    assert zoom_payload(plot + gg.coord_fixed())["plots"][0]["fixed_aspect"] is True


def run_selections(drags: list[dict]) -> list[dict]:
    """The runtime's box zoom selection for each drag, via node."""
    script = (
        "global.window = {}; global.document = {};"
        f"eval(require('fs').readFileSync({json.dumps(str(RUNTIME))}, 'utf8') + '(\"test\")');"
        f"const drags = {json.dumps(drags)};"
        "const result = drags.map(d => window.ggrust.selection("
        "{bounds: d.bounds, from: d.from, mode: null, fixed: d.fixed}, d.to, false));"
        "console.log(JSON.stringify(result));"
    )
    output = subprocess.run(["node", "-e", script], capture_output=True, text=True, check=True)
    return json.loads(output.stdout)


@pytest.mark.skipif(shutil.which("node") is None, reason="needs node")
def test_fixed_aspect_box_zoom_keeps_the_panel_shape():
    # A 400 x 200 panel at (100, 50); a tall narrow drag and a near-vertical one.
    panel = [100, 50, 400, 200]
    tall = {"bounds": panel, "from": [300, 60], "to": [340, 240]}
    upright = {"bounds": panel, "from": [120, 60], "to": [124, 200]}
    free, fixed, free_band, fixed_band = run_selections(
        [
            {**tall, "fixed": False},
            {**tall, "fixed": True},
            {**upright, "fixed": False},
            {**upright, "fixed": True},
        ]
    )
    assert free["rect"] == [300, 60, 340, 240]
    # Widened about its centre to 2:1, then kept inside the panel.
    left, top, right, bottom = fixed["rect"]
    assert (right - left) / (bottom - top) == pytest.approx(2.0)
    assert (bottom - top) == pytest.approx(180) and (left + right) / 2 == pytest.approx(320)
    assert free_band["only"] == "vertical"
    # No band on a fixed aspect panel: a box, shifted right to stay inside.
    left, top, right, bottom = fixed_band["rect"]
    assert "only" not in fixed_band and left == pytest.approx(100)
    assert (right - left) / (bottom - top) == pytest.approx(2.0)


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
