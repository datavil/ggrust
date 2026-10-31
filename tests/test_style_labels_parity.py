"""Style scale labels must match lets-plot: ``format=...`` on colorbars, continuous and
discrete legends, and ``lablim=...`` (discrete legends only, 0 means no limit).
"""

import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_bar_parity import _texts  # noqa: E402

FRAME = {
    "x": [1.0, 2.0, 3.0],
    "y": [1.0, 2.0, 3.0],
    "value": [1000.0, 2500.5, 3000.0],
    "name": ["alphabet", "be", "charlie_long"],
}


def _points(module, **aes):
    return module.ggplot(FRAME, module.aes("x", "y", **aes)) + module.geom_point()


CASES = {
    "colorbar_number": lambda m: _points(m, color="value") + m.scale_color_continuous(format=".1f"),
    "colorbar_template": lambda m: (
        _points(m, color="value") + m.scale_color_gradient(format="{d} u")
    ),
    "colorbar_si": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y", fill="value"))
        + m.geom_tile()
        + m.scale_fill_gradient(format="~s")
    ),
    "colorbar_breaks": lambda m: (
        _points(m, color="value") + m.scale_color_continuous(breaks=[1200, 2800], format="{d}$")
    ),
    "colorbar_labels_win": lambda m: (
        _points(m, color="value") + m.scale_color_continuous(labels=["a", "b"], format="{d}$")
    ),
    "colorbar_log": lambda m: (
        _points(m, color="value") + m.scale_color_continuous(trans="log10", format=".1f")
    ),
    "colorbar_lablim_ignored": lambda m: (
        _points(m, color="value")
        + m.scale_color_continuous(breaks=[1500, 2500], labels=["abcdef", "g"], lablim=3)
    ),
    "size_format": lambda m: _points(m, size="value") + m.scale_size(format="{.0f}k"),
    "size_lablim_ignored": lambda m: _points(m, size="value") + m.scale_size(lablim=2),
    "discrete_format": lambda m: _points(m, color="name") + m.scale_color_hue(format="<{}>"),
    "discrete_lablim": lambda m: _points(m, color="name") + m.scale_color_hue(lablim=4),
    "discrete_lablim_zero": lambda m: _points(m, color="name") + m.scale_color_hue(lablim=0),
    "discrete_format_then_lablim": lambda m: (
        _points(m, color="name") + m.scale_color_hue(lablim=3, format="<{}>")
    ),
    "discrete_given_labels_lablim": lambda m: (
        _points(m, color="name") + m.scale_color_hue(labels=["abcdefgh", "x", "yyyyyyy"], lablim=3)
    ),
    "discrete_fractional_lablim": lambda m: (
        _points(m, color="name") + m.scale_color_hue(lablim=2.5)
    ),
    "later_lablim_wins": lambda m: (
        _points(m, color="name") + m.scale_color_hue(lablim=3) + m.scale_color_hue(lablim=0)
    ),
    "later_format_wins": lambda m: (
        _points(m, color="value")
        + m.scale_color_continuous(format=".1f")
        + m.scale_color_continuous(format="{d} u")
    ),
}


@pytest.mark.parametrize("name", sorted(CASES))
def test_style_labels_match_lets_plot(name):
    expected, actual = (CASES[name](module).to_svg() for module in (lp, gg))
    assert sorted(_texts(actual)) == sorted(_texts(expected))


def test_bad_lablim_and_format_are_reported():
    plot = _points(gg, color="name") + gg.scale_color_hue(lablim="x", format="%Y-%m")
    with pytest.warns(UserWarning) as records:
        plot.to_svg()
    messages = " ".join(str(record.message) for record in records)
    assert "lablim" in messages and "number of characters" in messages
    assert "datetime formats" in messages
