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


def labels_of(module, low, high, size):
    frame = {"x": [low, high], "y": [low, high]}
    plot = module.ggplot(frame, module.aes("x", "y")) + module.geom_point()
    if size is not None:
        plot += module.ggsize(*size)
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
