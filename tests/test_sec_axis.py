"""Secondary axes: ``sec_axis()`` / ``dup_axis()`` (ggplot2; lets-plot has none).

The reference is ``tests/fixtures/sec_axis_reference.json``, written by
``tests/fixtures/generate_sec_axis_reference.R`` (ggplot2 3.5.2): the secondary
break positions on the primary scale. ggrust samples the transform as ggplot2
does, over its own (lets-plot) expanded range, so positions agree to the
interpolation error.
"""

from __future__ import annotations

import json
import math
import re
import warnings
from pathlib import Path
from typing import TYPE_CHECKING, Any

import ggrust as gg
import pytest
from ggrust import GgrustWarning
from ggrust.errors import SpecError, UnsupportedValueError

if TYPE_CHECKING:
    from ggrust._typing import AxisScale
    from ggrust.plot.core import PlotSpec

REFERENCE: dict[str, Any] = json.loads(
    (Path(__file__).parent / "fixtures" / "sec_axis_reference.json").read_text()
)
CELSIUS = {
    "month": list(range(1, 13)),
    "celsius": [3.1, 4.2, 8.0, 12.5, 16.9, 20.3, 22.6, 22.1, 18.4, 13.2, 7.6, 4.1],
}
AREA = {
    "area": [103, 385, 506, 378, 3287, 8516, 0.316],
    "people": [0.38, 5.5, 48, 124, 1430, 216, 0.53],
}


def to_fahrenheit(celsius: float) -> float:
    """Convert a temperature in °C to °F."""
    return celsius * 1.8 + 32


def scales(plot: PlotSpec, panel: int = 0) -> dict[str, AxisScale]:
    """Build ``plot`` and return the panel's x and y scales."""
    built = plot.build()["panels"][panel]
    return {"x": built["x"], "y": built["y"]}


def reference_plot(case: str) -> tuple[PlotSpec, str]:
    """The ggrust version of a case of the R reference, with its axis."""
    temperature = gg.ggplot(CELSIUS, gg.aes("month", "celsius")) + gg.geom_line()
    plots: dict[str, tuple[PlotSpec, str]] = {
        "linear_given": (
            temperature
            + gg.scale_y_continuous(sec_axis=gg.sec_axis(to_fahrenheit, breaks=[40, 50, 60, 70])),
            "y",
        ),
        "linear_auto": (
            temperature + gg.scale_y_continuous(sec_axis=gg.sec_axis(to_fahrenheit)),
            "y",
        ),
        "log_given": (
            gg.ggplot(AREA, gg.aes("area", "people"))
            + gg.geom_point()
            + gg.scale_x_log10(
                sec_axis=gg.sec_axis(lambda a: a * 1000 / 2.59, breaks=[1e3, 1e4, 1e5, 1e6])
            ),
            "x",
        ),
        "reverse_given": (
            temperature
            + gg.scale_y_reverse(sec_axis=gg.sec_axis(lambda c: c * 2, breaks=[10, 20, 30, 40])),
            "y",
        ),
        "sqrt_given": (
            gg.ggplot(CELSIUS, gg.aes("celsius", "month"))
            + gg.geom_point()
            + gg.scale_x_continuous(sec_axis=gg.sec_axis(math.sqrt, breaks=[2, 3, 4])),
            "x",
        ),
    }
    return plots[case]


@pytest.mark.parametrize("case", ["linear_given", "log_given", "reverse_given", "sqrt_given"])
def test_break_positions_match_r(case: str) -> None:
    plot, axis = reference_plot(case)
    secondary = scales(plot)[axis]["secondary"]
    expected = REFERENCE[case]
    assert secondary["breaks"] == pytest.approx(expected["positions"], rel=1e-4, abs=1e-4)
    if case != "log_given":  # R writes 1e+03, ggrust lets-plot's 1,000
        assert secondary["labels"] == expected["labels"]


def test_generated_breaks_include_rs() -> None:
    # lets-plot's break rule asks for more breaks than R's extended(): every R
    # break is a ggrust break (deviations.md).
    plot, _ = reference_plot("linear_auto")
    secondary = scales(plot)["y"]["secondary"]
    expected = REFERENCE["linear_auto"]
    for position, label in zip(expected["positions"], expected["labels"], strict=True):
        index = secondary["labels"].index(label)
        assert secondary["breaks"][index] == pytest.approx(position, abs=1e-6)
    assert secondary["labels"] == ["40", "45", "50", "55", "60", "65", "70"]


def test_log_primary_takes_log_breaks() -> None:
    plot, _ = reference_plot("log_given")
    unset = plot + gg.scale_x_log10(sec_axis=gg.sec_axis(lambda a: a * 1000 / 2.59))
    secondary = scales(unset)["x"]["secondary"]
    # The primary axis pads in log units (ggplot2), so it reaches below 1 (100 here).
    assert secondary["labels"] == ["100", "1,000", "10,000", "100,000", "10^6"]
    assert secondary["breaks"][0] == pytest.approx(math.log10(2.59) - 1, abs=1e-6)


def test_dup_axis_mirrors_the_primary_axis() -> None:
    plot = (
        gg.ggplot(CELSIUS, gg.aes("month", "celsius"))
        + gg.geom_line()
        + gg.scale_y_continuous("Temperature", sec_axis=gg.dup_axis())
    )
    y = scales(plot)["y"]
    assert y["secondary"] == {"breaks": y["breaks"], "labels": y["labels"]}
    assert plot.to_svg().count(">Temperature<") == 2


def test_titles_follow_name_and_derive() -> None:
    base = gg.ggplot(CELSIUS, gg.aes("month", "celsius")) + gg.geom_line()
    unnamed = base + gg.scale_y_continuous("°C", sec_axis=gg.sec_axis(to_fahrenheit))
    assert unnamed.to_svg().count(">°C<") == 1
    named = base + gg.scale_y_continuous("°C", sec_axis=gg.sec_axis(to_fahrenheit, name="°F"))
    assert ">°F<" in named.to_svg()
    derived = base + gg.scale_y_continuous(
        "°C", sec_axis=gg.sec_axis(to_fahrenheit, name=gg.derive())
    )
    assert derived.to_svg().count(">°C<") == 2


def title_x(svg: str, title: str) -> float:
    """The x of the text element holding ``title``."""
    match = re.search(r'<text[^>]* x="([-0-9.]+)"[^>]*>' + re.escape(title) + "<", svg)
    assert match, title
    return float(match.group(1))


def test_secondary_axis_takes_the_side_opposite_position() -> None:
    base = gg.ggplot(CELSIUS, gg.aes("month", "celsius")) + gg.geom_line()
    axis = gg.sec_axis(to_fahrenheit, name="°F")
    default = (base + gg.scale_y_continuous("°C", sec_axis=axis)).to_svg()
    assert title_x(default, "°C") < title_x(default, "°F")
    moved = (base + gg.scale_y_continuous("°C", position="right", sec_axis=axis)).to_svg()
    assert title_x(moved, "°F") < title_x(moved, "°C")


def test_labels_and_format() -> None:
    base = gg.ggplot(CELSIUS, gg.aes("month", "celsius")) + gg.geom_line()
    formatted = base + gg.scale_y_continuous(
        sec_axis=gg.sec_axis(to_fahrenheit, labels="{d} °F", breaks=[40, 60])
    )
    assert scales(formatted)["y"]["secondary"]["labels"] == ["40 °F", "60 °F"]
    listed = base + gg.scale_y_continuous(
        sec_axis=gg.sec_axis(to_fahrenheit, breaks=[40, 60], labels=["cold", "mild"])
    )
    assert scales(listed)["y"]["secondary"]["labels"] == ["cold", "mild"]


def test_guide_none_hides_the_axis() -> None:
    plot = (
        gg.ggplot(CELSIUS, gg.aes("month", "celsius"))
        + gg.geom_line()
        + gg.scale_y_continuous(sec_axis=gg.sec_axis(to_fahrenheit, guide="none"))
    )
    assert "secondary" not in scales(plot)["y"]


def test_facets_flip_and_free_scales() -> None:
    cities = {
        "month": [1, 2, 3, 1, 2, 3],
        "celsius": [-4.0, 0.0, 4.0, 20.0, 25.0, 30.0],
        "city": ["Oslo"] * 3 + ["Cairo"] * 3,
    }
    base = gg.ggplot(cities, gg.aes("month", "celsius")) + gg.geom_line()
    axis = gg.scale_y_continuous(sec_axis=gg.sec_axis(to_fahrenheit))
    free = base + axis + gg.facet_wrap("city", scales="free_y")
    first, second = scales(free, 0)["y"], scales(free, 1)["y"]
    assert first["secondary"]["labels"] != second["secondary"]["labels"]
    # coord_flip: the secondary axis moves with its scale (data y is drawn on top).
    flipped = scales(base + axis + gg.coord_flip())["y"]
    assert flipped["secondary"]["labels"]
    assert "<svg" in (base + axis + gg.coord_flip()).to_svg()


def test_grid_sharing_resamples_the_transform() -> None:
    low = gg.ggplot({"x": [1, 2], "y": [0.0, 10.0]}, gg.aes("x", "y")) + gg.geom_point()
    high = gg.ggplot({"x": [1, 2], "y": [50.0, 100.0]}, gg.aes("x", "y")) + gg.geom_point()
    axis = gg.scale_y_continuous(sec_axis=gg.sec_axis(to_fahrenheit, breaks=[200]))
    # 200 °F = 93.3 °C is outside the first plot's own range but inside the shared one.
    grid = gg.gggrid([low + axis, high], ncol=2, sharey=True, align=True)
    assert ">200<" in grid.to_svg()
    assert ">200<" not in (low + axis).to_svg()


def test_html_zoom_payload_has_fixed_secondary_breaks() -> None:
    plot = (
        gg.ggplot(CELSIUS, gg.aes("month", "celsius"))
        + gg.geom_line()
        + gg.scale_y_continuous(sec_axis=gg.sec_axis(to_fahrenheit))
        + gg.ggtb()
    )
    html = plot.to_html()
    assert '"secondary":[null,{' in html


def monotonic_error(transform: Any) -> str:
    plot = (
        gg.ggplot({"x": [-2.0, 3.0], "y": [1, 2]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.scale_x_continuous(sec_axis=gg.sec_axis(transform))
    )
    with pytest.raises(SpecError) as caught:
        plot.to_svg()
    return str(caught.value)


def test_engine_errors_name_the_transform() -> None:
    message = monotonic_error(lambda x: x * x)
    assert "scales[0].sec_axis.transform" in message
    assert "strictly monotonic" in message
    assert "the transform failed" in monotonic_error(lambda x: 1 / 0)
    assert "ZeroDivisionError" in monotonic_error(lambda x: 1 / 0)
    assert "return a number" in monotonic_error(lambda x: "hot")


@pytest.mark.parametrize(
    ("call", "error", "match"),
    [
        (lambda: gg.sec_axis(3), UnsupportedValueError, "expected a function"),
        (lambda: gg.sec_axis(name=3), UnsupportedValueError, "a text or derive"),
        (lambda: gg.sec_axis(breaks="1"), UnsupportedValueError, "list of numbers"),
        (lambda: gg.sec_axis(breaks=[1, "a"]), UnsupportedValueError, "numbers in the breaks"),
        (lambda: gg.sec_axis(labels=["a"]), SpecError, "needs a list of breaks"),
        (lambda: gg.sec_axis(breaks=[1, 2], labels=["a"]), SpecError, "1 labels for 2 breaks"),
        (lambda: gg.sec_axis(guide="legend"), SpecError, "use 'axis' or 'none'"),
        (
            lambda: gg.scale_y_continuous(sec_axis=to_fahrenheit),
            UnsupportedValueError,
            "a bare function goes in sec_axis",
        ),
    ],
)
def test_bad_options_fail_early(call: Any, error: type[Exception], match: str) -> None:
    with pytest.raises(error, match=match):
        call()


def test_discrete_axes_warn() -> None:
    discrete = (
        gg.ggplot({"x": ["a", "b"], "y": [1, 2]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.scale_x_continuous(sec_axis=gg.dup_axis())
    )
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        discrete.to_svg()
    assert any("needs a continuous axis" in str(item.message) for item in caught)
    assert all(item.category is GgrustWarning for item in caught)


def test_a_constant_axis_keeps_its_secondary_breaks() -> None:
    plot = (
        gg.ggplot({"x": [1, 1, 1], "y": [2, 2, 2]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.scale_y_continuous(sec_axis=gg.sec_axis(lambda value: value * 2))
    )
    secondary = scales(plot)["y"]["secondary"]
    assert secondary["labels"], secondary
    # 2 is widened to 1.5..2.5, so the secondary range is 3..5.
    assert all(3.0 <= float(label) <= 5.0 for label in secondary["labels"])


def test_a_scale_with_a_callable_prints() -> None:
    text = str(gg.scale_y_continuous(sec_axis=gg.sec_axis(to_fahrenheit)))
    assert "to_fahrenheit" in text
