"""ggplot2's ``stat_ellipse``: confidence ellipses per group, against R.

The reference is ``tests/fixtures/ellipse_reference.json``, written by
``generate_ellipse_reference.R`` (ggplot2 3.5.2 ``layer_data`` of the same
plots, MASS 7.3 ``cov.trob``). Every ellipse point must match R to 1e-6, which
needs ``cov.trob``'s iterations to match exactly.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError

REFERENCE: dict[str, Any] = json.loads(
    (Path(__file__).parent / "fixtures" / "ellipse_reference.json").read_text()
)
FRAME = pl.DataFrame(REFERENCE["data"])


@pytest.mark.parametrize("case", sorted(REFERENCE["cases"]))
def test_points_match_r(case: str) -> None:
    expected = REFERENCE["cases"][case]
    plot = gg.ggplot(FRAME, gg.aes("x", "y", group="g")) + gg.stat_ellipse(
        type=expected["type"], level=expected["level"], segments=expected["segments"]
    )
    frame = plot.layer_data(0)
    groups = expected["groups"]
    # Group c has 3 points: no ellipse, as in R.
    assert frame.height == sum(len(group["x"]) for group in groups.values())
    for index, group in enumerate(groups.values()):
        rows = frame.filter(pl.col("group") == index)
        assert rows["x"].to_list() == pytest.approx(group["x"], abs=1e-6)
        assert rows["y"].to_list() == pytest.approx(group["y"], abs=1e-6)


def test_polygon_geom_fills() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y", fill="g")) + gg.stat_ellipse(geom="polygon", alpha=0.3)
    svg = plot.to_svg()
    assert isinstance(svg, str)
    assert svg.count("fill-opacity") >= 2 or "fill=" in svg
    assert plot.layer_data(0).height == 2 * 52


def test_too_few_points_draw_nothing() -> None:
    data = {"x": [1.0, 2.0, 3.0], "y": [1.0, 3.0, 2.0]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.stat_ellipse()
    assert plot.layer_data(0).height == 0
    assert isinstance(plot.to_svg(), str)


def test_degenerate_shape_draws_nothing() -> None:
    # Constant x: R's chol() fails (an error there); ggrust draws no ellipse.
    data = {"x": [2.0, 2.0, 2.0, 2.0, 2.0], "y": [2.0, 4.0, 6.0, 8.0, 11.0]}
    for kind in ("t", "norm", "euclid"):
        plot = gg.ggplot(data, gg.aes("x", "y")) + gg.stat_ellipse(type=kind)
        assert plot.layer_data(0).height == 0


@pytest.mark.parametrize(
    ("options", "match"),
    [({"type": "normal"}, "unknown type 'normal'"), ({"segments": 0}, "segments")],
)
def test_bad_options_fail_early(options: dict[str, Any], match: str) -> None:
    with pytest.raises(SpecError, match=match):
        gg.stat_ellipse(**options)


@pytest.mark.parametrize(
    ("options", "match"),
    [({"level": 1.5}, "between 0 and 1"), ({"type": "euclid", "level": -1}, "above 0")],
)
def test_bad_level_is_a_spec_error(options: dict[str, Any], match: str) -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.stat_ellipse(**options)
    with pytest.raises(SpecError, match=match):
        plot.to_svg()


def test_needs_x_and_y() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x")) + gg.stat_ellipse()
    with pytest.raises(SpecError, match="y"):
        plot.to_svg()


def test_numpy_segments_are_accepted() -> None:
    import numpy as np

    layer = gg.stat_ellipse(segments=np.int64(20))
    plot = gg.ggplot(FRAME, gg.aes("x", "y", group="g")) + layer
    assert plot.layer_data(0).height == 2 * 21
