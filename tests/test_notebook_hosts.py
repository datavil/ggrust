"""Notebook HTML in hosts that rewrite output scripts (Google Colab).

Colab re-creates every ``<script>`` of an HTML output with only its ``type`` and
text, so the runtime must find its payloads by ``type``, never by ``class``. The
browser test drives headless Chromium through playwright and is skipped when it
is not installed (``uv run playwright install chromium``).
"""

import re
from collections.abc import Iterator
from pathlib import Path

import ggrust as gg
import polars as pl
import pytest


def strip_script_classes(html: str) -> str:
    """Drop the ``class`` of every ``<script>`` element, as Colab does."""
    return re.sub(r'(<script[^>]*?) class="[^"]*"', r"\1", html)


@pytest.fixture(scope="module")
def page() -> Iterator[object]:
    """A headless Chromium page collecting runtime errors."""
    sync_api = pytest.importorskip("playwright.sync_api")
    with sync_api.sync_playwright() as playwright:
        try:
            browser = playwright.chromium.launch()
        except sync_api.Error as error:
            pytest.skip(f"Chromium for playwright is not installed: {error}")
        page = browser.new_page()
        page.errors = []
        page.on("pageerror", lambda error: page.errors.append(str(error)))
        yield page
        browser.close()


def test_payloads_are_typed() -> None:
    frame = pl.DataFrame({"x": [1, 2, 3], "y": [3, 1, 2]})
    html = (gg.ggplot(frame) + gg.geom_point(gg.aes(x="x", y="y")) + gg.ggtb())._repr_html_()
    assert 'type="application/vnd.ggrust-tooltips+json"' in html
    assert 'type="application/vnd.ggrust-zoom+json"' in html


def test_runtime_mounts_without_script_classes(page, tmp_path: Path) -> None:
    frame = pl.DataFrame({"x": [1, 2, 3], "y": [3, 1, 2], "g": ["a", "b", "a"]})
    plot = gg.ggplot(frame) + gg.geom_point(gg.aes(x="x", y="y", color="g")) + gg.ggtb()
    html = strip_script_classes(plot._repr_html_())
    assert 'class="ggrust-zoom"' not in html
    path = tmp_path / "colab.html"
    path.write_text(html, encoding="utf-8")
    page.errors.clear()
    page.goto(path.as_uri())
    page.wait_for_selector("[data-ggrust-mounted]")
    assert page.errors == []
    assert page.locator(".ggrust-toolbar").count() == 1
    assert page.evaluate("document.querySelector('.ggrust-plot').ggrust.layers.length") > 0
