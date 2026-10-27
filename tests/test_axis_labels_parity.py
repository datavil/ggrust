"""Axis `breaks`, `labels` and `format` must give lets-plot's tick labels.

Renders the same plot with lets-plot and ggrust and compares the label lists of both
axes (superscript powers flattened, as in `test_breaks_parity.py`).
"""

import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_breaks_parity import ggrust_labels, lets_plot_labels  # noqa: E402

FRAME = {"x": [1.0, 2.0, 3.0, 4.5], "y": [2.0, 1.0, 4.0, 3.0], "c": ["a", "b", "c", "d"]}
WIDE = {"x": [1.0, 1000.0], "y": [1e6, 3e6]}
TINY = {"x": [1e-9, 5e-9], "y": [0.001, 0.002]}


def point(m, frame=None, x="x", y="y"):
    return m.ggplot(frame or FRAME, m.aes(x, y)) + m.geom_point()


CASES = {
    "breaks": lambda m: point(m) + m.scale_x_continuous(breaks=[1, 2, 3]),
    "breaks_out_of_domain": lambda m: point(m) + m.scale_x_continuous(breaks=[0, 1, 2, 10]),
    "breaks_unsorted": lambda m: point(m) + m.scale_x_continuous(breaks=[3, 1, 2]),
    "breaks_null": lambda m: point(m) + m.scale_x_continuous(breaks=[1, None, 3]),
    "breaks_near_edges": lambda m: point(m) + m.scale_y_continuous(breaks=[0.8, 0.85, 4.1, 4.2]),
    "breaks_labels": lambda m: (
        point(m) + m.scale_x_continuous(breaks=[1, 2, 3], labels=["a", "b", "c"])
    ),
    "labels_dict": lambda m: point(m) + m.scale_x_continuous(labels={1: "one", 3: "three"}),
    "labels_without_breaks": lambda m: point(m) + m.scale_x_continuous(labels=["a", "b"]),
    "breaks_and_limits": lambda m: (
        point(m) + m.scale_x_continuous(limits=[1, 3], breaks=[0.5, 1, 2, 4])
    ),
    "default_fixed_labels": lambda m: (
        point(m, WIDE) + m.scale_x_continuous(breaks=[1, 10, 100, 1000])
    ),
    "fixed_powers": lambda m: point(m, WIDE) + m.scale_y_continuous(breaks=[1e6, 2e6, 3e6]),
    "fixed_fractions": lambda m: (
        point(m, {"x": [1.5, 3.125], "y": [1, 2]}) + m.scale_x_continuous(breaks=[1.5, 2.25, 3.125])
    ),
    "log_breaks": lambda m: point(m, WIDE) + m.scale_x_log10(breaks=[1, 10, 100, 1000]),
    "reverse_breaks": lambda m: point(m) + m.scale_x_reverse(breaks=[1, 2, 4]),
    "format_fixed": lambda m: point(m) + m.scale_x_continuous(format=".2f"),
    "format_template": lambda m: point(m) + m.scale_x_continuous(format="{d} cells"),
    "format_exponent": lambda m: point(m) + m.scale_x_continuous(format="e"),
    "format_percent": lambda m: point(m) + m.scale_y_continuous(format="%"),
    "format_value": lambda m: point(m) + m.scale_x_continuous(format="{} u"),
    "format_literal": lambda m: point(m) + m.scale_x_continuous(format="abc"),
    "format_suffix": lambda m: point(m) + m.scale_y_continuous(format="{.1f}%"),
    "format_powers": lambda m: point(m, WIDE) + m.scale_y_continuous(format=".2e"),
    "format_small": lambda m: point(m, TINY) + m.scale_x_continuous(format="g"),
    "format_log": lambda m: point(m) + m.scale_x_log10(format=".1f"),
    "format_with_breaks": lambda m: (
        point(m) + m.scale_x_continuous(breaks=[1, 2.5, 4], format="{.1f} u")
    ),
    "labels_win_over_format": lambda m: (
        point(m) + m.scale_x_continuous(breaks=[1, 2], labels=["a", "b"], format=".1f")
    ),
    "discrete_breaks": lambda m: point(m, x="c") + m.scale_x_discrete(breaks=["b", "d"]),
    "discrete_labels": lambda m: point(m, x="c") + m.scale_x_discrete(labels=["A", "B", "C", "D"]),
    "discrete_fewer_labels": lambda m: point(m, x="c") + m.scale_x_discrete(labels=["A", "B"]),
    "discrete_breaks_labels": lambda m: (
        point(m, x="c") + m.scale_x_discrete(breaks=["b", "d"], labels=["B", "D"])
    ),
    "discrete_labels_dict": lambda m: (
        point(m, x="c") + m.scale_x_discrete(labels={"a": "A", "c": "C"})
    ),
    "discrete_format": lambda m: point(m, x="c") + m.scale_x_discrete(format="<{}>"),
    "discrete_reversed_labels": lambda m: (
        point(m, x="c") + m.scale_x_discrete_reversed(labels=["A", "B", "C", "D"])
    ),
    # Bug hunt 1.5a.
    "discrete_breaks_lead": lambda m: (
        point(m, x="c") + m.scale_x_discrete(breaks=["d", "a"], labels=["D", "A"])
    ),
    "discrete_break_without_data": lambda m: (
        point(m, x="c") + m.scale_x_discrete(breaks=["zz", "b"])
    ),
    "discrete_breaks_and_limits": lambda m: (
        point(m, x="c") + m.scale_x_discrete(limits=["c", "a", "b"], breaks=["b", "c"])
    ),
    "template_two_fields": lambda m: point(m) + m.scale_y_continuous(format="{.1f}{.2f}"),
    "template_two_values": lambda m: point(m, x="c") + m.scale_x_discrete(format="{}-{}"),
    "percent_literal": lambda m: point(m) + m.scale_y_continuous(format="50%off"),
    "log_value_format": lambda m: (
        point(m, {"x": [1.0, 1e5], "y": [1.0, 2.0]}) + m.scale_x_log10(format="{}")
    ),
    "labels_without_breaks_ignore_format": lambda m: (
        point(m) + m.scale_y_continuous(labels=["a", "b", "c"], format=".2f")
    ),
    "numeric_categories": lambda m: (
        point(m, {"c": ["1", "2.5", "inf", "x"], "y": [1.0, 2.0, 3.0, 4.0]}, x="c")
        + m.scale_x_discrete(format="{.1f}")
    ),
    # Decided with the user: powers inside a label stay superscripts.
    "power_then_text": lambda m: point(m, WIDE) + m.scale_y_continuous(format="{.1e} m"),
    "power_in_brackets": lambda m: point(m, WIDE) + m.scale_y_continuous(format="[{.1e}]"),
    # Spec numbers parse exactly (serde_json float_roundtrip).
    "exact_spec_numbers": lambda m: (
        point(m, {"x": [0.0, 1e30], "y": [1.0, 2.0]})
        + m.scale_x_continuous(breaks=[1e30, 5e29], format=",d")
    ),
    "cellestial_heatmap_axis": lambda m: (
        m.ggplot({"x": [0.0, 1.0, 2.0], "y": [0.0, 1.0, 2.0]}, m.aes("x", "y"))
        + m.geom_point()
        + m.scale_x_continuous(
            breaks=[0, 1, 2], labels=["CD3E", "MS4A1", "LYZ"], limits=[-0.5, 2.5], expand=[0, 0]
        )
    ),
}


def labels(module, name):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        svg = CASES[name](module).to_svg()
    reader = lets_plot_labels if module is lp else ggrust_labels
    # The lets-plot reader drops spaces (they separate superscripts); so do we here.
    clean = lambda texts: [  # noqa: E731
        text.replace("&lt;", "<").replace("&gt;", ">").replace(" ", "") for text in texts
    ]
    return clean(reader(svg, "x")), clean(reader(svg, "y"))


# lets-plot drops a label (with its tick) that would overlap one drawn before it
# (`AxisUtil.TickLabelsMap`). ggrust keeps every break, as ggplot2 does by default
# (`guide_axis(check.overlap = FALSE)`): decided with the user.
DROPS_OVERLAPPING = {"default_fixed_labels"}


@pytest.mark.parametrize(
    "name",
    [
        pytest.param(
            name, marks=pytest.mark.xfail(reason="ggplot2 keeps overlapping labels", strict=True)
        )
        if name in DROPS_OVERLAPPING
        else name
        for name in CASES
    ],
)
def test_axis_labels_match_lets_plot(name):
    assert labels(gg, name) == labels(lp, name)


def test_breaks_and_labels_of_different_lengths_fail_like_lets_plot():
    plot = point(gg) + gg.scale_x_continuous(breaks=[1, 2, 3], labels=["a"])
    with pytest.raises(ValueError, match=r"scales\[0\].labels': 1 labels for 3 breaks"):
        plot.to_svg()


def test_datetime_formats_warn_and_are_ignored():
    plot = point(gg) + gg.scale_x_continuous(format="%Y-%m-%d")
    with pytest.warns(gg.GgrustWarning, match="datetime formats"):
        plot.to_svg()
