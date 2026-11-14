"""gganimate enter/exit effects and shadows (plan.md an4, trimmed).

The tweened rows of entering and exiting marks and the wake steps are checked
against tweenr's ``.complete_states``, gganimate's transmuters and
``shadow_wake`` in the Rust unit tests
(``tests/fixtures/generate_animation_reference.R``). These tests cover the Python
API, its errors, how effects compose with ``+`` and the frames drawn.
"""

from __future__ import annotations

import re
import warnings

import polars as pl
import pytest
from ggrust import (
    aes,
    animate,
    enter_fade,
    enter_grow,
    exit_fade,
    exit_shrink,
    geom_col,
    geom_point,
    ggload,
    ggplot,
    ggsave,
    shadow_mark,
    shadow_wake,
    transition_states,
    transition_time,
)
from ggrust.errors import SpecError
from ggrust.plot.core import PlotSpec

# 2021: apple, pear, plum; 2022: apple, pear, kiwi; 2023: apple, kiwi, fig.
FRUIT = pl.DataFrame(
    {
        "year": [2021] * 3 + [2022] * 3 + [2023] * 3,
        "fruit": ["apple", "pear", "plum", "apple", "pear", "kiwi", "apple", "kiwi", "fig"],
        "sold": [10.0, 6.0, 4.0, 12.0, 8.0, 5.0, 15.0, 9.0, 7.0],
    }
)


def fruit_points() -> PlotSpec:
    return (
        ggplot(FRUIT, aes("fruit", "sold", group="fruit"))
        + geom_point(size=5)
        + transition_states("year", wrap=False)
    )


def point_count(plot: PlotSpec) -> int:
    """Data points drawn in the panel of a still frame (SVG circles)."""
    return len(re.findall(r"<circle", plot.to_svg()))


def test_effects_compose_with_plus():
    plot = fruit_points() + enter_fade() + enter_grow(size=0.2) + exit_shrink() + exit_fade(0.1)
    animation = plot.as_dict()["animation"]
    assert animation["enter"] == [
        {"name": "fade", "alpha": 0.0},
        {"name": "grow", "size": 0.2},
    ]
    assert animation["exit"] == [
        {"name": "shrink", "size": 0.0},
        {"name": "fade", "alpha": 0.1},
    ]
    # The transition is kept; a later shadow replaces an earlier one.
    assert animation["transition"]["name"] == "states"
    shadowed = plot + shadow_wake(0.1) + shadow_mark()
    assert shadowed.as_dict()["animation"]["shadow"]["name"] == "mark"


def test_shadow_wake_spec_follows_gganimate_defaults():
    shadow = shadow_wake(0.1).props()["shadow"]
    assert shadow == {
        "name": "wake",
        "wake_length": 0.1,
        "size": 0.0,
        "alpha": 0.0,
        "colour": None,
        "fill": None,
        "falloff": "cubic-in",
        "wrap": True,
        "exclude_layer": None,
        "exclude_phase": ["enter", "exit"],
    }
    custom = shadow_wake(
        0.5, size=False, alpha=0.2, color="grey", exclude_layer=2, exclude_phase=None
    ).props()["shadow"]
    assert custom["size"] is False
    assert shadow_wake(0.1, size=False).as_dict()["shadow"]["size"] is False
    assert custom["alpha"] == 0.2
    assert custom["colour"] == "grey"
    assert custom["exclude_layer"] == [2]
    assert custom["exclude_phase"] == []
    assert shadow_wake(0.1, exclude_phase="enter").props()["shadow"]["exclude_phase"] == ["enter"]


@pytest.mark.parametrize(
    ("make", "path"),
    [
        (lambda: enter_fade(alpha=1.5), "animation.enter.alpha"),
        (lambda: exit_fade(alpha="0"), "animation.exit.alpha"),
        (lambda: enter_grow(size=-1), "animation.enter.size"),
        (lambda: exit_shrink(size=True), "animation.exit.size"),
        (lambda: shadow_wake(0), "animation.shadow.wake_length"),
        (lambda: shadow_wake(1.5), "animation.shadow.wake_length"),
        (lambda: shadow_wake(0.1, alpha=2), "animation.shadow.alpha"),
        (lambda: shadow_wake(0.1, wrap=1), "animation.shadow.wrap"),
        (lambda: shadow_wake(0.1, exclude_layer=0), "animation.shadow.exclude_layer"),
        (lambda: shadow_wake(0.1, colour="red", color="blue"), "animation.shadow.colour"),
        (lambda: shadow_mark(past="yes"), "animation.shadow.past"),
        (lambda: shadow_mark(alpha=[0.3]), "animation.shadow.aesthetics.alpha"),
        (lambda: shadow_mark(alpha=5), "animation.shadow.aesthetics.alpha"),
        (lambda: shadow_mark(size=-3), "animation.shadow.aesthetics.size"),
        (lambda: shadow_mark(alpha=float("nan")), "animation.shadow.aesthetics.alpha"),
        (lambda: shadow_mark(color="red", colour="blue"), "animation.shadow.aesthetics.colour"),
    ],
)
def test_bad_arguments_raise_spec_errors(make, path):
    with pytest.raises(SpecError) as caught:
        make()
    assert caught.value.path == path


def test_unknown_names_suggest_a_close_match():
    with pytest.raises(SpecError, match="did you mean 'cubic-in'"):
        shadow_wake(0.1, falloff="cubic_in")
    with pytest.raises(SpecError, match="did you mean 'enter'"):
        shadow_wake(0.1, exclude_phase=["entr"])
    with pytest.raises(SpecError, match="did you mean 'alpha'"):
        shadow_mark(alpah=0.3)


def test_enter_and_exit_draw_marks_while_moving():
    # 10 frames, no wrap: 2021 2021 | tween | 2022 2022 2022 | tween | 2023 ...
    plain = animate(fruit_points(), nframes=10)
    assert point_count(plain.frame(2)) == 2  # apple and pear move, plum pops out
    faded = animate(fruit_points() + enter_fade() + exit_fade(), nframes=10)
    # Plum fades out and kiwi fades in while apple and pear move.
    assert point_count(faded.frame(2)) == 4
    assert point_count(faded.frame(0)) == 3
    assert point_count(faded.frame(3)) == 3


def test_enter_grow_grows_bars_from_the_axis():
    def bars(plot: PlotSpec) -> list[float]:
        """Heights of the bars drawn in frame 2 (a move from 2021 to 2022)."""
        svg = animate(plot, nframes=10).frame(2).to_svg()
        return [
            float(height)
            for height in re.findall(
                r'<rect x="[^"]+" y="[^"]+" width="[^"]+" height="([^"]+)"', svg
            )
        ]

    plot = (
        ggplot(FRUIT, aes("fruit", "sold", group="fruit"))
        + geom_col()
        + transition_states("year", wrap=False)
    )
    plain, grown = bars(plot), bars(plot + enter_grow())
    # The entering kiwi bar is drawn while moving, shorter than its 2022 height.
    assert len(grown) == len(plain) + 1
    assert 0 < min(grown) < min(plain)


def test_effects_work_with_transition_time():
    plot = (
        ggplot(FRUIT, aes("fruit", "sold", group="fruit"))
        + geom_point(size=5)
        + transition_time("year")
        + enter_fade()
        + exit_fade()
    )
    animation = animate(plot, nframes=11)
    # Frame 3 lies between 2021 (frame 1) and 2022 (frame 6).
    assert point_count(animation.frame(2)) == 4


def test_shadow_mark_keeps_earlier_states():
    plot = fruit_points() + shadow_mark(alpha=0.3, color="grey")
    animation = animate(plot, nframes=10)
    # Raw frames: 1 (2021), 5 (2022), 9 (2023).
    assert point_count(animation.frame(0)) == 3
    assert point_count(animation.frame(3)) == 3 + 3
    assert point_count(animation.frame(9)) == 3 + 3 + 3
    svg = animation.frame(9).to_svg()
    assert svg.count('fill="#808080" fill-opacity="0.302"') == 6


def test_shadow_wake_trails_moving_marks():
    plot = (
        ggplot(FRUIT, aes("fruit", "sold", group="fruit"))
        + geom_point(size=5)
        + transition_time("year")
        + shadow_wake(0.2, size=False, alpha=False, wrap=False)
    )
    animation = animate(plot, nframes=11)
    # Frame 4: wake of frames 2 and 3, each drawing apple and pear, unchanged
    # (size=False, alpha=False), each element's wake drawn right before it.
    circles = re.findall(r'<circle cx="([^"]+)"', animation.frame(3).to_svg())
    assert len(circles) == 2 + 2 + 2
    assert circles[:3] == [circles[0]] * 3
    # With the default size=True the oldest step has size 0 and is not drawn.
    shrinking = animate(plot + shadow_wake(0.2, wrap=False), nframes=11)
    assert point_count(shrinking.frame(3)) == 2 + 2
    excluded = animate(plot + shadow_wake(0.2, wrap=False, exclude_layer=1), nframes=11)
    assert point_count(excluded.frame(3)) == 2


def test_effects_alone_draw_a_still_plot():
    plot = ggplot(FRUIT, aes("fruit", "sold")) + geom_point() + enter_fade() + shadow_wake(0.1)
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        assert "<svg" in plot.to_svg()


def test_plot_files_keep_effects(tmp_path):
    plot = fruit_points() + enter_fade() + exit_shrink() + shadow_wake(0.1, colour="red")
    path = ggsave(plot, "fruit.ggrs", path=str(tmp_path))
    assert ggload(path).as_dict()["animation"] == plot.as_dict()["animation"]


def test_engine_errors_name_the_spec_path():
    plot = fruit_points()
    plot.props()["animation"].props()["enter"] = [{"name": "fly"}]
    with pytest.raises(SpecError, match=r"animation\.enter\[0\]\.name.*accepted: fade, grow"):
        animate(plot).nframes  # noqa: B018
