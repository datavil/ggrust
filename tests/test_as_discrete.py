"""``as_discrete``: discrete scales, level order and labels match lets_plot."""

import re
import warnings
from typing import Any

import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

FRAME = pl.DataFrame(
    {
        "x": range(5),
        "y": range(5),
        "c": [3.0, 1, 2.5, 1, 10],
        "s": ["b", "a", "c", "a", "d"],
        "w": [5.0, 1, 9, 2, None],
    }
)


def svg_texts(svg: str) -> list[str]:
    """Short text nodes in document order, without the y title (placed apart)."""
    texts = [text.strip() for text in re.findall(r">([^<>]{1,12})<", svg) if text.strip()]
    return [text for text in texts if text != "y"]


def render(module: Any, aesthetic: str, column: str, options: dict[str, Any]) -> list[str]:
    mapping = {aesthetic: module.as_discrete(column, **options)}
    if aesthetic != "x":
        mapping["x"] = "x"
    plot = module.ggplot(FRAME) + module.geom_point(module.aes(y="y", **mapping))
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        svg = plot.to_svg()
    if module is gg:
        assert not [str(warning.message) for warning in caught]
    return svg_texts(svg)


@pytest.mark.parametrize(
    ("aesthetic", "column", "options"),
    [
        ("color", "c", {}),
        ("color", "c", {"order": -1}),
        ("color", "c", {"order": 1}),
        ("color", "c", {"label": "Cl"}),
        ("color", "s", {"order": 1}),
        ("color", "s", {"order_by": "w"}),
        ("color", "s", {"order_by": "w", "order": 1}),
        ("color", "c", {"levels": [10, 1, 3, 2.5]}),
        ("color", "c", {"levels": [10, 1, 3, 2.5], "order": -1}),
        ("color", "s", {"levels": ["c", "a"]}),
        ("x", "c", {"order": 1, "label": "C"}),
    ],
)
def test_as_discrete_matches_lets_plot(
    aesthetic: str, column: str, options: dict[str, Any]
) -> None:
    assert render(gg, aesthetic, column, options) == render(lp, aesthetic, column, options)


def test_as_discrete_numeric_column_gets_a_discrete_legend() -> None:
    plot = gg.ggplot(FRAME) + gg.geom_point(gg.aes("x", "y", color=gg.as_discrete("c")))
    assert "linearGradient" not in plot.to_svg()


def test_as_discrete_order_by_a_discrete_column_is_an_error() -> None:
    mapping = gg.aes("x", "y", color=gg.as_discrete("c", order_by="s"))
    with pytest.raises(SpecError, match="needs a numeric column"):
        (gg.ggplot(FRAME) + gg.geom_point(mapping)).to_svg()


@pytest.mark.parametrize("geom", ["bar", "boxplot"])
@pytest.mark.parametrize("order", [None, 1])
def test_as_discrete_order_by_on_stat_layers_sorts_levels(geom: str, order: int | None) -> None:
    frame = pl.DataFrame({"c": [3.0, 1, 2.5, 1, 10, 3], "y": [5.0, 3, 4, 1, 2, 9]})
    options = {} if order is None else {"order": order}

    def first_texts(module: Any) -> list[str]:
        x = module.as_discrete("c", order_by="y", **options)
        mapping = module.aes(x=x) if geom == "bar" else module.aes(x=x, y="y")
        plot = module.ggplot(frame) + getattr(module, f"geom_{geom}")(mapping)
        return svg_texts(plot.to_svg())[:4]

    assert first_texts(gg) == first_texts(lp)
