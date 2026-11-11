"""``qplot()``."""

from __future__ import annotations

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError

DATA = {
    "weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1],
    "mpg": [31.0, 28.0, 24.0, 22.0, 19.0, 15.0],
    "origin": ["EU", "EU", "US", "EU", "US", "US"],
}


def geoms(plot: gg.plot.core.PlotSpec) -> list[str]:
    return [layer["geom"] for layer in plot.as_dict()["layers"]]


def test_qplot_matches_the_ggplot_spelling() -> None:
    quick = gg.qplot("weight", "mpg", DATA, color="origin", size=4, main="Cars")
    full = (
        gg.ggplot(DATA, gg.aes("weight", "mpg", color="origin"))
        + gg.geom_point(size=4)
        + gg.ggtitle("Cars")
    )
    assert quick.as_dict() == full.as_dict()
    assert quick.to_svg().startswith("<svg")


@pytest.mark.parametrize(
    ("x", "y", "expected"),
    [("weight", "mpg", "point"), ("weight", None, "histogram"), ("origin", None, "bar")],
)
def test_auto_geom(x: str, y: str | None, expected: str) -> None:
    assert geoms(gg.qplot(x, y, DATA)) == [expected]


def test_auto_geom_from_values_and_polars() -> None:
    assert geoms(gg.qplot(["a", "b", "a"])) == ["bar"]
    assert geoms(gg.qplot([1.0, 2.0, 2.5])) == ["histogram"]
    assert geoms(gg.qplot("origin", data=pl.DataFrame(DATA))) == ["bar"]
    assert geoms(gg.qplot()) == ["blank"]


def test_several_geoms_get_the_geom_options() -> None:
    plot = gg.qplot("weight", "mpg", DATA, geom=["point", "smooth"], alpha=0.5)
    assert geoms(plot) == ["point", "smooth"]
    assert all(layer["alpha"] == 0.5 for layer in plot.as_dict()["layers"])


def test_options_become_scales_coord_and_labels() -> None:
    plot = gg.qplot(
        "weight", "mpg", DATA, log="y", xlim=[2, 5], ylim=[10, 40], xlab="w", ylab="m", asp=0.1
    )
    spec = plot.as_dict()
    scales = {scale["aesthetic"]: scale for scale in spec["scales"]}
    assert scales["x"]["limits"] == [2, 5] and "trans" not in scales["x"]
    assert scales["y"]["limits"] == [10, 40] and scales["y"]["trans"] == "log10"
    assert spec["coord"]["ratio"] == 0.1
    assert plot.labels["x"] == "w" and plot.labels["y"] == "m"


@pytest.mark.parametrize(
    ("formula", "expected"),
    [
        ("~ origin", {"name": "wrap", "facets": ["origin"]}),
        ("~ origin + weight", {"name": "wrap", "facets": ["origin", "weight"]}),
        ("origin ~ .", {"name": "grid", "y": "origin"}),
        ("origin ~ weight", {"name": "grid", "x": "weight", "y": "origin"}),
    ],
)
def test_facets_formula(formula: str, expected: dict[str, object]) -> None:
    facet = gg.qplot("weight", "mpg", DATA, facets=formula).as_dict()["facet"]
    assert {key: facet.get(key) for key in expected} == expected


@pytest.mark.parametrize(
    ("arguments", "match"),
    [
        ({"geom": "piont"}, "unknown geom 'piont'; did you mean 'point'"),
        ({"facets": "origin"}, "is not a formula"),
        ({"facets": "a + b ~ c"}, "more than one column"),
        ({"facets": "~ ."}, "names no column"),
        ({"log": "z"}, "log='z' is not supported"),
    ],
)
def test_qplot_errors(arguments: dict[str, str], match: str) -> None:
    with pytest.raises(SpecError, match=match):
        gg.qplot("weight", "mpg", DATA, **arguments)  # ty: ignore[invalid-argument-type]


def test_y_without_x() -> None:
    with pytest.raises(SpecError, match="'y' needs 'x'"):
        gg.qplot(y="mpg", data=DATA)


def test_ggplot_with_a_positional_mapping_points_to_the_data_form() -> None:
    with pytest.raises(SpecError, match=r"ggplot\(frame, aes"):
        gg.ggplot(gg.aes("weight", "mpg"))  # ty: ignore[invalid-argument-type]


def test_array_like_aesthetics_are_mapped() -> None:
    import numpy as np

    data = {"a": [1, 2, 3], "b": [3, 2, 1]}
    for values in (np.array(["x", "y", "z"]), ("x", "y", "z")):
        plot = gg.qplot("a", "b", data, color=values)
        assert "color" not in plot.as_dict()["layers"][0]
        assert plot.to_svg().startswith("<svg")


@pytest.mark.parametrize("geom", [None, [], 3])
def test_bad_geom_argument(geom: object) -> None:
    with pytest.raises(SpecError, match="geom must be a geom name"):
        gg.qplot("weight", "mpg", DATA, geom=geom)  # ty: ignore[invalid-argument-type]
