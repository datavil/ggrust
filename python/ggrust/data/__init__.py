"""Small example datasets, as polars DataFrames: ``from ggrust.data import penguins``.

Each name reads its CSV file shipped with ggrust on access and returns a new
DataFrame. Sources and licenses are in ``THIRD_PARTY_NOTICES.md``; the files are
written by ``scripts/make_datasets.R``. Categorical columns that are R factors in
the source are ``pl.Enum`` columns with the same level order, so plots show the
levels as ggplot2 does.

- ``penguins``: 344 Palmer Archipelago penguins, 3 species (CC0).
- ``mpg``: fuel economy of 234 cars, 1999 and 2008 (ggplot2's ``mpg``, MIT).
- ``economics``: monthly US economic series, 1967 to 2015 (ggplot2's ``economics``, MIT).
- ``iris``: Fisher's 150 iris flowers, 3 species (public domain).

An unknown name raises ``DatasetNotFoundError`` with a close match
(``ggrust.data.pengiuns``); ``from ggrust.data import pengiuns`` raises Python's own
``ImportError``, which drops that message.

What it does not do: download anything, or cache frames (each access reads the file,
a millisecond or two, so callers never share a frame).

Examples
--------
>>> from ggrust import *
>>> from ggrust.data import penguins
>>> penguins.shape
(344, 8)
>>> plot = ggplot(penguins, aes("flipper_length_mm", "body_mass_g", color="species")) + (
...     geom_point(na_rm=True)
... )
"""

from __future__ import annotations

from pathlib import Path

import polars as pl

from ggrust.errors import DatasetNotFoundError, did_you_mean

__all__ = ["economics", "iris", "mpg", "penguins"]

_DIRECTORY = Path(__file__).parent

# Dataset name -> columns read with a fixed type (R factors keep their level order).
_SCHEMAS: dict[str, dict[str, pl.DataType]] = {
    "penguins": {
        "species": pl.Enum(["Adelie", "Chinstrap", "Gentoo"]),
        "island": pl.Enum(["Biscoe", "Dream", "Torgersen"]),
        "sex": pl.Enum(["female", "male"]),
    },
    "mpg": {},
    "economics": {"date": pl.Date()},
    "iris": {"species": pl.Enum(["setosa", "versicolor", "virginica"])},
}

penguins: pl.DataFrame
"""Palmer penguins: species, island, bill and flipper sizes, body mass, sex, year."""
mpg: pl.DataFrame
"""Fuel economy of 234 cars: maker, model, engine, transmission, city and highway mpg."""
economics: pl.DataFrame
"""Monthly US economic series: spending, population, savings rate, unemployment."""
iris: pl.DataFrame
"""Fisher's iris flowers: sepal and petal length and width in cm, species."""


def _read(name: str) -> pl.DataFrame:
    """Read one shipped dataset.

    Parameters
    ----------
    name : str
        A name from ``__all__``.

    Returns
    -------
    pl.DataFrame
        A new frame.
    """
    return pl.read_csv(
        _DIRECTORY / f"{name}.csv", schema_overrides=_SCHEMAS[name], infer_schema_length=None
    )


def __getattr__(name: str) -> pl.DataFrame:
    """Return the dataset ``name`` (PEP 562 module attribute).

    Parameters
    ----------
    name : str
        The dataset name.

    Returns
    -------
    pl.DataFrame
        A new frame of the dataset.

    Raises
    ------
    DatasetNotFoundError
        If there is no dataset ``name`` (an ``AttributeError``).
    """
    if name in __all__:
        return _read(name)
    if name.startswith("__"):  # dunder lookups (copy, pickle, doctest) must stay silent
        raise AttributeError(name)
    available = sorted(__all__)
    suggestion = did_you_mean(name, available)
    message = (
        f"ggrust.data has no dataset '{name}'{suggestion}{' ' if suggestion else '. '}"
        f"Available datasets: {', '.join(available)}."
    )
    raise DatasetNotFoundError(message, name=name, available=available)


def __dir__() -> list[str]:
    """Return the dataset names with the module's own attributes.

    Returns
    -------
    list of str
        The names ``dir(ggrust.data)`` shows.
    """
    return sorted([*globals(), *__all__])
