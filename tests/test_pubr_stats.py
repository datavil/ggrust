"""ggpubr / ggpmisc statistics: labels and brackets where the R packages put them.

The reference comes from ggpubr 1.0 and ggpmisc 1.0 (``ggplot_build``), written by
``tests/fixtures/generate_pubr_reference.R``. The p-values, coefficients and stat
rows are checked against R in Rust (``stat/hypothesis.rs``, ``stat/compare_means.rs``,
``stat/cor.rs``, ``stat/poly_eq.rs``); here the public API, the drawn label texts and
the drawn bracket geometry are compared.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

import ggrust as gg
import numpy as np
import pytest
from ggrust.errors import SpecError, UnsupportedValueError

REFERENCE = json.loads((Path(__file__).parent / "fixtures" / "pubr_reference.json").read_text())
LAYERS = REFERENCE["layers"]
TOOTH = {
    "dose": LAYERS["tooth"]["dose"],
    "len": LAYERS["tooth"]["len"],
    "supp": LAYERS["tooth"]["supp"],
}
CARS = LAYERS["cars"]
COMPARISONS = [("0.5", "1"), ("1", "2"), ("0.5", "2")]


def texts(svg: str) -> list[str]:
    """Visible texts in document order."""
    flat = re.sub(r"\s*\n\s*", "", svg)
    found = re.findall(r">([^<>]+)</t(?:span|ext)>", flat)
    return [text.replace("&lt;", "<").replace("&gt;", ">") for text in found if text.strip()]


def bracket_paths(svg: str) -> list[np.ndarray]:
    """Vertices of every 4-point stroked path (the brackets)."""
    found = []
    for tag in re.findall(r"<path[^>]*>", svg):
        data = re.search(r' d="([^"]+)"', tag)
        if data is None or 'fill="none"' not in tag:
            continue
        numbers = [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?", data.group(1))]
        if len(numbers) == 8:
            found.append(np.array(numbers).reshape(-1, 2))
    return found


def fit_error(pixels: np.ndarray, data: np.ndarray) -> float:
    """Largest distance, in data units, from the best linear pixel mapping."""
    design = np.vstack([data, np.ones(len(data))]).T
    coefficients, *_ = np.linalg.lstsq(design, pixels, rcond=None)
    return float(np.abs(design @ coefficients - pixels).max() / abs(coefficients[0]))


def boxes() -> gg.PlotSpec:
    """ToothGrowth length by dose, as boxplots."""
    return gg.ggplot(TOOTH, gg.aes("dose", "len")) + gg.geom_boxplot()


@pytest.mark.parametrize("case", ["comparisons", "signif", "t_test"])
def test_brackets_are_drawn_where_ggsignif_puts_them(case: str) -> None:
    options = {"signif": {"label": "p.signif"}, "t_test": {"method": "t.test"}}.get(case, {})
    svg = (boxes() + gg.stat_compare_means(comparisons=COMPARISONS, **options)).to_svg()
    drawn = bracket_paths(svg)
    rows = LAYERS[case]
    # ggsignif: three segments per bracket (left tip, bar, right tip).
    expected = []
    for index in range(len(rows) // 3):
        left, bar, right = rows[3 * index : 3 * index + 3]
        expected.append(
            [
                (left["x"], left["y"]),
                (bar["x"], bar["y"]),
                (bar["xend"], bar["y"]),
                (right["x"], right["yend"]),
            ]
        )
    assert len(drawn) == len(expected)
    # Match by bar height (both sides sort brackets bottom-up).
    drawn.sort(key=lambda path: -path[1, 1])
    expected.sort(key=lambda path: path[1][1])
    pixels = np.vstack(drawn)
    data = np.array([point for path in expected for point in path])
    assert fit_error(pixels[:, 0], data[:, 0]) < 0.01
    assert fit_error(pixels[:, 1], data[:, 1]) < 0.01
    labels = [row["annotation"] for row in rows[1::3]]
    assert sorted(labels) == sorted(text for text in texts(svg) if text in labels)


@pytest.mark.parametrize(
    ("case", "build"),
    [
        ("global", lambda: boxes() + gg.stat_compare_means()),
        ("anova", lambda: boxes() + gg.stat_compare_means(method="anova")),
        ("reference", lambda: boxes() + gg.stat_compare_means(ref_group="0.5", label="p.signif")),
        (
            "reference_all",
            lambda: boxes() + gg.stat_compare_means(ref_group=".all.", label="p.signif"),
        ),
        (
            "grouped",
            lambda: (
                gg.ggplot(TOOTH, gg.aes("dose", "len", color="supp"))
                + gg.geom_boxplot()
                + gg.stat_compare_means(label="p.signif")
            ),
        ),
        (
            "paired",
            lambda: (
                gg.ggplot(TOOTH, gg.aes("supp", "len"))
                + gg.geom_boxplot()
                + gg.stat_compare_means(method="t.test", paired=True)
            ),
        ),
    ],
)
def test_test_labels_match_ggpubr(case: str, build: object) -> None:
    svg = build().to_svg()  # type: ignore[operator]
    drawn = texts(svg)
    for row in LAYERS[case]:
        assert row["label"] in drawn, (row["label"], drawn)


def test_correlation_labels_stack_per_group() -> None:
    plot = gg.ggplot(CARS, gg.aes("wt", "mpg", color="cyl")) + gg.geom_point() + gg.stat_cor()
    svg = plot.to_svg()
    flat = re.sub(r"\s*\n\s*", "", svg)
    placed = {
        text: float(y)
        for y, text in re.findall(r'<text[^>]* y="([-\d.]+)"[^>]*>(R = [^<]+)</text>', flat)
    }
    # ggpubr text with 2 significant p digits (ggpubr 1.0 prints the raw p).
    expected = [
        row["label"].split(", p = ")[0] + ", p = " + f"{float(row['label'].split(', p = ')[1]):.2g}"
        for row in LAYERS["cor_groups"]
    ]
    assert sorted(placed) == sorted(expected)
    # Group g is 1.4 g lines below the top: equal steps, in ggrust's group order
    # (first appearance, as the legend: cyl 6, 4, 8; R sorts factor levels: 4, 6, 8).
    by_level = dict(zip(("4", "6", "8"), expected, strict=True))
    heights = [placed[by_level[level]] for level in ("6", "4", "8")]
    assert heights == sorted(heights)
    assert abs((heights[2] - heights[1]) - (heights[1] - heights[0])) < 0.1


def test_poly_eq_labels_match_ggpmisc() -> None:
    plot = (
        gg.ggplot(CARS, gg.aes("wt", "mpg"))
        + gg.geom_point()
        + gg.stat_poly_eq(label=["eq", "R2"], formula="y ~ poly(x, 2, raw = TRUE)")
        + gg.stat_poly_eq(label_y_npc="bottom")
    )
    drawn = texts(plot.to_svg())
    # ggpmisc: italic(y)~`=`~49.9 - 13.4*~italic(x) + 1.17*~italic(x)^2, R^2 = 0.82
    assert "y = 49.9 - 13.4x + 1.17x², R² = 0.82" in drawn
    assert "R² = 0.75" in drawn


def test_layers_render_to_png(tmp_path: Path) -> None:
    plot = (
        boxes() + gg.stat_compare_means(comparisons=COMPARISONS) + gg.stat_compare_means(label_y=50)
    )
    path = tmp_path / "compare.png"
    plot.to_png(str(path))
    assert path.read_bytes()[:8] == b"\x89PNG\r\n\x1a\n"


def test_hide_ns_and_adjustment() -> None:
    svg = (
        gg.ggplot(TOOTH, gg.aes("dose", "len", color="supp"))
        + gg.geom_boxplot()
        + gg.stat_compare_means(label="p.signif", hide_ns=True)
    ).to_svg()
    assert "ns" not in texts(svg)
    adjusted = boxes() + gg.stat_compare_means(
        comparisons=COMPARISONS[:2], label="p.format", p_adjust_method="bonferroni"
    )
    # format.pval(p.adjust(c(7.020855e-06, 1.772382e-04), "bonferroni"), digits = 2)
    assert {"1.4e-05", "0.00035"} <= set(texts(adjusted.to_svg()))


def test_spec_errors_are_specific() -> None:
    with pytest.raises(SpecError, match=r"did you mean 'kruskal\.test'"):
        gg.stat_compare_means(method="kruskal.tset")  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="pair of x categories"):
        gg.stat_compare_means(comparisons=[("0.5", "1", "2")])
    with pytest.raises(UnsupportedValueError, match="category name or 1-based position"):
        gg.stat_compare_means(ref_group=0.5)  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="not both"):
        gg.stat_compare_means(comparisons=COMPARISONS, ref_group="0.5")
    with pytest.raises(SpecError, match="did you mean 'spearman'"):
        gg.stat_cor(method="spearmen")  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="unknown label 'R3'"):
        gg.stat_poly_eq(label=["eq", "R3"])  # type: ignore[list-item]
    with pytest.raises(SpecError, match=r"\[0, 1\]"):
        gg.stat_cor(label_x_npc=1.5)


def test_engine_errors_name_the_problem() -> None:
    with pytest.raises(SpecError, match=r"'3' is not an x category.*0\.5, 1, 2"):
        (boxes() + gg.stat_compare_means(comparisons=[("0.5", "3")])).to_svg()
    with pytest.raises(SpecError, match="compares 2 groups, but the layer has 3"):
        (boxes() + gg.stat_compare_means(method="t.test")).to_svg()
    with pytest.raises(SpecError, match="unsupported formula"):
        (gg.ggplot(CARS, gg.aes("wt", "mpg")) + gg.stat_poly_eq(formula="y ~ poly(x, 2)")).to_svg()


def test_panel_without_a_category_draws_the_other_brackets() -> None:
    frame = {
        "g": ["a", "b", "c"] * 4 + ["b", "c", "d"] * 4,
        "v": [1, 2, 3, 2, 3, 4, 1.5, 2.5, 3.5, 1.2, 2.2, 3.3] * 2,
        "f": ["u"] * 12 + ["v"] * 12,
    }
    plot = (
        gg.ggplot(frame, gg.aes("g", "v"))
        + gg.geom_boxplot()
        + gg.stat_compare_means(comparisons=[("a", "b"), ("b", "c")])
        + gg.facet_wrap("f")
    )
    # Panel u has both brackets, panel v only b vs c (no a there).
    assert len(bracket_paths(plot.to_svg())) == 3
