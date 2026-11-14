"""gganimate transition_time and transition_reveal (plan.md an3).

Frame positions, frame times and the revealed rows are checked against gganimate's
``get_times`` / ``get_row_along`` and tweenr's ``tween_along`` in the Rust unit tests
(``tests/fixtures/generate_animation_reference.R``). These tests cover the Python
API, its errors, labels and the frames drawn.
"""

from __future__ import annotations

import datetime
import io
import re
import warnings

import numpy as np
import polars as pl
import pytest
from ggrust import (
    aes,
    animate,
    geom_line,
    geom_point,
    ggload,
    ggplot,
    ggsave,
    ggsize,
    ggtitle,
    theme,
    transition_reveal,
    transition_time,
)
from ggrust._render import GgrustWarning
from ggrust.errors import SpecError
from ggrust.plot.core import PlotSpec
from PIL import Image

CITIES = pl.DataFrame(
    {
        "year": [2000, 2000, 2010, 2010, 2020, 2020],
        "city": ["Oslo", "Lima"] * 3,
        "income": [30.0, 8.0, 42.0, 11.0, 55.0, 15.0],
        "life": [79.0, 72.0, 81.0, 75.0, 83.0, 77.0],
    }
)
VISITS = pl.DataFrame(
    {
        "day": [datetime.date(2024, 5, day) for day in (1, 2, 3, 4, 5)],
        "visitors": [120.0, 180.0, 150.0, 210.0, 260.0],
    }
)


def cities_plot() -> PlotSpec:
    return (
        ggplot(CITIES, aes("income", "life", color="city"))
        + geom_point(size=4)
        + transition_time("year")
        + ggtitle("{frame_time}")
        + theme(legend_position="none")
        + ggsize(300, 200)
    )


def visits_plot() -> PlotSpec:
    return (
        ggplot(VISITS, aes("day", "visitors"))
        + geom_line()
        + geom_point(size=4)
        + transition_reveal("day")
        + ggtitle("{frame_along}")
        + ggsize(300, 200)
    )


def pixels(plot: PlotSpec) -> np.ndarray:
    buffer = io.BytesIO()
    plot.to_png(buffer, scale=1)
    return np.asarray(Image.open(io.BytesIO(buffer.getvalue())).convert("RGB"), dtype=np.int16)


def titles(plot: PlotSpec) -> list[str]:
    return re.findall(r"<text[^>]*>([^<]*)</text>", plot.to_svg())


def circles(plot: PlotSpec) -> list[tuple[float, float]]:
    svg = plot.to_svg()
    return [
        (float(x), float(y))
        for x, y in re.findall(r'<circle[^>]*cx="([^"]+)"[^>]*cy="([^"]+)"', svg)
    ]


def test_spec_and_round_trip(tmp_path):
    plot = cities_plot()
    assert plot.as_dict()["animation"]["transition"] == {"name": "time", "time": "year"}
    revealed = visits_plot() + transition_reveal(
        "day", range=(datetime.date(2024, 5, 1), datetime.date(2024, 5, 9)), keep_last=False
    )
    assert revealed.as_dict()["animation"]["transition"] == {
        "name": "reveal",
        "along": "day",
        "range": [1714521600000.0, 1715212800000.0],
        "keep_last": False,
    }
    for original in (plot, revealed):
        loaded = ggload(ggsave(original, "plot.ggrs", path=str(tmp_path)))
        assert loaded.as_dict()["animation"] == original.as_dict()["animation"]
        assert animate(loaded, nframes=5).nframes == 5


@pytest.mark.parametrize(
    ("make", "path"),
    [
        (lambda: transition_time(""), "animation.transition.time"),
        (lambda: transition_time(3), "animation.transition.time"),
        (lambda: transition_time("year", range=(5, 1)), "animation.transition.range"),
        (lambda: transition_time("year", range="2000"), "animation.transition.range"),
        (lambda: transition_time("year", range=(1, "b")), "animation.transition.range"),
        (lambda: transition_reveal(None), "animation.transition.along"),
        (lambda: transition_reveal("day", keep_last=1), "animation.transition.keep_last"),
        (lambda: transition_reveal("day", range=(1, 2, 3)), "animation.transition.range"),
    ],
)
def test_bad_arguments_name_the_option(make, path):
    with pytest.raises(SpecError) as error:
        make()
    assert error.value.path == path


def test_engine_errors():
    with pytest.raises(SpecError, match="did you mean 'year'"):
        animate(cities_plot() + transition_time("yaer")).nframes  # noqa: B018
    with pytest.raises(SpecError, match="categorical") as error:
        animate(cities_plot() + transition_time("city")).nframes  # noqa: B018
    assert error.value.path == "animation.transition.time"
    one = CITIES.filter(pl.col("year") == 2000)
    with pytest.raises(SpecError, match="one value"):
        animate(ggplot(one, aes("income", "life")) + geom_point() + transition_time("year")).nframes  # noqa: B018
    # A range makes one value fine.
    ranged = ggplot(one, aes("income", "life")) + geom_point()
    assert animate(ranged + transition_time("year", range=(1990, 2010)), nframes=3).nframes == 3
    with pytest.raises(SpecError, match="frame_time"):
        animate(cities_plot() + ggtitle("{frame_along}")).nframes  # noqa: B018


def test_time_frames_hold_tween_and_label():
    animation = animate(cities_plot(), nframes=21)
    assert animation.nframes == 21
    # 2000 at frame 0, 2010 at frame 10, 2020 at frame 20; integers print rounded.
    assert "2000" in titles(animation.frame(0))
    assert "2005" in titles(animation.frame(5))
    assert "2020" in titles(animation.frame(20))
    start, middle, arrived = (circles(animation.frame(index)) for index in (0, 5, 10))
    assert len(start) == len(middle) == len(arrived) == 2
    for begin, half, end in zip(start, middle, arrived, strict=True):
        assert half == pytest.approx(((begin[0] + end[0]) / 2, (begin[1] + end[1]) / 2), abs=0.02)
    numeric = cities_plot().with_data(CITIES.with_columns(pl.col("year").cast(pl.Float64)))
    assert "2006.66666666667" in titles(animate(numeric, nframes=4).frame(1))


def test_time_layers_use_their_own_times():
    late = CITIES.filter(pl.col("year") >= 2010)
    plot = (
        ggplot(CITIES, aes("income", "life"))
        + geom_point(size=3)
        + geom_point(data=late, color="red", size=6)
        + transition_time("year")
        + ggsize(300, 200)
    )
    animation = animate(plot, nframes=21)
    # Before 2010 the late layer draws nothing; at 2010 it shows as it is.
    assert len(circles(animation.frame(3))) == 2
    assert len(circles(animation.frame(10))) == 4


def test_times_on_one_frame_warn():
    data = pl.DataFrame({"t": [0.0, 0.1, 1.0, 2.0], "x": [1.0, 2.0, 3.0, 4.0], "y": [1.0] * 4})
    plot = ggplot(data, aes("x", "y")) + geom_point() + transition_time("t")
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        animate(plot, nframes=3).frame(0).to_svg()
    messages = [str(warning.message) for warning in caught if warning.category is GgrustWarning]
    assert any("1 of 4 values of 't'" in message for message in messages), messages


def test_reveal_grows_lines_and_moves_points():
    animation = animate(visits_plot(), nframes=9)
    assert "2024-05-01" in titles(animation.frame(0))
    assert "2024-05-01" in titles(animation.frame(1))
    assert "2024-05-02" in titles(animation.frame(2))
    assert "2024-05-05" in titles(animation.frame(8))
    # One point travels: halfway between day 1 and day 2 at frame 1.
    first, half, second = (circles(animation.frame(index)) for index in (0, 1, 2))
    assert len(first) == len(half) == len(second) == 1
    assert half[0] == pytest.approx(
        ((first[0][0] + second[0][0]) / 2, (first[0][1] + second[0][1]) / 2), abs=0.02
    )
    # The line keeps every reached row plus the moving end: its path grows.
    paths = [
        re.findall(r'<path[^>]*d="([^"]+)"', animation.frame(index).to_svg()) for index in (1, 4, 8)
    ]
    lengths = [max(len(re.findall(r"[ML]", d)) for d in found) for found in paths]
    assert lengths == [2, 3, 5]


def test_reveal_keep_last_holds_points():
    plot = visits_plot() + transition_reveal(
        "day", range=(datetime.date(2024, 5, 1), datetime.date(2024, 5, 9))
    )
    kept = animate(plot, nframes=9).frame(8)
    dropped = animate(
        plot
        + transition_reveal(
            "day", range=(datetime.date(2024, 5, 1), datetime.date(2024, 5, 9)), keep_last=False
        ),
        nframes=9,
    ).frame(8)
    assert len(circles(kept)) == 1
    assert len(circles(dropped)) == 0
    # Lines keep their rows either way.
    assert re.search(r"<path[^>]*d=", dropped.to_svg())


def test_datetime_and_time_of_day_labels():
    stamps = [datetime.datetime(2024, 1, 1, hour) for hour in (0, 6, 12)]
    data = pl.DataFrame({"at": stamps, "y": [1.0, 2.0, 3.0]})
    plot = (
        ggplot(data, aes("at", "y"))
        + geom_line()
        + transition_reveal("at")
        + ggtitle("{frame_along}")
    )
    animation = animate(plot, nframes=5)
    assert "2024-01-01" in titles(animation.frame(0))
    assert "2024-01-01 03:00:00" in titles(animation.frame(1))
    times = pl.DataFrame({"at": [datetime.time(9), datetime.time(17)], "y": [1.0, 2.0]})
    plot = (
        ggplot(times, aes("at", "y"))
        + geom_point()
        + transition_time("at")
        + ggtitle("{frame_time}")
    )
    assert "13:00:00" in titles(animate(plot, nframes=3).frame(1))


def test_range_accepts_numpy_and_rejects_missing_values():
    import numpy as np

    plot = transition_time(
        "day",
        range=(np.datetime64("2024-05-01T00:00:00.000000000"), np.datetime64("2024-05-09")),
    )
    assert plot.props()["transition"]["range"] == [1714521600000.0, 1715212800000.0]
    for bad in [
        (np.datetime64("NaT", "ns"), np.datetime64("2024-05-09")),
        (np.timedelta64(5, "D"), np.timedelta64(9, "D")),
    ]:
        with pytest.raises(SpecError) as error:
            transition_time("day", range=bad)
        assert error.value.path == "animation.transition.range"
