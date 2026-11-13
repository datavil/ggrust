"""ggpubr helpers (plan 5.7): compare_means(), geom_pwc() and stat_pvalue_manual().

The reference comes from ggpubr and rstatix (``ggplot_build`` and ``compare_means``),
written by ``tests/fixtures/generate_pubr_helpers_reference.R``. The p-values and
stat rows are also checked in Rust (``stat/compare_table.rs``, ``stat/pwc.rs``); here
the public API, the stat rows ``layer_data`` returns and the errors are compared.
"""

from __future__ import annotations

import json
import math
import re
from pathlib import Path
from typing import Any

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import ColumnNotFoundError, SpecError, UnsupportedValueError

REFERENCE = json.loads(
    (Path(__file__).parent / "fixtures" / "pubr_helpers_reference.json").read_text()
)
DATA = REFERENCE["data"]
# R's ToothGrowth: supp is a factor with levels OJ, VC; dose a factor 0.5, 1, 2.
TOOTH = pl.DataFrame(
    {
        "len": DATA["tooth"]["len"],
        "supp": pl.Series(DATA["tooth"]["supp"], dtype=pl.Enum(["OJ", "VC"])),
        "dose": DATA["tooth"]["dose"],
    }
)
FOUR = pl.DataFrame(DATA["four"])
THREE = pl.DataFrame(DATA["three"])


def texts(svg: str) -> list[str]:
    """Visible texts of an SVG, in document order."""
    flat = re.sub(r"\s*\n\s*", "", svg)
    found = re.findall(r">([^<>]+)</t(?:span|ext)>", flat)
    return [text.replace("&lt;", "<") for text in found if text.strip()]


def snake(options: Any) -> dict[str, Any]:
    """R option names (``ref.group``) as ggrust's (``ref_group``)."""
    if not isinstance(options, dict):  # jsonlite writes empty R lists as [].
        return {}
    return {name.replace(".", "_"): value for name, value in options.items()}


def close(actual: float, expected: float) -> bool:
    """Equal to 1e-9 (positions) or relatively to 1e-6 (p-values)."""
    return math.isclose(actual, expected, rel_tol=1e-6, abs_tol=1e-9)


@pytest.mark.parametrize("case", REFERENCE["compare_means"], ids=lambda case: case["name"])
def test_compare_means_matches_ggpubr(case: dict[str, Any]) -> None:
    data = FOUR if case["formula"].startswith("v ~") else TOOTH
    options = snake(case["options"])
    table = gg.compare_means(case["formula"], data, **options)
    rows = case["rows"]
    assert table.height == len(rows)
    for row, expected in zip(table.iter_rows(named=True), rows, strict=True):
        assert close(row["p"], expected["p"])
        assert row["p.adj"] == expected["p.adj"]
        # ggrust formats each p on its own (deviations.md), R the column at once.
        assert row["p.format"] == expected["p.format.each"]
        assert row["p.signif"] == expected["p.signif"]
        assert row["method"] == expected["method"]
        assert row[".y."] == expected[".y."]
        for column in ("group1", "group2", "supp", "dose"):
            if column in expected:
                assert str(row[column]) == expected[column], column


def test_compare_means_columns_and_levels() -> None:
    table = gg.compare_means("len ~ dose", TOOTH, method="anova", group_by="supp")
    assert table.columns == ["supp", ".y.", "p", "p.adj", "p.format", "p.signif", "method"]
    assert table["supp"].dtype == pl.Enum(["OJ", "VC"])
    # Levels: polars Enum categories, pandas categories, else first appearance.
    data = pl.DataFrame({"g": ["b", "a", "b", "a", "c", "c"], "v": [1.0, 2, 3, 4, 5, 6.5]})
    assert gg.compare_means("v ~ g", data)["group1"].to_list() == ["b", "b", "a"]
    enum = data.with_columns(pl.col("g").cast(pl.Enum(["c", "b", "a"])))
    assert gg.compare_means("v ~ g", enum)["group1"].to_list() == ["c", "c", "b"]
    pandas = pytest.importorskip("pandas")
    frame = data.to_pandas()
    frame["g"] = pandas.Categorical(frame["g"], categories=["a", "c", "b"])
    assert gg.compare_means("v ~ g", frame)["group1"].to_list() == ["a", "a", "c"]
    # Numeric groups are named as R prints them.
    numbers = pl.DataFrame({"g": [0.5, 0.5, 1.0, 1.0, 2.0, 2.0], "v": [1.0, 2, 3, 4, 5, 6]})
    assert gg.compare_means("v ~ g", numbers)["group2"].to_list() == ["1", "2", "2"]


def test_compare_means_errors() -> None:
    with pytest.raises(SpecError, match=re.escape("did you mean 't.test'")):
        gg.compare_means("len ~ dose", TOOTH, method="t.tset")  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match="'response ~ group'"):
        gg.compare_means("len dose", TOOTH)
    with pytest.raises(SpecError, match="one-sample"):
        gg.compare_means("len ~ 1", TOOTH)
    with pytest.raises(ColumnNotFoundError, match="did you mean 'dose'") as error:
        gg.compare_means("len ~ dsoe", TOOTH)
    assert error.value.column == "dsoe"
    with pytest.raises(SpecError, match="not a level of 'dose'"):
        gg.compare_means("len ~ dose", TOOTH, ref_group="5")
    with pytest.raises(SpecError, match="must be a numeric column"):
        gg.compare_means("supp ~ dose", TOOTH)
    with pytest.raises(UnsupportedValueError, match="column name"):
        gg.compare_means("len ~ dose", TOOTH, group_by=3)  # ty: ignore[invalid-argument-type]
    constant = pl.DataFrame({"g": ["a", "a", "b", "b"], "v": [1.0, 1.0, 1.0, 1.0]})
    with pytest.raises(SpecError, match="cannot test 'a' vs 'b'"):
        gg.compare_means("v ~ g", constant, method="t.test")


def pwc_plot(name: str, options: dict[str, Any]) -> gg.PlotSpec:
    """The boxplot + geom_pwc of a reference case."""
    if name == "four":
        base = gg.ggplot(FOUR, gg.aes("g", "v"))
    elif name.startswith("three"):
        base = gg.ggplot(THREE, gg.aes("x", "v", color="color"))
    elif name.startswith(("x_var", "legend_var")):
        base = gg.ggplot(TOOTH, gg.aes("dose", "len", color="supp"))
    else:
        base = gg.ggplot(TOOTH, gg.aes("dose", "len"))
    # ggpubr dodges by 0.8; ggrust's default 0.95 matches its own boxplots.
    return base + gg.geom_boxplot() + gg.geom_pwc(**{"dodge": 0.8, **options})


@pytest.mark.parametrize("case", REFERENCE["pwc"], ids=lambda case: case["name"])
def test_geom_pwc_matches_ggpubr(case: dict[str, Any]) -> None:
    plot = pwc_plot(case["name"], snake(case["options"]))
    rows = plot.layer_data(-1).to_dicts()
    expected = case["rows"]
    assert len(rows) == len(expected)
    for row, reference in zip(rows, expected, strict=True):
        # R positions are 1-based.
        assert close(row["xmin"] + 1, reference["xmin"])
        assert close(row["xmax"] + 1, reference["xmax"])
        assert close(row["y"], reference["y"])
        assert close(row["lenstart"], reference["tip_left"])
        assert close(row["lenend"], reference["tip_right"])
        assert row["label"] == reference["label"]


def test_geom_pwc_legend_groups_keep_their_color() -> None:
    plot = pwc_plot("legend_var", {"group_by": "legend.var"})
    colors = plot.layer_data(-1)["color"].to_list()
    assert colors == ["OJ"] * 3 + ["VC"] * 3


def test_geom_pwc_fill_groups_like_color() -> None:
    by_color = pwc_plot("x_var", {}).layer_data(-1)
    plot = gg.ggplot(TOOTH, gg.aes("dose", "len", fill="supp")) + gg.geom_boxplot()
    by_fill = (plot + gg.geom_pwc(dodge=0.8)).layer_data(-1)
    assert by_fill["label"].to_list() == by_color["label"].to_list()
    assert by_fill["xmin"].to_list() == by_color["xmin"].to_list()


def test_stat_pvalue_manual_keeps_facet_columns() -> None:
    table = gg.compare_means("len ~ dose", TOOTH, group_by="supp")
    plot = (
        gg.ggplot(TOOTH, gg.aes("dose", "len"))
        + gg.geom_boxplot()
        + gg.stat_pvalue_manual(table, label="p.adj", y_position=36, step_increase=0.1)
        + gg.facet_wrap("supp")
    )
    data = plot.layer_data(-1)
    assert data.group_by("panel").len().sort("panel")["len"].to_list() == [3, 3]


def test_geom_pwc_default_dodge_matches_the_boxes() -> None:
    plot = gg.ggplot(TOOTH, gg.aes("dose", "len", color="supp")) + gg.geom_boxplot()
    boxes = plot.layer_data(0)
    brackets = (plot + gg.geom_pwc()).layer_data(-1)
    centers = sorted(boxes["x"].to_list())
    ends = sorted(brackets["xmin"].to_list() + brackets["xmax"].to_list())
    assert all(close(a, b) for a, b in zip(centers, ends, strict=True))


def test_geom_pwc_per_facet_panel() -> None:
    plot = (
        gg.ggplot(TOOTH, gg.aes("dose", "len"))
        + gg.geom_boxplot()
        + gg.geom_pwc()
        + gg.facet_wrap("supp")
    )
    data = plot.layer_data(-1)
    assert data.group_by("panel").len().sort("panel")["len"].to_list() == [3, 3]
    # Per supp, as ggpubr's legend.var case: OJ 0.001, 0.00018, 0.082; VC 0.00018 (twice),
    # 0.00044.
    drawn = texts(plot.to_svg())
    assert drawn.count("0.00018") == 3
    assert {"0.001", "0.082", "0.00044"} <= set(drawn)


def test_geom_pwc_errors() -> None:
    base = gg.ggplot(TOOTH, gg.aes("dose", "len")) + gg.geom_boxplot()
    with pytest.raises(SpecError, match="not available in ggrust yet"):
        gg.geom_pwc(method="dunn_test")  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match=re.escape("did you mean 'legend.var'")):
        gg.geom_pwc(group_by="legend_var")  # ty: ignore[invalid-argument-type]
    with pytest.raises(UnsupportedValueError):
        gg.geom_pwc(y_position="top")  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match=re.escape("did you mean 'p.adj.signif'")):
        (base + gg.geom_pwc(label="p.adj.signf")).to_svg()
    grouped = gg.ggplot(TOOTH, gg.aes("dose", "len", color="supp")) + gg.geom_boxplot()
    with pytest.raises(SpecError, match="1-based rank of a legend group"):
        (grouped + gg.geom_pwc(ref_group="OJ")).to_svg()


def manual_table(case: dict[str, Any]) -> pl.DataFrame:
    return pl.DataFrame(case["table"])


@pytest.mark.parametrize("case", REFERENCE["pvalue_manual"], ids=lambda case: case["name"])
def test_stat_pvalue_manual_matches_ggpubr(case: dict[str, Any]) -> None:
    plot = (
        gg.ggplot(TOOTH, gg.aes("dose", "len"))
        + gg.geom_boxplot()
        + gg.stat_pvalue_manual(manual_table(case), **snake(case["options"]))
    )
    rows = plot.layer_data(-1).to_dicts()
    expected = case["rows"]
    assert len(rows) == len(expected)
    for row, reference in zip(rows, expected, strict=True):
        assert close(row["xmin"] + 1, reference["xmin"])
        assert close(row["xmax"] + 1, reference["xmax"])
        assert close(row["y"], reference["y"])
        assert close(row["lenstart"], reference["tip_left"])
        assert row["label"] == reference["label"]


def test_stat_pvalue_manual_from_compare_means() -> None:
    table = gg.compare_means("len ~ dose", TOOTH, ref_group=".all.", method="t.test")
    plot = gg.ggplot(TOOTH, gg.aes("dose", "len")) + gg.geom_boxplot()
    # Comparisons against .all. are labels at each group, without brackets.
    layer = gg.stat_pvalue_manual(table, label="p.signif", y_position=36)
    assert layer.as_dict()["geom"] == "text"
    data = (plot + layer).layer_data(-1)
    assert data["x"].to_list() == [0.0, 1.0, 2.0]
    assert data["label"].to_list() == ["****", "ns", "****"]
    # remove_bracket: a reference table drawn as labels at xmax.
    reference = gg.compare_means("len ~ dose", TOOTH, ref_group="0.5")
    labels = gg.stat_pvalue_manual(reference, y_position=36, remove_bracket=True)
    assert (plot + labels).layer_data(-1)["x"].to_list() == [1.0, 2.0]
    assert {"7e-06", "1.7e-07"} <= set(texts((plot + labels).to_svg()))


def test_stat_pvalue_manual_errors() -> None:
    table = gg.compare_means("len ~ dose", TOOTH)
    with pytest.raises(ColumnNotFoundError, match=re.escape("did you mean 'p.adj'")):
        gg.stat_pvalue_manual(table, label="p.adk", y_position=35)
    with pytest.raises(ColumnNotFoundError, match=re.escape("y.position")):
        gg.stat_pvalue_manual(table)
    with pytest.raises(SpecError, match="one height or one per row"):
        gg.stat_pvalue_manual(table, y_position=[35, 37])
    with pytest.raises(SpecError, match="unclosed"):
        gg.stat_pvalue_manual(table, label="p = {p", y_position=35)
    with pytest.raises(SpecError, match=re.escape("did you mean 'p.adj'")):
        gg.stat_pvalue_manual(table, y_position=35, hide_ns="p.adk")  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match=">= 0"):
        gg.stat_pvalue_manual(table, y_position=35, tip_length=-1)
    with pytest.raises(SpecError, match="no label column"):
        gg.stat_pvalue_manual(pl.DataFrame({"group1": ["a"], "group2": ["b"]}), y_position=1)
