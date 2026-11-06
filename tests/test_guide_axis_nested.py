"""``guide_axis_nested()``: brackets naming groups of breaks (plan 10.3, R legendry).

Groups come as a mapping ``{group: [levels]}`` or ``key_range_manual(start, end,
name)``; each group's bracket spans its first to last category (padded by
``pad_discrete``) past the axis labels, or across the panel (``side="opposite"``).
Checked on every side, under ``coord_flip()``, in facets, in ``gggrid``, on a side
band axis, as boxes, and with the HTML zoom (brackets follow the breaks).
"""

from __future__ import annotations

import re
import warnings
from typing import TYPE_CHECKING

import ggrust as gg
import pytest
from ggrust.errors import SpecError, UnsupportedValueError

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path

FRUIT = ["lemon", "lime", "orange", "apple", "pear", "plum", "cherry"]
DATA = {"fruit": FRUIT, "sales": [4, 3, 6, 8, 5, 2, 7], "shop": ["A", "B"] * 3 + ["A"]}
GROUPS = {"Citrus\nfruits": ["lemon", "lime", "orange"], "Pome": ["apple", "pear"]}

Bracket = tuple[tuple[float, float], ...]


def _bars(guide: object, *, flip: bool = False, **scale: object) -> gg.PlotSpec:
    plot = (
        gg.ggplot(DATA, gg.aes("fruit", "sales"))
        + gg.geom_col()
        + gg.scale_x_discrete(limits=FRUIT, guide=guide, **scale)
        + gg.ggsize(500, 350)
    )
    return plot + gg.coord_flip() if flip else plot


def _render(plot: gg.PlotSpec) -> str:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        return plot.to_svg()


def _groups(svg: str, name: str) -> list[str]:
    """Bodies of every ``<g class=name>`` group (they hold no deeper same-depth end)."""
    bodies = []
    for start in [match.end() for match in re.finditer(rf'<g class="{name}">', svg)]:
        depth, index = 1, start
        while depth:
            opening = svg.find("<g", index)
            closing = svg.find("</g>", index)
            if opening != -1 and opening < closing:
                depth, index = depth + 1, opening + 2
            else:
                depth, index = depth - 1, closing + 4
        bodies.append(svg[start : index - 4])
    return bodies


def _brackets(group: str) -> list[Bracket]:
    found = []
    for path in re.findall(r'<path d="([^"]+)"', group):
        numbers = [float(number) for number in re.findall(r"-?[\d.]+", path)]
        found.append(tuple(zip(numbers[::2], numbers[1::2], strict=True)))
    return found


def _texts(group: str) -> list[tuple[str, float, float]]:
    return [
        (text, float(x), float(y))
        for x, y, text in re.findall(
            r'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*>([^<]+)</text>', group
        )
    ]


def _tick_positions(svg: str, axis: str) -> dict[str, float]:
    """Label text -> its position along the axis."""
    (group,) = _groups(svg, f"axis-{axis}")
    return {text: (x if axis == "x" else y) for text, x, y in _texts(group)}


# --- Python API -----------------------------------------------------------------


def test_mapping_key_writes_groups_in_order() -> None:
    guide = gg.guide_axis_nested(GROUPS, bracket="square", pad_discrete=0.3)
    options = guide.as_dict()
    assert options["name"] == "axis_nested"
    assert options["key"] == [
        {"name": "Citrus\nfruits", "levels": ["lemon", "lime", "orange"]},
        {"name": "Pome", "levels": ["apple", "pear"]},
    ]
    assert options["bracket"] == "square"
    assert options["pad_discrete"] == 0.3


def test_key_range_manual_writes_ranges() -> None:
    key = gg.key_range_manual(start=["lemon", "apple"], end=["orange", "pear"], name=["A", "B"])
    assert gg.guide_axis_nested(key).as_dict()["key"] == [
        {"name": "A", "start": "lemon", "end": "orange"},
        {"name": "B", "start": "apple", "end": "pear"},
    ]
    unnamed = gg.key_range_manual(start=1.5, end=3)
    assert unnamed.as_spec() == [{"name": None, "start": 1.5, "end": 3}]


def test_python_errors_are_typed_and_helpful() -> None:
    with pytest.raises(SpecError, match="did you mean 'square'"):
        gg.guide_axis_nested(GROUPS, bracket="sqare")  # type: ignore[arg-type]
    with pytest.raises(UnsupportedValueError, match="expected one of"):
        gg.guide_axis_nested(GROUPS, type=1)  # type: ignore[arg-type]
    with pytest.raises(UnsupportedValueError, match="must be a sequence"):
        gg.guide_axis_nested({"A": "lemon"})
    with pytest.raises(SpecError, match="'lime' is in groups 'A' and 'B'"):
        gg.guide_axis_nested({"A": ["lemon", "lime"], "B": ["lime"]})
    # A level listed twice in one group is one membership.
    assert gg.guide_axis_nested({"A": ["lemon", "lemon"]}).as_dict()["key"][0]["levels"] == [
        "lemon",
        "lemon",
    ]
    with pytest.raises(UnsupportedValueError, match="mapping"):
        gg.guide_axis_nested(["lemon"])  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="one entry per group"):
        gg.key_range_manual(["a", "b"], ["c"])
    with pytest.raises(UnsupportedValueError, match="finite"):
        gg.guide_axis_nested(GROUPS, angle=float("nan"))


def test_engine_errors_name_the_spec_path() -> None:
    with pytest.raises(SpecError, match="continuous"):
        _render(
            gg.ggplot(DATA, gg.aes("sales", "sales"))
            + gg.geom_point()
            + gg.guides(x=gg.guide_axis_nested({"A": ["lemon"]}))
        )


# --- Layout and drawing ---------------------------------------------------------


def test_bottom_brackets_span_their_breaks_below_the_labels() -> None:
    svg = _render(_bars(gg.guide_axis_nested(GROUPS, bracket="square")))
    ticks = _tick_positions(svg, "x")
    (nested,) = _groups(svg, "nested-x")
    citrus, pome = _brackets(nested)
    step = ticks["lime"] - ticks["lemon"]
    # Square brackets: tips first and last, spanning the group +/- 0.4 positions.
    assert citrus[0][0] == pytest.approx(ticks["orange"] + 0.4 * step, abs=0.01)
    assert pome[0][0] == pytest.approx(ticks["pear"] + 0.4 * step, abs=0.01)
    assert pome[-1][0] == pytest.approx(ticks["apple"] - 0.4 * step, abs=0.01)
    # Bars expand the axis past the pad, so nothing is squished here.
    assert citrus[-1][0] == pytest.approx(ticks["lemon"] - 0.4 * step, abs=0.01)
    # Tips point back to the axis; the bracket sits below the labels.
    label_y = max(y for _, _, y in _texts(_groups(svg, "axis-x")[0]))
    assert citrus[0][1] < citrus[1][1]
    assert citrus[0][1] > label_y
    names = _texts(nested)
    # Multi-line names: one text per line, centred on the group.
    assert [text for text, _, _ in names] == ["Citrus", "fruits", "Pome"]
    assert names[0][1] == pytest.approx(ticks["lime"], abs=0.01)
    assert names[1][2] > names[0][2]


def test_room_grows_by_the_guide() -> None:
    plain = _render(_bars(None))
    nested = _render(_bars(gg.guide_axis_nested(GROUPS)))
    panel_bottom = lambda svg: max(y for _, _, y in _texts(_groups(svg, "axis-x")[0]))  # noqa: E731
    # The panel shrinks to make room, so labels move up.
    assert panel_bottom(nested) < panel_bottom(plain) - 20


def test_opposite_side_draws_above_without_labels_there() -> None:
    svg = _render(_bars(gg.guide_axis_nested(GROUPS, side="opposite", bracket="square")))
    (nested,) = _groups(svg, "nested-x")
    citrus = _brackets(nested)[0]
    label_y = min(y for _, _, y in _texts(_groups(svg, "axis-x")[0]))
    assert all(y < label_y - 100 for _, y in citrus)
    # Tips point down, back to the panel.
    assert citrus[0][1] > citrus[1][1]


@pytest.mark.parametrize("position", ["left", "right"])
def test_vertical_axes_and_coord_flip(position: str) -> None:
    plot = (
        gg.ggplot(DATA, gg.aes("sales", "fruit"))
        + gg.geom_col(orientation="y")
        + gg.scale_y_discrete(
            limits=FRUIT, position=position, guide=gg.guide_axis_nested(GROUPS, bracket="square")
        )
    )
    svg = _render(plot)
    ticks = _tick_positions(svg, "y")
    (nested,) = _groups(svg, "nested-y")
    citrus, pome = _brackets(nested)
    ys = sorted({y for _, y in pome})
    span = sorted([ticks["apple"], ticks["pear"]])
    step = abs(ticks["lime"] - ticks["lemon"])
    # Spans apple..pear plus 0.4 positions each way.
    assert ys[-1] - ys[0] == pytest.approx(span[1] - span[0] + 0.8 * step, abs=0.03)
    assert (ys[0] + ys[-1]) / 2 == pytest.approx(
        (ticks["apple"] + ticks["pear"]) / 2,
        abs=5,  # label baselines sit off the tick
    )
    label_xs = [x for _, x, _ in _texts(_groups(svg, "axis-y")[0])]
    if position == "left":
        assert max(x for x, _ in citrus) < min(label_xs)
    else:
        assert min(x for x, _ in citrus) > max(label_xs)
    # coord_flip moves the x guide to the vertical axis.
    flipped = _render(_bars(gg.guide_axis_nested(GROUPS), flip=True))
    assert _groups(flipped, "nested-y") and not _groups(flipped, "nested-x")


def test_box_type_frames_each_group() -> None:
    svg = _render(_bars(gg.guide_axis_nested(GROUPS, type="box")))
    (nested,) = _groups(svg, "nested-x")
    assert nested.count("<rect") - nested.count("<clipPath") == 2
    assert [text for text, _, _ in _texts(nested)] == ["Citrus", "fruits", "Pome"]


def test_groups_without_levels_on_the_axis_are_dropped() -> None:
    svg = _render(_bars(gg.guide_axis_nested({"A": ["kiwi"], "B": ["plum", "fig"]})))
    (nested,) = _groups(svg, "nested-x")
    assert [text for text, _, _ in _texts(nested)] == ["B"]


def test_facets_draw_beside_each_outer_axis() -> None:
    plot = _bars(gg.guide_axis_nested(GROUPS)) + gg.facet_wrap("shop", ncol=2)
    svg = _render(plot)
    assert len(_groups(svg, "nested-x")) == 2
    with pytest.warns(UserWarning, match="side='axis'"):
        (_bars(gg.guide_axis_nested(GROUPS, side="opposite")) + gg.facet_wrap("shop")).to_svg()


def test_gggrid_align_keeps_panels_aligned() -> None:
    nested = _bars(gg.guide_axis_nested(GROUPS))
    grid = gg.gggrid([nested, _bars(None)], align=True)
    svg = grid.to_svg()
    assert len(_groups(svg, "nested-x")) == 1
    panels = re.findall(r'<g class="panel"[^>]*clip-path="url\(#([^)]+)\)"', svg)
    assert len(panels) >= 2


def test_side_band_axis_takes_the_guide() -> None:
    tracks = {"fruit": FRUIT, "kind": ["citrus"] * 3 + ["pome"] * 2 + ["stone"] * 2}
    plot = (
        gg.ggplot(DATA, gg.aes("sales", "fruit"))
        + gg.geom_col(orientation="y")
        + gg.geom_ysidetile(gg.aes(y="fruit", x="kind", fill="kind"), data=tracks)
        + gg.scale_ysidex_discrete(
            guide=gg.guide_axis_nested({"Group": ["citrus", "pome"]}, bracket="square")
        )
    )
    svg = _render(plot)
    assert len(_groups(svg, "nested-x")) == 1


# --- HTML zoom ------------------------------------------------------------------


@pytest.fixture(scope="module")
def page() -> Iterator[object]:
    """A headless Chromium page, collecting runtime errors."""
    sync_api = pytest.importorskip("playwright.sync_api")
    with sync_api.sync_playwright() as playwright:
        try:
            browser = playwright.chromium.launch()
        except sync_api.Error as error:
            pytest.skip(f"Chromium for playwright is not installed: {error}")
        page = browser.new_page(device_scale_factor=1, viewport={"width": 900, "height": 600})
        page.errors = []
        page.on("pageerror", lambda error: page.errors.append(str(error)))
        yield page
        browser.close()


MEASURE = """() => {
  const root = document.querySelector('.ggrust-plot').ggrust;
  const tile = root.zoom.plots[0].tiles[0];
  const nested = tile.element.querySelector(':scope > g.nested-x');
  const ends = [...nested.querySelectorAll('path')].map((path) => {
    const numbers = path.getAttribute('d').match(/-?[\\d.]+/g).map(Number);
    const matrix = path.getScreenCTM();
    const xs = [];
    for (let i = 0; i < numbers.length; i += 2) {
      xs.push(new DOMPoint(numbers[i], numbers[i + 1]).matrixTransform(matrix).x);
    }
    return [Math.min(...xs), Math.max(...xs)];
  });
  const names = [...nested.querySelectorAll(':scope > g')]
    .filter((group) => group.hasAttribute('data-anchor'))
    .map((group) => {
      const box = group.getBoundingClientRect();
      return { shown: group.getAttribute('display') !== 'none', at: box.x + box.width / 2 };
    });
  const ticks = {};
  for (const text of tile.element.querySelectorAll(':scope > g.axis-x text')) {
    const box = text.getBoundingClientRect();
    ticks[text.textContent] = box.x + box.width / 2;
  }
  return { ends, names, ticks };
}"""


def test_brackets_follow_zoom_and_reset(page, tmp_path: Path) -> None:
    plot = _bars(gg.guide_axis_nested(GROUPS, bracket="square")) + gg.ggtb()
    path = tmp_path / "plot.html"
    path.write_text(plot.to_html(), encoding="utf-8")
    page.goto(path.as_uri())
    page.wait_for_selector("[data-ggrust-mounted]")
    before = page.evaluate(MEASURE)
    step = before["ticks"]["lime"] - before["ticks"]["lemon"]
    assert before["ends"][1][0] == pytest.approx(before["ticks"]["apple"] - 0.4 * step, abs=1)

    panel, box = page.evaluate(
        """() => {
          const root = document.querySelector('.ggrust-plot').ggrust;
          const b = root.svg.getBoundingClientRect();
          return [root.zoom.plots[0].tiles[0].spec.panel, [b.x, b.y]];
        }"""
    )
    centre = (box[0] + panel[0] + panel[2] / 2, box[1] + panel[1] + panel[3] / 2)
    page.locator(".ggrust-toolbar button").first.click()
    page.mouse.move(*centre)
    for _ in range(3):
        page.mouse.wheel(0, -100)
    page.wait_for_function(
        "() => !document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0].dirty"
    )
    page.wait_for_timeout(50)
    after = page.evaluate(MEASURE)
    assert page.errors == []
    ticks = after["ticks"]
    zoomed_step = ticks["pear"] - ticks["apple"] if "pear" in ticks and "apple" in ticks else None
    assert zoomed_step and zoomed_step > step * 1.2
    # The Pome bracket still spans apple..pear +/- 0.4 positions.
    assert after["ends"][1][0] == pytest.approx(ticks["apple"] - 0.4 * zoomed_step, abs=1)
    assert after["ends"][1][1] == pytest.approx(ticks["pear"] + 0.4 * zoomed_step, abs=1)
    # A name shows exactly while its group's centre is inside the panel (hidden
    # names keep their box, so their centre is still measured).
    left, right = box[0] + panel[0], box[0] + panel[0] + panel[2]
    for name in after["names"]:
        if abs(name["at"] - left) > 2 and abs(name["at"] - right) > 2:
            assert name["shown"] == (left < name["at"] < right), name

    page.locator(".ggrust-toolbar button").last.click()  # reset
    page.wait_for_function(
        "() => !document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0].dirty"
    )
    page.wait_for_timeout(50)
    reset = page.evaluate(MEASURE)
    assert [end for pair in reset["ends"] for end in pair] == pytest.approx(
        [end for pair in before["ends"] for end in pair], abs=0.01
    )
    assert all(name["shown"] for name in reset["names"])
    assert page.errors == []
