"""``geom_text`` / ``geom_label`` ``lineheight``: lines are lineheight x font size apart."""

import re
import warnings

import ggrust as gg
import pytest
from ggrust.plot.core import PlotSpec

FRAME = {"x": [0.0], "y": [0.0], "label": ["top\nbottom"]}


def _text_ys(plot: PlotSpec) -> list[float]:
    """The y positions of the two label lines in the SVG."""
    svg = plot.to_svg()
    ys = {}
    for line in ("top", "bottom"):
        match = re.search(r'<text[^>]*\by="([-\d.]+)"[^>]*>' + line + "<", svg)
        assert match, f"line {line!r} not found"
        ys[line] = float(match.group(1))
    return [ys["top"], ys["bottom"]]


def _plot(geom: str, **options: float) -> PlotSpec:
    layer = getattr(gg, geom)(size=10, **options)
    return gg.ggplot(FRAME, gg.aes("x", "y", label="label")) + layer


@pytest.mark.parametrize("geom", ["geom_text", "geom_label"])
def test_lineheight_scales_the_line_distance(geom: str) -> None:
    top, bottom = _text_ys(_plot(geom))
    single = bottom - top
    assert single == pytest.approx(20.0)  # size 10 -> 20 px font, one font size apart
    top, bottom = _text_ys(_plot(geom, lineheight=1.5))
    assert bottom - top == pytest.approx(1.5 * single)


def test_lineheight_is_no_longer_reported_as_unsupported() -> None:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        _plot("geom_text", lineheight=0.8).to_svg()


@pytest.mark.parametrize("value", [-1.0, -0.01])
def test_invalid_lineheight_names_the_parameter(value: float) -> None:
    with pytest.raises(ValueError, match=r"lineheight.*non-negative") as error:
        _plot("geom_text", lineheight=value).to_svg()
    assert "layers[0].lineheight" in str(error.value)
