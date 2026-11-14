"""joint_plot and qq_plot (bistro) against lets-plot 4.11.

joint_plot is a Python builder in lets_plot too, so its ``as_dict()`` (without the
data) must equal lets_plot's for the same arguments. qq_plot is expanded by
lets-plot's engine; its rules were measured from ``to_svg()`` output (point and
line styles, group coloring, marginal styles, axis titles, theme_light) and are
checked here against live lets-plot SVGs: titles, legend labels, point colors and
counts, and the number and colors of the fitted lines.
"""

import io
import re
import warnings
from collections import Counter
from typing import Any

import numpy as np
import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

from ggrust.bistro.joint import joint_plot  # noqa: E402
from ggrust.bistro.qq import qq_plot  # noqa: E402
from ggrust.errors import ColumnNotFoundError, SpecError  # noqa: E402
from lets_plot.bistro.joint import joint_plot as lp_joint_plot  # noqa: E402
from lets_plot.bistro.qq import qq_plot as lp_qq_plot  # noqa: E402


def _data(n: int = 60) -> dict[str, Any]:
    rng = np.random.default_rng(7)
    return {
        "x": rng.normal(0, 1, n),
        "y": rng.normal(1, 2, n),
        "g": rng.choice(["a", "b"], n),
        "n": rng.choice([1, 2], n),
    }


def _without_data(spec: dict[str, Any]) -> dict[str, Any]:
    spec = dict(spec)
    spec.pop("data", None)
    return spec


# ---------------------------------------------------------------- joint_plot

JOINT_CASES: dict[str, dict[str, Any]] = {
    "default": {},
    "tile_bins": {"geom": "tile", "bins": 10, "color": "black", "marginal": "hist:tr:.3,box:bl"},
    "hex_binwidth": {"geom": "hex", "binwidth": 0.4},
    "binwidth_pair": {"geom": "tile", "binwidth": [0.5, 0.25], "marginal": "hist:tblr"},
    "density2d_color_by": {"geom": "density2d", "color_by": "g"},
    "density2df": {"geom": "density2df", "alpha": 0.5},
    "none": {"geom": "none", "marginal": "dens:lb"},
    "styled_points": {"show_legend": False, "size": 2, "alpha": 0.4, "color": "red"},
    "no_reg_line": {"reg_line": False},
    "reg_line_se": {"geom": "tile", "reg_line": True, "se": False},
    "no_marginal": {"marginal": "none"},
    "color_by_box": {"color_by": "g", "marginal": "box : tr : .05, dens : bl"},
}


@pytest.mark.parametrize("case", JOINT_CASES)
def test_joint_plot_spec_matches_lets_plot(case: str) -> None:
    data = _data()
    arguments = JOINT_CASES[case]
    ours = joint_plot(data, "x", "y", **arguments).as_dict()
    theirs = lp_joint_plot(data, "x", "y", **arguments).as_dict()
    # ggrust has no `..group..` contour index: 2D densities map `..level..` (deviation).
    for layer in theirs["layers"]:
        for aesthetic, value in layer.get("mapping", {}).items():
            if value == "..group..":
                layer["mapping"][aesthetic] = "..level.."
    assert _without_data(ours) == _without_data(theirs)


def test_joint_plot_pointdensity_uses_the_ggrust_stat() -> None:
    data = _data()
    ours = joint_plot(data, "x", "y", geom="pointdensity").as_dict()
    theirs = lp_joint_plot(data, "x", "y", geom="pointdensity").as_dict()
    main = ours["layers"][0]
    assert (main["geom"], main["stat"]) == ("point", "pointdensity")
    assert main["mapping"] == theirs["layers"][0]["mapping"] == {"color": "..density.."}
    assert ours["layers"][1:] == theirs["layers"][1:]


def test_joint_plot_default_binwidth_is_the_wider_range_over_30() -> None:
    data = {"x": [0.0, 3.0, 1.0], "y": [0.0, 6.0, 2.0]}
    layers = joint_plot(data, "x", "y").as_dict()["layers"]
    histograms = [layer for layer in layers if layer["geom"] == "histogram"]
    assert [layer["binwidth"] for layer in histograms] == [0.2, 0.2]


def test_joint_plot_polars_and_empty_data() -> None:
    frame = pl.DataFrame({"x": [1.0, 2.0, 4.0], "y": [3.0, 1.0, 2.0]})
    assert "<svg" in joint_plot(frame, "x", "y").to_svg()
    empty = joint_plot({"x": [], "y": []}, "x", "y").as_dict()
    assert all("marginal" not in layer for layer in empty["layers"])


def test_joint_plot_renders_every_geom() -> None:
    data = _data(200)
    for geom in ("point", "tile", "hex", "density2d", "density2df", "pointdensity", "none"):
        buffer = io.BytesIO()
        joint_plot(data, "x", "y", geom=geom).to_png(buffer)
        assert buffer.getvalue()[:8] == b"\x89PNG\r\n\x1a\n", geom


def test_mapped_fill_beats_the_stat_default_level() -> None:
    """Engine: a data-mapped style aesthetic keeps its column (ggplot2), lp5.12."""
    from ggrust import aes, geom_density2df, ggplot

    data = _data(200)
    plot = ggplot(data, aes("x", "y")) + geom_density2df(aes(fill="g"))
    assert sorted(plot.layer_data(0)["fill"].unique().cast(pl.String).to_list()) == ["a", "b"]
    default = ggplot(data, aes("x", "y")) + geom_density2df()
    assert default.layer_data(0)["fill"].dtype == pl.Float64
    spec = joint_plot(data, "x", "y", geom="density2df", color_by="g").as_dict()
    assert spec["layers"][0]["mapping"] == {"fill": "g"}


def test_joint_plot_scalar_and_temporal_bins() -> None:
    data = _data()
    spec = joint_plot(data, "x", "y", geom="tile", bins=np.int64(7)).as_dict()
    assert spec["layers"][0]["bins"] == [7, 7]
    start = pl.datetime(2026, 1, 1)
    frame = pl.DataFrame({"y": np.arange(30.0)}).with_columns(
        x=pl.datetime_range(start, start + pl.duration(days=29), "1d")
    )
    layers = joint_plot(frame, "x", "y").as_dict()["layers"]
    assert all("binwidth" not in layer for layer in layers)


def test_joint_plot_errors() -> None:
    data = _data()
    with pytest.raises(SpecError, match=r"unknown geom 'piont'; did you mean 'point'\?") as error:
        joint_plot(data, "x", "y", geom="piont")  # ty: ignore[invalid-argument-type]
    assert error.value.path == "geom"
    with pytest.raises(SpecError, match=r"unknown marginal geom 'hst'; did you mean 'hist'\?"):
        joint_plot(data, "x", "y", marginal="hst:tr")
    with pytest.raises(SpecError, match=r"needs a geom and sides"):
        joint_plot(data, "x", "y", marginal="hist")
    with pytest.raises(SpecError, match=r"marginal size 'big' is not a number"):
        joint_plot(data, "x", "y", marginal="hist:tr:big")
    with pytest.raises(SpecError, match=r"expected one number or a pair \[x, y\], got 1"):
        joint_plot(data, "x", "y", bins=[5])
    with pytest.raises(
        ColumnNotFoundError, match=r"column 'xx' not found; available columns: x, y, g"
    ):
        joint_plot(data, "xx", "y")


# ------------------------------------------------------------------- qq_plot


def _texts(svg: str) -> list[str]:
    found = re.findall(r">([^<>]+)<", svg)
    texts = [text.strip().replace("&quot;", '"') for text in found]
    return sorted(
        text for text in texts if text and not re.fullmatch(r"-?[\d.]+", text) and "{" not in text
    )


def _hex(color: str) -> str:
    match = re.fullmatch(r"rgb\((\d+),(\d+),(\d+)\)", color)
    if match:
        return "#" + "".join(f"{int(value):02x}" for value in match.groups())
    return color.lower()


def _marks(svg: str, tag: str) -> list[dict[str, str]]:
    """Attributes of every ``tag`` element, with those inherited from ``<g>`` parents."""
    stack: list[dict[str, str]] = [{}]
    marks: list[dict[str, str]] = []
    for closing, name, attributes, self_closing in re.findall(r"<(/?)(\w+)\b([^>]*?)(/?)>", svg):
        if name == "g":
            if closing:
                stack.pop()
            elif not self_closing:
                values = dict(re.findall(r'([\w-]+)="([^"]*)"', attributes))
                stack.append(stack[-1] | values)
        elif name == tag and not closing:
            marks.append(stack[-1] | dict(re.findall(r'([\w-]+)="([^"]*)"', attributes)))
    return marks


def _points(svg: str) -> Counter[tuple[str, str]]:
    """Point marks by (fill, fill opacity); legend keys (opaque) left out."""
    found: Counter[tuple[str, str]] = Counter()
    for values in _marks(svg, "circle"):
        if "fill-opacity" not in values or "fill" not in values:
            continue
        found[_hex(values["fill"]), f"{float(values['fill-opacity']):.2f}"] += 1
    return found


def _lines(svg: str, letsplot: bool) -> Counter[str]:
    """Fitted lines (slanted straight paths of 2 or 3 vertices) by stroke color."""
    found: Counter[str] = Counter()
    for values in _marks(svg, "path"):
        stroke = values.get("stroke", "")
        if values.get("fill") != "none" or _hex(stroke) in _GRID_COLORS:
            continue
        path = values.get("d", "")
        vertices = len(re.findall(r"[ML]", path))
        if vertices == 3 if letsplot else vertices == 2 and _is_diagonal(path):
            found[_hex(stroke)] += 1
    return found


_GRID_COLORS = ("", "#e9e9e9", "#c9c9c9")


def _is_diagonal(path: str) -> bool:
    """True for a slanted segment (not a legend key, tick, whisker or grid line)."""
    x0, y0, x1, y1 = (float(value) for value in re.findall(r"-?[\d.]+", path))
    return abs(x1 - x0) > 1 and abs(y1 - y0) > 1


QQ_CASES: dict[str, dict[str, Any]] = {
    "sample": {"sample": "x"},
    "two_samples": {"x": "x", "y": "y"},
    "exp": {"sample": "x", "distribution": "exp", "dparams": [2.0], "quantiles": [0.0, 0.9]},
    "group": {"sample": "x", "group": "g"},
    "group_color": {"sample": "x", "group": "g", "color": "black"},
    "group_two_samples": {"x": "x", "y": "y", "group": "g", "shape": 21, "alpha": 0.2},
    "styled": {"sample": "x", "color": "green", "fill": "yellow", "shape": 21, "alpha": 0.3},
    "line": {"sample": "x", "line_color": "blue", "line_size": 0.25, "linetype": 2},
    "hist_box": {"sample": "x", "marginal": "hist:tb,box:lr"},
    "no_marginal": {"sample": "x", "marginal": "none"},
    "t": {"sample": "x", "distribution": "t", "dparams": [3]},
    "numeric_group": {"sample": "x", "group": "n"},
}


@pytest.mark.parametrize("case", QQ_CASES)
def test_qq_plot_matches_lets_plot(case: str) -> None:
    data = _data()
    arguments = QQ_CASES[case]
    ours = qq_plot(data, **arguments).to_svg()
    theirs = lp_qq_plot(data, **arguments).to_svg()
    assert _texts(ours) == _texts(theirs)
    assert _points(ours) == _points(theirs)
    assert _lines(ours, letsplot=False) == _lines(theirs, letsplot=True)


def test_qq_plot_layers() -> None:
    spec = qq_plot(_data(), "x", group="g").as_dict()
    points, line, *margins = spec["layers"]
    assert (points["geom"], points["alpha"], points["size"]) == ("qq", 0.5, 3.0)
    assert points["mapping"] == {"sample": "x", "color": "g", "fill": "g"}
    assert line["geom"] == "qq_line" and "color" not in line
    assert (line["mapping"], line["size"]) == ({"sample": "x", "color": "g"}, 0.75)
    assert [(layer["geom"], layer["margin_side"]) for layer in margins] == [
        ("area", "t"),
        ("area", "r"),
    ]
    assert spec["theme"]["name"] == "light"
    plain = qq_plot(_data(), "x").as_dict()["layers"][1]
    assert plain["color"] == "#FF0000"


def test_qq_plot_line_color_keeps_one_line_per_group() -> None:
    """lets-plot fits one pooled line here (deviation, see deviations.md)."""
    spec = qq_plot(_data(), "x", group="g", line_color="black").as_dict()
    line = spec["layers"][1]
    assert line["mapping"] == {"sample": "x", "group": "g"}
    assert line["color"] == "black"
    svg = qq_plot(_data(), "x", group="g", line_color="black").to_svg()
    assert _lines(svg, letsplot=False) == Counter({"#000000": 2})


def test_qq_plot_margins_show_the_computed_quantiles() -> None:
    data = _data()
    plot = qq_plot(data, "x", group="g")
    points = plot.layer_data(0)
    top = plot.layer_data(2)
    assert top["group"].n_unique() == 2
    assert top["x"].min() == pytest.approx(points["x"].min())
    assert top["x"].max() == pytest.approx(points["x"].max())
    right = plot.layer_data(3)
    assert right["y"].min() == pytest.approx(points["y"].min())


def test_qq_plot_facets_on_the_group() -> None:
    """Faceted plots drop marginal layers for now (deviations.md, side panels)."""
    from ggrust import facet_grid

    plot = qq_plot(_data(), "x", group="g") + facet_grid(x="g")
    buffer = io.BytesIO()
    with pytest.warns(UserWarning, match="faceted plots draw no side panels"):
        assert plot.layer_data(0)["panel"].n_unique() == 2
        plot.to_png(buffer)
    assert buffer.getvalue()[:4] == b"\x89PNG"


def test_qq_plot_group_named_like_margin_columns() -> None:
    data = _data()
    data["__qq_x"] = data.pop("g")
    svg = qq_plot(data, "x", group="__qq_x").to_svg()
    assert _lines(svg, letsplot=False) == Counter({"#e41a1c": 1, "#377eb8": 1})


def test_qq_plot_missing_values_warn_once() -> None:
    data = {"x": [1.0, None, 2.5, 0.3, float("nan"), 4.0, 2.2]}
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        plot = qq_plot(data, "x")
    assert not caught
    with pytest.warns(UserWarning, match="removed 2 rows"):
        assert "<svg" in plot.to_svg()


def test_marginal_option_errors() -> None:
    data = _data()
    cases = {
        "dens:xy": r"unknown side 'x' in 'xy'",
        "dens:": r"needs a geom and sides",
        "dens:t:5": r"marginal size '5' is not a number in \[0.01, 0.95\]",
    }
    for marginal, pattern in cases.items():
        with pytest.raises(SpecError, match=pattern) as error:
            qq_plot(data, "x", marginal=marginal)
        assert error.value.path == "marginal"
        with pytest.raises(SpecError, match=pattern) as error:
            joint_plot(data, "x", "y", marginal=marginal)
        assert error.value.path == "marginal"


def test_qq_plot_errors() -> None:
    data = _data()
    with pytest.raises(SpecError, match=r"no data column given"):
        qq_plot(data)
    with pytest.raises(SpecError, match=r"y= is missing") as error:
        qq_plot(data, x="x")
    assert error.value.path == "y"
    with pytest.raises(SpecError, match=r"not both"):
        qq_plot(data, "x", x="x", y="y")
    with pytest.raises(SpecError, match=r"unknown marginal geom 'dense'; did you mean 'dens'\?"):
        qq_plot(data, "x", marginal="dense:tr")
    with pytest.raises(ColumnNotFoundError):
        qq_plot(data, "missing")
