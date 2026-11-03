"""corr_plot (bistro) against lets-plot.

Compared with lets-plot 4.11: the plot size, the axis labels and their order, every
text (correlation labels and colorbar labels), relative point radii, and which
cells each layer draws (layer defaults measured over every layer combination).
Pixel positions differ by a px or two (real fonts move the panel).
"""

import math
import re
import warnings

import numpy as np
import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
from ggrust._ggrust import correlation_matrix  # noqa: E402
from ggrust.bistro import corr_plot  # noqa: E402
from ggrust.bistro.corr import _configure, _LayerParams, _plot_size  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402
from lets_plot.bistro.corr import corr_plot as lp_corr_plot  # noqa: E402


def _data() -> dict[str, list[float]]:
    rng = np.random.default_rng(42)
    data = {name: list(rng.normal(size=12)) for name in "abcd"}
    data["b"] = [value * 0.8 + rng.normal() * 0.3 for value in data["a"]]
    return data


def _size(svg: str) -> tuple[float, float]:
    match = re.search(r'<svg[^>]*?width="([\d.]+)" height="([\d.]+)"', svg)
    assert match, svg[:200]
    return float(match.group(1)), float(match.group(2))


def _texts(svg: str) -> list[str]:
    found = re.findall(r"<tspan>([^<]*)</tspan>", svg) + re.findall(
        r"<text[^>]*>([^<]+)</text>", svg
    )
    return sorted(text.strip() for text in found if text.strip())


def _relative_radii(svg: str) -> list[float]:
    radii = [float(r) for r in re.findall(r'<circle[^>]*\br="([0-9.]+)"', svg)]
    if not radii:
        return []
    largest = max(radii)
    return sorted(round(radius / largest, 2) for radius in radii)


CASES = {
    "tiles": lambda m, d: m(d).tiles(),
    "points": lambda m, d: m(d).points(),
    "labels": lambda m, d: m(d).labels(),
    "tiles_labels": lambda m, d: m(d).tiles().labels(),
    "points_labels": lambda m, d: m(d).points().labels(),
    "tiles_points_labels": lambda m, d: m(d).tiles().points().labels(),
    "tiles_upper": lambda m, d: m(d).tiles(type="upper"),
    "tiles_lower_diag": lambda m, d: m(d).tiles(type="lower", diag=True),
    "upper_lower": lambda m, d: m(d).tiles(type="upper").labels(type="lower"),
    "noflip_upper": lambda m, d: m(d, flip=False).tiles(type="upper").labels(type="upper"),
    "no_legend": lambda m, d: m(d, show_legend=False).points().labels(),
    "threshold": lambda m, d: m(d, threshold=0.2).tiles().labels(),
    "map_size": lambda m, d: m(d).tiles().labels(map_size=True, color="black"),
    "color": lambda m, d: m(d).tiles().points().labels(color="red"),
    "points_color": lambda m, d: m(d).points().labels(color="red"),
    "brewer": lambda m, d: m(d).points().palette_RdBu(),
    "gradient": lambda m, d: m(d).tiles().palette_gradient("red", "white", "blue"),
}


@pytest.mark.parametrize("name", list(CASES))
def test_matches_lets_plot(name: str) -> None:
    data = _data()
    expected = CASES[name](lp_corr_plot, data).build().to_svg()
    actual = CASES[name](corr_plot, data).build().to_svg()
    assert _size(actual) == _size(expected)
    assert _texts(actual) == _texts(expected)
    assert _relative_radii(actual) == _relative_radii(expected)


def test_long_names_size_matches_lets_plot() -> None:
    rng = np.random.default_rng(1)
    for count, length in [(2, 10), (3, 9), (4, 8), (5, 13), (18, 7), (25, 6)]:
        data = {
            "x" * (length - 2) + chr(97 + i % 26) + str(i // 26): list(rng.normal(size=20))
            for i in range(count)
        }
        expected = _size(lp_corr_plot(data).tiles().build().to_svg())
        assert _size(corr_plot(data).tiles().build().to_svg()) == expected, (count, length)


def test_plot_size_formula() -> None:
    assert _plot_size(list("abcd"), list("abcd"), True) == (235, 180)
    assert _plot_size(list("abc"), list("abc"), False) == (155, 170)
    assert _plot_size(["v" * 20] * 3, ["v" * 20] * 3, True) == (334, 207)


def _types(*layers: _LayerParams | None) -> list[tuple[str | None, bool | None] | None]:
    return [None if layer is None else (layer.type, layer.diag) for layer in _configure(*layers)]


def test_layer_defaults_match_lets_plot() -> None:
    unset = _LayerParams()
    assert _types(unset, None, None) == [("full", True), None, None]
    assert _types(unset, unset, None) == [("lower", False), ("upper", False), None]
    assert _types(unset, None, unset) == [("lower", False), None, ("upper", False)]
    assert _types(None, unset, unset) == [None, ("upper", False), ("lower", False)]
    assert _types(unset, unset, unset) == [("lower", False), ("upper", False), ("upper", False)]
    assert _types(unset, _LayerParams("full"), None) == [("lower", False), ("full", True), None]
    assert _types(_LayerParams("full"), unset, None) == [("full", True), ("full", True), None]
    assert _types(unset, None, _LayerParams("upper")) == [("full", True), None, ("upper", False)]
    assert _types(_LayerParams("upper"), _LayerParams("upper"), unset) == [
        ("upper", False),
        ("upper", False),
        ("lower", False),
    ]
    assert _types(_LayerParams(diag=True), unset, None) == [("lower", True), ("upper", False), None]


def test_label_color_and_size_defaults() -> None:
    unset = _LayerParams()
    _, _, over_points = _configure(unset, unset, unset)
    assert over_points is not None and over_points.color == "white" and over_points.map_size
    _, _, apart = _configure(unset, None, unset)
    assert apart is not None and apart.color is None and not apart.map_size
    _, _, over_tiles = _configure(_LayerParams("full"), None, _LayerParams("full"))
    assert over_tiles is not None and over_tiles.color == "white" and not over_tiles.map_size


def test_correlations_are_pairwise_pearson() -> None:
    frame = pl.DataFrame(
        {
            "a": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            "i": [1, 3, 2, 5, 4, 6],
            "s": list("uvwxyz"),
            "n": [1.0, None, 2.0, 5.0, float("nan"), 9.0],
        }
    )
    names, values, given = correlation_matrix(gg._data.as_arrow(frame))
    assert names == ["a", "i", "n"] and not given
    assert math.isclose(values[1], np.corrcoef([1, 2, 3, 4, 5, 6], [1, 3, 2, 5, 4, 6])[0, 1])
    assert math.isclose(values[2], np.corrcoef([1, 3, 4, 6], [1, 2, 5, 9])[0, 1])


def test_correlation_matrix_input_is_used_as_given() -> None:
    matrix = pl.DataFrame({"a": [1.0, 0.5], "b": [0.5, 1.0]})
    names, values, given = correlation_matrix(gg._data.as_arrow(matrix))
    assert given and names == ["a", "b"] and values == [1.0, 0.5, 0.5, 1.0]
    svg = corr_plot(matrix).labels().build().to_svg()
    assert _texts(svg).count("0.50") == 2


def test_builder_methods_do_not_mutate() -> None:
    base = corr_plot(_data())
    base.tiles().palette_BrBG()
    with pytest.raises(SpecError, match="no layer to draw"):
        base.build()


def test_errors() -> None:
    with pytest.raises(SpecError, match=r"type='uper'.*did you mean 'upper'"):
        corr_plot(_data()).tiles(type="uper")  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="between 0 and 1"):
        corr_plot(_data(), threshold=1.5)


def test_point_size_unit_x() -> None:
    plot = gg.ggplot({"x": ["a", "b"], "y": ["a", "b"]}, gg.aes("x", "y")) + gg.geom_point(
        size=1, size_unit="x"
    )
    plot += gg.coord_fixed(xlim=[-0.5, 1.5], ylim=[-0.5, 1.5]) + gg.ggsize(300, 300)
    svg = plot.to_svg()
    radii = [float(r) for r in re.findall(r'<circle[^>]*\br="([0-9.]+)"', svg)]
    ticks = [
        float(x) for x in re.findall(r'<path d="M([\d.]+) [\d.]+L[\d.]+ [\d.]+" fill="none"', svg)
    ][:2]
    assert len(radii) == 2 and len(ticks) == 2
    assert math.isclose(radii[0] * 2, abs(ticks[1] - ticks[0]), rel_tol=1e-3)


@pytest.mark.parametrize("name", ["geom_jitter", "geom_sina"])
def test_size_unit_is_read_by_point_like_geoms(name: str) -> None:
    data = {"x": ["a", "a", "b", "b"], "y": [1.0, 2.0, 3.0, 4.0]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + getattr(gg, name)(size_unit="x", seed=1)
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        plot.to_svg()
