"""Side band panel theme elements: R ggside's ``ggside.panel.*`` (step 10.1).

``ggside_panel_border`` / ``ggside_panel_background`` style every ggside band, the
``ggside_xside_`` / ``ggside_yside_`` forms only the bands of one axis; the most
specific one set wins and anything unset falls back to the main ``panel_*``
elements. Checked on the SVG: unfilled frames (``<g fill="none" stroke=..>`` with
one ``<rect>``) and filled background rectangles.
"""

from __future__ import annotations

import re
import warnings

import ggrust as gg
import polars as pl
import pytest

GENES = ["CD3E", "MS4A1", "NKG7", "LYZ"]
CLUSTERS = ["T", "B", "NK"]
DOTS = pl.DataFrame(
    {
        "gene": [gene for _ in CLUSTERS for gene in GENES],
        "cluster": [cluster for cluster in CLUSTERS for _ in GENES],
        "pct": [float(10 + index * 7) for index in range(12)],
    }
)
GROUPS = pl.DataFrame({"gene": GENES, "group": ["lymphoid"] * 3 + ["myeloid"]})
SIZES = pl.DataFrame({"cluster": CLUSTERS, "cells": [420.0, 180.0, 95.0]})
FRAME = "#123456"


def _dotplot(*features: gg.FeatureSpec) -> gg.PlotSpec:
    """A dotplot with an x side band (on top) and a y side band (on the right)."""
    plot = (
        gg.ggplot(DOTS, gg.aes("gene", "cluster"))
        + gg.geom_point(gg.aes(size="pct"))
        + gg.geom_xsidetile(gg.aes(x="gene", fill="group"), data=GROUPS)
        + gg.geom_ysidecol(gg.aes(y="cluster", x="cells"), data=SIZES)
        + gg.ggsize(500, 360)
    )
    for feature in features:
        plot = plot + feature
    return plot


def _render(plot: gg.PlotSpec) -> str:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        return plot.to_svg()


def _frames(svg: str) -> list[tuple[str, float, float, float, float]]:
    """Every unfilled single-rectangle frame: (stroke, x, y, width, height)."""
    found = re.findall(
        r'<g fill="none" stroke="([^"]+)" stroke-width="[^"]+">\s*'
        r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"/>\s*</g>',
        svg,
    )
    return [(stroke, *(float(value) for value in rest)) for stroke, *rest in found]


def _by_shape(frames: list[tuple[str, float, float, float, float]]) -> dict[str, list[str]]:
    """Frame strokes by shape: x bands are flat, y bands tall, the main panel neither."""
    shapes: dict[str, list[str]] = {"main": [], "x": [], "y": []}
    for stroke, _x, _y, width, height in frames:
        if width > 3 * height:
            shapes["x"].append(stroke)
        elif height > 3 * width:
            shapes["y"].append(stroke)
        else:
            shapes["main"].append(stroke)
    return shapes


def test_bands_take_the_main_panel_border_by_default() -> None:
    frames = _by_shape(
        _frames(_render(_dotplot(gg.theme(panel_border=gg.element_rect(color=FRAME)))))
    )
    assert frames == {"main": [FRAME], "x": [FRAME], "y": [FRAME]}


def test_blank_ggside_panel_border_keeps_only_the_main_border() -> None:
    svg = _render(
        _dotplot(
            gg.theme(
                panel_border=gg.element_rect(color=FRAME), ggside_panel_border=gg.element_blank()
            )
        )
    )
    assert _by_shape(_frames(svg)) == {"main": [FRAME], "x": [], "y": []}


def test_xside_and_yside_borders_win_over_ggside_and_main() -> None:
    svg = _render(
        _dotplot(
            gg.theme(
                panel_border=gg.element_rect(color=FRAME),
                ggside_panel_border=gg.element_rect(color="#00ff00", size=2),
                ggside_yside_panel_border=gg.element_rect(color="#ff0000"),
                ggside_xside_panel_border=gg.element_blank(),
            )
        )
    )
    assert _by_shape(_frames(svg)) == {"main": [FRAME], "x": [], "y": ["#ff0000"]}


def test_ggside_panel_border_alone_frames_only_the_bands() -> None:
    # The default theme has no panel border; a band border can still be set.
    svg = _render(_dotplot(gg.theme(ggside_panel_border=gg.element_rect(color=FRAME))))
    assert _by_shape(_frames(svg)) == {"main": [], "x": [FRAME], "y": [FRAME]}


def _fills(svg: str, color: str) -> int:
    """How many rectangles are filled with ``color``."""
    return len(re.findall(rf'<g fill="{color}"[^>]*>\s*<rect ', svg))


def test_ggside_panel_background_styles_the_bands_and_falls_back_to_the_main() -> None:
    # No legends: their keys take the panel background's fill too.
    main = gg.theme(
        panel_background=gg.element_rect(fill="#eeeeee", size=0), legend_position="none"
    )
    assert _fills(_render(_dotplot(main)), "#eeeeee") == 3
    blue = gg.theme(ggside_panel_background=gg.element_rect(fill="#0000ff", size=0))
    svg = _render(_dotplot(main, blue))
    assert (_fills(svg, "#eeeeee"), _fills(svg, "#0000ff")) == (1, 2)
    only_x = gg.theme(ggside_yside_panel_background=gg.element_blank())
    svg = _render(_dotplot(main, blue, only_x))
    assert (_fills(svg, "#eeeeee"), _fills(svg, "#0000ff")) == (1, 1)


def test_a_frame_from_the_background_drops_with_size_zero() -> None:
    # cellestial frames panels with panel_background=element_rect(color=, size=);
    # the bands inherit that frame unless their background drops its border.
    framed = gg.theme(panel_background=gg.element_rect(color=FRAME, size=2), legend_position="none")
    plain = _render(_dotplot(framed))
    assert f'stroke="{FRAME}"' in plain
    assert plain.count(f'stroke="{FRAME}"') == 3
    svg = _render(_dotplot(framed, gg.theme(ggside_panel_background=gg.element_rect(size=0))))
    assert svg.count(f'stroke="{FRAME}"') == 1
    # The fill stays, inherited from the main background.
    assert _fills(svg, "#ffffff") == _fills(plain, "#ffffff")


def test_margins_draw_no_band_panel_elements() -> None:
    plot = (
        gg.ggplot(DOTS, gg.aes("pct", "pct"))
        + gg.geom_point()
        + gg.ggmarginal("t", layer=gg.geom_density())
        + gg.theme(
            ggside_panel_border=gg.element_rect(color=FRAME),
            ggside_panel_background=gg.element_rect(fill="#0000ff"),
        )
    )
    svg = _render(plot)
    assert FRAME not in svg and "#0000ff" not in svg


def test_ggside_panel_options_in_the_spec_and_typos_raise() -> None:
    spec = gg.theme(
        ggside_panel_border=gg.element_blank(),
        ggside_xside_panel_background=gg.element_rect(fill="red"),
    ).as_dict()
    assert spec["ggside_panel_border"] == {"blank": True}
    assert spec["ggside_xside_panel_background"]["fill"] == "red"
    # Python 3.13+ appends "Did you mean 'ggside_panel_border'?"; 3.11 and 3.12 do not.
    with pytest.raises(TypeError, match="unexpected keyword argument 'ggside_panel_boder'"):
        gg.theme(ggside_panel_boder=gg.element_blank())  # ty: ignore[unknown-argument]


def test_coord_flip_keeps_xside_on_the_band_sharing_x() -> None:
    # Under coord_flip the x side band shares the vertical screen axis; its
    # ggside_xside_* elements follow it, as the ggside_axis_* ones do.
    svg = _render(
        _dotplot(
            gg.coord_flip(),
            gg.theme(
                ggside_xside_panel_border=gg.element_rect(color="#ff0000"),
                ggside_yside_panel_border=gg.element_rect(color="#0000ff"),
            ),
        )
    )
    frames = _by_shape(_frames(svg))
    # The x band (genes) now stands tall beside the panel, the y band lies flat.
    assert frames["y"] == ["#ff0000"] and frames["x"] == ["#0000ff"]
