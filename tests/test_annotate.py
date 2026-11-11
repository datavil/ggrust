"""``annotate()``, checked against ggplot2's semantics."""

from __future__ import annotations

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import SpecError
from ggrust.plot.core import FeatureSpecArray


def base() -> gg.PlotSpec:
    frame = {"day": [1.0, 2.0, 3.0], "sales": [10.0, 14.0, 9.0], "kind": ["a", "b", "a"]}
    return gg.ggplot(frame, gg.aes("day", "sales", color="kind")) + gg.geom_point()


def test_text_label_and_positions_train_the_axes():
    plot = base() + gg.annotate("text", x=8, y=100, label="note")
    info = plot.scale_info("x")[0]
    assert info["domain"][1] >= 8
    assert plot.scale_info("y")[0]["domain"][1] >= 100
    assert "note" in plot.to_svg()


def test_inherits_nothing_and_adds_no_legend():
    plot = base() + gg.annotate("point", x=2, y=12, size=8)
    data = plot.layer_data(1)
    assert data.height == 1
    assert plot.guide_data("color").height == 2


def test_scalars_repeat_to_the_sequence_length():
    plot = base() + gg.annotate("segment", x=[1, 2], xend=[2, 3], y=11, yend=13)
    data = plot.layer_data(1)
    assert data["y"].to_list() == [11, 11]
    assert data["xend"].to_list() == [2, 3]


@pytest.mark.parametrize("sequence", [[1, 2], (1, 2), np.array([1, 2]), pl.Series([1, 2])])
def test_accepts_any_sequence(sequence: object):
    plot = base() + gg.annotate("point", x=sequence, y=11)
    assert plot.layer_data(1)["x"].to_list() == [1, 2]


def test_vector_labels_stay_one_layer():
    annotation = gg.annotate("text", x=[1, 2], y=11, label=["a", "b"], size=5)
    assert not isinstance(annotation, FeatureSpecArray)
    svg = (base() + annotation).to_svg()
    assert ">a<" in svg
    assert ">b<" in svg


def test_style_sequence_makes_a_layer_per_row():
    plot = base() + gg.annotate("point", x=[1, 2], y=11, color=["#ff0000", "#00ff00"])
    assert len(plot.as_dict()["layers"]) == 3
    svg = plot.to_svg()
    assert "#ff0000" in svg
    assert "#00ff00" in svg


def test_rect_and_constant_styles():
    plot = base() + gg.annotate("rect", xmin=1, xmax=2, ymin=9, ymax=12, fill="#123456", alpha=0.5)
    assert "#123456" in plot.to_svg()


def test_position_free_geoms_work():
    plot = base() + gg.annotate("hline", yintercept=11, color="#abcdef")
    assert "#abcdef" in plot.to_svg()


def test_discrete_positions():
    frame = {"fruit": ["apple", "kiwi"], "n": [1.0, 2.0]}
    plot = gg.ggplot(frame, gg.aes("fruit", "n")) + gg.geom_point()
    plot += gg.annotate("text", x="kiwi", y=1.5, label="here")
    assert "here" in plot.to_svg()


def test_facets_repeat_the_annotation():
    frame = {"x": [1.0, 2.0], "y": [1.0, 2.0], "g": ["a", "b"]}
    plot = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point() + gg.facet_wrap("g")
    plot += gg.annotate("text", x=1.5, y=1.5, label="everywhere")
    assert plot.to_svg().count("everywhere") == 2


def test_tooltips_are_off_unless_given():
    assert gg.annotate("point", x=1, y=1).as_dict()["tooltips"] == "none"


@pytest.mark.parametrize(
    ("call", "message"),
    [
        (lambda: gg.annotate("txt", x=1, y=1), r"unknown geom 'txt'.*did you mean 'text'"),
        (lambda: gg.annotate("text", x=1, y=1, stat="count"), r"stat= is not accepted"),
        (
            lambda: gg.annotate("point", x=[1, 2], y=[1, 2, 3]),
            r"differ in length: x \(2\), y \(3\)",
        ),
        (
            lambda: gg.annotate("line", x=[1, 2], y=[1, 2], color=["red", "blue"]),
            r"color must be single values for geom 'line'",
        ),
    ],
)
def test_errors(call, message: str):
    with pytest.raises(SpecError, match=message):
        call()
