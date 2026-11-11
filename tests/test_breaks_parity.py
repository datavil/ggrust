"""Axis breaks and labels must match lets-plot exactly ("same rules, real fonts").

Renders the same scatter with lets-plot and ggrust and compares tick label lists.
"""

import re
import warnings

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402


def lets_plot_labels(svg: str, axis: str) -> list[str]:
    svg = re.sub(r"\s+", " ", svg)
    texts = re.findall(rf'class="axis-text-{axis}"[^>]*>(.*?)</text>', svg)
    # Superscripts are tspans; zero-width spaces separate them.
    return [
        re.sub(r"<[^>]+>", "", text).replace("\u200b", "").replace(" ", "").replace("\u2212", "-")
        for text in texts
    ]


def ggrust_labels(svg: str, axis: str) -> list[str]:
    group = re.search(rf'<g class="axis-{axis}">(.*?)</g>', svg, re.S).group(1)
    texts = re.findall(r"<text[^>]*>(.*?)</text>", group)
    return [re.sub(r"<[^>]+>", "", text).replace("\u2212", "-") for text in texts]


def labels_of(module, low, high, size, trans=None):
    frame = {"x": [low, high], "y": [low, high]}
    plot = module.ggplot(frame, module.aes("x", "y")) + module.geom_point()
    if size is not None:
        plot += module.ggsize(*size)
    if trans is not None:
        # Unpadded: ggrust pads transformed axes in transformed units (ggplot2),
        # lets-plot in data units, so only the break rule itself can match.
        options = {"trans": trans}
        if trans != "reverse":
            options["expand"] = [0, 0]
        plot += module.scale_x_continuous(**options) + module.scale_y_continuous(**options)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        svg = plot.to_svg()
    reader = lets_plot_labels if module is lp else ggrust_labels
    return reader(svg, "x"), reader(svg, "y")


FIXED_RANGES = [
    (0.1, 0.3),
    (-7, 13),
    (0, 1000),
    (99.5, 100.5),
    (0, 1.2e6),
    (-1e-5, 1e-5),
    (2e-17, 9e-17),
    (1e-9, 3e-9),
    (0, 12345),
    (1, 4.5),
    (3, 3),
    (1e6, 1e6),
]

RANDOM = np.random.default_rng(42)
RANDOM_RANGES = []
for _ in range(40):
    magnitude = 10.0 ** RANDOM.uniform(-12, 12)
    low = RANDOM.uniform(-1, 1) * magnitude
    RANDOM_RANGES.append((low, low + RANDOM.uniform(0.05, 3) * magnitude))


@pytest.mark.parametrize("low, high", FIXED_RANGES)
def test_labels_match_lets_plot_at_default_size(low, high):
    theirs = labels_of(lp, low, high, None)
    ours = labels_of(gg, low, high, None)
    assert ours == theirs, f"labels for [{low}, {high}]"


def test_labels_agree_on_random_ranges_and_sizes():
    # Real-font y labels make ggrust's panel a few pixels wider than lets-plot's, so a
    # near-tie in the overlap test (within ~1 px) can flip the x break count. Exact
    # agreement is required above; here at least 95% of axes must match.
    axes = []
    for low, high in FIXED_RANGES + RANDOM_RANGES:
        for size in [None, (400, 250)]:
            theirs = labels_of(lp, low, high, size)
            ours = labels_of(gg, low, high, size)
            axes.extend(
                (low, high, size, axis, ours[index], theirs[index])
                for index, axis in enumerate("xy")
            )
    mismatches = [axis for axis in axes if axis[4] != axis[5]]
    assert len(mismatches) <= 0.05 * len(axes), "\n".join(map(str, mismatches))


TRANSFORMED_RANGES = {
    "log10": [(1, 1000), (0.01, 50), (5, 5e6), (1, 2), (1e-6, 1), (3, 30), (100, 100)],
    "log2": [(1, 1024), (0.5, 3), (1, 100), (7, 9), (0.001, 1)],
    "sqrt": [(0, 100), (2, 50), (0, 1), (10, 1e4)],
    "reverse": [(0, 1), (-7, 13), (1, 4.5), (0, 1.2e6)],
    "symlog": [(-1000, 1000), (0.1, 500), (-5, 5), (-0.5, 0.5), (-1e6, 10), (1, 2), (5, 5e5)],
}


@pytest.mark.parametrize(
    "trans, low, high",
    [(trans, low, high) for trans, ranges in TRANSFORMED_RANGES.items() for low, high in ranges],
)
def test_transformed_labels_match_lets_plot(trans, low, high):
    theirs = labels_of(lp, low, high, None, trans)
    ours = labels_of(gg, low, high, None, trans)
    # Unpadded, a break exactly on the axis end is kept by ggrust (as ggplot2);
    # lets-plot's round trip through data units can lose it (sqrt 10..1e4).
    ours = tuple(
        labels[:-1] if len(labels) == len(other) + 1 and labels[:-1] == other else labels
        for labels, other in zip(ours, theirs, strict=True)
    )
    assert ours == theirs, f"{trans} labels for [{low}, {high}]"


def test_transformed_labels_agree_on_random_ranges():
    # Same near-tie allowance as for linear scales.
    random = np.random.default_rng(7)
    axes = []
    for trans in TRANSFORMED_RANGES:
        for _ in range(12):
            if trans in ("log10", "log2"):
                low = 10.0 ** random.uniform(-6, 6)
                high = low * 10.0 ** random.uniform(0.1, 6)
            else:
                low = random.uniform(0, 100) if trans == "sqrt" else random.uniform(-100, 100)
                high = low + 10.0 ** random.uniform(-2, 4)
            theirs = labels_of(lp, low, high, None, trans)
            ours = labels_of(gg, low, high, None, trans)
            axes.extend(
                (trans, low, high, axis, ours[index], theirs[index])
                for index, axis in enumerate("xy")
            )
    mismatches = [axis for axis in axes if axis[4] != axis[5]]
    assert len(mismatches) <= 0.05 * len(axes), "\n".join(map(str, mismatches))


@pytest.mark.parametrize(
    "trans, low, high, expected",
    [
        # R: ggplot_build(...)$layout$panel_params[[1]]$x.range, ggplot2 4.0.1.
        ("log10", 1, 1e4, (-0.2, 4.2)),
        ("log2", 0.001, 1, (-10.464, 0.498)),
        ("sqrt", 0, 1, (-0.05, 1.05)),
        ("sqrt", 0, 100, (-0.5, 10.5)),
    ],
)
def test_transformed_axes_pad_in_transformed_units_as_ggplot2(trans, low, high, expected):
    plot = gg.ggplot({"x": [low, high]}, gg.aes("x", "x")) + gg.geom_point()
    plot += gg.scale_x_continuous(trans=trans)
    domain = plot.scale_info("x")[0]["domain"]
    assert domain == pytest.approx(expected, abs=1e-3)


def test_padded_sqrt_axis_keeps_its_zero_break():
    plot = gg.ggplot({"x": [0, 100]}, gg.aes("x", "x")) + gg.geom_point()
    breaks = (plot + gg.scale_x_continuous(trans="sqrt")).scale_info("x")[0]["breaks"]
    assert breaks[0] == 0


def test_break_width_skips_positions_below_a_sqrt_range():
    # The padded axis reaches below 0, where sqrt's inverse would fold -0.1 onto 0.01.
    plot = gg.ggplot({"x": [0, 1, 4, 9, 16]}, gg.aes("x", "x")) + gg.geom_point()
    plot += gg.scale_x_continuous(trans="sqrt", break_width=0.1)
    breaks = plot.scale_info("x")[0]["breaks"]
    assert breaks[0] == 0
    assert len(breaks) == len(set(breaks))


def test_exp_axis_has_no_break_where_exp_cannot_reach():
    plot = gg.ggplot({"x": [0, 1, 4, 9, 16]}, gg.aes("x", "x")) + gg.geom_point()
    breaks = (plot + gg.scale_x_continuous(trans="exp")).scale_info("x")[0]["breaks"]
    assert all(position > 1 for position in breaks)
