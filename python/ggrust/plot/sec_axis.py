"""Secondary axes: ``sec_axis()``, ``dup_axis()`` and ``derive()`` (ggplot2).

lets-plot has no secondary axes; these follow ggplot2 4.x. Pass the result as
``scale_x_continuous(sec_axis=...)`` (or ``scale_y_*``, the log and reverse
variants). The transform is any Python callable: the engine calls it once per value
on 1000 points over the axis range (ggplot2's sampling), so it must be strictly
monotonic there.
"""

from __future__ import annotations

from collections.abc import Callable, Sequence
from typing import Literal, TypeAlias

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec

__all__ = ["derive", "dup_axis", "sec_axis"]


class DerivedSpec(FeatureSpec):
    """``derive()``: take this option from the primary axis."""

    def __init__(self) -> None:
        super().__init__("derive", None, derive=True)


class SecondaryAxisSpec(FeatureSpec):
    """A secondary axis (``sec_axis()`` / ``dup_axis()``) for a continuous position scale."""

    def __init__(
        self,
        transform: Callable[[float], float] | None,
        name: str | DerivedSpec | None,
        breaks: list[float | None] | DerivedSpec | None,
        labels: list[str] | str | DerivedSpec | None,
        guide: str | DerivedSpec | None,
    ) -> None:
        super().__init__(
            "sec_axis",
            None,
            transform=transform,
            breaks=breaks,
            labels=labels,
            guide=guide,
        )
        if name is not None:
            self.props()["name"] = name


SecondaryName: TypeAlias = str | DerivedSpec | None
SecondaryBreaks: TypeAlias = Sequence[float | None] | DerivedSpec | None
SecondaryLabels: TypeAlias = Sequence[str] | str | DerivedSpec | None
SecondaryGuide: TypeAlias = Literal["axis", "none"] | DerivedSpec | None


def derive() -> DerivedSpec:
    """Take a secondary axis option from the primary axis (ggplot2 ``derive()``).

    Returns
    -------
    DerivedSpec
        A marker for ``sec_axis(name=, breaks=, labels=, guide=)``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": [1, 2, 3, 4], "celsius": [12.0, 15.5, 21.0, 18.5]}
    >>> plot = ggplot(data, aes("day", "celsius")) + geom_line()
    >>> plot = plot + scale_y_continuous(sec_axis=sec_axis(lambda c: c * 1.8 + 32, name=derive()))
    """
    return DerivedSpec()


def _invalid(function: str, key: str, reason: str) -> SpecError:
    return SpecError(f"{function}({key}=...): {reason}", path=f"sec_axis.{key}", reason=reason)


def _wrong_type(function: str, key: str, value: object, accepted: str) -> UnsupportedValueError:
    reason = f"expected {accepted}, got {type(value).__name__}"
    return UnsupportedValueError(
        f"{function}({key}={value!r}): {reason}", path=f"sec_axis.{key}", reason=reason
    )


def _secondary_axis(
    function: str,
    transform: Callable[[float], float] | None,
    name: SecondaryName,
    breaks: SecondaryBreaks,
    labels: SecondaryLabels,
    guide: SecondaryGuide,
) -> SecondaryAxisSpec:
    """Check the options of ``sec_axis()`` / ``dup_axis()`` and build the spec.

    Raises
    ------
    UnsupportedValueError
        If an option has the wrong type (a transform that is not callable, ...).
    SpecError
        If labels do not match the breaks, or the guide is unknown.
    """
    if transform is not None and not callable(transform):
        raise _wrong_type(function, "transform", transform, "a function (or None for the identity)")
    if name is not None and not isinstance(name, str | DerivedSpec):
        raise _wrong_type(function, "name", name, "a text or derive()")
    break_values: list[float | None] | DerivedSpec | None = None
    if isinstance(breaks, DerivedSpec):
        break_values = breaks
    elif breaks is not None:
        if isinstance(breaks, str) or not isinstance(breaks, Sequence):
            raise _wrong_type(function, "breaks", breaks, "a list of numbers or derive()")
        for value in breaks:
            if value is not None and (
                isinstance(value, bool) or not isinstance(value, int | float)
            ):
                raise _wrong_type(function, "breaks", value, "numbers in the breaks list")
        break_values = [None if value is None else float(value) for value in breaks]
    label_values: list[str] | str | DerivedSpec | None
    if labels is None or isinstance(labels, str | DerivedSpec):
        label_values = labels
    elif isinstance(labels, Sequence):
        label_values = [str(label) for label in labels]
        if not isinstance(break_values, list):
            raise _invalid(
                function,
                "labels",
                "a list of labels needs a list of breaks (one label per break); give "
                "breaks=[...], or a format text such as '{.1f}'",
            )
        if len(label_values) != len(break_values):
            raise _invalid(
                function,
                "labels",
                f"{len(label_values)} labels for {len(break_values)} breaks; give one label "
                "per break",
            )
    else:
        raise _wrong_type(function, "labels", labels, "a list of texts, a format text or derive()")
    if guide is not None and not isinstance(guide, DerivedSpec) and guide not in ("axis", "none"):
        raise _invalid(function, "guide", f"unsupported guide {guide!r}; use 'axis' or 'none'")
    return SecondaryAxisSpec(transform, name, break_values, label_values, guide)


def sec_axis(
    transform: Callable[[float], float] | None = None,
    *,
    name: SecondaryName = None,
    breaks: SecondaryBreaks = None,
    labels: SecondaryLabels = None,
    guide: SecondaryGuide = None,
) -> SecondaryAxisSpec:
    """Add a secondary axis on the other side of a continuous position scale (ggplot2).

    The secondary axis shows the primary axis through ``transform``: its breaks
    are chosen over the transformed axis range and placed where the primary axis
    has the matching value. Works with ``position=``, facets, ``coord_flip()`` and
    log or reverse primary scales.

    Parameters
    ----------
    transform : callable, optional
        Function of one primary value returning the secondary value, e.g.
        ``lambda celsius: celsius * 1.8 + 32``. It is called once per value on 1000
        points over the axis range and must be strictly monotonic over the data.
        ``None`` is the identity.
    name : str or derive(), optional
        Secondary axis title; ``derive()`` takes the primary title. No title by default.
    breaks : list of float or derive(), optional
        Break values in secondary units; ``derive()`` takes the primary scale's
        ``breaks``. Chosen over the secondary range by default (log breaks on a
        log scale).
    labels : list of str, str or derive(), optional
        One label per break (needs ``breaks``), or a format such as ``"{.0f} °F"``;
        ``derive()`` takes the primary scale's ``labels`` and ``format``.
    guide : {'axis', 'none'} or derive(), optional
        ``'none'`` hides the secondary axis.

    Returns
    -------
    SecondaryAxisSpec
        Pass it as ``sec_axis=`` to ``scale_x_continuous()`` / ``scale_y_continuous()``.

    Raises
    ------
    UnsupportedValueError
        If ``transform`` is not callable, or another option has the wrong type.
    SpecError
        If labels do not match the breaks or the guide is unknown. A transform that
        is not monotonic over the axis range fails when the plot is rendered.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"month": [1, 2, 3, 4, 5, 6], "celsius": [3.1, 4.2, 8.0, 12.5, 16.9, 20.3]}
    >>> plot = (
    ...     ggplot(data, aes("month", "celsius"))
    ...     + geom_line()
    ...     + scale_y_continuous(
    ...         "Temperature (°C)", sec_axis=sec_axis(lambda c: c * 1.8 + 32, name="°F")
    ...     )
    ... )
    """
    return _secondary_axis("sec_axis", transform, name, breaks, labels, guide)


def dup_axis(
    transform: Callable[[float], float] | None = None,
    *,
    name: SecondaryName = None,
    breaks: SecondaryBreaks = None,
    labels: SecondaryLabels = None,
    guide: SecondaryGuide = None,
) -> SecondaryAxisSpec:
    """Copy the primary axis to the other side (ggplot2 ``dup_axis()``).

    As ``sec_axis()`` with every unset option ``derive()``d: the same title,
    breaks and labels as the primary axis.

    Parameters
    ----------
    transform : callable, optional
        As in ``sec_axis()``; the identity by default.
    name, breaks, labels, guide
        As in ``sec_axis()``; unset ones are ``derive()``.

    Returns
    -------
    SecondaryAxisSpec
        Pass it as ``sec_axis=`` to a continuous position scale.

    Raises
    ------
    UnsupportedValueError
        If an option has the wrong type.
    SpecError
        If labels do not match the breaks or the guide is unknown.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [2.0, 4.5, 3.0]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point() + scale_y_continuous(sec_axis=dup_axis())
    """
    return _secondary_axis(
        "dup_axis",
        transform,
        derive() if name is None else name,
        derive() if breaks is None else breaks,
        derive() if labels is None else labels,
        derive() if guide is None else guide,
    )
