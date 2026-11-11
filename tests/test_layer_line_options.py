"""Layer options once ignored: notch, closed, pad, outline_type, just, lineend, linejoin."""

from __future__ import annotations

import warnings
from pathlib import Path

import pytest
from ggrust import (
    aes,
    geom_area,
    geom_bar,
    geom_boxplot,
    geom_histogram,
    geom_line,
    geom_path,
    geom_point,
    geom_ribbon,
    geom_segment,
    ggplot,
)
from ggrust.errors import SpecError

LINE = {"x": [1, 2, 3, 4], "y": [1.0, 3.0, 2.0, 4.0]}


def svg_without_warnings(plot: object) -> str:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        return plot.to_svg()  # type: ignore[attr-defined]


def test_lineend_and_linejoin_reach_the_svg() -> None:
    svg = svg_without_warnings(
        ggplot(LINE, aes("x", "y")) + geom_path(lineend="round", linejoin="bevel")
    )
    assert 'stroke-linecap="round"' in svg and 'stroke-linejoin="bevel"' in svg
    svg = svg_without_warnings(
        ggplot(LINE, aes("x", "y"))
        + geom_segment(x=1, y=1, xend=2, yend=2, lineend="square")
        + geom_line()
    )
    assert svg.count('stroke-linecap="square"') == 1
    assert "stroke-linecap" not in svg_without_warnings(ggplot(LINE, aes("x", "y")) + geom_line())


def test_bad_lineend_suggests() -> None:
    with pytest.raises(SpecError, match="did you mean 'round'"):
        (ggplot(LINE, aes("x", "y")) + geom_path(lineend="rund")).to_svg()


def test_outline_type_picks_edges() -> None:
    band = {"x": [1, 2, 3], "low": [0.0, 1.0, 0.0], "high": [2.0, 3.0, 2.0]}
    base = ggplot(band, aes("x", ymin="low", ymax="high"))
    both = svg_without_warnings(base + geom_ribbon())
    upper = svg_without_warnings(base + geom_ribbon(outline_type="upper"))
    full = svg_without_warnings(base + geom_ribbon(outline_type="full"))
    assert both.count("<path") > upper.count("<path")
    assert full != both
    area = ggplot(LINE, aes("x", "y"))
    assert svg_without_warnings(area + geom_area()) != svg_without_warnings(
        area + geom_area(outline_type="lower")
    )
    with pytest.raises(SpecError, match="outline_type"):
        (base + geom_ribbon(outline_type="sides")).to_svg()


def test_histogram_closed_and_pad() -> None:
    data = {"v": [0.0, 1.0, 2.0, 3.0]}
    left = (
        ggplot(data, aes("v")) + geom_histogram(breaks=[0, 1, 2, 3], closed="left")
    ).layer_data()
    right = (ggplot(data, aes("v")) + geom_histogram(breaks=[0, 1, 2, 3])).layer_data()
    assert left["count"].to_list() == [1.0, 1.0, 2.0]
    assert right["count"].to_list() == [2.0, 1.0, 1.0]
    padded = (ggplot(data, aes("v")) + geom_histogram(breaks=[0, 1, 2, 3], pad=True)).layer_data()
    assert padded["count"].to_list() == [0.0, 2.0, 1.0, 1.0, 0.0]


def test_notch_draws_a_notched_outline() -> None:
    data = {"g": ["a"] * 20, "y": [float(value) for value in range(20)]}
    plain = svg_without_warnings(ggplot(data, aes("g", "y")) + geom_boxplot())
    notched = svg_without_warnings(ggplot(data, aes("g", "y")) + geom_boxplot(notch=True))
    narrow = svg_without_warnings(
        ggplot(data, aes("g", "y")) + geom_boxplot(notch=True, notchwidth=0.2)
    )
    assert plain != notched != narrow


def test_bar_just_moves_bars() -> None:
    data = {"c": ["a", "b", "b"]}
    centred = (ggplot(data, aes("c")) + geom_bar()).layer_data()
    left = (ggplot(data, aes("c")) + geom_bar(just=0)).layer_data()
    assert (left["xmin"] - centred["xmin"]).to_list() == pytest.approx([0.45, 0.45])


def test_closed_right_keeps_the_minimum_in_the_first_bin_as_ggplot2() -> None:
    data = {"x": [0, 0.5, 1, 1.5, 2, 2.5, 3, 3, 3.5, 4]}
    frame = (
        ggplot(data, aes("x")) + geom_histogram(closed="right", binwidth=1, boundary=0)
    ).layer_data()
    counts = frame.filter(frame["count"] > 0)["count"].to_list()
    assert counts == [3.0, 2.0, 3.0, 2.0]  # ggplot2 4.0.1


def test_pdf_markers_keep_butt_caps_after_round_lines(tmp_path: Path) -> None:
    import zlib

    plot = (
        ggplot(LINE, aes("x", "y"))
        + geom_line(lineend="round", linejoin="round", size=3)
        + geom_point(shape=3, size=5)
    )
    plot.to_pdf(tmp_path / "lines.pdf")
    pdf = (tmp_path / "lines.pdf").read_bytes()
    streams = []
    for chunk in pdf.split(b"stream")[1:]:
        body = chunk.split(b"endstream")[0].strip(b"\r\n")
        try:
            streams.append(zlib.decompress(body))
        except zlib.error:
            continue
    content = b"\n".join(streams)
    assert b"1 J" in content
    assert content.index(b"0 J", content.index(b"1 J")) < content.index(
        b"Do", content.index(b"1 J")
    )
