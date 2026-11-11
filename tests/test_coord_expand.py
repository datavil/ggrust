"""``coord_cartesian(expand=)`` and ``coord_fixed(expand=)``, as ggplot2."""

from __future__ import annotations

import ggrust as gg
import pytest

DATA = {"fruit": ["apple", "kiwi", "pear"], "weight": [1.0, 3.0, 2.0]}


def domains(coord: gg.FeatureSpec) -> tuple[list[float], list[float]]:
    plot = gg.ggplot(DATA, gg.aes("fruit", "weight")) + gg.geom_point() + coord
    return plot.scale_info("x")[0]["domain"], plot.scale_info("y")[0]["domain"]


def test_expand_false_drops_padding_on_both_axes():
    x, y = domains(gg.coord_cartesian(expand=False))
    assert x == pytest.approx([0.0, 2.0])
    assert y == pytest.approx([1.0, 3.0])


def test_expand_true_is_the_default():
    assert domains(gg.coord_cartesian(expand=True)) == domains(gg.coord_cartesian())
    padded_x, padded_y = domains(gg.coord_cartesian())
    assert padded_x[0] < 0.0
    assert padded_y[0] < 1.0


def test_coord_fixed_and_scale_expand_are_overridden():
    plot = gg.ggplot(DATA, gg.aes("fruit", "weight")) + gg.geom_point()
    plot += gg.scale_y_continuous(expand=[0.5, 1]) + gg.coord_fixed(expand=False)
    assert plot.scale_info("y")[0]["domain"] == pytest.approx([1.0, 3.0])
