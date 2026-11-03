"""A later style scale that picks a mapper replaces the earlier mapper (ggplot2).

lets-plot merges scale calls key by key and lets ``scale_mapper_kind`` win over
``values``, so ``scale_color_brewer() + scale_color_manual(values)`` keeps brewer
there. ggrust follows ggplot2 for the mapper (deviations.md); other options
(name, breaks, labels, limits) still merge.
"""

import re

import ggrust as gg
from ggrust.plot.core import PlotSpec

FRAME = {"x": [1.0, 2.0, 3.0], "y": [1.0, 2.0, 3.0], "g": ["a", "b", "c"], "v": [0.0, 1.0, 2.0]}
MANUAL = {"a": "#ff0000", "b": "#00ff00", "c": "#0000ff"}
SET2 = {"#66c2a5", "#fc8d62", "#8da0cb"}


def _colors(plot: PlotSpec) -> set[str]:
    """Lowercase hex colors in the SVG output."""
    return {color.lower() for color in re.findall(r"#[0-9a-fA-F]{6}", plot.to_svg())}


def _points(color: str) -> PlotSpec:
    return gg.ggplot(FRAME, gg.aes("x", "y", color=color)) + gg.geom_point(size=8)


def test_manual_after_brewer_uses_manual_values() -> None:
    plot = (
        _points("g") + gg.scale_color_brewer(palette="Set2") + gg.scale_color_manual(values=MANUAL)
    )
    colors = _colors(plot)
    assert set(MANUAL.values()) <= colors
    assert not SET2 & colors


def test_brewer_after_manual_uses_brewer() -> None:
    plot = (
        _points("g") + gg.scale_color_manual(values=MANUAL) + gg.scale_color_brewer(palette="Set2")
    )
    colors = _colors(plot)
    assert colors >= SET2
    assert not set(MANUAL.values()) & colors


def test_identity_after_brewer_uses_identity() -> None:
    frame = {**FRAME, "hex": list(MANUAL.values())}
    plot = (
        gg.ggplot(frame, gg.aes("x", "y", color="hex"))
        + gg.geom_point(size=8)
        + gg.scale_color_brewer(palette="Set2")
        + gg.scale_color_identity()
    )
    assert set(MANUAL.values()) <= _colors(plot)


def test_gradient_after_gradient_drops_earlier_mapper_options() -> None:
    later = _points("v") + gg.scale_color_gradient(low="#000000", high="#ffffff")
    stacked = (
        _points("v")
        + gg.scale_color_gradient2(low="#ff0000", mid="#00ff00", high="#0000ff", midpoint=1.0)
        + gg.scale_color_gradient(low="#000000", high="#ffffff")
    )
    assert _colors(stacked) == _colors(later)


def test_name_and_breaks_still_merge_across_calls() -> None:
    plot = (
        _points("g")
        + gg.scale_color_discrete(name="Group", breaks=["a", "b"])
        # A list, not a dict: a dict's keys become breaks that override the first call.
        + gg.scale_color_manual(values=list(MANUAL.values()))
    )
    svg = plot.to_svg()
    assert "Group" in svg
    assert ">c<" not in svg  # breaks from the first call still limit the legend
    assert {"#ff0000", "#00ff00"} <= _colors(plot)


def test_options_without_a_mapper_keep_the_earlier_mapper() -> None:
    plot = _points("g") + gg.scale_color_manual(values=MANUAL) + gg.scale_color_discrete(name="G")
    assert set(MANUAL.values()) <= _colors(plot)


def test_size_range_after_manual_sizes_uses_the_range() -> None:
    frame = {**FRAME, "s": [1.0, 2.0, 3.0]}

    def radii(plot: PlotSpec) -> list[float]:
        return sorted(float(r) for r in re.findall(r'\br="([\d.]+)"', plot.to_svg()))

    base = gg.ggplot(frame, gg.aes("x", "y", size="g")) + gg.geom_point()
    alone = radii(base + gg.scale_size_manual(values=[1, 2, 3]))
    stacked = radii(
        base + gg.scale_size_manual(values=[6, 8, 10]) + gg.scale_size_manual(values=[1, 2, 3])
    )
    assert stacked == alone
    continuous = gg.ggplot(frame, gg.aes("x", "y", size="s")) + gg.geom_point()
    ranged = radii(continuous + gg.scale_size(range=[1, 3]))
    after_area = radii(continuous + gg.scale_size_area(max_size=10) + gg.scale_size(range=[1, 3]))
    assert after_area == ranged


def test_na_value_of_a_replaced_mapper_is_dropped() -> None:
    frame = {"x": [1.0, 2.0, 3.0, 4.0], "y": [1.0, 2.0, 3.0, 4.0], "g": ["a", "b", "c", None]}
    base = gg.ggplot(frame, gg.aes("x", "y", color="g")) + gg.geom_point(size=8)
    plot = (
        base
        + gg.scale_color_brewer(palette="Set2", na_value="#123456")
        + gg.scale_color_manual(values=MANUAL)
    )
    assert "#123456" not in _colors(plot)
