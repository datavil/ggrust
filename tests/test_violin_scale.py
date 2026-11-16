"""``scale`` of the violin layers is a mode name; ``geom_area_ridges(scale=)`` is a number.

lets_plot documents ``geom_violin``/``geom_sina`` ``scale`` as ``{'area', 'count',
'width'}``; ggrust typed it as a number ("multiplier of the pixel size") up to 0.9.0,
which the engine rejects.
"""

from __future__ import annotations

import inspect
from collections.abc import Callable
from typing import Any

import ggrust as gg
import pytest
from ggrust.errors import SpecError

DATA = {"g": ["a"] * 5 + ["b"] * 3, "v": [1.0, 2.0, 2.5, 3.0, 4.0, 2.0, 3.0, 3.5]}
VIOLINS = [gg.geom_violin, gg.geom_sina, gg.stat_ydensity, gg.stat_sina]


@pytest.mark.parametrize("layer", VIOLINS, ids=lambda layer: layer.__name__)
def test_scale_is_typed_and_documented_as_a_mode(layer: Callable[..., Any]) -> None:
    hint = str(inspect.signature(layer).parameters["scale"].annotation).replace("'", '"')
    assert hint == 'Literal["area", "count", "width"] | None'
    assert layer.__doc__ is not None
    assert 'scale : {"area", "count", "width"}, default="area"' in layer.__doc__
    assert "pixel size" not in layer.__doc__


@pytest.mark.parametrize("layer", VIOLINS, ids=lambda layer: layer.__name__)
@pytest.mark.parametrize("mode", ["area", "count", "width"])
def test_every_scale_mode_renders(layer: Callable[..., Any], mode: str) -> None:
    plot = gg.ggplot(DATA, gg.aes("g", "v")) + layer(scale=mode)
    assert plot.to_svg().startswith("<svg")


def test_a_number_is_rejected_with_the_parameter_path() -> None:
    plot = gg.ggplot(DATA, gg.aes("g", "v")) + gg.geom_violin(scale=1.5)  # type: ignore[arg-type]
    with pytest.raises(SpecError, match=r"layers\[0\]\.scale"):
        plot.to_svg()


def test_ridges_scale_stays_a_height_factor() -> None:
    hint = inspect.signature(gg.geom_area_ridges).parameters["scale"].annotation
    assert hint == "Number | None"
    assert gg.geom_area_ridges.__doc__ is not None
    assert "multiplicative factor applied to the height" in gg.geom_area_ridges.__doc__
    plot = gg.ggplot(DATA, gg.aes("v", "g")) + gg.geom_area_ridges(scale=2)
    assert plot.to_svg().startswith("<svg")
