"""ggforce marks (``geom_mark_hull``, ``geom_mark_ellipse``, ``geom_mark_rect``).

Checked on the drawn SVG: every point of a group lies inside its mark with at least
``expand`` of room, callout boxes stay clear of every mark, ``filter`` and facets
work, connectors follow their options, and bad options raise typed errors. The
outline algorithms themselves are unit-tested in Rust (``geom/mark_shape.rs``,
``geom/mark_label.rs``); the reference renders are ggforce 0.5's (see deviations.md).
"""

from __future__ import annotations

import math
import re
from typing import TYPE_CHECKING

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import SpecError, UnsupportedValueError

if TYPE_CHECKING:
    from collections.abc import Callable

PIXELS_PER_MM = 96 / 25.4
Point = tuple[float, float]


def _clusters(seed: int = 4) -> pl.DataFrame:
    """Three UMAP-like clusters, one of them a crescent."""
    generator = np.random.default_rng(seed)
    angle = generator.uniform(0, math.pi, 150)
    return pl.DataFrame(
        {
            "x": np.concatenate(
                [
                    generator.normal(0, 1, 200),
                    generator.normal(6, 0.7, 120),
                    5 + 2.5 * np.cos(angle) + generator.normal(0, 0.15, 150),
                ]
            ),
            "y": np.concatenate(
                [
                    generator.normal(0, 0.8, 200),
                    generator.normal(4, 0.9, 120),
                    -3 + 2.5 * np.sin(angle) + generator.normal(0, 0.15, 150),
                ]
            ),
            "cluster": ["T cells"] * 200 + ["B cells"] * 120 + ["Monocytes"] * 150,
        }
    )


def _panel_svg(svg: str) -> list[str]:
    """The SVG of each panel group."""
    return re.findall(r'<g class="panel"[^>]*>(.*?)\n</g>\n(?=<g class|</svg>|<g clip)', svg, re.S)


def _closed_paths(svg: str) -> list[tuple[str, list[Point]]]:
    """Filled closed paths: (fill color, vertices)."""
    paths = []
    for match in re.finditer(r'<path d="([^"]+)"([^>]*)/>', svg):
        data, attributes = match.groups()
        fill = re.search(r'fill="(#[0-9a-f]{6})"', attributes)
        if not data.endswith("Z") or fill is None:
            continue
        numbers = [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?", data)]
        paths.append((fill.group(1), list(zip(numbers[::2], numbers[1::2], strict=True))))
    return paths


def _circles(svg: str) -> list[tuple[str, Point]]:
    """Drawn points: (fill color, centre)."""
    return [
        (fill, (float(x), float(y)))
        for x, y, fill in re.findall(
            r'<circle cx="([\d.]+)" cy="([\d.]+)" r="[\d.]+" fill="(#[0-9a-f]{6})"', svg
        )
    ]


def _label_boxes(svg: str) -> list[tuple[float, float, float, float]]:
    """Callout backgrounds in the panels: (x, y, width, height)."""
    boxes = []
    for panel in _panel_svg(svg):
        for match in re.finditer(
            r'<g fill="#ffffff"[^>]*>\n<rect x="([\d.]+)" y="([\d.]+)" '
            r'width="([\d.]+)" height="([\d.]+)"',
            panel,
        ):
            boxes.append(tuple(float(value) for value in match.groups()))
    return boxes


def _contains(polygon: list[Point], point: Point) -> bool:
    inside = False
    previous = polygon[-1]
    for current in polygon:
        if (current[1] > point[1]) != (previous[1] > point[1]):
            x = previous[0] + (point[1] - previous[1]) / (current[1] - previous[1]) * (
                current[0] - previous[0]
            )
            if point[0] < x:
                inside = not inside
        previous = current
    return inside


def _distance_to_outline(polygon: list[Point], point: Point) -> float:
    best = math.inf
    for index, start in enumerate(polygon):
        end = polygon[(index + 1) % len(polygon)]
        dx, dy = end[0] - start[0], end[1] - start[1]
        length = dx * dx + dy * dy
        t = (
            0.0
            if length == 0
            else ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / length
        )
        t = min(1.0, max(0.0, t))
        best = min(best, math.hypot(point[0] - start[0] - t * dx, point[1] - start[1] - t * dy))
    return best


MARKS: dict[str, Callable[..., gg.LayerSpec]] = {
    "hull": gg.geom_mark_hull,
    "ellipse": gg.geom_mark_ellipse,
    "rect": gg.geom_mark_rect,
}


def _plot(mark: Callable[..., gg.LayerSpec], **options: object) -> gg.PlotSpec:
    return (
        gg.ggplot(_clusters(), gg.aes("x", "y"))
        + mark(gg.aes(fill="cluster"), **options)
        + gg.geom_point(gg.aes(color="cluster"), size=1)
        + gg.expand_limits(x=[-8, 14], y=[-9, 10])
        + gg.ggsize(700, 500)
    )


@pytest.mark.parametrize("kind", list(MARKS))
@pytest.mark.parametrize("expand", [5, 2, 0])
def test_marks_enclose_every_point_with_the_expansion(kind: str, expand: float) -> None:
    svg = _plot(MARKS[kind], expand=expand).to_svg()
    marks = _closed_paths(svg)
    assert len(marks) == 3
    by_color = dict(marks)
    points = _circles(svg)
    assert len(points) == 470
    room = expand * PIXELS_PER_MM
    for color, point in points:
        outline = by_color[color]
        distance = _distance_to_outline(outline, point)
        # Points sit on the outline when expand is 0 (hull vertices, rectangle
        # sides); SVG coordinates are rounded to 0.01 px.
        assert _contains(outline, point) or distance < 0.05, (kind, point)
        if expand > 0:
            assert distance >= room - 0.1, (kind, distance, room)


@pytest.mark.parametrize("kind", list(MARKS))
def test_callouts_stay_clear_of_the_marks(kind: str) -> None:
    plot = (
        gg.ggplot(_clusters(), gg.aes("x", "y"))
        + MARKS[kind](gg.aes(fill="cluster", label="cluster"))
        + gg.expand_limits(x=[-10, 16], y=[-10, 12])
        + gg.ggsize(800, 600)
    )
    svg = plot.to_svg()
    boxes = _label_boxes(svg)
    assert len(boxes) == 3
    labels = re.findall(r'font-weight="bold" fill="#000000">([^<]+)<', svg)
    assert sorted(labels) == ["B cells", "Monocytes", "T cells"]
    buffer = 10 * PIXELS_PER_MM
    for x, y, width, height in boxes:
        corners = [(x, y), (x + width, y), (x + width, y + height), (x, y + height)]
        for _, outline in _closed_paths(svg):
            assert not any(_contains(outline, corner) for corner in corners)
            nearest = min(_distance_to_outline(corners, vertex) for vertex in outline)
            assert nearest >= buffer, nearest
    for index, first in enumerate(boxes):
        for second in boxes[index + 1 :]:
            apart_x = first[0] + first[2] <= second[0] or second[0] + second[2] <= first[0]
            apart_y = first[1] + first[3] <= second[1] or second[1] + second[3] <= first[1]
            assert apart_x or apart_y


def test_connectors_follow_their_options() -> None:
    def connectors(**options: object) -> list[str]:
        plot = (
            gg.ggplot(_clusters(), gg.aes("x", "y"))
            + gg.geom_mark_hull(gg.aes(fill="cluster", label="cluster"), **options)
            + gg.expand_limits(x=[-10, 16], y=[-10, 12])
        )
        panel = "".join(_panel_svg(plot.to_svg()))
        return re.findall(r'<path d="([^"Z]+)" fill="none" stroke="#000000"', panel)

    def vertices(path: str) -> int:
        return path.count("L") + 1

    # elbow + border line: 4 vertices; straight: 3; no border: one fewer.
    assert [vertices(path) for path in connectors()] == [4, 4, 4]
    assert [vertices(path) for path in connectors(con_type="straight")] == [3, 3, 3]
    assert [vertices(path) for path in connectors(con_border="none")] == [3, 3, 3]
    assert connectors(con_type="none") == []
    plot = (
        gg.ggplot(_clusters(), gg.aes("x", "y"))
        + gg.geom_mark_rect(gg.aes(label="cluster"), con_border="all", con_color="#ff0000")
        + gg.expand_limits(x=[-10, 16], y=[-10, 12])
    )
    assert plot.to_svg().count('<g fill="#ffffff" stroke="#ff0000"') == 3


def test_descriptions_wrap_under_the_label() -> None:
    frame = _clusters().with_columns(
        note=pl.when(pl.col("cluster") == "T cells")
        .then(pl.lit("CD3 positive lymphocytes, the largest population in this sample"))
        .otherwise(None)
    )
    plot = (
        gg.ggplot(frame, gg.aes("x", "y"))
        + gg.geom_mark_hull(gg.aes(fill="cluster", label="cluster", description="note"))
        + gg.expand_limits(x=[-10, 16], y=[-10, 12])
        + gg.ggsize(800, 600)
    )
    svg = plot.to_svg()
    lines = re.findall(r'font-size="16" xml:space="preserve" fill="#000000">([^<]+)<', svg)
    # The description wraps to the 50 mm minimum width: several plain lines.
    assert len(lines) >= 3
    assert " ".join(lines).startswith("CD3 positive")
    # A missing description draws no text, only the label.
    assert len(re.findall(r'font-weight="bold"', svg)) == 3


def test_filter_outlines_only_the_kept_rows() -> None:
    frame = _clusters().with_columns(keep=pl.col("cluster") != "Monocytes")
    plot = (
        gg.ggplot(frame, gg.aes("x", "y"))
        + gg.geom_mark_ellipse(gg.aes(fill="cluster", filter="keep"))
        + gg.expand_limits(x=[-8, 14], y=[-9, 10])
    )
    assert len(_closed_paths(plot.to_svg())) == 2
    numbers = frame.with_columns(keep=pl.col("x") > 0)
    with pytest.raises(SpecError, match=r"mapping\.filter.*boolean column"):
        (
            gg.ggplot(numbers.with_columns(keep=pl.col("x")), gg.aes("x", "y"))
            + gg.geom_mark_hull(gg.aes(filter="keep"))
        ).to_svg()


def test_marks_are_drawn_per_facet_panel() -> None:
    frame = _clusters().with_columns(
        sample=pl.when(pl.int_range(pl.len()) % 2 == 0).then(pl.lit("a")).otherwise(pl.lit("b"))
    )
    plot = (
        gg.ggplot(frame, gg.aes("x", "y"))
        + gg.geom_mark_hull(gg.aes(fill="cluster", label="cluster"))
        + gg.facet_wrap("sample")
        + gg.expand_limits(x=[-12, 18], y=[-12, 14])
        + gg.ggsize(1000, 500)
    )
    panels = _panel_svg(plot.to_svg())
    assert len(panels) == 2
    for panel in panels:
        assert len(_closed_paths(panel)) == 3


def test_single_points_and_lines_get_rounded_marks() -> None:
    frame = {"x": [0, 5, 6, 7], "y": [0, 5, 5, 5], "g": ["a", "b", "b", "b"]}
    for mark in MARKS.values():
        plot = (
            gg.ggplot(frame, gg.aes("x", "y"))
            + mark(gg.aes(fill="g"))
            + gg.expand_limits(x=[-5, 12], y=[-5, 10])
        )
        marks = _closed_paths(plot.to_svg())
        assert len(marks) == 2
        for _, outline in marks:
            assert len(outline) >= 8


ERRORS: list[tuple[Callable[[], object], type[Exception], str]] = [
    (lambda: gg.geom_mark_hull(concavity=0), SpecError, "concavity must be a positive number"),
    (lambda: gg.geom_mark_hull(concavity="2"), UnsupportedValueError, "expected a positive number"),
    (lambda: gg.geom_mark_hull(expand=-1), SpecError, "finite, non-negative length in millimetres"),
    (lambda: gg.geom_mark_rect(radius="2mm"), UnsupportedValueError, "length in millimetres"),
    (lambda: gg.geom_mark_ellipse(con_type="elbo"), SpecError, "did you mean 'elbow'"),
    (lambda: gg.geom_mark_rect(con_border="both"), SpecError, "unknown con_border 'both'"),
    (lambda: gg.geom_mark_ellipse(tol=0), SpecError, "tol must be a positive number"),
    (lambda: gg.geom_mark_hull(expand=math.inf), SpecError, "finite, non-negative length"),
]


@pytest.mark.parametrize(("make", "error", "message"), ERRORS)
def test_bad_options_fail_early(
    make: Callable[[], object], error: type[Exception], message: str
) -> None:
    with pytest.raises(error, match=re.escape(message)):
        make()


@pytest.mark.parametrize(
    ("options", "message"),
    [
        ({"n": 2}, r"layers\[0\]\.n': n must be a whole number of ellipse points"),
        (
            {"label_fontsize": [10, 11, 12]},
            r"layers\[0\]\.label_fontsize': label_fontsize takes one value, or two",
        ),
        ({"label_colour": "nocolor"}, r"layers\[0\]\.label_colour"),
        ({"con_linetype": "wavy"}, r"layers\[0\]\.con_linetype"),
        ({"con_size": -1}, r"con_size must be a non-negative line width"),
        ({"label_lineheight": -3}, r"label_lineheight must be a non-negative"),
        ({"label_margin": [1, 2]}, r"label_margin takes one length or four"),
    ],
)
def test_engine_errors_name_the_option(options: dict[str, object], message: str) -> None:
    plot = gg.ggplot(_clusters(), gg.aes("x", "y")) + gg.geom_mark_ellipse(
        gg.aes(label="cluster"), **options
    )
    with pytest.raises(SpecError, match=message):
        plot.to_svg()
