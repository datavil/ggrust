"""Position scale ``oob`` and ``minor_breaks`` (ggplot2)."""

from __future__ import annotations

import re
import warnings

import pytest
from ggrust import (
    aes,
    element_line,
    geom_point,
    ggplot,
    scale_x_continuous,
    scale_x_discrete,
    scale_x_log10,
    scale_y_continuous,
    theme,
)
from ggrust.errors import SpecError

DATA = {"x": [1, 2, 3, 4, 5, 6, 7, 8], "y": [2, 9, 4, 12, 6, 7, 15, 3]}
MINOR = theme(panel_grid_minor_x=element_line(color="#ff00ff"))


def svg(*specs: object) -> str:
    plot = ggplot(DATA, aes("x", "y")) + geom_point()
    for spec in specs:
        plot = plot + spec
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return plot.to_svg()


def circles(out: str) -> int:
    return len(re.findall(r"<circle", out))


def minor_lines(out: str) -> int:
    return len(re.findall(r'stroke="#ff00ff"', out))


def test_oob_censor_squish_keep() -> None:
    assert circles(svg(scale_y_continuous(limits=[3, 10]))) == 5
    assert circles(svg(scale_y_continuous(limits=[3, 10], oob="squish"))) == 8
    assert circles(svg(scale_y_continuous(limits=[3, 10], oob="keep"))) == 8
    assert svg(scale_y_continuous(limits=[3, 10], oob="squish")) != svg(
        scale_y_continuous(limits=[3, 10], oob="keep")
    )


def test_minor_breaks_replace_the_halfway_lines() -> None:
    default = minor_lines(svg(MINOR))
    assert default > 0
    assert minor_lines(svg(MINOR, scale_x_continuous(minor_breaks=[1.5, 6.25]))) == 2
    assert minor_lines(svg(MINOR, scale_x_continuous(minor_breaks=[]))) == 0
    # Data units on a transformed axis.
    assert minor_lines(svg(MINOR, scale_x_log10(minor_breaks=[2, 5]))) == 2


def test_errors() -> None:
    with pytest.raises(SpecError, match="did you mean 'squish'"):
        scale_y_continuous(oob="sqush")  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="minor_breaks"):
        scale_x_continuous(minor_breaks=[1, float("nan")])
    with pytest.raises(SpecError, match="minor_breaks"):
        scale_x_continuous(minor_breaks="1")  # type: ignore[arg-type]
    plot = ggplot({"x": ["a", "b"], "y": [1, 2]}, aes("x", "y")) + geom_point()
    plot = plot + scale_x_discrete() + scale_x_continuous(oob="keep")
    with pytest.warns(match="applies to continuous axes"):
        plot.to_svg()


def test_minor_breaks_skip_repeats_and_major_breaks() -> None:
    out = svg(MINOR, scale_x_continuous(breaks=[2, 4, 6], minor_breaks=[2, 2.5, 4, 3, 3]))
    assert minor_lines(out) == 2
