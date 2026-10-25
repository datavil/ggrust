"""Scale options lets-plot accepts must draw the same: labels and kept points.

Cases come from the 0.10.2 bug hunt (limits order, open sides, empty lists, nulls,
constants winning over mappings).
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_breaks_parity import ggrust_labels, lets_plot_labels  # noqa: E402

NUMBERS = {"x": [1.0, 10.0], "y": [1.0, 2.0]}
CATEGORIES = {"x": ["b", "a", "c", "a"], "y": [1.0, 2.0, 3.0, 4.0]}


def base(m, frame):
    return m.ggplot(frame, m.aes("x", "y")) + m.geom_point()


CASES = {
    "reverse_descending_limits": lambda m: base(m, NUMBERS) + m.scale_x_reverse(limits=[10, 1]),
    "reverse_then_xlim": lambda m: base(m, NUMBERS) + m.scale_x_reverse() + m.xlim(10, 1),
    "descending_xlim": lambda m: base(m, NUMBERS) + m.xlim(10, 1),
    "log_limit_at_zero": lambda m: base(m, NUMBERS) + m.scale_x_log10(limits=[0, 100]),
    "log2_open_limit": lambda m: base(m, NUMBERS) + m.scale_y_log2(limits=[0, None]),
    "empty_continuous_limits": lambda m: base(m, NUMBERS) + m.scale_x_continuous(limits=[]),
    "empty_discrete_limits": lambda m: base(m, CATEGORIES) + m.scale_x_discrete(limits=[]),
    "discrete_limit_with_none": lambda m: base(m, CATEGORIES) + m.xlim("a", None),
    "log_on_discrete_axis": lambda m: base(m, CATEGORIES) + m.scale_x_log10(),
    "constant_overrides_mapping": lambda m: base(m, CATEGORIES) + m.geom_point(x=2.5),
    "constant_only_layer": lambda m: m.ggplot({"a": [1.0, 2.0]}) + m.geom_point(x=1.5, y=1.5),
}


@pytest.mark.parametrize("name", CASES)
def test_scale_case_matches_lets_plot(name):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs = CASES[name](lp).to_svg()
        ours = CASES[name](gg).to_svg()
    assert ours.count("<circle") == len(re.findall(r"<circle", theirs)), "kept points"
    for axis in "xy":
        assert ggrust_labels(ours, axis) == lets_plot_labels(theirs, axis), f"{axis} labels"
