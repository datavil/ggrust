"""Interactive legend of HTML output (users_asks.md 4, lets-plot issue #1080).

Clicking a discrete legend key hides or shows the marks of that key in every
layer (canvas points included) and mutes their tooltips; double-clicking
isolates the key within its legend. Static output (SVG, PNG) is unchanged.
The browser tests drive headless Chromium through playwright and are skipped
when it is not installed (``uv run playwright install chromium``).
"""

import base64
import json
import re
from collections.abc import Iterator
from pathlib import Path

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import SpecError
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


def batch_body(html: str, start: int) -> str:
    """The content of the ``<g>`` element whose opening tag ends at ``start``."""
    depth = 1
    for tag in re.finditer(r"<g\b|</g>", html[start:]):
        depth += 1 if tag.group() == "<g" else -1
        if depth == 0:
            return html[start : start + tag.start()]
    raise AssertionError("unclosed <g>")


def batch_items(tag: str) -> list[int]:
    """The combination index of every child element of a keyed batch (``data-ki``)."""
    found = re.search(r'data-ki="([^"]*)"', tag)
    assert found is not None, tag
    return list(base64.b64decode(found.group(1)))


def marker_keys(html: str) -> list[list[str]]:
    """The key ids of every tagged SVG marker, in document order.

    A keyed point batch holds its combination table (``data-keys``) and the
    index into it of every child element (``data-ki``, base64 bytes); markers
    of the empty combination are untagged.
    """
    found = []
    for batch in re.finditer(r'<g[^>]*data-keys="([^"]*)"[^>]*>', html):
        table = json.loads(batch.group(1).replace("&quot;", '"'))
        found.extend(table[index] for index in batch_items(batch.group()) if table[index])
    return found


def test_html_tags_legend_keys_and_marks() -> None:
    data = frame()
    html = scatter(data).to_html()
    assert key_tags(html, "legend-key") == [["kind\x1fa"], ["kind\x1fb"], ["kind\x1fc"]]
    # Smooth: a band and a line per kind; points: one batch, a key index per marker.
    assert {tuple(ids) for ids in key_tags(html, "keyed")} == {(f"kind\x1f{k}",) for k in "abc"}
    assert marker_keys(html) == [[f"kind\x1f{kind}"] for kind in data["kind"]]


def test_interleaved_point_keys_write_no_per_run_groups() -> None:
    """Keys alternating row by row: one batch, markers in data order, no ``g.keyed``."""
    data = frame().with_columns(pl.Series("kind", ["a", "b", "c"] * 30))
    plot = gg.ggplot(data, gg.aes("x", "y", color="kind")) + gg.geom_point()
    html = plot.to_html()
    assert '<g class="keyed"' not in html
    assert html.count("data-keys=") == 1
    assert marker_keys(html) == [[f"kind\x1f{kind}"] for kind in data["kind"]]
    # Markers keep the static order (data order, so stacking order too).
    circle = r'<circle cx="([^"]+)" cy="([^"]+)" r="[^"]+"'
    batch = re.search(r"<g[^>]*data-keys=[^>]*>", html)
    assert batch is not None
    tagged = re.findall(circle, batch_body(html, batch.end()))
    assert len(tagged) == data.height == len(batch_items(batch.group()))
    # One byte string per batch, no attribute per marker.
    assert 'data-k="' not in html[: html.index("</svg>")]
    static = re.findall(circle, plot.to_svg())
    assert tagged == static[: data.height]


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
    assert ["kind\x1fb"] in outlier_groups + marker_keys(html)


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
    assert key_tags(html, "keyed") == []
    # Kind c and the black layer stay untagged (and visible).
    assert marker_keys(html) == [[f"kind\x1f{kind}"] for kind in data["kind"] if kind != "c"]


def test_two_legends_tag_marks_with_both_keys() -> None:
    data = frame().with_columns(pl.Series("size", ["s", "t"] * 45))
    plot = gg.ggplot(data, gg.aes("x", "y", color="kind", shape="size")) + gg.geom_point()
    marks = marker_keys(plot.to_html())
    assert len(marks) == data.height
    assert len({tuple(ids) for ids in marks}) == 6
    assert all(len(ids) == 2 and ids[1].startswith("size\x1f") for ids in marks)


def has_legend_data(html: str) -> bool:
    """Whether the HTML carries any interactive legend data (marks or tooltips)."""
    svg = html[: html.index("</svg>")]
    tooltips = re.search(r"vnd\.ggrust-tooltips\+json\">(.*?)</script>", html, re.S)
    layers = json.loads(tooltips.group(1))["layers"] if tooltips else []
    return (
        'class="keyed"' in svg
        or "data-keys=" in svg
        or "data-ki=" in svg
        or any("keys" in layer for layer in layers)
    )


def test_legend_interactive_false_drops_the_key_data() -> None:
    data = frame()
    plot = scatter(data)
    assert has_legend_data(plot.to_html())
    html = (plot + gg.theme(legend_interactive=False)).to_html()
    assert not has_legend_data(html)
    # The legend still draws (same as static), with no clickable keys.
    assert 'class="legend-key"' not in html
    assert html.count("<text") == plot.to_svg().count("<text")


def test_hidden_legend_writes_no_key_data() -> None:
    plot = scatter(frame()) + gg.theme(legend_position="none")
    assert not has_legend_data(plot.to_html())


def test_canvas_tooltips_reuse_the_batch_keys() -> None:
    """Dense points carry their keys once: in the canvas batch, not the tooltips."""
    data = frame(6_000)
    html = (gg.ggplot(data, gg.aes("x", "y", color="kind")) + gg.geom_point()).to_html()
    batches = json.loads(
        re.search(r'vnd\.ggrust-canvas\+json"[^>]*>(.*?)</script>', html, re.S).group(1)
    )
    assert all("key" in batch for batch in batches)
    tooltips = re.search(r'vnd\.ggrust-tooltips\+json"[^>]*>(.*?)</script>', html, re.S)
    layers = json.loads(tooltips.group(1))["layers"]
    assert [layer["targets"].get("canvas") for layer in layers] == [0]
    assert "keys" not in layers[0]


def test_legend_interactive_takes_a_bool() -> None:
    with pytest.raises(SpecError) as error:
        (scatter(frame()) + gg.theme(legend_interactive="no")).to_html()  # ty: ignore[invalid-argument-type]
    assert "theme.legend_interactive" in str(error.value)
    assert "True or False" in str(error.value)


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


HIDDEN_MARKS = """() => [
  ...[...document.querySelectorAll('g.keyed')]
    .filter(group => getComputedStyle(group).display === 'none')
    .map(group => JSON.parse(group.getAttribute('data-key'))[0]),
  ...[...document.querySelectorAll('[data-k]')]
    .filter(marker => getComputedStyle(marker).display === 'none')
    .map(marker => JSON.parse(marker.parentElement.getAttribute('data-keys'))
      [+marker.dataset.k][0]),
]"""

# Key ids of every SVG marker of a keyed batch, ``null`` when hidden; untagged: "".
MARKERS = """() => [...document.querySelectorAll('g[data-keys]')].flatMap(batch => {
  const table = JSON.parse(batch.getAttribute('data-keys'));
  return [...batch.children].map(marker => getComputedStyle(marker).display === 'none'
    ? null : (marker.dataset.k === undefined ? '' : table[+marker.dataset.k][0]));
})"""

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


def spread_scatter() -> tuple[pl.DataFrame, PlotSpec]:
    """300 non-overlapping points, kinds interleaved row by row, zoomable."""
    n = 300
    data = pl.DataFrame(
        {
            "x": np.arange(n, dtype=float),
            "y": (np.arange(n) * 7 % n).astype(float),
            "kind": ["a", "b", "c"] * (n // 3),
            "name": [f"row{index}" for index in range(n)],
        }
    )
    plot = (
        gg.ggplot(data, gg.aes("x", "y", color="kind"))
        + gg.geom_point(tooltips=gg.layer_tooltips().line("@name"))
        + gg.ggsize(500, 400)
        + gg.ggtb()
    )
    return data, plot


def target_point(page, row: int) -> tuple[float, float]:
    """Client coordinates of the tooltip target of point ``row``."""
    return page.evaluate(
        """(row) => {
          const plot = document.querySelector('.ggrust-plot').ggrust;
          const layer = plot.layers.find(layer => layer.targets.kind === 'points');
          const i = layer.targets.rows.indexOf(row);
          const box = plot.svg.getBoundingClientRect();
          const scale = plot.scale;
          return [box.left + layer.targets.x[i] * scale, box.top + layer.targets.y[i] * scale];
        }""",
        row,
    )


def test_svg_markers_hide_by_key_with_tooltips_and_zoom(page, tmp_path: Path) -> None:
    """A click hides exactly that key's SVG markers and their tooltips; zoom still moves them."""
    data, plot = spread_scatter()
    html = plot.to_html()
    assert '<g class="keyed"' not in html and 'data-batch="' not in html
    load(page, html, tmp_path)
    kinds = [f"kind\x1f{kind}" for kind in data["kind"]]
    assert page.evaluate(MARKERS) == kinds
    box = page.locator(".ggrust-plot > svg").bounding_box()
    page.mouse.move(box["x"] + 5, box["y"] + 5)
    page.wait_for_function("document.querySelector('.ggrust-plot').ggrust.scale > 0")
    page.locator("g.legend-key").nth(1).click()
    hidden = [None if kind == "kind\x1fb" else kind for kind in kinds]
    assert page.evaluate(MARKERS) == hidden
    assert set(page.evaluate(HIDDEN_MARKS)) == {"kind\x1fb"}
    plot_text = "document.querySelector('.ggrust-plot').innerText"
    # Row 151 is kind b (hidden, silent), row 150 kind a (answers).
    page.mouse.move(*target_point(page, 151))
    page.wait_for_timeout(150)
    assert "row151" not in page.locator(".ggrust-plot").inner_text()
    page.mouse.move(*target_point(page, 150))
    page.wait_for_function(f"{plot_text}.includes('row150')")
    # Zoom with b hidden: every circle moves, b stays hidden.
    centres = (
        "() => [...document.querySelectorAll('g[data-keys] > circle')]"
        ".map(c => c.getAttribute('cx'))"
    )
    before = page.evaluate(centres)
    page.locator(".ggrust-toolbar button").first.click()
    page.mouse.move(box["x"] + 250, box["y"] + 200)
    for _ in range(3):
        page.mouse.wheel(0, -100)
    page.wait_for_function(
        """() => {
          const tile = document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0];
          return !tile.dirty && tile.views.h[0] !== tile.spec.horizontal.domain[0];
        }"""
    )
    after = page.evaluate(centres)
    assert len(after) == len(before)
    assert all(old != new for old, new in zip(before, after, strict=True))
    assert page.evaluate(MARKERS) == hidden
    page.locator("g.legend-key").nth(1).click()
    assert page.evaluate(MARKERS) == kinds
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
