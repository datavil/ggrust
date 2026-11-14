"""gganimate transition_states and ease_aes (plan.md an2).

The frame split, the colour mixing in Lab and the easing curves are checked against
R (gganimate's ``distribute_frames`` and farver) in the Rust unit tests
(``tests/fixtures/generate_animation_reference.R``); gganimate itself is not
installed here. These tests cover the Python API, its errors and the frames drawn.
"""

from __future__ import annotations

import io
import re
import warnings

import numpy as np
import polars as pl
import pytest
from ggrust import (
    aes,
    animate,
    ease_aes,
    geom_line,
    geom_point,
    ggload,
    ggplot,
    ggsave,
    ggsize,
    ggtitle,
    transition_manual,
    transition_states,
)
from ggrust._render import GgrustWarning
from ggrust.errors import SpecError
from ggrust.plot.core import PlotSpec
from PIL import Image

SALES = pl.DataFrame(
    {
        "year": [2021, 2021, 2021, 2022, 2022, 2022, 2023, 2023, 2023],
        "fruit": ["apple", "pear", "plum"] * 3,
        "sold": [10.0, 4.0, 3.0, 12.0, 7.0, 5.0, 15.0, 9.0, 8.0],
        "price": [1.0, 1.5, 2.0, 1.1, 1.6, 2.2, 1.3, 1.4, 2.4],
    }
)


def sales_plot() -> PlotSpec:
    return (
        ggplot(SALES, aes("price", "sold", color="fruit"))
        + geom_point(size=4)
        + transition_states("year", wrap=False)
        + ggtitle("{closest_state}", subtitle="{previous_state}>{next_state} {transitioning}")
        + ggsize(300, 200)
    )


def pixels(plot: PlotSpec) -> np.ndarray:
    buffer = io.BytesIO()
    plot.to_png(buffer, scale=1)
    return np.asarray(Image.open(io.BytesIO(buffer.getvalue())).convert("RGB"), dtype=np.int16)


def titles(plot: PlotSpec) -> list[str]:
    return re.findall(r"<text[^>]*>([^<]*)</text>", plot.to_svg())


def test_spec_and_merge():
    plot = sales_plot() + ease_aes("cubic-in-out", y="bounce-out", colour="sine-in")
    animation = plot.as_dict()["animation"]
    assert animation["transition"] == {
        "name": "states",
        "states": "year",
        "transition_length": 1.0,
        "state_length": 1.0,
        "wrap": False,
    }
    assert animation["ease"] == {
        "default": "cubic-in-out",
        "aesthetics": {"y": "bounce-out", "colour": "sine-in"},
    }
    # A later transition replaces the earlier one and keeps the easing.
    replaced = plot + transition_manual("year")
    assert replaced.as_dict()["animation"]["transition"]["name"] == "manual"
    assert replaced.as_dict()["animation"]["ease"]["default"] == "cubic-in-out"
    lengths = transition_states("year", transition_length=[1, 2], state_length=0)
    assert lengths.as_dict()["transition"]["transition_length"] == [1.0, 2.0]


@pytest.mark.parametrize(
    ("arguments", "path"),
    [
        ({"states": ""}, "animation.transition.states"),
        ({"states": 3}, "animation.transition.states"),
        ({"states": "year", "transition_length": -1}, "animation.transition.transition_length"),
        ({"states": "year", "state_length": []}, "animation.transition.state_length"),
        ({"states": "year", "state_length": "1"}, "animation.transition.state_length"),
        ({"states": "year", "wrap": 1}, "animation.transition.wrap"),
    ],
)
def test_transition_states_rejects_bad_arguments(arguments, path):
    with pytest.raises(SpecError) as caught:
        transition_states(**arguments)
    assert caught.value.path == path


def test_ease_aes_rejects_unknown_eases_with_a_close_match():
    with pytest.raises(SpecError, match="did you mean 'cubic-in-out'") as caught:
        ease_aes("cubic-inout")
    assert caught.value.path == "animation.ease.default"
    with pytest.raises(SpecError, match="-in-out") as caught:
        ease_aes(y="bounce")
    assert caught.value.path == "animation.ease.aesthetics.y"


def test_engine_rejects_unknown_aesthetics_and_columns():
    with pytest.raises(SpecError, match="did you mean 'color'") as caught:
        animate(sales_plot() + ease_aes(colr="linear")).nframes  # noqa: B018
    assert caught.value.path == "animation.ease.aesthetics.colr"
    plot = ggplot(SALES, aes("price", "sold")) + geom_point() + transition_states("yeer")
    with pytest.raises(SpecError, match="did you mean 'year'") as caught:
        animate(plot).nframes  # noqa: B018
    assert caught.value.path == "animation.transition.states"


def test_frame_counts_follow_gganimate():
    # No wrap: 10 frames over 3 holds and 2 moves of equal length.
    assert animate(sales_plot(), nframes=10).nframes == 10
    # Wrap plans nframes + 1 scene frames; 100 of them are shown.
    wrapped = sales_plot() + transition_states("year")
    assert animate(wrapped).nframes == 100


def test_frames_hold_tween_and_label():
    animation = animate(sales_plot(), nframes=10)
    # Holds of 2 frames, moves of 2: 2021 2021 | tween 2022 2022 2022 | tween 2023...
    first, held, tween, arrived = (pixels(animation.frame(index)) for index in (0, 1, 2, 3))
    assert np.array_equal(first, held)
    assert not np.array_equal(tween, first)
    assert not np.array_equal(tween, arrived)
    assert "2021" in titles(animation.frame(0))
    assert "2021&gt;2021 FALSE" in titles(animation.frame(0))
    assert "2021&gt;2022 TRUE" in titles(animation.frame(2))
    assert "2022" in titles(animation.frame(3))


def test_ease_aes_changes_only_the_moving_frames():
    linear = animate(sales_plot(), nframes=10)
    eased = animate(sales_plot() + ease_aes("exponential-in"), nframes=10)
    assert np.array_equal(pixels(linear.frame(0)), pixels(eased.frame(0)))
    assert np.array_equal(pixels(linear.frame(3)), pixels(eased.frame(3)))
    assert not np.array_equal(pixels(linear.frame(2)), pixels(eased.frame(2)))


def test_lines_with_uneven_points_warn():
    frame = pl.DataFrame(
        {"step": [1, 1, 1, 2, 2], "x": [0.0, 1.0, 2.0, 0.0, 2.0], "y": [0.0, 1.0, 0.0, 1.0, 1.0]}
    )
    plot = ggplot(frame, aes("x", "y")) + geom_line() + transition_states("step")
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        animate(plot, nframes=6).frame(2).to_svg()
    messages = [str(warning.message) for warning in caught if warning.category is GgrustWarning]
    assert any("geom_line" in message and "matched in order" in message for message in messages)


def test_plot_files_keep_the_easing(tmp_path):
    plot = sales_plot() + ease_aes("cubic-in-out", y="bounce-out")
    path = ggsave(plot, "sales.ggrs", path=str(tmp_path))
    loaded = ggload(path)
    assert loaded.as_dict()["animation"] == plot.as_dict()["animation"]
    assert animate(loaded, nframes=10).nframes == 10


def test_ease_aes_alone_draws_a_still_plot():
    plot = ggplot(SALES, aes("price", "sold")) + geom_point() + ease_aes("cubic-in-out")
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        assert "<svg" in plot.to_svg()
    with pytest.raises(SpecError, match="no transition"):
        animate(plot).nframes  # noqa: B018


def test_lengths_accept_numpy_and_polars():
    states = transition_states(
        "year", transition_length=np.int64(2), state_length=pl.Series([1, 2])
    ).as_dict()["transition"]
    assert states["transition_length"] == 2.0
    assert states["state_length"] == [1.0, 2.0]
    assert transition_states("year", state_length=np.array([1, 3])).as_dict()["transition"][
        "state_length"
    ] == [1.0, 3.0]
