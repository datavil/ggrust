"""Text layer options match lets-plot: `label_format`, `na_text`, `check_overlap`.

Texts are read from both SVGs in drawing order (markup and zero-width spaces
removed, so `1.23457·10⁶` reads `1.23457·106` on both sides). Overlap counts are
compared with ggplot2's rule (draw order, first wins) on boxes whose widths come
from the real font, so they are checked against geometry rather than lets-plot's
counts (lets-plot measures text with estimated glyph widths).
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

DATA = {
    "x": [1, 2, 3, 4, 5],
    "y": [1, 2, 3, 4, 5],
    "number": [3.14159, 1234567.0, None, -0.5, 1e-7],
    "word": ["a", "b", None, "4.5", "e"],
}


def drawn_texts(svg: str) -> list[str]:
    """The texts of an SVG in drawing order, without markup or whitespace."""
    found = re.findall(r"<text[^>]*>(.*?)</text>", svg, re.S)
    return [re.sub(r"\s+", "", re.sub(r"<[^>]+>|​", "", text)) for text in found]


def texts_of(module, geom: str, column: str, **options: object) -> list[str]:
    plot = (
        module.ggplot(DATA, module.aes("x", "y"))
        + getattr(module, geom)(module.aes(label=column), **options)
        + module.theme_void()
    )
    return drawn_texts(plot.to_svg())


OPTIONS = [
    {},
    {"label_format": ".2f"},
    {"label_format": ".3~f"},
    {"label_format": ".2e"},
    {"label_format": "{.1f} kg"},
    {"label_format": "{} kg"},
    {"label_format": "{d} (n)"},
    {"na_text": "NA"},
    {"na_text": ""},
    {"label_format": ".1f", "na_text": "-"},
]


@pytest.mark.parametrize("geom", ["geom_text", "geom_label"])
@pytest.mark.parametrize("column", ["number", "word"])
@pytest.mark.parametrize("options", OPTIONS, ids=str)
def test_label_texts_match_lets_plot(geom, column, options):
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        ours = texts_of(gg, geom, column, **options)
    assert ours == texts_of(lp, geom, column, **options)


def test_formatted_label_strings():
    assert texts_of(gg, "geom_text", "number", label_format="{.1f} kg") == [
        "3.1kg",
        "1234567.0kg",
        "n/a",
        "-0.5kg",
        "0.0kg",
    ]
    # Strings that read as numbers are formatted, others kept (lets-plot).
    assert texts_of(gg, "geom_text", "word", label_format=".2f") == [
        "a",
        "b",
        "n/a",
        "4.50",
        "e",
    ]


@pytest.mark.parametrize(
    "options",
    [
        {"label": 3.14159, "label_format": ".1f"},
        {"label": "abc", "label_format": "{} kg"},
        {"label": 12, "label_format": "{} kg"},
    ],
    ids=str,
)
def test_constant_labels_are_formatted_like_lets_plot(options):
    def texts(module) -> list[str]:
        plot = module.ggplot(DATA, module.aes("x", "y")) + module.geom_text(**options)
        return drawn_texts((plot + module.theme_void()).to_svg())

    assert texts(gg) == texts(lp)


def test_na_text_default_is_n_a():
    assert texts_of(gg, "geom_label", "word")[2] == "n/a"
    assert texts_of(gg, "geom_label", "word", na_text="missing")[2] == "missing"


def test_datetime_label_format_is_a_spec_error():
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_text(
        gg.aes(label="number"), label_format="%Y-%m"
    )
    with pytest.raises(SpecError, match=r"layers\[0\]\.label_format.*not supported.*'\.2f'"):
        plot.to_svg()


@pytest.mark.parametrize("module", [gg, lp], ids=["ggrust", "lets_plot"])
def test_unknown_pattern_is_literal_text(module):
    plot = module.ggplot(DATA, module.aes("x", "y")) + module.geom_text(
        module.aes(label="number"), label_format=".2q"
    )
    assert drawn_texts((plot + module.theme_void()).to_svg())[0] == ".2q"


def overlap_plot(module, labels: list[str], xs: list[float], ys: list[float], **options):
    data = {"x": xs, "y": ys, "label": labels}
    return (
        module.ggplot(data, module.aes("x", "y", label="label"))
        + module.geom_text(check_overlap=True, **options)
        + module.scale_x_continuous(limits=[-100, 100], expand=[0, 0])
        + module.scale_y_continuous(limits=[-100, 100], expand=[0, 0])
        + module.theme_void()
        + module.ggsize(400, 400)
    )


@pytest.mark.parametrize("module", [gg, lp], ids=["ggrust", "lets_plot"])
def test_check_overlap_keeps_the_first_in_draw_order(module):
    plot = overlap_plot(module, ["FIRST", "SECOND", "third"], [0, 2, 60], [0, 0, 0])
    assert drawn_texts(plot.to_svg()) == ["FIRST", "third"]


def test_check_overlap_drops_only_when_asked():
    plot = overlap_plot(gg, ["FIRST", "SECOND"], [0, 2], [0, 0])
    assert len(drawn_texts(plot.to_svg())) == 1
    data = {"x": [0, 2], "y": [0, 0], "label": ["FIRST", "SECOND"]}
    plain = gg.ggplot(data, gg.aes("x", "y", label="label")) + gg.geom_text() + gg.theme_void()
    assert len(drawn_texts(plain.to_svg())) == 2


def smallest_gap(module, axis: str, label: str, geom: str = "geom_text") -> float:
    """The smallest distance (px) at which two equal texts both stay."""
    low, high = 0.0, 60.0
    for _ in range(20):
        middle = (low + high) / 2
        xs, ys = ([0, middle], [0, 0]) if axis == "x" else ([0, 0], [0, middle])
        data = {"x": xs, "y": ys, "label": [label, label]}
        plot = (
            module.ggplot(data, module.aes("x", "y", label="label"))
            + getattr(module, geom)(check_overlap=True)
            + module.scale_x_continuous(limits=[-100, 100], expand=[0, 0])
            + module.scale_y_continuous(limits=[-100, 100], expand=[0, 0])
            + module.theme_void()
            + module.ggsize(400, 400)
        )
        lines = label.count("\n") + 1
        if len(drawn_texts(plot.to_svg())) == 2 * lines:
            high = middle
        else:
            low = middle
    return high * 2  # 400 px over 200 units


@pytest.mark.parametrize("geom", ["geom_text", "geom_label"])
@pytest.mark.parametrize("label", ["lab12", "ab\ncd"])
def test_overlap_box_heights_match_lets_plot(geom, label):
    ours = smallest_gap(gg, "y", label, geom)
    assert ours == pytest.approx(smallest_gap(lp, "y", label, geom), abs=0.01)


@pytest.mark.parametrize("geom", ["geom_text", "geom_label"])
def test_overlap_box_widths_are_the_drawn_widths(geom):
    # lets-plot's estimated widths are ~15 % wider than the real font's.
    ours = smallest_gap(gg, "x", "lab12", geom)
    theirs = smallest_gap(lp, "x", "lab12", geom)
    assert 0.8 * theirs < ours < theirs


def test_check_overlap_culling_count():
    # 200 random labels: the count is stable (a golden number) and close to lets-plot.
    import numpy as np

    generator = np.random.default_rng(1)
    data = {
        "x": generator.uniform(0, 10, 200),
        "y": generator.uniform(0, 10, 200),
        "label": [f"lab{index}" for index in range(200)],
    }
    counts = []
    for module in (gg, lp):
        plot = module.ggplot(data, module.aes("x", "y", label="label")) + module.geom_text(
            check_overlap=True
        )
        counts.append(len(drawn_texts((plot + module.theme_void()).to_svg())))
    ours, theirs = counts
    assert theirs == 93
    assert ours == 103  # narrower real-font boxes keep a few more
