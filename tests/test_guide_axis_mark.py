"""``guide_axis_mark()``: leader-line labels for a subset of an axis's breaks (plan 9.9).

The scale's ``breaks=`` picks the labelled categories; the guide spreads their
labels along the axis (``"nearest"``: at their breaks when there is room, as
ComplexHeatmap ``anno_mark``; ``"even"``: equal slots) and links each break's true
position to its label with a three-part line. Checked on every side, under
``coord_flip()``, on side panel axes, in an aligned ``gggrid`` and with the HTML zoom.
"""

from __future__ import annotations

import itertools
import re
import warnings
from typing import TYPE_CHECKING

import ggrust as gg
import pytest
from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path

GENES = [f"gene{index:02d}" for index in range(30)]
CELLS = [f"cell{index}" for index in range(6)]
HEATMAP = {
    "gene": [gene for gene in GENES for _ in CELLS],
    "cell": [cell for _ in GENES for cell in CELLS],
    "expr": [float((row * 7 + column * 3) % 11) for row in range(30) for column in range(6)],
}
# Five crowded rows at the bottom, two lone rows higher up.
MARKED = ["gene00", "gene01", "gene02", "gene03", "gene04", "gene15", "gene27"]

Link = tuple[tuple[float, float], ...]


def _heatmap(**scale: object) -> gg.PlotSpec:
    """The heatmap with ``MARKED`` labelled on its y axis by ``guide_axis_mark()``."""
    guide = scale.pop("guide", gg.guide_axis_mark())
    return (
        gg.ggplot(HEATMAP, gg.aes("cell", "gene", fill="expr"))
        + gg.geom_tile()
        + gg.scale_y_discrete(breaks=MARKED, limits=GENES, guide=guide, **scale)
        + gg.ggsize(500, 400)
    )


def _render(plot: gg.PlotSpec) -> str:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        return plot.to_svg()


def _group(svg: str, name: str) -> str:
    found = re.search(rf'<g class="{name}">(.*?)</g>\n</g>', svg, re.S)
    assert found, name
    return found.group(1)


def _links(group: str) -> list[Link]:
    """The four-point link paths of an axis group."""
    links = []
    for path in re.findall(r'<path d="([^"]+)"', group):
        numbers = [float(number) for number in re.findall(r"-?[\d.]+", path)]
        points = tuple(zip(numbers[::2], numbers[1::2], strict=True))
        if len(points) == 4:
            links.append(points)
    return links


def _labels(group: str) -> list[tuple[str, float, float]]:
    """(text, x, y) of each label of an axis group."""
    return [
        (text, float(x), float(y))
        for x, y, text in re.findall(
            r'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*>([^<]+)</text>', group
        )
    ]


def test_guide_spec_and_scale_option() -> None:
    guide = gg.guide_axis_mark(spread="even", link_width=30, padding=2)
    assert guide.as_dict() == {
        "name": "axis_mark",
        "spread": "even",
        "link_width": 30,
        "padding": 2,
    }
    scale = gg.scale_y_discrete(breaks=["a"], guide=gg.guide_axis_mark())
    assert scale.as_dict()["guide"] == {"name": "axis_mark"}


@pytest.mark.parametrize(
    ("options", "error", "message"),
    [
        ({"spread": "evn"}, SpecError, "did you mean 'even'"),
        ({"spread": 1}, UnsupportedValueError, "expected one of 'nearest', 'even'"),
        ({"link_width": -1}, UnsupportedValueError, "link_width=-1"),
        ({"padding": True}, UnsupportedValueError, "a number of 0 or more"),
        ({"padding": float("nan")}, UnsupportedValueError, "padding=nan"),
    ],
)
def test_python_options_fail_early(
    options: dict[str, object], error: type[Exception], message: str
) -> None:
    with pytest.raises(error, match=re.escape(message)):
        gg.guide_axis_mark(**options)  # ty: ignore[invalid-argument-type]


def test_rust_checks_raw_guide_options_with_their_path() -> None:
    guide = FeatureSpec("guide", name="axis_mark", spred="even")
    with pytest.raises(SpecError, match=re.escape("did you mean 'spread'")) as raised:
        _heatmap(guide=guide).to_svg()
    assert "guide.spred" in str(raised.value)


def test_nearest_keeps_lone_labels_at_their_breaks_and_spreads_crowded_ones() -> None:
    group = _group(_render(_heatmap()), "axis-y")
    links, labels = _links(group), _labels(group)
    assert [text for text, _, _ in labels] == MARKED
    assert len(links) == len(MARKED)
    # Each link leaves its row horizontally and enters its label horizontally.
    for link in links:
        assert link[0][1] == link[1][1] and link[2][1] == link[3][1]
        assert link[0][0] > link[1][0] > link[2][0] > link[3][0]  # outward, to the left
    starts = {text: link[0][1] for (text, _, _), link in zip(labels, links, strict=True)}
    ends = {text: link[3][1] for (text, _, _), link in zip(labels, links, strict=True)}
    # Lone rows: straight links.
    for text in ("gene15", "gene27"):
        assert ends[text] == pytest.approx(starts[text], abs=0.01)
    # Crowded rows: pushed apart to at least a label line plus the padding (4 px),
    # order kept (the y axis grows up the screen).
    crowded = [ends[text] for text in MARKED[:5]]
    gaps = [low - high for low, high in itertools.pairwise(crowded)]
    assert all(gap >= 14.5 + 4 - 0.01 for gap in gaps), gaps
    assert crowded[0] != pytest.approx(starts["gene00"], abs=0.5)
    # Labels sit past the link end (after the text spacing), aligned in a column.
    assert {round(x, 2) for _, x, _ in labels} == {round(links[0][3][0] - 2, 2)}


def test_even_spread_takes_equal_slots_over_the_axis() -> None:
    group = _group(_render(_heatmap(guide=gg.guide_axis_mark(spread="even"))), "axis-y")
    ends = sorted(link[3][1] for link in _links(group))
    steps = [high - low for low, high in itertools.pairwise(ends)]
    assert max(steps) - min(steps) < 0.02, steps


def test_link_width_and_theme_text_style() -> None:
    plot = _heatmap(guide=gg.guide_axis_mark(link_width=45)) + gg.theme(
        axis_text_y=gg.element_text(face="italic", color="#aa0000"),
        axis_ticks_y=gg.element_line(color="#00aa00"),
    )
    group = _group(_render(plot), "axis-y")
    link = _links(group)[0]
    ticks = 4.0  # the default tick length, part of the link
    assert link[0][0] - link[3][0] == pytest.approx(ticks + 45, abs=0.02)
    assert group.count('font-style="italic"') == len(MARKED)
    assert 'fill="#aa0000"' in group
    # Links take the ticks' line; no separate ticks are drawn.
    assert group.count('stroke="#00aa00"') == len(MARKED)


def test_axis_room_includes_the_links() -> None:
    narrow = _render(_heatmap(guide=gg.guide_axis_mark(link_width=0)))
    wide = _render(_heatmap(guide=gg.guide_axis_mark(link_width=60)))

    def panel_left(svg: str) -> float:
        links = _links(_group(svg, "axis-y"))
        return links[0][0][0]

    assert panel_left(wide) - panel_left(narrow) == pytest.approx(60, abs=0.5)


@pytest.mark.parametrize("position", ["left", "right"])
def test_vertical_sides_link_outward(position: str) -> None:
    group = _group(_render(_heatmap(position=position)), "axis-y")
    link = _links(group)[0]
    outward = 1 if position == "right" else -1
    assert (link[3][0] - link[0][0]) * outward > 0
    # Right labels start at their link (SVG's default anchor), left ones end there.
    ends = len(MARKED) if position == "left" else 0
    assert group.count('text-anchor="end"') == ends


@pytest.mark.parametrize("position", ["bottom", "top"])
def test_horizontal_sides_turn_labels_to_read_up(position: str) -> None:
    plot = (
        gg.ggplot(HEATMAP, gg.aes("gene", "cell", fill="expr"))
        + gg.geom_tile()
        + gg.scale_x_discrete(
            breaks=MARKED, limits=GENES, position=position, guide=gg.guide_axis_mark()
        )
        + gg.ggsize(500, 400)
    )
    group = _group(_render(plot), "axis-x")
    links = _links(group)
    assert len(links) == len(MARKED)
    outward = 1 if position == "bottom" else -1
    for link in links:
        assert link[0][0] == link[1][0] and link[2][0] == link[3][0]
        assert (link[3][1] - link[0][1]) * outward > 0
    assert group.count("rotate(-90") == len(MARKED)
    ends = sorted(link[3][0] for link in links)
    assert all(high - low >= 14.5 + 4 - 0.01 for low, high in itertools.pairwise(ends))


def test_coord_flip_moves_the_marks_to_the_horizontal_axis() -> None:
    svg = _render(_heatmap() + gg.coord_flip())
    assert len(_links(_group(svg, "axis-x"))) == len(MARKED)
    assert not _links(_group(svg, "axis-y"))


def test_guides_function_sets_the_guide_without_warnings() -> None:
    plot = (
        gg.ggplot(HEATMAP, gg.aes("cell", "gene", fill="expr"))
        + gg.geom_tile()
        + gg.scale_y_discrete(breaks=MARKED, limits=GENES)
        + gg.guides(y=gg.guide_axis_mark())
    )
    assert len(_links(_group(_render(plot), "axis-y"))) == len(MARKED)


def test_blank_labels_draw_plain_ticks() -> None:
    plot = _heatmap() + gg.theme(axis_text_y=gg.element_blank())
    group = _group(_render(plot), "axis-y")
    assert not _links(group)
    assert not _labels(group)


def test_continuous_axis_marks_its_breaks() -> None:
    data = {"x": list(range(100)), "y": [value % 7 for value in range(100)]}
    plot = (
        gg.ggplot(data, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.scale_y_continuous(breaks=[0, 0.2, 0.4, 6])
        + gg.guides(y=gg.guide_axis_mark())
    )
    group = _group(_render(plot), "axis-y")
    ends = sorted(link[3][1] for link in _links(group))
    assert len(ends) == 4
    assert all(high - low >= 14.5 + 4 - 0.01 for low, high in itertools.pairwise(ends))


def test_side_panel_axis_takes_the_guide() -> None:
    tracks = [f"track{index}" for index in range(8)]
    top = {
        "cell": [cell for _ in tracks for cell in CELLS],
        "track": [track for track in tracks for _ in CELLS],
        "level": [float(index % 5) for index in range(48)],
    }
    plot = (
        gg.ggplot(HEATMAP, gg.aes("cell", "gene", fill="expr"))
        + gg.geom_tile()
        + gg.new_scale_fill()
        + gg.geom_xsidetile(gg.aes(x="cell", y="track", fill="level"), data=top)
        + gg.scale_xsidey_discrete(
            breaks=["track0", "track1", "track2"], limits=tracks, guide=gg.guide_axis_mark()
        )
        + gg.scale_y_discrete(breaks=MARKED, limits=GENES, guide=gg.guide_axis_mark())
        + gg.theme(ggside_panel_scale_x=0.5)
        + gg.ggsize(500, 500)
    )
    svg = _render(plot)
    band = re.search(r'<g class="side-axis">(.*?)</g>', svg, re.S)
    assert band
    assert len(_links(band.group(1))) == 3
    assert [text for text, _, _ in _labels(band.group(1))] == ["track0", "track1", "track2"]
    # Band and main labels end in one column (the band axis lines up with the y axis).
    main = _labels(_group(svg, "axis-y"))
    assert {round(x, 2) for _, x, _ in _labels(band.group(1))} == {round(main[0][1], 2)}


def test_aligned_grid_keeps_the_marks() -> None:
    plain = gg.ggplot(HEATMAP, gg.aes("cell", "gene", fill="expr")) + gg.geom_tile()
    grid = gg.gggrid([_heatmap(), plain], ncol=2, align=True) + gg.ggsize(900, 400)
    svg = _render(grid)
    assert len(_links(svg)) >= len(MARKED)


# HTML zoom: zoomed labels sit at their breaks without links; reset restores the
# spread layout. Headless Chromium through playwright (skipped when missing).


@pytest.fixture(scope="module")
def page() -> Iterator[object]:
    """A headless Chromium page at device scale 1, collecting runtime errors."""
    sync_api = pytest.importorskip("playwright.sync_api")
    with sync_api.sync_playwright() as playwright:
        try:
            browser = playwright.chromium.launch()
        except sync_api.Error as error:
            pytest.skip(f"Chromium for playwright is not installed: {error}")
        page = browser.new_page(device_scale_factor=1, viewport={"width": 1000, "height": 700})
        page.errors = []
        page.on("pageerror", lambda error: page.errors.append(str(error)))
        yield page
        browser.close()


AXIS_STATE = """() => {
  const group = document.querySelector('.ggrust-plot svg g.axis-y');
  const links = [...group.querySelectorAll('path')]
    .filter((path) => (path.getAttribute('d').match(/-?[\\d.]+/g) || []).length === 8).length;
  const labels = [...group.querySelectorAll('text')].map((text) => {
    const rect = text.getBoundingClientRect();
    return [text.textContent, rect.y + rect.height / 2];
  });
  const ticks = [...group.querySelectorAll('path')].map((path) => {
    const rect = path.getBoundingClientRect();
    return rect.y + rect.height / 2;
  });
  return { links, labels, ticks };
}"""


def test_zoom_places_labels_at_their_breaks_and_reset_restores(page, tmp_path: Path) -> None:
    path = tmp_path / "plot.html"
    # The default theme draws no y ticks; turn them on so labels can be checked against them.
    plot = _heatmap() + gg.theme(axis_ticks_y=gg.element_line()) + gg.ggtb()
    path.write_text(plot.to_html(), encoding="utf-8")
    page.errors.clear()
    page.goto(path.as_uri())
    page.wait_for_selector("[data-ggrust-mounted]")
    before = page.evaluate(AXIS_STATE)
    assert before["links"] == len(MARKED)

    panel, box = page.evaluate(
        """() => {
          const root = document.querySelector('.ggrust-plot').ggrust;
          const box = root.svg.getBoundingClientRect();
          return [root.zoom.plots[0].tiles[0].spec.panel, [box.x, box.y]];
        }"""
    )
    page.locator(".ggrust-toolbar button").first.click()  # the pan tool; the wheel zooms
    page.mouse.move(box[0] + panel[0] + panel[2] / 2, box[1] + panel[1] + panel[3] * 0.85)
    for _ in range(2):
        page.mouse.wheel(0, -100)
    page.wait_for_function(
        "() => !document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0].dirty"
    )
    page.wait_for_timeout(50)
    zoomed = page.evaluate(AXIS_STATE)
    assert zoomed["links"] == 0
    assert zoomed["labels"], "no labels left on screen"
    # Each zoomed label is centred on a tick (its break), within a text box's offset.
    for text, at in zoomed["labels"]:
        assert min(abs(at - tick) for tick in zoomed["ticks"]) < 3, text

    page.locator(".ggrust-toolbar button", has_text="").last.click()  # reset
    page.wait_for_timeout(100)
    assert page.evaluate(AXIS_STATE)["links"] == len(MARKED)
    assert page.errors == []
