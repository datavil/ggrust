"""ua.8: jittered boxplot outliers (``geom_boxplot(outlier_jitter=)``, ggplot2 issue
4480) and the "NA" legend key / colorbar swatch of a scale with ``na_value`` (ggplot2
issue 4567). Neither exists in lets-plot."""

import re
from pathlib import Path

import ggrust as gg
import polars as pl
import pytest
from ggrust.plot.core import PlotSpec

BOXES = pl.DataFrame(
    {
        "team": ["a"] * 12 + ["b"] * 12,
        "y": [1.0, 2, 2, 3, 3, 3, 4, 4, 5, 20, 20, 20] * 2,
    }
)


def _outliers(plot: PlotSpec) -> pl.DataFrame:
    """The finite rows of the outlier layer."""
    return plot.layer_data(1).filter(pl.col("y").is_not_nan())


def _boxplot(**options: object) -> PlotSpec:
    return gg.ggplot(BOXES, gg.aes("team", "y")) + gg.geom_boxplot(**options)


def test_outlier_jitter_spreads_outliers_within_the_width_only() -> None:
    plain = _outliers(_boxplot())
    jittered = _outliers(_boxplot(outlier_jitter=0.2))
    assert plain["y"].to_list() == jittered["y"].to_list()
    offsets = (jittered["x"] - plain["x"]).abs()
    assert offsets.max() <= 0.2 + 1e-12
    assert jittered["x"].n_unique() == jittered.height  # tied outliers separate
    # The box layer does not move.
    assert _boxplot(outlier_jitter=0.2).layer_data(0).equals(_boxplot().layer_data(0))


def test_outlier_jitter_is_seeded_and_reproducible() -> None:
    first = _outliers(_boxplot(outlier_jitter=0.2))["x"]
    assert first.equals(_outliers(_boxplot(outlier_jitter=0.2))["x"])
    other = _outliers(_boxplot(outlier_jitter=0.2, outlier_seed=7))["x"]
    assert not first.equals(other)


def test_outlier_jitter_keeps_the_dodge_slots() -> None:
    data = BOXES.with_columns(sex=pl.Series(["f", "m"] * 12))
    plot = gg.ggplot(data, gg.aes("team", "y", color="sex"))
    plain = _outliers(plot + gg.geom_boxplot(position=gg.position_dodge(width=0.6)))
    jittered = _outliers(
        plot + gg.geom_boxplot(position=gg.position_dodge(width=0.6), outlier_jitter=0.05)
    )
    assert ((jittered["x"] - plain["x"]).abs() <= 0.05 + 1e-12).all()


@pytest.mark.parametrize(
    ("options", "message"),
    [
        ({"outlier_jitter": -1}, "finite number >= 0"),
        ({"outlier_jitter": float("nan")}, "finite number >= 0"),
        ({"outlier_seed": 3}, "only applies with outlier_jitter"),
        ({"outlier_jitter": 0.1, "position": "stack"}, "jitters around dodged boxes"),
        ({"outlier_jitter": 0.1, "stat": "identity"}, "needs the default stat"),
    ],
)
def test_outlier_jitter_errors(options: dict[str, object], message: str) -> None:
    with pytest.raises(ValueError, match=message):
        gg.geom_boxplot(**options)


POINTS = {"x": [1, 2, 3, 4], "y": [1, 2, 3, 4], "c": ["a", None, "b", "a"]}
NUMBERS = {"x": [1, 2, 3, 4], "y": [1, 2, 3, 4], "v": [1.0, None, 3.0, 4.0]}


def _labels(svg: str) -> list[str]:
    return re.findall(r"<text[^>]*>([^<]*)</text>", svg)


def test_discrete_legend_gets_an_na_key_with_the_na_value() -> None:
    plot = gg.ggplot(POINTS, gg.aes("x", "y", color="c")) + gg.geom_point()
    svg = (plot + gg.scale_color_discrete(na_value="red")).to_svg()
    assert _labels(svg)[-3:] == ["a", "b", "NA"]
    assert svg.count("#ff0000") == 2  # the point and its key
    # lets-plot parity without na_value, or without missing values.
    assert "NA" not in _labels(plot.to_svg())
    complete = {**POINTS, "c": ["a", "b", "b", "a"]}
    no_missing = gg.ggplot(complete, gg.aes("x", "y", color="c")) + gg.geom_point()
    assert "NA" not in _labels((no_missing + gg.scale_color_discrete(na_value="red")).to_svg())


def test_manual_fill_legend_gets_an_na_key() -> None:
    plot = (
        gg.ggplot({"g": ["p", "q", "q", "s"], "c": POINTS["c"]}, gg.aes("g", fill="c"))
        + gg.geom_bar()
        + gg.scale_fill_manual(values=["#111111", "#222222"], na_value="#00ff00")
    )
    svg = plot.to_svg()
    assert _labels(svg)[-1] == "NA"
    assert svg.count("#00ff00") == 2  # the bar and its key


def test_colorbar_gets_an_na_swatch(tmp_path: Path) -> None:
    plot = gg.ggplot(NUMBERS, gg.aes("x", "y", color="v")) + gg.geom_point()
    svg = (plot + gg.scale_color_continuous(na_value="red")).to_svg()
    assert _labels(svg).count("NA") == 1
    assert svg.count("#ff0000") == 2  # the point and the swatch
    assert "NA" not in _labels(plot.to_svg())
    bottom = plot + gg.scale_color_continuous(na_value="red") + gg.theme(legend_position="bottom")
    assert _labels(bottom.to_svg()).count("NA") == 1
    path = tmp_path / "na.png"
    (plot + gg.scale_color_continuous(na_value="red")).to_png(str(path))
    assert path.read_bytes()[:4] == b"\x89PNG"


def test_values_outside_limits_get_the_na_key_too() -> None:
    data = {"x": [1, 2, 3], "y": [1, 2, 3], "c": ["a", "b", "z"], "v": [1.0, 5.0, 50.0]}
    discrete = (
        gg.ggplot(data, gg.aes("x", "y", color="c"))
        + gg.geom_point()
        + gg.scale_color_manual(values=["#111111", "#222222"], limits=["a", "b"], na_value="red")
    )
    assert _labels(discrete.to_svg())[-3:] == ["a", "b", "NA"]
    continuous = (
        gg.ggplot(data, gg.aes("x", "y", color="v"))
        + gg.geom_point()
        + gg.scale_color_gradient(limits=[0, 10], na_value="red")
    )
    svg = continuous.to_svg()
    assert _labels(svg).count("NA") == 1
    assert svg.count("#ff0000") == 2
    inside = (
        gg.ggplot(data, gg.aes("x", "y", color="v"))
        + gg.geom_point()
        + gg.scale_color_gradient(limits=[0, 100], na_value="red")
    )
    assert "NA" not in _labels(inside.to_svg())
