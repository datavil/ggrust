"""``geom_smooth``: ``fullrange``, ``formula`` and ``method_args``, checked against ggplot2."""

from __future__ import annotations

import ggrust as gg
import pytest
from ggrust.errors import SpecError

DATA = {"x": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], "y": [2.0, 3.0, 5.0, 4.0, 7.0, 8.0]}


def smooth(**options: object) -> gg.PlotSpec:
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_smooth(n=5, **options)
    return plot + gg.expand_limits(x=[0, 10])


def test_fullrange_polynomial_matches_ggplot2():
    # R: layer_data(ggplot(d, aes(x, y)) + geom_smooth(method = "lm",
    #    formula = y ~ poly(x, 2), fullrange = TRUE, n = 5) + expand_limits(x = c(0, 10)))
    data = smooth(method="lm", formula="y ~ poly(x, 2)", fullrange=True).layer_data(0)
    assert data["x"].to_list() == pytest.approx([0.0, 2.5, 5.0, 7.5, 10.0])
    assert data["y"].to_list() == pytest.approx([1.4, 3.525, 6.5428571, 10.4535714, 15.2571429])
    assert data["ymin"].to_list() == pytest.approx(
        [-3.9288726, 1.834054, 4.8919199, 3.3627411, -4.5076567], abs=1e-6
    )


def test_without_fullrange_the_curve_spans_the_data():
    data = smooth(method="lm").layer_data(0)
    assert (data["x"].min(), data["x"].max()) == (1.0, 6.0)


def test_loess_fullrange_stops_at_the_data():
    data = smooth(method="loess", fullrange=True, span=0.9).layer_data(0)
    assert data["x"].min() >= 1.0
    assert data["x"].max() <= 6.0


def test_formula_equals_deg():
    with_formula = smooth(method="lm", formula="y~poly(x, degree = 2)").layer_data(0)
    with_deg = smooth(method="lm", deg=2).layer_data(0)
    assert with_formula["y"].to_list() == pytest.approx(with_deg["y"].to_list())
    linear = smooth(method="lm", formula="y ~ x").layer_data(0)
    assert linear["y"].to_list() == pytest.approx(smooth(method="lm").layer_data(0)["y"].to_list())


def test_method_args_span_equals_span():
    with_args = smooth(method="loess", method_args={"span": 0.9}).layer_data(0)
    with_span = smooth(method="loess", span=0.9).layer_data(0)
    assert with_args["y"].to_list() == pytest.approx(with_span["y"].to_list())


@pytest.mark.parametrize(
    ("options", "message"),
    [
        ({"formula": "y ~ log(x)"}, r"layers\[0\]\.formula.*unsupported formula 'y ~ log\(x\)'"),
        ({"method": "loess", "formula": "y ~ poly(x, 2)"}, r"with method='lm'"),
        ({"formula": "y ~ poly(x, 2)", "deg": 3}, r"deg= sets another"),
        ({"method_args": {"span": 0.5}}, r"unknown method argument 'span'; accepted: none"),
        ({"method": "loess", "method_args": {"degree": 2}}, r"accepted: span"),
        ({"method": "loess", "span": 0.5, "method_args": {"span": 0.6}}, r"span is given twice"),
    ],
)
def test_errors(options: dict[str, object], message: str):
    with pytest.raises(SpecError, match=message):
        smooth(**options).to_svg()
