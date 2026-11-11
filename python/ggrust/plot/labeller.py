"""Facet strip labellers: ``label_value``, ``label_both``, ``label_context``, ``as_labeller``.

lets-plot has no labellers; these follow ggplot2 4.x. Pass one as
``facet_wrap(labeller=...)`` or ``facet_grid(labeller=...)``. A labeller can be
used as is (``labeller=label_both``) or called to change its options
(``labeller=label_both(sep=" = ")``). The engine applies it after ``format=`` and
before ``labwidth=`` wrapping.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Literal, TypeAlias

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec

__all__ = ["Labeller", "as_labeller", "label_both", "label_context", "label_value"]

LabellerBase: TypeAlias = Literal["value", "both", "context"]
LabellerName: TypeAlias = Literal["label_value", "label_both", "label_context"]

_NAMES: tuple[LabellerName, ...] = ("label_value", "label_both", "label_context")


class Labeller(FeatureSpec):
    """A facet strip labeller.

    Build one with :data:`label_value`, :data:`label_both`, :data:`label_context`
    or :func:`as_labeller`. Calling a labeller returns a copy with new options.

    Parameters
    ----------
    base : {"value", "both", "context"}
        How variable names and values combine: the value alone, ``name: value``,
        or ``name: value`` only when a strip group has several variables.
    multi_line : bool, default True
        Whether each variable gets its own strip. ``False`` joins them into one
        strip (``"cyl, drv: 4, f"``), wrapped at the first ``labwidth``.
    sep : str, default ": "
        Separator between the variable name and the value.
    lookup : Mapping[str, str], optional
        Per-value labels (``as_labeller``); values not in it keep their text.
    function : Callable[[str], str], optional
        Per-value function (``as_labeller``), called with each level's text (a
        number as its plain value, ``"1000"``, as ggplot2).

    Raises
    ------
    TypeError
        When ``multi_line`` is not a bool or ``sep`` is not a string.

    Examples
    --------
    >>> from ggrust import *
    >>> labeller = Labeller("both", sep=" = ")
    >>> labeller.as_dict()["sep"]
    ' = '
    """

    def __init__(
        self,
        base: LabellerBase,
        *,
        multi_line: bool = True,
        sep: str = ": ",
        lookup: Mapping[str, str] | None = None,
        function: Callable[[str], str] | None = None,
    ) -> None:
        if not isinstance(multi_line, bool):
            raise TypeError(f"labeller: multi_line must be True or False, got {multi_line!r}")
        if not isinstance(sep, str):
            raise TypeError(f"labeller: sep must be a string, got {sep!r}")
        super().__init__(
            "labeller",
            None,
            base=base,
            multi_line=multi_line,
            sep=sep,
            lookup=dict(lookup) if lookup is not None else None,
            function=function,
        )

    def __call__(self, *, multi_line: bool | None = None, sep: str | None = None) -> Labeller:
        """Return a copy with new options.

        Parameters
        ----------
        multi_line : bool, optional
            Whether each variable gets its own strip; unchanged when ``None``.
        sep : str, optional
            Separator between the variable name and the value; unchanged when ``None``.

        Returns
        -------
        Labeller
            The new labeller.

        Examples
        --------
        >>> from ggrust import *
        >>> labeller = label_both(sep=" = ", multi_line=False)
        """
        props = self.props()
        return Labeller(
            props["base"],
            multi_line=props["multi_line"] if multi_line is None else multi_line,
            sep=props["sep"] if sep is None else sep,
            lookup=props.get("lookup"),
            function=props.get("function"),
        )


def _named(labeller: Labeller, doc: str) -> Labeller:
    """Return ``labeller`` with its own docstring (shown by ``help()`` and the docs)."""
    labeller.__doc__ = doc
    return labeller


label_value: Labeller = _named(
    Labeller("value"),
    """Label facet strips with the value alone (the default).

    Call it to change options: ``label_value(multi_line=False)`` joins the values of
    several faceting columns into one strip (``"4, f"``).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 2, 4], "weight": [4, 6, 8, 12], "pet": ["cat", "cat", "dog", "dog"]}
    >>> plot = ggplot(data, aes("age", "weight")) + geom_point()
    >>> plot = plot + facet_wrap("pet", labeller=label_value)
    """,
)

label_both: Labeller = _named(
    Labeller("both"),
    """Label facet strips with ``variable: value``.

    Call it to change options: ``label_both(sep=" = ")``, or
    ``label_both(multi_line=False)`` for one strip (``"cyl, drv: 4, f"``).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 2, 4], "weight": [4, 6, 8, 12], "pet": ["cat", "cat", "dog", "dog"]}
    >>> plot = ggplot(data, aes("age", "weight")) + geom_point()
    >>> plot = plot + facet_wrap("pet", labeller=label_both)
    """,
)

label_context: Labeller = _named(
    Labeller("context"),
    """Label facet strips with the value, or ``variable: value`` for several variables.

    ``facet_wrap`` over two or more columns gets ``label_both``, one column gets
    ``label_value``. Each margin of a ``facet_grid`` has one variable, so it gets
    the value.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 2, 4], "weight": [4, 6, 8, 12],
    ...         "pet": ["cat", "cat", "dog", "dog"], "year": [1, 2, 1, 2]}
    >>> plot = ggplot(data, aes("age", "weight")) + geom_point()
    >>> plot = plot + facet_wrap(["pet", "year"], labeller=label_context)
    """,
)


def as_labeller(
    x: Mapping[str | int | float | bool, str] | Callable[[str], str] | Labeller | LabellerName,
    default: Labeller | LabellerName = label_value,
    multi_line: bool = True,
) -> Labeller:
    """Build a labeller from a lookup table or a function (ggplot2 ``as_labeller``).

    Parameters
    ----------
    x : Mapping, callable, Labeller or str
        A table from level to label (levels not in it keep their text; a numeric
        key matches a numeric level by value), a function called with each level's
        text (a number as its plain value, ``"1000"``) that returns its label, or a
        labeller (or its name), returned with ``multi_line`` set.
    default : Labeller or str, default label_value
        The labeller applied after the table or function, for example
        ``label_both`` for ``variable: label``.
    multi_line : bool, default True
        Whether each variable gets its own strip.

    Returns
    -------
    Labeller
        The labeller, for ``facet_wrap(labeller=...)`` or ``facet_grid(labeller=...)``.

    Raises
    ------
    SpecError
        When ``x`` or ``default`` is an unknown labeller name.
    TypeError
        When ``x`` is not a table, a callable or a labeller, or a table value is not
        a string.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4], "weight": [4, 6, 6, 8, 12, 14],
    ...         "pet": ["cat"] * 3 + ["dog"] * 3}
    >>> names = as_labeller({"cat": "Cats", "dog": "Dogs"})
    >>> plot = ggplot(data, aes("age", "weight")) + geom_point()
    >>> plot = plot + facet_wrap("pet", labeller=names)
    """
    base = _resolve(default, "default")
    if isinstance(x, Labeller | str):
        return _resolve(x, "x")(multi_line=multi_line)
    props = base.props()
    if isinstance(x, Mapping):
        lookup: dict[str, str] = {}
        for key, value in x.items():
            if not isinstance(value, str):
                raise TypeError(
                    f"as_labeller: the label of {key!r} must be a string, got {value!r}"
                )
            lookup[_level_text(key)] = value
        return Labeller(props["base"], multi_line=multi_line, sep=props["sep"], lookup=lookup)
    if callable(x):
        return Labeller(props["base"], multi_line=multi_line, sep=props["sep"], function=x)
    raise TypeError(
        f"as_labeller: expected a dict of level to label, a function or a labeller, got {x!r}"
    )


def resolve_labeller(labeller: Labeller | LabellerName | None) -> Labeller | None:
    """Return the labeller a facet's ``labeller=`` names, checked.

    Parameters
    ----------
    labeller : Labeller, str or None
        A labeller, its name (``"label_both"``) or ``None``.

    Returns
    -------
    Labeller or None
        The labeller, or ``None`` for the default.

    Raises
    ------
    SpecError
        When the name is unknown.
    TypeError
        When ``labeller`` is not a labeller.
    """
    if labeller is None:
        return None
    return _resolve(labeller, "labeller")


def _resolve(labeller: object, parameter: str) -> Labeller:
    """Return a labeller from a labeller or its name; errors name ``parameter``."""
    if isinstance(labeller, Labeller):
        return labeller
    if isinstance(labeller, str):
        bases: dict[str, LabellerBase] = {"value": "value", "both": "both", "context": "context"}
        base = bases.get(labeller.removeprefix("label_"))
        if base is not None:
            return Labeller(base)
        reason = (
            f"unknown labeller '{labeller}'; use one of {', '.join(_NAMES)}"
            f"{did_you_mean(labeller, _NAMES)}"
        )
        raise SpecError(f"{parameter}: {reason}", path=parameter, reason=reason)
    if callable(labeller):
        raise TypeError(
            f"{parameter}: a plain function is not a labeller; wrap it as "
            "as_labeller(function), which calls it with each level's text"
        )
    raise TypeError(
        f"{parameter}: expected a labeller such as label_both or as_labeller(...), got {labeller!r}"
    )


def _level_text(key: object) -> str:
    """Return a lookup key as the engine's level text (``4.0`` and ``4`` give ``"4"``)."""
    item = getattr(key, "item", None)
    if callable(item) and not isinstance(key, str):
        key = item()  # a numpy scalar
    if isinstance(key, bool):
        return str(key).lower()
    if isinstance(key, float) and key.is_integer():
        return str(int(key))
    return str(key)
