"""ggsci journal palettes: discrete ``scale_color_*`` / ``scale_fill_*`` (R package ggsci).

Not in lets-plot; ggplot2 needs the ggsci package. The palette colors are ggsci's
published hex values; the code is ggrust's own. Each scale maps categories to the
palette's colors in order; more categories than colors get overflow colors (see
``overflow``) and a warning.
"""

from __future__ import annotations

from numbers import Real
from typing import TYPE_CHECKING, Literal, TypeAlias

from ggrust.errors import SpecError, UnsupportedValueError, did_you_mean
from ggrust.plot.scale import _scale

if TYPE_CHECKING:
    from ggrust._typing import Breaks, Color, Guide, Labels, Limits
    from ggrust.plot.core import FeatureSpec
__all__ = [
    "scale_color_aaas",
    "scale_color_d3",
    "scale_color_igv",
    "scale_color_jama",
    "scale_color_jco",
    "scale_color_lancet",
    "scale_color_nejm",
    "scale_color_npg",
    "scale_fill_aaas",
    "scale_fill_d3",
    "scale_fill_igv",
    "scale_fill_jama",
    "scale_fill_jco",
    "scale_fill_lancet",
    "scale_fill_nejm",
    "scale_fill_npg",
]

Overflow: TypeAlias = Literal["interpolate", "cycle", "generate", "i", "c", "g"]
"""How to get more colors than a palette has."""

_OVERFLOW_NAMES = ("interpolate", "cycle", "generate", "i", "c", "g")
"""``overflow`` values the engine accepts (any case)."""
NpgPalette: TypeAlias = Literal["nrc"]
AaasPalette: TypeAlias = Literal["default"]
NejmPalette: TypeAlias = Literal["default"]
LancetPalette: TypeAlias = Literal["lanonc"]
JamaPalette: TypeAlias = Literal["default"]
JcoPalette: TypeAlias = Literal["default"]
D3Palette: TypeAlias = Literal["category10", "category20", "category20b", "category20c"]
IgvPalette: TypeAlias = Literal["default", "alternating"]

_PALETTES: dict[str, tuple[str, ...]] = {
    "npg": ("nrc",),
    "aaas": ("default",),
    "nejm": ("default",),
    "lancet": ("lanonc",),
    "jama": ("default",),
    "jco": ("default",),
    "d3": (
        "category10",
        "category20",
        "category20b",
        "category20c",
    ),
    "igv": (
        "default",
        "alternating",
    ),
}
"""The palette names of each family, the default first (ggsci's ``palette`` choices)."""


def _ggsci(
    aesthetic: Literal["color", "fill"],
    family: str,
    palette: str | None,
    alpha: float,
    overflow: Overflow | None,
    name: str | None,
    breaks: Breaks | None,
    labels: Labels | None,
    lablim: int | None,
    limits: Limits | None,
    na_value: Color | None,
    guide: Guide | None,
    format: str | None,
) -> FeatureSpec:
    """Build a ggsci scale spec, checking ``palette`` and ``alpha`` early.

    Raises
    ------
    SpecError
        If ``palette`` is not one of the family's palettes, ``alpha`` is not in
        (0, 1], or ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.
    """
    function = f"scale_{aesthetic}_{family}"
    names = _PALETTES[family]
    accepted = ", ".join(f"'{item}'" for item in names)
    if palette is not None:
        if not isinstance(palette, str):
            reason = f"expected a palette name ({accepted}), got {type(palette).__name__}"
            raise UnsupportedValueError(
                f"{function}(palette={palette!r}): {reason}", path="palette", reason=reason
            )
        if palette not in names:
            reason = (
                f"{family} palette '{palette}' was not found; use {accepted}"
                f"{did_you_mean(palette, names)}"
            )
            raise SpecError(
                f"{function}(palette={palette!r}): {reason}", path="palette", reason=reason
            )
    if isinstance(alpha, bool) or not isinstance(alpha, Real):
        reason = f"expected a number in (0, 1], got {type(alpha).__name__}"
        raise UnsupportedValueError(
            f"{function}(alpha={alpha!r}): {reason}", path="alpha", reason=reason
        )
    if not 0 < alpha <= 1:
        reason = f"alpha must be in (0, 1], got {alpha}"
        raise SpecError(f"{function}(alpha={alpha!r}): {reason}", path="alpha", reason=reason)
    if overflow is not None and (
        not isinstance(overflow, str) or overflow.lower() not in _OVERFLOW_NAMES
    ):
        reason = (
            "overflow expected one of 'interpolate' ('i'), 'cycle' ('c'), 'generate' ('g'), "
            f"got {overflow!r}{did_you_mean(str(overflow), _OVERFLOW_NAMES)}"
        )
        raise SpecError(
            f"{function}(overflow={overflow!r}): {reason}", path="overflow", reason=reason
        )
    return _scale(
        aesthetic,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        na_value=na_value,
        guide=guide,
        format=format,
        scale_mapper_kind=f"color_{family}",
        palette=palette,
        alpha=alpha,
        overflow=overflow,
    )


def scale_color_npg(
    palette: NpgPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with the ggsci ``npg`` palette, from Nature Reviews Cancer (NPG).

    A discrete scale (10 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"nrc"}, default="nrc"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a npg palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_npg()
    """
    return _ggsci(
        "color",
        "npg",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_fill_npg(
    palette: NpgPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with the ggsci ``npg`` palette. See ``scale_color_npg``.

    A discrete scale (10 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"nrc"}, default="nrc"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a npg palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_npg(alpha=0.8)
    """
    return _ggsci(
        "fill",
        "npg",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_color_aaas(
    palette: AaasPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with the ggsci ``aaas`` palette, from Science (AAAS).

    A discrete scale (10 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a aaas palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_aaas()
    """
    return _ggsci(
        "color",
        "aaas",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_fill_aaas(
    palette: AaasPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with the ggsci ``aaas`` palette. See ``scale_color_aaas``.

    A discrete scale (10 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a aaas palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_aaas(alpha=0.8)
    """
    return _ggsci(
        "fill",
        "aaas",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_color_nejm(
    palette: NejmPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with the ggsci ``nejm`` palette, from NEJM.

    A discrete scale (8 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a nejm palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_nejm()
    """
    return _ggsci(
        "color",
        "nejm",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_fill_nejm(
    palette: NejmPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with the ggsci ``nejm`` palette. See ``scale_color_nejm``.

    A discrete scale (8 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a nejm palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_nejm(alpha=0.8)
    """
    return _ggsci(
        "fill",
        "nejm",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_color_lancet(
    palette: LancetPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with the ggsci ``lancet`` palette, from Lancet Oncology.

    A discrete scale (9 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"lanonc"}, default="lanonc"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a lancet palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_lancet()
    """
    return _ggsci(
        "color",
        "lancet",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_fill_lancet(
    palette: LancetPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with the ggsci ``lancet`` palette. See ``scale_color_lancet``.

    A discrete scale (9 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"lanonc"}, default="lanonc"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a lancet palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_lancet(alpha=0.8)
    """
    return _ggsci(
        "fill",
        "lancet",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_color_jama(
    palette: JamaPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with the ggsci ``jama`` palette, from JAMA.

    A discrete scale (7 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a jama palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_jama()
    """
    return _ggsci(
        "color",
        "jama",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_fill_jama(
    palette: JamaPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with the ggsci ``jama`` palette. See ``scale_color_jama``.

    A discrete scale (7 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a jama palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_jama(alpha=0.8)
    """
    return _ggsci(
        "fill",
        "jama",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_color_jco(
    palette: JcoPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with the ggsci ``jco`` palette, from the Journal of Clinical Oncology (JCO).

    A discrete scale (10 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a jco palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_jco()
    """
    return _ggsci(
        "color",
        "jco",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_fill_jco(
    palette: JcoPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with the ggsci ``jco`` palette. See ``scale_color_jco``.

    A discrete scale (10 colors).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a jco palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_jco(alpha=0.8)
    """
    return _ggsci(
        "fill",
        "jco",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_color_d3(
    palette: D3Palette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with the ggsci ``d3`` palette, from D3.js.

    A discrete scale (10 colors, 20 for the ``category20*`` palettes).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"category10", "category20", "category20b", "category20c"}, default="category10"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a d3 palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_d3(palette="category20")
    """
    return _ggsci(
        "color",
        "d3",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_fill_d3(
    palette: D3Palette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with the ggsci ``d3`` palette. See ``scale_color_d3``.

    A discrete scale (10 colors, 20 for the ``category20*`` palettes).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"category10", "category20", "category20b", "category20c"}, default="category10"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a d3 palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_d3(palette="category20")
    """
    return _ggsci(
        "fill",
        "d3",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_color_igv(
    palette: IgvPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with the ggsci ``igv`` palette, from the IGV genome browser.

    A discrete scale (51 colors, 2 for ``"alternating"``).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default", "alternating"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a igv palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": list(range(30)), "y": [value % 7 for value in range(30)],
    ...         "chromosome": [f"chr{value + 1}" for value in range(30)]}
    >>> plot = ggplot(data, aes("x", "y", color="chromosome")) + geom_point(size=5)
    >>> plot = plot + scale_color_igv()
    """
    return _ggsci(
        "color",
        "igv",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )


def scale_fill_igv(
    palette: IgvPalette | None = None,
    alpha: float = 1,
    overflow: Overflow | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with the ggsci ``igv`` palette. See ``scale_color_igv``.

    A discrete scale (51 colors, 2 for ``"alternating"``).
    Not in lets-plot (R package ggsci).

    Parameters
    ----------
    palette : {"default", "alternating"}, default="default"
        ggsci palette name.
    alpha : float, default=1
        Opacity of the palette colors, in (0, 1].
    overflow : {"generate", "cycle", "interpolate"}, optional
        How to get more colors than the palette has (``"g"``, ``"c"``, ``"i"``).
        Default: ``"generate"``, as ``scale_*_brewer()`` for qualitative palettes.
        ggsci leaves the extra categories missing; ggrust colors them and warns.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order) of the scale. Other data gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``palette`` is not a igv palette name, ``alpha`` is not in (0, 1], or
        ``overflow`` is not an overflow name.
    UnsupportedValueError
        If ``palette`` is not a string or ``alpha`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": list(range(30)), "y": [value % 7 for value in range(30)],
    ...         "chromosome": [f"chr{value + 1}" for value in range(30)]}
    >>> plot = ggplot(data, aes("x", "y", fill="chromosome")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_igv(alpha=0.8)
    """
    return _ggsci(
        "fill",
        "igv",
        palette,
        alpha,
        overflow,
        name,
        breaks,
        labels,
        lablim,
        limits,
        na_value,
        guide,
        format,
    )
