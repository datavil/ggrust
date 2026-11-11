"""``geom_quantile`` / ``stat_quantile``: linear quantile regression (ggplot2, quantreg)."""

from __future__ import annotations

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import SpecError


def noisy() -> dict[str, np.ndarray]:
    rng = np.random.default_rng(4)
    x = rng.uniform(0, 10, 200)
    return {"x": x, "y": 2 + 0.5 * x + rng.normal(0, 1, 200) * (0.3 + 0.2 * x)}


def line_ends(plot: gg.PlotSpec, layer: int = 1) -> pl.DataFrame:
    return (
        plot.layer_data(layer)
        .group_by("group", maintain_order=True)
        .agg(
            pl.col("quantile").first(),
            pl.col("y").first().alias("y_left"),
            pl.col("y").last().alias("y_right"),
            pl.len(),
        )
    )


def test_median_line_matches_quantreg():
    """quantreg::rq(y ~ x, tau = 0.5) on four points: -1/3 + 4/3 x."""
    data = {"x": [1.0, 2, 3, 4], "y": [1.0, 3, 2, 5]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_quantile(quantiles=[0.5])
    ends = line_ends(plot, 0)
    assert ends["y_left"].to_list() == pytest.approx([1.0], abs=1e-9)
    assert ends["y_right"].to_list() == pytest.approx([5.0], abs=1e-9)


def test_default_quantiles_one_line_each_of_100_points():
    plot = gg.ggplot(noisy(), gg.aes("x", "y")) + gg.geom_point() + gg.geom_quantile()
    ends = line_ends(plot)
    assert ends["quantile"].to_list() == [0.25, 0.5, 0.75]
    assert ends["len"].to_list() == [100, 100, 100]
    # Higher quantiles sit higher at the right end.
    right = ends["y_right"].to_list()
    assert right[0] < right[1] < right[2]


def test_lines_per_group():
    data = noisy() | {"side": np.where(noisy()["x"] > 5, "right", "left")}
    plot = gg.ggplot(data, gg.aes("x", "y", color="side")) + gg.stat_quantile(quantiles=[0.5])
    assert len(line_ends(plot, 0)) == 2


def test_quantiles_outside_zero_one_are_an_error():
    plot = gg.ggplot(noisy(), gg.aes("x", "y")) + gg.geom_quantile(quantiles=[0.5, 1.0])
    with pytest.raises(SpecError, match=r"quantiles.*between 0 and 1.*got 1"):
        plot.to_svg()


@pytest.mark.parametrize(
    ("argument", "value"), [("formula", "y ~ splines::bs(x, 3)"), ("method", "rqss")]
)
def test_other_models_are_rejected(argument: str, value: str):
    with pytest.raises(SpecError, match=f"only.*{argument}|{argument}.*only"):
        gg.geom_quantile(**{argument: value})
    gg.stat_quantile(formula="y ~ x", method="rq")


def test_too_few_points_draw_nothing():
    data = {"x": [1.0, 1.0], "y": [1.0, 2.0]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_quantile()
    assert plot.layer_data(0).height == 0
    assert "<svg" in plot.to_svg()


def test_one_number_is_one_quantile():
    plot = gg.ggplot(noisy(), gg.aes("x", "y")) + gg.geom_quantile(quantiles=0.9)
    assert line_ends(plot, 0)["quantile"].to_list() == [0.9]
