"""Example datasets in ``ggrust.data``."""

from __future__ import annotations

import ggrust as gg
import ggrust.data
import polars as pl
import pytest
from ggrust.errors import DatasetNotFoundError

SHAPES = {"penguins": (344, 8), "mpg": (234, 11), "economics": (574, 6), "iris": (150, 5)}


@pytest.mark.parametrize(("name", "shape"), sorted(SHAPES.items()))
def test_dataset_loads_as_a_polars_frame(name: str, shape: tuple[int, int]) -> None:
    frame = getattr(ggrust.data, name)
    assert isinstance(frame, pl.DataFrame)
    assert frame.shape == shape


def test_every_dataset_is_listed() -> None:
    assert sorted(ggrust.data.__all__) == sorted(SHAPES)
    assert set(SHAPES) <= set(dir(ggrust.data))


def test_each_access_returns_a_new_frame() -> None:
    first = ggrust.data.iris
    first.columns = ["a", "b", "c", "d", "e"]
    assert ggrust.data.iris.columns[0] == "sepal_length"


def test_column_types() -> None:
    penguins = ggrust.data.penguins
    assert penguins.schema["species"] == pl.Enum(["Adelie", "Chinstrap", "Gentoo"])
    assert penguins["sex"].null_count() == 11
    assert penguins["bill_length_mm"].null_count() == 2
    assert ggrust.data.economics.schema["date"] == pl.Date
    assert ggrust.data.economics["date"].min().isoformat() == "1967-07-01"
    assert ggrust.data.mpg["hwy"].dtype == pl.Int64


def test_factor_level_order_reaches_the_plot() -> None:
    plot = gg.ggplot(ggrust.data.penguins, gg.aes("species", "body_mass_g")) + gg.geom_boxplot(
        na_rm=True
    )
    assert plot.scale_info("x")[0]["levels"] == ["Adelie", "Chinstrap", "Gentoo"]


def test_unknown_dataset_suggests_a_close_name() -> None:
    with pytest.raises(DatasetNotFoundError, match="did you mean 'penguins'") as raised:
        ggrust.data.pengiuns  # noqa: B018
    assert raised.value.name == "pengiuns"
    assert raised.value.available == ["economics", "iris", "mpg", "penguins"]
    assert isinstance(raised.value, AttributeError)


def test_unknown_dataset_without_a_close_name() -> None:
    with pytest.raises(DatasetNotFoundError, match=r"no dataset 'cars'\. Available datasets"):
        ggrust.data.cars  # noqa: B018


def test_from_import_of_unknown_dataset_is_an_import_error() -> None:
    with pytest.raises(ImportError):
        from ggrust.data import mgp  # noqa: F401
