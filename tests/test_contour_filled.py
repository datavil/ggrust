"""geom_contour_filled: bands filled by an ordered discrete level (ggplot2)."""

from __future__ import annotations

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import SpecError


def _grid() -> dict[str, np.ndarray]:
    x, y = np.meshgrid(np.linspace(-3, 3, 40), np.linspace(-3, 3, 40))
    z = np.exp(-(x**2 + y**2))
    return {"x": x.ravel(), "y": y.ravel(), "z": z.ravel()}


def _plot(**options: object) -> gg.PlotSpec:
    return gg.ggplot(_grid(), gg.aes("x", "y", z="z")) + gg.geom_contour_filled(**options)


def test_level_is_an_ordered_band_label() -> None:
    frame = _plot().layer_data(0)
    level = frame["fill"]
    assert level.dtype in (pl.String, pl.Categorical, pl.Enum)
    # R: levels of ggplot2 layer_data(...)$level for this grid (pretty breaks 0..1 by 0.1).
    expected = [f"({low / 10:.1f}, {(low + 1) / 10:.1f}]" for low in range(10)]
    assert sorted(set(level.to_list())) == expected


def test_breaks_bins_and_binwidth() -> None:
    labels = set(_plot(breaks=[0.0, 0.5, 1.0]).layer_data(0)["fill"].to_list())
    assert labels == {"(0.0, 0.5]", "(0.5, 1.0]"}
    labels = set(_plot(binwidth=0.25).layer_data(0)["fill"].to_list())
    assert labels == {"(0.00, 0.25]", "(0.25, 0.50]", "(0.50, 0.75]", "(0.75, 1.00]"}
    assert len(set(_plot(bins=5).layer_data(0)["fill"].to_list())) == 5


def test_discrete_legend_lists_bands_in_order() -> None:
    svg = _plot(breaks=[0.0, 0.3, 0.6, 1.0]).to_svg()
    positions = [svg.index(label) for label in ["(0.0, 0.3]", "(0.3, 0.6]", "(0.6, 1.0]"]]
    assert positions == sorted(positions)
    # Default fill: viridis (ggplot2's scale for ordered factors), first band #440154.
    assert "#440154" in svg.lower()


def test_user_fill_scale_wins() -> None:
    svg = (_plot(breaks=[0.0, 0.5, 1.0]) + gg.scale_fill_manual(["red", "blue"])).to_svg()
    assert "#440154" not in svg.lower()


def test_geom_contourf_keeps_a_continuous_level() -> None:
    plot = gg.ggplot(_grid(), gg.aes("x", "y", z="z")) + gg.geom_contourf()
    assert plot.layer_data(0)["fill"].dtype == pl.Float64


def test_bad_breaks_raise() -> None:
    with pytest.raises(SpecError, match="at least two"):
        _plot(breaks=[0.5]).to_svg()
    with pytest.raises(SpecError, match="binwidth"):
        _plot(binwidth=-1).to_svg()


def test_narrow_ranges_get_more_digits() -> None:
    # R: ggplot2 labels z = x * y + 100 as "(100.0, 100.1]" ... "(100.9, 101.0]".
    x, y = np.meshgrid(np.linspace(0, 1, 15), np.linspace(0, 1, 15))
    data = {"x": x.ravel(), "y": y.ravel(), "z": (x * y + 100).ravel()}
    plot = gg.ggplot(data, gg.aes("x", "y", z="z")) + gg.geom_contour_filled()
    labels = sorted(set(plot.layer_data(0)["fill"].to_list()))
    assert labels[0] == "(100.0, 100.1]" and labels[-1] == "(100.9, 101.0]"
    assert len(labels) == 10
