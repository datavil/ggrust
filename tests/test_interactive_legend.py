"""Interactive legend of HTML output (users_asks.md 4, lets-plot issue #1080).

Clicking a discrete legend key hides or shows the marks of that key in every
layer (canvas points included) and mutes their tooltips; double-clicking
isolates the key within its legend. Static output (SVG, PNG) is unchanged.
The browser tests drive headless Chromium through playwright and are skipped
when it is not installed (``uv run playwright install chromium``).
"""

import json
import re
from collections.abc import Iterator
from pathlib import Path

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.plot.core import PlotSpec


def frame(n: int = 90, seed: int = 3) -> pl.DataFrame:
    """Points in three kinds, the last row a far outlier of kind ``b``."""
    rng = np.random.default_rng(seed)
    body = pl.DataFrame(
        {
            "x": rng.normal(size=n - 1),
            "y": rng.normal(size=n - 1),
            "kind": rng.choice(["a", "b", "c"], n - 1),
            "name": ["body"] * (n - 1),
        }
    )
    outlier = pl.DataFrame({"x": [6.0], "y": [6.0], "kind": ["b"], "name": ["outlier"]})
    return pl.concat([body, outlier])


def scatter(data: pl.DataFrame) -> PlotSpec:
    """Points and a smooth per kind, tooltips naming the row."""
    return (
        gg.ggplot(data, gg.aes("x", "y", color="kind"))
        + gg.geom_point(tooltips=gg.layer_tooltips().line("@name"))
        + gg.geom_smooth(method="lm", tooltips="none")
        + gg.ggsize(500, 400)
    )


def key_tags(html: str, role: str) -> list[list[str]]:
    """The key ids of every ``g.<role>`` group, in document order."""
    found = re.findall(rf'<g class="{role}" data-key="([^"]*)"', html)
    return [json.loads(tag.replace("&quot;", '"')) for tag in found]


def test_html_tags_legend_keys_and_marks() -> None:
    html = scatter(frame()).to_html()
    assert key_tags(html, "legend-key") == [["kind\x1fa"], ["kind\x1fb"], ["kind\x1fc"]]
    marks = key_tags(html, "keyed")
    # Points: runs of one kind, in data order; smooth: one line per kind.
    assert {tuple(ids) for ids in marks} == {(f"kind\x1f{kind}",) for kind in "abc"}
    assert len(marks) > 6


def test_html_marks_match_static_svg_apart_from_tags() -> None:
    """Tagging never changes a mark: stripping the tags gives the untagged output."""
    plot = scatter(frame())
    html = plot.to_html()
    untagged = re.sub(r'<g class="keyed" data-key="[^"]*">\n|</g>\n', "", html)
    plain = (
        gg.ggplot(frame(), gg.aes("x", "y"))
        + gg.geom_point(gg.aes(color="kind"), tooltips=gg.layer_tooltips().line("@name"))
        + gg.geom_smooth(gg.aes(color="kind"), method="lm", tooltips="none")
        + gg.ggsize(500, 400)
    )
    assert untagged.count("<circle") == re.sub(r"</g>\n", "", plain.to_html()).count("<circle")


def test_boxplot_outliers_carry_their_box_key() -> None:
    data = frame().with_columns(pl.col("y") * pl.lit(1.0))
    plot = gg.ggplot(data, gg.aes("kind", "y", fill="kind")) + gg.geom_boxplot()
    html = plot.to_html()
    # The far outlier of kind b (y = 6) is a grey marker inside b's keyed group.
    groups = re.findall(r'<g class="keyed" data-key="([^"]*)">(.*?)</g>', html, re.S)
    outlier_groups = [
        json.loads(key.replace("&quot;", '"')) for key, body in groups if "<circle" in body
    ]
    assert ["kind\x1fb"] in outlier_groups


def test_static_output_has_no_legend_tags() -> None:
    plot = scatter(frame())
    svg = plot.to_svg()
    assert "data-key" not in svg
    assert 'class="keyed"' not in svg and 'class="legend-key"' not in svg


def test_rows_outside_the_breaks_and_layers_without_legends_stay_untagged() -> None:
    data = frame()
    plot = (
        gg.ggplot(data, gg.aes("x", "y"))
        + gg.geom_point(gg.aes(color="kind"))
        + gg.geom_point(color="black")
        + gg.scale_color_discrete(breaks=["a", "b"])
    )
    html = plot.to_html()
    assert {tuple(ids) for ids in key_tags(html, "keyed")} == {("kind\x1fa",), ("kind\x1fb",)}


def test_two_legends_tag_marks_with_both_keys() -> None:
    data = frame().with_columns(pl.Series("size", ["s", "t"] * 45))
    plot = gg.ggplot(data, gg.aes("x", "y", color="kind", shape="size")) + gg.geom_point()
    marks = key_tags(plot.to_html(), "keyed")
    assert len({tuple(ids) for ids in marks}) == 6
    assert all(len(ids) == 2 and ids[1].startswith("size\x1f") for ids in marks)


# ---- Browser tests ----


@pytest.fixture(scope="module")
def page() -> Iterator[object]:
    """A headless Chromium page, collecting runtime errors."""
    sync_api = pytest.importorskip("playwright.sync_api")
    with sync_api.sync_playwright() as playwright:
        try:
            browser = playwright.chromium.launch()
        except sync_api.Error as error:
            pytest.skip(f"Chromium for playwright is not installed: {error}")
        page = browser.new_page(device_scale_factor=1)
        page.errors = []
        page.on("pageerror", lambda error: page.errors.append(str(error)))
        yield page
        browser.close()


def load(page, html: str, tmp_path: Path) -> None:
    """Open ``html`` and wait for the runtime to mount it."""
    path = tmp_path / "plot.html"
    path.write_text(html, encoding="utf-8")
    page.errors.clear()
    page.goto(path.as_uri())
    page.wait_for_selector("[data-ggrust-mounted]")
    assert page.errors == []


HIDDEN_MARKS = """() => [...document.querySelectorAll('g.keyed')]
  .filter(group => getComputedStyle(group).display === 'none')
  .map(group => JSON.parse(group.getAttribute('data-key'))[0])"""

OPACITIES = """() => [...document.querySelectorAll('g.legend-key')]
  .map(key => key.style.opacity)"""


def outlier_point(page) -> tuple[float, float]:
    """Client coordinates of the outlier's tooltip target."""
    return page.evaluate(
        """() => {
          const plot = document.querySelector('.ggrust-plot').ggrust;
          const layer = plot.layers.find(layer => layer.targets.kind === 'points');
          const i = layer.targets.x.length - 1;
          const box = plot.svg.getBoundingClientRect();
          const scale = plot.scale;
          return [box.left + layer.targets.x[i] * scale, box.top + layer.targets.y[i] * scale];
        }"""
    )


def test_click_hides_a_key_and_its_tooltips(page, tmp_path: Path) -> None:
    load(page, scatter(frame()).to_html(), tmp_path)
    # Hover once so the runtime knows its scale, and see the outlier answer.
    box = page.locator(".ggrust-plot > svg").bounding_box()
    page.mouse.move(box["x"] + 5, box["y"] + 5)
    page.wait_for_function("document.querySelector('.ggrust-plot').ggrust.scale > 0")
    x, y = outlier_point(page)
    page.mouse.move(x, y)
    page.wait_for_function("document.querySelector('.ggrust-plot').innerText.includes('outlier')")
    page.locator("g.legend-key").nth(1).click()
    assert set(page.evaluate(HIDDEN_MARKS)) == {"kind\x1fb"}
    assert page.evaluate(OPACITIES) == ["", "0.35", ""]
    page.mouse.move(x + 1, y + 1)
    page.wait_for_timeout(150)
    assert "outlier" not in page.locator(".ggrust-plot").inner_text()
    # Clicked again: shown and answering.
    page.locator("g.legend-key").nth(1).click()
    assert page.evaluate(HIDDEN_MARKS) == []
    page.mouse.move(x, y)
    page.wait_for_function("document.querySelector('.ggrust-plot').innerText.includes('outlier')")
    assert page.errors == []


def test_double_click_isolates_then_restores(page, tmp_path: Path) -> None:
    load(page, scatter(frame()).to_html(), tmp_path)
    page.locator("g.legend-key").nth(2).dblclick()
    assert set(page.evaluate(HIDDEN_MARKS)) == {"kind\x1fa", "kind\x1fb"}
    assert page.evaluate(OPACITIES) == ["0.35", "0.35", ""]
    page.locator("g.legend-key").nth(2).dblclick()
    assert page.evaluate(HIDDEN_MARKS) == []
    assert page.errors == []


def test_dense_canvas_points_hide_by_key(page, tmp_path: Path) -> None:
    """A dense layer stays one canvas batch; its markers hide by key."""
    data = frame(60_000)
    plot = gg.ggplot(data, gg.aes("x", "y", color="kind")) + gg.geom_point()
    html = plot.to_html()
    assert len(re.findall(r'data-batch="\d+"', html)) == 1
    load(page, html, tmp_path)
    painted = """() => {
      const canvas = document.querySelector('foreignObject.ggrust-canvas canvas');
      const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
      let count = 0;
      for (let i = 3; i < pixels.length; i += 4) if (pixels[i] > 0) count++;
      return count;
    }"""
    before = page.evaluate(painted)
    page.locator("g.legend-key").first.click()
    page.wait_for_function(f"({painted})() < {before}")
    hidden = page.evaluate(painted)
    assert 0 < hidden < before
    page.locator("g.legend-key").first.click()
    page.wait_for_function(f"({painted})() === {before}")
    assert page.errors == []


def test_canvas_redraws_are_deferred_and_coalesced(page, tmp_path: Path) -> None:
    """A click leaves the canvas to the next frame; a double-click draws at most twice."""
    data = frame(60_000)
    plot = gg.ggplot(data, gg.aes("x", "y", color="kind")) + gg.geom_point()
    load(page, plot.to_html(), tmp_path)
    page.evaluate("""() => {
      const context = document.querySelector('foreignObject.ggrust-canvas canvas').getContext('2d');
      window.draws = 0;
      const clear = context.clearRect.bind(context);
      context.clearRect = (...args) => { window.draws++; return clear(...args); };
    }""")
    key = page.locator("g.legend-key").first
    # The click handler returns before the canvas is drawn.
    assert (
        page.evaluate("""() => {
      const key = document.querySelector('g.legend-key');
      key.dispatchEvent(new MouseEvent('click', {bubbles: true}));
      return window.draws;
    }""")
        == 0
    )
    page.wait_for_function("window.draws === 1")
    # Toggling back and forth within one frame draws nothing.
    page.evaluate("""() => {
      const key = document.querySelector('g.legend-key');
      key.dispatchEvent(new MouseEvent('click', {bubbles: true}));
      key.dispatchEvent(new MouseEvent('click', {bubbles: true}));
    }""")
    page.wait_for_timeout(200)
    assert page.evaluate("window.draws") == 1
    key.click()  # show all again
    page.wait_for_function("window.draws === 2")
    key.dblclick()  # click, click, dblclick: the first click and the isolation
    page.wait_for_timeout(300)
    assert page.evaluate("window.draws") <= 4
    assert page.errors == []


def test_bars_with_a_constant_outline_hide_by_fill_key() -> None:
    """A constant outline must not stop bars from being tagged by their fill."""
    data = pl.DataFrame({"k": ["a", "b", "c", "a"], "v": [1, 2, 3, 4]})
    for plot in [
        gg.ggplot(data, gg.aes("k", "v", fill="k")) + gg.geom_col(),
        gg.ggplot(data, gg.aes("k", fill="k")) + gg.geom_bar(color="black"),
    ]:
        tags = {tuple(ids) for ids in key_tags(plot.to_html(), "keyed")}
        assert tags == {(f"k\x1f{kind}",) for kind in "abc"}
