"""residual_plot (bistro) against lets-plot 4.11.

residual_plot is a Python builder in lets_plot too, so its ``as_dict()`` without the
data must equal lets_plot's, and the ``..residual..`` column must equal lets_plot's
(statsmodels OLS and lowess) row for row. lets_plot needs statsmodels for any fit,
so those comparisons skip without it; the ``lm`` fit is also checked against
numpy's least squares, which needs no extra package.
"""

import sys
import warnings
from typing import Any

import numpy as np
import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

from ggrust.bistro import residual_plot  # noqa: E402
from ggrust.errors import ColumnNotFoundError, SpecError  # noqa: E402
from lets_plot.bistro.residual import residual_plot as lp_residual_plot  # noqa: E402

RESIDUAL = "..residual.."


def _data(n: int = 80) -> dict[str, Any]:
    rng = np.random.default_rng(11)
    hours = rng.uniform(0, 10, n)
    return {
        "hours": hours,
        "score": 50 + 4 * hours + 0.6 * hours**2 + rng.normal(0, 6, n),
        "group": list(rng.choice(["a", "b", "c"], n)),
    }


def _split(spec: dict[str, Any]) -> tuple[np.ndarray, dict[str, Any]]:
    data = spec["data"]
    residuals = np.array(data[RESIDUAL], dtype=float)
    return residuals, {key: value for key, value in spec.items() if key != "data"}


def _residuals(plot) -> np.ndarray:
    return _split(plot.as_dict())[0]


OPTIONS = [
    {},
    {"deg": 0},
    {"deg": 2},
    {"deg": 3, "color_by": "group"},
    {"method": "none"},
    {"method": "loess", "span": 0.3},
    {"method": "lowess", "max_n": 40, "seed": 3},
    {"geom": "tile", "marginal": "box:r,hist:t"},
    {"geom": "hex", "bins": 12, "hline": False},
    {"geom": "density2d", "color_by": "group", "show_legend": False},
    {"geom": "none", "marginal": "none"},
    {"color": "red", "size": 2, "alpha": 0.5, "binwidth": [0.5, 2.0]},
]


@pytest.mark.parametrize("options", OPTIONS, ids=str)
def test_matches_lets_plot(options: dict[str, Any]) -> None:
    if options.get("method", "lm") != "none":
        pytest.importorskip("statsmodels")
        pytest.importorskip("scipy")
    ours, our_spec = _split(residual_plot(_data(), "hours", "score", **options).as_dict())
    theirs, their_spec = _split(lp_residual_plot(_data(), "hours", "score", **options).as_dict())
    assert our_spec == their_spec
    np.testing.assert_allclose(ours, theirs, rtol=0, atol=1e-9)


@pytest.mark.parametrize("deg", [0, 1, 2, 4])
def test_lm_is_least_squares(deg: int) -> None:
    data = _data()
    expected = np.polynomial.polynomial.Polynomial.fit(data["hours"], data["score"], deg)
    residuals = _residuals(residual_plot(data, "hours", "score", deg=deg))
    np.testing.assert_allclose(residuals, data["score"] - expected(data["hours"]), atol=1e-8)


def test_groups_fit_apart_in_first_appearance_order() -> None:
    data = _data()
    spec = residual_plot(data, "hours", "score", color_by="group").as_dict()
    frame = pl.DataFrame(spec["data"]) if isinstance(spec["data"], dict) else spec["data"]
    order = list(dict.fromkeys(data["group"]))
    assert list(dict.fromkeys(frame["group"].to_list())) == order
    for group in order:
        part = frame.filter(pl.col("group") == group)
        fit = np.polynomial.polynomial.Polynomial.fit(part["hours"], part["score"], 1)
        expected = part["score"].to_numpy() - fit(part["hours"].to_numpy())
        np.testing.assert_allclose(part[RESIDUAL].to_numpy(), expected, atol=1e-8)


def test_constant_x_fits_the_mean() -> None:
    data = {"x": [2.0, 2.0, 2.0, 2.0], "y": [1.0, 2.0, 3.0, 6.0]}
    residuals = _residuals(residual_plot(data, "x", "y", deg=2))
    np.testing.assert_allclose(residuals, [-2.0, -1.0, 0.0, 3.0], atol=1e-12)


def test_missing_rows_are_left_out() -> None:
    data = pl.DataFrame(
        {
            "x": [1.0, None, 3.0, 4.0, float("nan"), 6.0, 7.0],
            "y": [1.0, 2.0, None, 4.0, 5.0, float("inf"), 6.0],
        }
    )
    residuals = _residuals(residual_plot(data, "x", "y"))
    assert len(residuals) == 3 and np.isfinite(residuals).all()


def test_numpy_integer_degree() -> None:
    data = _data()
    np.testing.assert_allclose(
        _residuals(residual_plot(data, "hours", "score", deg=np.int64(2))),
        _residuals(residual_plot(data, "hours", "score", deg=2)),
    )


def test_text_column_is_an_error() -> None:
    with pytest.raises(SpecError, match=r"'y' holds String, not numbers"):
        residual_plot({"x": [1.0, 2.0], "y": ["a", "b"]}, "x", "y")


def test_polars_input_and_titles() -> None:
    spec = residual_plot(pl.DataFrame(_data()), "hours", "score").as_dict()
    assert spec["mapping"] == {"x": "hours", "y": RESIDUAL}
    assert spec["guides"]["y"]["title"] == "score residual"
    none = residual_plot(_data(), "hours", "score", method="none").as_dict()
    assert none["guides"]["y"]["title"] == "score"


def test_renders() -> None:
    svg = residual_plot(_data(), "hours", "score", color_by="group").to_svg()
    assert "score residual" in svg and "stroke-dasharray" in svg


@pytest.mark.parametrize(
    ("options", "match"),
    [
        ({"method": "lme"}, r"unknown method 'lme'.*did you mean 'lm'"),
        ({"geom": "pont"}, r"unknown geom 'pont'.*did you mean 'point'"),
        ({"deg": -1}, r"deg=-1.*whole number >= 0"),
        ({"deg": 1.5}, r"deg=1.5"),
        ({"marginal": "violin:r"}, r"violin"),
    ],
)
def test_bad_options(options: dict[str, Any], match: str) -> None:
    with pytest.raises(SpecError, match=match):
        residual_plot(_data(), "hours", "score", **options)


def test_one_row_group_is_an_error() -> None:
    data = {"x": [1.0, 2.0, 3.0], "y": [1.0, 2.0, 4.0], "g": ["a", "a", "b"]}
    with pytest.raises(SpecError, match="at least two rows"):
        residual_plot(data, "x", "y", color_by="g")


def test_unknown_column() -> None:
    with pytest.raises(ColumnNotFoundError, match=r"'hour'.*did you mean 'hours'"):
        residual_plot(_data(), "hour", "score")


def test_loess_without_statsmodels(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(sys.modules, "statsmodels", None)
    monkeypatch.setitem(sys.modules, "statsmodels.api", None)
    with pytest.raises(SpecError, match=r"needs numpy, statsmodels and scipy.*method='lm'"):
        residual_plot(_data(), "hours", "score", method="loess")
