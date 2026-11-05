"""Side panels: ggside side layers and lets-plot's ``ggmarginal()`` (step ex2.2).

Spec marks (``"ggside"`` on layers and scales, lets-plot's marginal keys), band
geometry read from the SVG (size, gap, order of tracks, axis labels), legends per
track, facets, ``coord_flip`` titles, and the errors.
"""

from __future__ import annotations

import re
import warnings

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError

POINTS = pl.DataFrame(
    {
        "x": [0.1, 0.5, 1.2, 1.9, 2.4, 2.8, 3.3, 4.1, 4.4, 5.0],
        "y": [1.0, 2.2, 1.7, 3.1, 2.6, 3.8, 3.0, 4.6, 4.1, 5.2],
        "group": ["a", "b"] * 5,
    }
)
CELLS = ["c1", "c2", "c3", "c4"]
HEATMAP = pl.DataFrame(
    {
        "cell": [cell for cell in CELLS for _ in range(3)],
        "gene": ["g1", "g2", "g3"] * 4,
        "expr": [float(index % 5) for index in range(12)],
        "cell_type": [kind for kind in ["T", "T", "B", "NK"] for _ in range(3)],
        "batch": [batch for batch in ["b1", "b2", "b1", "b2"] for _ in range(3)],
        "module": ["M1", "M2", "M1"] * 4,
    }
)


def _heatmap() -> gg.PlotSpec:
    return (
        gg.ggplot(HEATMAP, gg.aes("cell", "gene", fill="expr"))
        + gg.geom_tile()
        + gg.new_scale_fill()
        + gg.geom_xsidetile(gg.aes(fill="cell_type"))
        + gg.new_scale_fill()
        + gg.geom_xsidetile(gg.aes(fill="batch"))
        + gg.new_scale_fill()
        + gg.geom_ysidetile(gg.aes(fill="module"))
        + gg.ggsize(600, 400)
    )


def _clips(svg: str) -> dict[str, list[float]]:
    """The clip rectangle of each ``side-x`` / ``side-y`` group, and the panel's."""
    clips = {
        found.group(1): [float(value) for value in found.group(2, 3, 4, 5)]
        for found in re.finditer(
            r'<clipPath id="([^"]+)"><rect x="([-\d.]+)" y="([-\d.]+)" '
            r'width="([\d.]+)" height="([\d.]+)"/>',
            svg,
        )
    }
    out = {}
    for role in ("panel", "side-x", "side-y"):
        found = re.search(rf'<g class="{role}" clip-path="url\(#([^)]+)\)"', svg)
        if found:
            out[role] = clips[found.group(1)]
    return out


def test_side_geoms_mark_their_layers() -> None:
    tile = gg.geom_xsidetile(gg.aes(fill="cell_type")).as_dict()
    assert tile["ggside"] == "x"
    assert tile["geom"] == "tile"
    density = gg.geom_ysidedensity().as_dict()
    assert density["ggside"] == "y"
    assert density["orientation"] == "y"
    assert "orientation" not in gg.geom_xsidedensity().as_dict()
    # Both layers of a boxplot (boxes and outliers) are side layers, at y=0.
    layers = [element.as_dict() for element in gg.geom_xsideboxplot()]
    assert [layer["ggside"] for layer in layers] == ["x", "x"]
    assert all(layer["y"] == 0 and layer["orientation"] == "y" for layer in layers)
    assert "y" not in gg.geom_xsideboxplot(gg.aes(y="group"))[0].as_dict()
    assert gg.scale_xsidey_discrete(limits=["b", "a"]).as_dict()["ggside"] == "x"
    assert gg.scale_ysidex_continuous().as_dict()["aesthetic"] == "x"
    plot = gg.ggplot() + gg.ggside(x_pos="bottom")
    assert plot.as_dict()["ggside"] == {"x_pos": "bottom"}


def test_ggmarginal_writes_lets_plot_spec() -> None:
    lets_plot = pytest.importorskip("lets_plot")

    def spec(module: object) -> dict:
        plot = (
            module.ggplot(POINTS.to_dict(as_series=False), module.aes("x", "y"))
            + module.geom_point()
            + module.ggmarginal("tr", size=[0.2, 0.15], layer=module.geom_histogram(bins=5))
            + module.ggmarginal("lt", layer=module.geom_boxplot())
            + module.ggmarginal("b", layer=module.geom_violin(color="red"))
        )
        layers = plot.as_dict()["layers"]
        keys = ("geom", "stat", "marginal", "margin_side", "margin_size", "orientation", "x", "y")
        return [
            {key: layer.get(key) for key in keys} | {"mapping": layer.get("mapping", {})}
            for layer in layers
        ]

    assert spec(gg) == spec(lets_plot)


def test_ggmarginal_errors() -> None:
    with pytest.raises(ValueError, match="unknown margin 'x'"):
        gg.ggmarginal("tx", layer=gg.geom_density())
    with pytest.raises(ValueError, match=r"expected sizes in \[0.01, 0.95\]"):
        gg.ggmarginal("t", size=2, layer=gg.geom_density())
    with pytest.raises(TypeError, match="expected a string"):
        gg.ggmarginal(1, layer=gg.geom_density())
    with pytest.raises(TypeError, match="expected a geom"):
        gg.ggmarginal("t", layer=gg.theme())


def test_ggside_position_errors() -> None:
    with pytest.raises(SpecError, match=r"ggside\(x_pos='left'\): expected 'top' or 'bottom'"):
        gg.ggside(x_pos="left")
    with pytest.raises(SpecError, match="did you mean 'right'"):
        gg.ggside(y_pos="rigth")


def test_tracks_are_bands_of_their_own_scale() -> None:
    svg = _heatmap().to_svg()
    clips = _clips(svg)
    panel, top, left = clips["panel"], clips["side-x"], clips["side-y"]
    # ggside defaults: 0.1 of the main panel, 2 pt (2.67 px) away; the y band on the
    # right, the x band on top, sharing the panel's span.
    gap = 2.0 * 96 / 72
    assert top[3] == pytest.approx(0.1 * panel[3], abs=0.01)
    assert top[1] + top[3] + gap == pytest.approx(panel[1], abs=0.01)
    assert (top[0], top[2]) == pytest.approx((panel[0], panel[2]), abs=0.01)
    assert left[2] == pytest.approx(0.1 * panel[2], abs=0.01)
    assert left[0] == pytest.approx(panel[0] + panel[2] + gap, abs=0.01)
    # Track labels on the band's own axis, a legend per track.
    texts = re.findall(r">([^<>]+)</text>", svg)
    for label in ("cell_type", "batch", "module", "expr"):
        assert label in texts, label
    for key in ("T", "B", "NK", "b1", "b2", "M1", "M2"):
        assert key in texts, key

    # cell_type (the first track) is nearest the panel: lower on screen.
    def label_y(text: str) -> float:
        found = re.search(rf'<text[^>]*y="([\d.]+)"[^>]*>{text}</text>', svg)
        assert found, text
        return float(found.group(1))

    assert label_y("cell_type") > label_y("batch")
    # Main titles come from the main layers only.
    assert "cell" in texts and "gene" in texts


def test_band_sizes_and_positions_follow_the_options() -> None:
    plot = (
        gg.ggplot(POINTS, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.geom_xsidehistogram(bins=4)
        + gg.geom_ysidedensity()
        + gg.ggside(x_pos="bottom", y_pos="left")
        + gg.theme(ggside_panel_scale_x=0.25, ggside_panel_spacing=6)
        + gg.ggsize(500, 400)
    )
    clips = _clips(plot.to_svg())
    panel, bottom, left = clips["panel"], clips["side-x"], clips["side-y"]
    assert bottom[3] == pytest.approx(0.25 * panel[3], abs=0.01)
    assert bottom[1] == pytest.approx(panel[1] + panel[3] + 6, abs=0.01)
    assert left[2] == pytest.approx(0.1 * panel[2], abs=0.01)
    assert left[0] + left[2] + 6 == pytest.approx(panel[0], abs=0.01)


def test_marginal_bands_match_lets_plot_sizes() -> None:
    # lets-plot 4.x: each margin is `size` of the panel plus its margins, no gap
    # (measured: 35.4 of 354 px for 0.1, 55.5 of 555 px).
    plot = (
        gg.ggplot(POINTS, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.ggmarginal("tr", layer=gg.geom_density())
        + gg.ggsize(600, 400)
    )
    clips = _clips(plot.to_svg())
    panel, top, right = clips["panel"], clips["side-x"], clips["side-y"]
    # The SVG writes coordinates to 0.01 px.
    assert top[3] / (top[3] + panel[3]) == pytest.approx(0.1, abs=1e-4)
    assert right[2] / (right[2] + panel[2]) == pytest.approx(0.1, abs=1e-4)
    assert top[1] + top[3] == pytest.approx(panel[1], abs=0.01)
    assert right[0] == pytest.approx(panel[0] + panel[2], abs=0.01)


def test_marginal_bands_draw_no_axis() -> None:
    plot = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point()
    assert 'class="side-axis"' in (plot + gg.geom_xsidedensity()).to_svg()
    marginal = (plot + gg.ggmarginal("t", layer=gg.geom_density())).to_svg()
    assert 'class="side-x"' in marginal
    assert 'class="side-axis"' not in marginal


def test_side_layers_train_the_shared_axis_but_margins_do_not() -> None:
    wide = pl.DataFrame({"x": [-20.0, 30.0]})

    def x_labels(layer: gg.FeatureSpec) -> list[str]:
        plot = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point() + layer
        svg = plot.to_svg()
        axis = svg[svg.index('class="axis-x"') :]
        return re.findall(r">(-?\d+)</text>", axis[: axis.index("</g>\n</g>")])

    ggside = x_labels(gg.geom_xsidepoint(gg.aes("x"), y=0, data=wide))
    marginal = x_labels(gg.ggmarginal("t", layer=gg.geom_point(gg.aes("x"), y=0, data=wide)))
    assert "-20" in ggside or "30" in ggside
    assert "-20" not in marginal and "30" not in marginal


def test_facets_drop_side_layers_with_a_warning() -> None:
    plot = (
        gg.ggplot(POINTS, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.geom_xsidedensity()
        + gg.facet_wrap("group")
    )
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        svg = plot.to_svg()
    messages = [str(warning.message) for warning in caught]
    assert any("faceted plots draw no side panels yet" in message for message in messages)
    assert "side-x" not in svg


def test_coord_flip_moves_bands_and_keeps_titles() -> None:
    plot = (
        gg.ggplot(POINTS, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.geom_ysideboxplot(gg.aes(x="group"))
        + gg.coord_flip()
    )
    svg = plot.to_svg()
    clips = _clips(svg)
    # The y band shares data y, horizontal under coord_flip: on top (its group is
    # named by the screen axis it shares).
    assert clips["side-x"][1] + clips["side-x"][3] < clips["panel"][1]
    texts = re.findall(r">([^<>]+)</text>", svg)
    # The band's cross mapping (x="group") does not name the main x axis.
    assert "group" not in texts
    assert "x" in texts and "y" in texts


@pytest.mark.parametrize(
    ("layer", "path", "message"),
    [
        (gg.geom_point(ggside="z"), "layers[1].ggside", "expected 'x'"),
        (
            gg.geom_point(marginal=True, margin_side="top"),
            "layers[1].margin_side",
            "expected one of 't', 'b', 'l', 'r'",
        ),
        (
            gg.geom_point(marginal=True, margin_side="t", margin_size=3),
            "layers[1].margin_size",
            "[0.01, 0.95]",
        ),
        (gg.geom_point(margin_side="t"), "layers[1].margin_side", "ggmarginal()"),
    ],
)
def test_layer_errors_name_the_key(layer: gg.FeatureSpec, path: str, message: str) -> None:
    plot = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point() + layer
    with pytest.raises(SpecError, match=re.escape(message)) as caught:
        plot.to_svg()
    assert caught.value.path == path


def test_theme_and_scale_errors() -> None:
    base = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point() + gg.geom_xsidedensity()
    with pytest.raises(SpecError, match="positive share of the main panel") as caught:
        (base + gg.theme(ggside_panel_scale=0)).to_svg()
    assert caught.value.path == "theme.ggside_panel_scale_x"
    with pytest.raises(SpecError, match="gap of 0 px or more"):
        (base + gg.theme(ggside_panel_spacing_y=-1)).to_svg()
    bad_scale = gg.scale_x_continuous()
    bad_scale.props()["ggside"] = "x"
    with pytest.raises(SpecError, match=r"use scale_xsidey_\*\(\)"):
        (base + bad_scale).to_svg()
    with pytest.raises(SpecError, match="unknown ggside option"):
        bad = gg.ggside()
        bad.props()["xpos"] = "top"
        (base + bad).to_svg()


def test_ggside_scale_orders_tracks() -> None:
    plot = _heatmap() + gg.scale_xsidey_discrete(limits=["batch", "cell_type"])
    svg = plot.to_svg()
    order = [
        float(re.search(rf'<text[^>]*y="([\d.]+)"[^>]*>{text}</text>', svg).group(1))
        for text in ("batch", "cell_type")
    ]
    # batch first now: nearest the panel (lowest on screen).
    assert order[0] > order[1]


def test_band_axis_labels_stay_inside_the_band() -> None:
    plot = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point() + gg.geom_xsidedensity()
    svg = plot.to_svg()
    band = _clips(svg)["side-x"]
    axis = svg[svg.index('class="side-axis"') :]
    axis = axis[: axis.index("</g>")]
    for found in re.finditer(r'<text[^>]*y="([\d.]+)"[^>]*>([^<]+)</text>', axis):
        assert band[1] - 1 <= float(found.group(1)) <= band[1] + band[3] + 6, found.group(2)


def test_own_scale_position_moves_the_band_axis() -> None:
    plot = (
        gg.ggplot(HEATMAP, gg.aes("cell", "gene", fill="expr"))
        + gg.geom_tile()
        + gg.new_scale_fill()
        + gg.geom_xsidetile(gg.aes(fill="cell_type"))
        + gg.scale_xsidey_discrete(position="right")
    )
    svg = plot.to_svg()
    band = _clips(svg)["side-x"]
    found = re.search(r'<text[^>]*x="([\d.]+)"[^>]*>cell_type</text>', svg)
    assert found and float(found.group(1)) > band[0] + band[2]


def test_large_margins_stay_on_the_canvas() -> None:
    plot = (
        gg.ggplot(POINTS, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.ggmarginal("t", size=0.95, layer=gg.geom_density())
        + gg.ggsize(300, 200)
    )
    top = _clips(plot.to_svg())["side-x"]
    assert top[1] >= 0


def test_ggside_options_merge_key_by_key() -> None:
    plot = gg.ggplot() + gg.ggside(x_pos="bottom") + gg.ggside(y_pos="left")
    assert plot.as_dict()["ggside"] == {"x_pos": "bottom", "y_pos": "left"}
    plot = plot + gg.ggside(x_pos="top")
    assert plot.as_dict()["ggside"] == {"x_pos": "top", "y_pos": "left"}


SPECIES = pl.DataFrame(
    {
        "length": [float(index % 17) for index in range(60)],
        "width": [float(index % 11) for index in range(60)],
        "species": [["Adelie", "Gentoo", "Chinstrap"][index % 3] for index in range(60)],
    }
)


def _band_axis_texts(svg: str) -> list[tuple[float, float, str, bool]]:
    """The labels of the first band axis: x, y, text and whether turned."""
    axis = svg[svg.index('class="side-axis"') :]
    axis = axis[: axis.index("</g>")]
    return [
        (float(found.group(1)), float(found.group(2)), found.group(4), bool(found.group(3)))
        for found in re.finditer(
            r'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*?( transform="rotate[^"]*")?>([^<]+)</text>',
            axis,
        )
    ]


@pytest.mark.parametrize(
    "layer",
    [
        gg.geom_ysideboxplot(gg.aes(x="species", fill="species"), alpha=0.4),
        gg.geom_ysideviolin(gg.aes(x="species")),
        gg.geom_ysidetile(gg.aes(x="species", fill="species")),
    ],
    ids=["boxplot", "violin", "tile"],
)
@pytest.mark.parametrize("position", ["bottom", "top"])
def test_discrete_y_band_labels_get_room(layer: gg.FeatureSpec, position: str) -> None:
    plot = (
        gg.ggplot(SPECIES, gg.aes("length", "width"))
        + gg.geom_point()
        + layer
        + gg.scale_ysidex_discrete(position=position)
        + gg.ggsize(600, 400)
    )
    svg = plot.to_svg()
    labels = _band_axis_texts(svg)
    assert [text for _, _, text, _ in labels] == ["Adelie", "Gentoo", "Chinstrap"]
    # The narrow band turns them (lets-plot rules); the plot keeps room for them.
    assert all(turned for *_, turned in labels)
    panel = _clips(svg)["panel"]
    widest = max(0.45 * 13 * len(text) for _, _, text, _ in labels)
    if position == "bottom":
        title = re.search(r'<text[^>]*y="([\d.]+)"[^>]*>length</text>', svg)
        assert title and float(title.group(1)) - 12 > labels[0][1] + widest
    else:
        assert all(y - widest > 0 for _, y, _, _ in labels)
        assert panel[1] > widest


@pytest.mark.parametrize(
    "layer",
    [
        gg.geom_xsideboxplot(gg.aes(y="species")),
        gg.geom_xsideviolin(gg.aes(y="species")),
        gg.geom_xsidetile(gg.aes(y="species", fill="species")),
    ],
    ids=["boxplot", "violin", "tile"],
)
@pytest.mark.parametrize("position", ["left", "right"])
def test_discrete_x_band_keeps_every_label(layer: gg.FeatureSpec, position: str) -> None:
    plot = (
        gg.ggplot(SPECIES, gg.aes("length", "width"))
        + gg.geom_point()
        + layer
        + gg.scale_xsidey_discrete(position=position)
        + gg.ggsize(600, 400)
    )
    svg = plot.to_svg()
    labels = _band_axis_texts(svg)
    assert [text for _, _, text, _ in labels] == ["Adelie", "Gentoo", "Chinstrap"]
    widest = max(0.45 * 13 * len(text) for _, _, text, _ in labels)
    for x, _, _, _ in labels:
        assert (x - widest > 0) if position == "left" else (x + widest < 600)
