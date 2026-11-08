"""Continuous axis ``labels=`` without ``breaks=`` follow ggplot2.

The labels name the breaks generated for the trained scale (target count 5, the same
at any plot size); those breaks are then fixed, so HTML zoom keeps the labels on their
values. A label count that differs from the break count is a ``SpecError``.
lets-plot ignores such labels (see deviations.md).
"""

import json
import re

import ggrust as gg
import pytest
from ggrust.errors import SpecError
from test_breaks_parity import ggrust_labels

FRAME = {"x": [1.0, 2.0, 3.0, 4.5], "y": [2.0, 1.0, 4.0, 3.0], "g": ["a", "a", "b", "b"]}


def point() -> gg.PlotSpec:
    return gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point()


def test_labels_name_the_generated_breaks() -> None:
    plot = point() + gg.scale_x_continuous(labels=["one", "two", "three", "four"])
    assert ggrust_labels(plot.to_svg(), "x") == ["one", "two", "three", "four"]


def test_labels_are_the_same_at_any_size() -> None:
    labels = ["one", "two", "three", "four"]
    for size in [(200, 150), (1200, 900)]:
        plot = point() + gg.scale_x_continuous(labels=labels) + gg.ggsize(*size)
        assert ggrust_labels(plot.to_svg(), "x") == labels


def test_labels_win_over_format() -> None:
    plot = point() + gg.scale_x_continuous(labels=["a", "b", "c", "d"], format=".2f")
    assert ggrust_labels(plot.to_svg(), "x") == ["a", "b", "c", "d"]


@pytest.mark.parametrize("count", [2, 5])
def test_label_count_must_match_the_breaks(count: int) -> None:
    plot = point() + gg.scale_x_continuous(labels=[str(index) for index in range(count)])
    with pytest.raises(SpecError) as error:
        plot.to_svg()
    message = str(error.value)
    assert "scales[0].labels" in message
    assert f"{count} labels for 4 computed breaks (1, 2, 3, 4)" in message
    assert "breaks=[...]" in message


def test_free_facets_check_each_panel() -> None:
    plot = (
        point()
        + gg.facet_wrap("g", scales="free_x")
        + gg.scale_x_continuous(labels=["low", "mid", "high"])
    )
    with pytest.raises(SpecError, match=r"3 labels for \d+ computed breaks"):
        plot.to_svg()


def test_html_zoom_keeps_the_labels_on_their_breaks() -> None:
    plot = point() + gg.scale_x_continuous(labels=["one", "two", "three", "four"]) + gg.ggtb()
    found = re.search(r'class="ggrust-zoom">(.*?)</script>', plot._repr_html_())
    assert found
    (tile,) = json.loads(found.group(1))["plots"][0]["tiles"]
    fixed = tile["horizontal"]["fixed"]
    assert [item[0] for item in fixed] == [1, 2, 3, 4]
    for item, label in zip(fixed, ["one", "two", "three", "four"], strict=True):
        assert f">{label}<" in item[2]


def test_gggrid_shared_axes_label_the_shared_range() -> None:
    narrow = (
        gg.ggplot({"x": [0.0, 10.0], "y": [0.0, 1.0]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.scale_x_continuous(labels=list("abcdef"))
    )
    wide = gg.ggplot({"x": [0.0, 1000.0], "y": [0.0, 1.0]}, gg.aes("x", "y")) + gg.geom_point()
    svg = gg.gggrid([narrow, wide], ncol=1, sharex="all").to_svg()
    groups = re.findall(r'<g class="axis-x">(.*?)</g>', svg, re.S)
    assert ggrust_labels(svg, "x") == list("abcdef")
    # The labels sit where the wide plot's 0, 200, ..., 1,000 are: spread across the axis.
    narrow_x = re.findall(r'<text x="([\d.]+)"', groups[0])
    wide_x = re.findall(r'<text x="([\d.]+)"', groups[1])
    assert narrow_x == wide_x and len(narrow_x) == 6
