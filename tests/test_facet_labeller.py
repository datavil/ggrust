"""Facet strip labellers: label_value, label_both, label_context, as_labeller."""

from __future__ import annotations

import pickle

import pytest
from ggrust import (
    Labeller,
    aes,
    as_labeller,
    facet_grid,
    facet_wrap,
    geom_point,
    ggplot,
    label_both,
    label_context,
    label_value,
)
from ggrust.errors import SpecError

DATA = {
    "x": [1, 2, 3, 4, 5, 6],
    "y": [2, 4, 3, 5, 6, 1],
    "pet": ["cat", "cat", "dog", "dog", "fox", "fox"],
    "year": [1999, 2008, 1999, 2008, 1999, 2008],
}


def svg(*specs: object) -> str:
    plot = ggplot(DATA, aes("x", "y")) + geom_point()
    for spec in specs:
        plot = plot + spec
    return plot.to_svg()


def test_label_value_is_the_default() -> None:
    assert svg(facet_wrap("pet")) == svg(facet_wrap("pet", labeller=label_value))


def test_label_both_prefixes_the_variable() -> None:
    out = svg(facet_wrap("pet", labeller=label_both))
    assert ">pet: cat<" in out and ">pet: fox<" in out


def test_label_both_by_name_and_options() -> None:
    out = svg(facet_wrap(["pet", "year"], labeller=label_both(sep=" = ", multi_line=False)))
    assert ">pet, year = cat, 1,999<" in out
    assert svg(facet_wrap("pet", labeller="label_both")) == svg(
        facet_wrap("pet", labeller=label_both)
    )


def test_label_context_uses_both_only_for_several_variables() -> None:
    assert ">cat<" in svg(facet_wrap("pet", labeller=label_context))
    out = svg(facet_wrap(["pet", "year"], labeller=label_context))
    assert ">pet: cat<" in out and ">year: 1,999<" in out


def test_grid_margins_are_labelled_separately() -> None:
    out = svg(facet_grid("pet", "year", labeller=label_context))
    assert ">cat<" in out and ">pet: cat<" not in out
    out = svg(facet_grid("pet", "year", labeller=label_both))
    assert ">pet: dog<" in out and ">year: 2,008<" in out


def test_as_labeller_lookup_keeps_unknown_levels_and_matches_numbers() -> None:
    out = svg(facet_grid("pet", "year", labeller=as_labeller({"cat": "Cats", 2008: "Recent"})))
    assert ">Cats<" in out and ">dog<" in out and ">Recent<" in out and ">1,999<" in out


def test_as_labeller_function_then_default() -> None:
    out = svg(facet_wrap("pet", labeller=as_labeller(str.upper, default=label_both)))
    assert ">pet: CAT<" in out


def test_labeller_function_errors_name_the_option() -> None:
    with pytest.raises(SpecError, match=r"facet\.labeller") as error:
        svg(facet_wrap("pet", labeller=as_labeller(lambda _: 3)))  # type: ignore[arg-type, return-value]
    assert "return a string" in str(error.value)


def test_unknown_names_and_plain_functions_fail_early() -> None:
    with pytest.raises(SpecError, match="did you mean 'label_both'"):
        facet_wrap("pet", labeller="label_bth")  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="as_labeller"):
        facet_wrap("pet", labeller=str.upper)  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="must be a string"):
        as_labeller({"cat": 1})  # type: ignore[dict-item]


def test_labeller_spec_and_copy() -> None:
    spec = facet_wrap("pet", labeller=label_both(sep="=")).as_dict()
    assert spec["labeller"] == {"base": "both", "multi_line": True, "sep": "="}
    assert isinstance(label_both(multi_line=False), Labeller)
    assert label_both.props()["multi_line"] is True
    assert pickle.loads(pickle.dumps(label_both)).as_dict() == label_both.as_dict()


def test_function_gets_plain_numbers_and_numpy_keys_match() -> None:
    import numpy as np

    seen: list[str] = []

    def record(text: str) -> str:
        seen.append(text)
        return text

    svg(facet_wrap("year", labeller=as_labeller(record)))
    assert seen == ["1999", "2008"]
    out = svg(facet_wrap("year", labeller=as_labeller({np.int64(1999): "old"})))
    assert ">old<" in out


def test_bad_options_fail_early() -> None:
    with pytest.raises(TypeError, match="multi_line"):
        label_both(multi_line="no")  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="sep"):
        label_both(sep=5)  # type: ignore[arg-type]
