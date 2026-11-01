"""Date-time and duration axes against lets-plot.

Axes are compared through their labels (same breaks and patterns give the same
texts). lets-plot reads naive datetimes in the local time zone, so the
comparisons run with ``TZ=UTC``.
"""

import datetime as dt
import re
import time
import warnings
from collections.abc import Callable, Iterator
from types import ModuleType

import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402


@pytest.fixture(autouse=True)
def _utc(monkeypatch: pytest.MonkeyPatch) -> Iterator[None]:
    monkeypatch.setenv("TZ", "UTC")
    time.tzset()
    yield
    monkeypatch.undo()
    time.tzset()


def _labels(svg: str) -> list[str]:
    texts = re.findall(r"<text[^>]*>(.*?)</text>", svg, flags=re.DOTALL)
    labels = [re.sub(r"<[^>]*>", "", text).strip() for text in texts]
    return sorted(label for label in labels if label)


def _times(count: int, step: dt.timedelta) -> list[dt.datetime]:
    return [dt.datetime(2021, 1, 1) + step * index for index in range(count)]


def _values(count: int) -> list[int]:
    return [(index * 37) % 11 - 5 for index in range(count)]


def _line(count: int, step: dt.timedelta) -> Callable[[ModuleType], object]:
    return lambda m: (
        m.ggplot({"d": _times(count, step), "v": _values(count)}, m.aes("d", "v")) + m.geom_line()
    )


DAY = dt.timedelta(days=1)
Case = Callable[[ModuleType], object]

CASES: dict[str, Case] = {
    "days": _line(31, DAY),
    "months": _line(400, DAY),
    "years": _line(40, 365 * DAY),
    "hours": _line(48, dt.timedelta(hours=1)),
    "minutes": _line(90, dt.timedelta(minutes=1)),
    "seconds": _line(40, dt.timedelta(seconds=1)),
    "break_width": lambda m: _line(90, DAY)(m) + m.scale_x_datetime(break_width="2 weeks"),
    "format": lambda m: _line(200, DAY)(m) + m.scale_x_datetime(format="%b %Y"),
    "polars_datetime": lambda m: (
        m.ggplot(pl.DataFrame({"d": _times(60, DAY), "v": _values(60)}), m.aes("d", "v"))
        + m.geom_point()
    ),
    "polars_date": lambda m: (
        m.ggplot(
            pl.DataFrame({"d": [day.date() for day in _times(5, DAY)], "v": _values(5)}),
            m.aes("d", "v"),
        )
        + m.geom_point()
    ),
    "y_axis": lambda m: (
        m.ggplot({"d": _times(31, DAY), "v": _values(31)}, m.aes("v", "d")) + m.geom_point()
    ),
    "epoch_numbers": lambda m: (
        m.ggplot({"d": [1.6e12 + index * 3.6e6 for index in range(30)], "v": _values(30)})
        + m.geom_point(m.aes("d", "v"))
        + m.scale_x_datetime()
    ),
    "durations": lambda m: (
        m.ggplot({"d": [index * 600_000 for index in range(40)], "v": _values(40)})
        + m.geom_point(m.aes("d", "v"))
        + m.scale_x_time()
    ),
    "duration_days": lambda m: (
        m.ggplot({"d": [index * 18_000_000 for index in range(40)], "v": _values(40)})
        + m.geom_point(m.aes("d", "v"))
        + m.scale_x_time()
    ),
    "timedelta": lambda m: (
        m.ggplot({"d": [dt.timedelta(seconds=17 * i) for i in range(40)], "v": _values(40)})
        + m.geom_point(m.aes("d", "v"))
        + m.scale_x_time()
    ),
}


@pytest.mark.parametrize("name", list(CASES))
def test_axis_labels_match_lets_plot(name: str) -> None:
    ours = CASES[name](gg).to_svg()  # type: ignore[attr-defined]
    theirs = CASES[name](lp).to_svg()  # type: ignore[attr-defined]
    assert _labels(ours) == _labels(theirs)


def test_fixed_breaks_use_the_axis_pattern() -> None:
    # lets-plot writes ISO timestamps here; ggrust labels them like generated breaks.
    plot = _line(31, DAY)(gg) + gg.scale_x_datetime(
        breaks=[dt.datetime(2021, 1, 5), dt.datetime(2021, 1, 20)]
    )
    labels = _labels(plot.to_svg())  # type: ignore[attr-defined]
    assert "Jan 5" in labels and "Jan 20" in labels


def test_zoned_columns_warn_about_utc() -> None:
    frame = pl.DataFrame({"d": _times(31, DAY), "v": _values(31)}).with_columns(
        pl.col("d").dt.replace_time_zone("Europe/Berlin")
    )
    with pytest.warns(UserWarning, match="time zone 'Europe/Berlin'.*UTC"):
        (gg.ggplot(frame, gg.aes("d", "v")) + gg.geom_point()).to_svg()


def test_bad_break_width_raises() -> None:
    plot = _line(31, DAY)(gg) + gg.scale_x_datetime(break_width="2 fortnights")
    with pytest.raises(SpecError, match="unknown time unit 'fortnights'"):
        plot.to_svg()  # type: ignore[attr-defined]
