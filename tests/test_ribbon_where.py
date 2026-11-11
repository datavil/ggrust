"""geom_ribbon's `where` aesthetic (plotnine): fill only where a condition holds."""

from __future__ import annotations

import math
import re

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError

XS = [index / 10 for index in range(63)]
FRAME = pl.DataFrame(
    {"x": XS, "a": [math.sin(x) for x in XS], "b": [math.cos(x) for x in XS]}
).with_columns(above=pl.col("a") > pl.col("b"))


def _bands(svg: str) -> list[str]:
    return re.findall(r'<path d="(M[^"]*Z)"[^>]*fill="#ff0000"', svg)


def _ribbon(**mapping: str) -> str:
    plot = gg.ggplot(FRAME, gg.aes("x", ymin="b", ymax="a", **mapping))
    return (plot + gg.geom_ribbon(fill="red", color="blue")).to_svg()


def test_where_fills_one_band_between_the_crossings() -> None:
    whole = _bands(_ribbon())
    cut = _bands(_ribbon(where="above"))
    assert len(whole) == 1 and len(cut) == 1
    # sin > cos from pi/4 to 5 pi/4: rows x = 0.8 .. 3.9 (32) plus two crossings.
    vertices = cut[0].count("L") + 1
    assert vertices == 2 * (32 + 2)


def test_where_does_not_split_the_edge_lines() -> None:
    edges = re.findall(r'<path d="M[^"]*"[^>]*stroke="#0000ff"', _ribbon(where="above"))
    assert len(edges) == 2


def test_where_needs_a_boolean_column() -> None:
    with pytest.raises(SpecError, match=r"mapping.where.*boolean column.*numeric column"):
        _ribbon(where="a")
