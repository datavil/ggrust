"""More than one side band per axis (step 9.10, not in R ggside).

``side_pos=`` puts a ggside layer's band on either side of its axis, ``side_band=``
stacks named bands on one side (outward in plot order), ``side_size=`` sizes a band,
and ``scale_*side*_*(side_pos=, side_band=)`` sets one band's own axis. Band geometry
is read from the SVG clip rectangles of the ``side-x`` / ``side-y`` groups.
"""

from __future__ import annotations

import re

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError

GENES = ["g1", "g2", "g3", "g4"]
HEATMAP = pl.DataFrame(
    {
        "cell": [cell for cell in ["c1", "c2", "c3"] for _ in GENES],
        "gene": GENES * 3,
        "expr": [float(index % 5) for index in range(12)],
        "module": ["M1", "M1", "M2", "M2"] * 3,
        "family": ["kinase", "receptor", "kinase", "ligand"] * 3,
    }
)
SCORES = pl.DataFrame({"gene": GENES, "score": [0.5, 2.0, 1.0, 3.0]})


def _base() -> gg.PlotSpec:
    return (
        gg.ggplot(HEATMAP, gg.aes("cell", "gene", fill="expr"))
        + gg.geom_tile()
        + gg.ggsize(600, 400)
    )


def _rects(svg: str, role: str) -> list[list[float]]:
    """The clip rectangle (x, y, width, height) of every ``role`` group, in SVG order."""
    clips = {
        found.group(1): [float(value) for value in found.group(2, 3, 4, 5)]
        for found in re.finditer(
            r'<clipPath id="([^"]+)"><rect x="([-\d.]+)" y="([-\d.]+)" '
            r'width="([\d.]+)" height="([\d.]+)"/>',
            svg,
        )
    }
    return [
        clips[found.group(1)]
        for found in re.finditer(rf'<g class="{role}" clip-path="url\(#([^)]+)\)"', svg)
    ]


def _text_x(svg: str, text: str) -> float:
    """The x of the first SVG text ``text`` (a band's track name)."""
    found = re.search(rf'<text[^>]*x="([-\d.]+)"[^>]*>{text}</text>', svg)
    assert found, text
    return float(found.group(1))


def test_band_keys_reach_the_spec() -> None:
    layer = gg.geom_ysidetile(gg.aes(fill="module"), side_pos="left", side_band="a", side_size=0.2)
    props = layer.as_dict()
    assert (props["ggside"], props["side_pos"], props["side_band"], props["side_size"]) == (
        "y",
        "left",
        "a",
        0.2,
    )
    scale = gg.scale_ysidex_discrete(side_pos="right", side_band="a", limits=["module"])
    assert scale.as_dict()["side_pos"] == "right"
    assert scale.as_dict()["side_band"] == "a"
    # Without the keys the spec is unchanged (one-band plots keep their dict).
    assert "side_pos" not in gg.geom_xsidetile(gg.aes(fill="module")).as_dict()
    assert "side_pos" not in gg.scale_xsidey_discrete(limits=["module"]).as_dict()


def test_a_band_on_each_side() -> None:
    plot = (
        _base()
        + gg.new_scale_fill()
        + gg.geom_ysidetile(gg.aes(fill="family"), side_pos="left")
        + gg.new_scale_fill()
        + gg.geom_ysidetile(gg.aes(fill="module"))
        + gg.theme(ggside_panel_spacing=4)
    )
    svg = plot.to_svg()
    (panel,) = _rects(svg, "panel")
    left, right = sorted(_rects(svg, "side-y"))
    assert left[0] + left[2] + 4 == pytest.approx(panel[0], abs=0.01)
    assert right[0] == pytest.approx(panel[0] + panel[2] + 4, abs=0.01)
    assert left[2] == pytest.approx(0.1 * panel[2], abs=0.01)
    assert right[2] == pytest.approx(left[2], abs=0.01)
    # Each band names its own track, under its own band.
    assert left[0] < _text_x(svg, "family") < left[0] + left[2]
    assert right[0] < _text_x(svg, "module") < right[0] + right[2]


def test_side_pos_equals_ggside_for_one_band() -> None:
    by_layer = _base() + gg.geom_ysidetile(gg.aes(fill="module"), side_pos="left")
    by_plot = _base() + gg.geom_ysidetile(gg.aes(fill="module")) + gg.ggside(y_pos="left")
    assert by_layer.to_svg() == by_plot.to_svg()
    # A named band alone is the same band too.
    named = _base() + gg.geom_ysidetile(gg.aes(fill="module"), side_band="only")
    plain = _base() + gg.geom_ysidetile(gg.aes(fill="module"))
    assert named.to_svg() == plain.to_svg()


def test_named_bands_stack_outward_in_plot_order() -> None:
    plot = (
        _base()
        + gg.new_scale_fill()
        + gg.geom_ysidetile(gg.aes(fill="module"), side_pos="left")
        + gg.geom_ysidecol(
            gg.aes(x="score", y="gene"),
            data=SCORES,
            inherit_aes=False,
            side_pos="left",
            side_band="score",
            side_size=0.3,
        )
        + gg.theme(ggside_panel_spacing=5)
    )
    svg = plot.to_svg()
    (panel,) = _rects(svg, "panel")
    outer, inner = sorted(_rects(svg, "side-y"))
    # The tile band (first in the plot) is nearest the panel.
    assert inner[2] == pytest.approx(0.1 * panel[2], abs=0.01)
    assert outer[2] == pytest.approx(0.3 * panel[2], abs=0.01)
    assert inner[0] + inner[2] + 5 == pytest.approx(panel[0], abs=0.01)
    assert outer[0] + outer[2] + 5 == pytest.approx(inner[0], abs=0.01)
    assert inner[0] < _text_x(svg, "module") < inner[0] + inner[2]
    # Every band spans the panel along the shared axis.
    for band in (inner, outer):
        assert (band[1], band[3]) == pytest.approx((panel[1], panel[3]), abs=0.01)


def test_stacked_x_bands_on_top_and_bottom() -> None:
    plot = (
        gg.ggplot(HEATMAP, gg.aes("gene", "cell", fill="expr"))
        + gg.geom_tile()
        + gg.new_scale_fill()
        + gg.geom_xsidetile(gg.aes(fill="module"))
        + gg.new_scale_fill()
        + gg.geom_xsidetile(gg.aes(fill="family"), side_band="family")
        + gg.geom_xsidecol(
            gg.aes(x="gene", y="score"), data=SCORES, inherit_aes=False, side_pos="bottom"
        )
        + gg.theme(ggside_panel_spacing=4)
        + gg.ggsize(500, 500)
    )
    svg = plot.to_svg()
    (panel,) = _rects(svg, "panel")
    outer, inner, bottom = sorted(_rects(svg, "side-x"), key=lambda rect: rect[1])
    assert inner[1] + inner[3] + 4 == pytest.approx(panel[1], abs=0.01)
    assert outer[1] + outer[3] + 4 == pytest.approx(inner[1], abs=0.01)
    assert bottom[1] == pytest.approx(panel[1] + panel[3] + 4, abs=0.01)


def test_targeted_scales_set_one_band() -> None:
    plot = (
        _base()
        + gg.new_scale_fill()
        + gg.geom_ysidetile(gg.aes(fill="family"), side_pos="left")
        + gg.new_scale_fill()
        + gg.geom_ysidetile(gg.aes(fill="module"))
        + gg.scale_ysidex_discrete(side_pos="left", labels=["FAMILY"])
        + gg.scale_ysidex_discrete(labels=["every band"])
    )
    texts = re.findall(r">([^<>]+)</text>", plot.to_svg())
    # The targeted scale wins for the left band; the plain one sets the rest.
    assert texts.count("FAMILY") == 1
    assert texts.count("every band") == 1


def test_python_checks_band_keys() -> None:
    with pytest.raises(SpecError, match=r"side_pos='top'.*'left' or 'right'"):
        gg.geom_ysidetile(side_pos="top")
    with pytest.raises(SpecError, match=r"did you mean 'bottom'"):
        gg.geom_xsidetile(side_pos="botom")
    with pytest.raises(SpecError, match=r"side_band=3"):
        gg.geom_xsidetile(side_band=3)  # type: ignore[arg-type]
    with pytest.raises(SpecError, match=r"side_size=0"):
        gg.geom_xsidetile(side_size=0)
    with pytest.raises(SpecError, match=r"scale_xsidey_discrete\(side_pos='left'\)"):
        gg.scale_xsidey_discrete(side_pos="left")
    with pytest.raises(SpecError, match=r"no band size"):
        gg.scale_xsidey_discrete(side_size=0.2)


def test_rust_checks_band_keys_on_main_layers() -> None:
    plot = _base() + gg.geom_point(side_pos="left")
    with pytest.raises(SpecError, match=r"layers\[1\]\.side_pos.*main panel"):
        plot.to_svg()


def test_boxplot_layer_lists_share_one_band() -> None:
    plot = _base() + gg.geom_ysideboxplot(
        gg.aes(x="expr"), side_pos="left", side_band="box", side_size=0.2
    )
    layers = plot.as_dict()["layers"][1:]
    assert len(layers) == 2
    assert all(layer["side_band"] == "box" and layer["side_pos"] == "left" for layer in layers)
    assert len(_rects(plot.to_svg(), "side-y")) == 1


def test_side_size_takes_numpy_numbers() -> None:
    np = pytest.importorskip("numpy")
    layer = gg.geom_xsidedensity(side_size=np.float32(0.2))
    assert layer.as_dict()["side_size"] == pytest.approx(0.2)
