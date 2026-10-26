"""Position scales: ``scale_x_continuous()``, ``scale_x_discrete()``, ``xlim()``, ...

Ported from lets-plot's ``lets_plot/plot/scale.py``, ``scale_position.py`` and
``scale_convenience.py`` (MIT, see THIRD_PARTY_NOTICES.md). Color scales come in a
later phase.
"""

from ggrust.plot.core import DummySpec, FeatureSpec, FeatureSpecArray

__all__ = [
    "lims",
    "scale_x_continuous",
    "scale_x_discrete",
    "scale_x_discrete_reversed",
    "scale_x_log2",
    "scale_x_log10",
    "scale_x_reverse",
    "scale_y_continuous",
    "scale_y_discrete",
    "scale_y_discrete_reversed",
    "scale_y_log2",
    "scale_y_log10",
    "scale_y_reverse",
    "xlim",
    "ylim",
]


def _scale(
    aesthetic,
    *,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    expand=None,
    na_value=None,
    trans=None,
    guide=None,
    format=None,
    position=None,
    **kwargs,
):
    """Create a scale spec for one aesthetic, or an array of specs for a list of them.

    ``breaks`` / ``labels`` given as dicts are split into lists as lets-plot does.
    """
    args = {
        "aesthetic": aesthetic,
        "name": name,
        "breaks": breaks,
        "labels": labels,
        "lablim": lablim,
        "limits": limits,
        "expand": expand,
        "na_value": na_value,
        "trans": trans,
        "guide": guide,
        "format": format,
        "position": position,
    }
    if isinstance(breaks, dict):
        if labels is None:
            args["labels"] = list(breaks.keys())
        args["breaks"] = breaks = list(breaks.values())
    if isinstance(labels, dict):
        if breaks is None:
            args["breaks"] = list(labels.keys())
            args["labels"] = list(labels.values())
        else:
            new_breaks = [value for value in breaks if value in labels]
            args["labels"] = [labels[value] for value in new_breaks]
            args["breaks"] = new_breaks + [value for value in breaks if value not in new_breaks]
    if isinstance(aesthetic, list):
        args.pop("aesthetic")
        return FeatureSpecArray(
            *[FeatureSpec("scale", aesthetic=item, **args, **kwargs) for item in aesthetic]
        )
    return FeatureSpec("scale", **args, **kwargs)


def scale_x_continuous(
    name=None,
    *,
    breaks=None,
    break_width=None,
    labels=None,
    lablim=None,
    limits=None,
    expand=None,
    na_value=None,
    trans=None,
    format=None,
    position=None,
) -> FeatureSpec:
    """Continuous position scale x.

    Parameters
    ----------
    name : str, optional
        Axis title (``labs(x=...)`` takes precedence).
    breaks : list of float, optional
        Fixed break positions in data units; breaks outside the axis are dropped.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels). Ignored without
        ``breaks``, as in lets-plot.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"e"``, ``"%"``) or a
        template (``"{.1f} %"``, ``"{d} cells"``). ``labels`` take precedence.
    break_width, lablim, na_value, position
        As in lets-plot; not drawn yet (ggrust warns).
    limits : list, optional
        ``[low, high]`` in data units; ``None`` leaves a side free.
    expand : list, optional
        ``[multiplicative]`` or ``[multiplicative, additive]`` expansion.
    trans : {'identity', 'log10', 'log2', 'sqrt', 'reverse'}, optional
        Transformation of the data.

    Returns
    -------
    FeatureSpec
        Scale specification.
    """
    return _scale(
        "x",
        name=name,
        breaks=breaks,
        break_width=break_width,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        trans=trans,
        format=format,
        position=position,
    )


def scale_y_continuous(
    name=None,
    *,
    breaks=None,
    break_width=None,
    labels=None,
    lablim=None,
    limits=None,
    expand=None,
    na_value=None,
    trans=None,
    format=None,
    position=None,
) -> FeatureSpec:
    """Continuous position scale y. See ``scale_x_continuous``.

    Returns
    -------
    FeatureSpec
        Scale specification.
    """
    return _scale(
        "y",
        name=name,
        breaks=breaks,
        break_width=break_width,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        trans=trans,
        format=format,
        position=position,
    )


def _transformed(scale, trans):
    def scale_function(
        name=None,
        *,
        breaks=None,
        break_width=None,
        labels=None,
        lablim=None,
        limits=None,
        expand=None,
        na_value=None,
        format=None,
        position=None,
    ) -> FeatureSpec:
        return scale(
            name,
            breaks=breaks,
            break_width=break_width,
            labels=labels,
            lablim=lablim,
            limits=limits,
            expand=expand,
            na_value=na_value,
            trans=trans,
            format=format,
            position=position,
        )

    axis = scale.__name__.split("_")[1]
    scale_function.__name__ = f"scale_{axis}_{trans}"
    scale_function.__qualname__ = scale_function.__name__
    scale_function.__doc__ = (
        f"Continuous position scale {axis} with the '{trans}' transformation. "
        f"See ``scale_{axis}_continuous``.\n\n"
        "Returns\n-------\nFeatureSpec\n    Scale specification.\n"
    )
    return scale_function


scale_x_log10 = _transformed(scale_x_continuous, "log10")
scale_y_log10 = _transformed(scale_y_continuous, "log10")
scale_x_log2 = _transformed(scale_x_continuous, "log2")
scale_y_log2 = _transformed(scale_y_continuous, "log2")
scale_x_reverse = _transformed(scale_x_continuous, "reverse")
scale_y_reverse = _transformed(scale_y_continuous, "reverse")


def _as_boolean(value, *, default):
    """lets-plot ``as_boolean``: ``None`` -> ``default``; the string ``'False'`` is false."""
    if value is None:
        return default
    return bool(value) and value != "False"


def scale_x_discrete(
    name=None,
    *,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    expand=None,
    na_value=None,
    reverse=None,
    format=None,
    position=None,
) -> FeatureSpec:
    """Discrete position scale x.

    Parameters
    ----------
    name : str, optional
        Axis title (``labs(x=...)`` takes precedence).
    breaks : list of str, optional
        Only these categories get a tick and a label.
    labels : list of str or dict, optional
        Labels aligned with ``breaks`` (or the categories in axis order); missing
        ones are empty. A dict maps categories to labels.
    format : str, optional
        Label template, e.g. ``"<{}>"``. ``labels`` take precedence.
    lablim, na_value, position
        As in lets-plot; not drawn yet (ggrust warns).
    limits : list of str, optional
        The categories to show, in axis order; other categories are dropped.
    expand : list, optional
        ``[multiplicative]`` or ``[multiplicative, additive]``; the default adds 0.2
        categories on each side.
    reverse : bool, default False
        Reverse the category order.

    Returns
    -------
    FeatureSpec
        Scale specification.
    """
    return _scale(
        "x",
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        trans=None,
        guide=None,
        format=format,
        position=position,
        discrete=True,
        reverse=_as_boolean(reverse, default=False),
    )


def scale_y_discrete(
    name=None,
    *,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    expand=None,
    na_value=None,
    reverse=None,
    format=None,
    position=None,
) -> FeatureSpec:
    """Discrete position scale y. See ``scale_x_discrete``.

    Returns
    -------
    FeatureSpec
        Scale specification.
    """
    return _scale(
        "y",
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        trans=None,
        guide=None,
        format=format,
        position=position,
        discrete=True,
        reverse=_as_boolean(reverse, default=False),
    )


def _discrete_reversed(scale):
    def scale_function(
        name=None,
        *,
        breaks=None,
        labels=None,
        lablim=None,
        limits=None,
        expand=None,
        na_value=None,
        format=None,
        position=None,
    ) -> FeatureSpec:
        return scale(
            name,
            breaks=breaks,
            labels=labels,
            lablim=lablim,
            limits=limits,
            expand=expand,
            na_value=na_value,
            format=format,
            position=position,
            reverse=True,
        )

    scale_function.__name__ = f"{scale.__name__}_reversed"
    scale_function.__qualname__ = scale_function.__name__
    scale_function.__doc__ = (
        f"``{scale.__name__}(reverse=True)``.\n\n"
        "Returns\n-------\nFeatureSpec\n    Scale specification.\n"
    )
    return scale_function


scale_x_discrete_reversed = _discrete_reversed(scale_x_discrete)
scale_y_discrete_reversed = _discrete_reversed(scale_y_discrete)


def lims(x, y) -> FeatureSpec:
    """Limits for both axes: ``xlim(*x) + ylim(*y)``.

    Parameters
    ----------
    x, y : list of float or None
        ``[low, high]``; ``None`` inside a list leaves that side free.

    Returns
    -------
    FeatureSpec
        Scale specifications.
    """
    return xlim(*list(x or [])) + ylim(*list(y or []))


def xlim(*limits) -> FeatureSpec:
    """Limits of the x axis, e.g. ``xlim(0, 10)``, ``xlim(None, 10)`` or ``xlim("B", "T")``.

    Returns
    -------
    FeatureSpec
        Scale specification.
    """
    return _limits("x", *limits)


def ylim(*limits) -> FeatureSpec:
    """Limits of the y axis. See ``xlim``.

    Returns
    -------
    FeatureSpec
        Scale specification.
    """
    return _limits("y", *limits)


def _limits(aesthetic, *limits) -> FeatureSpec:
    if len(limits) == 0:
        return DummySpec()
    if any(isinstance(value, str | bytes) for value in limits):
        scale = scale_x_discrete if aesthetic == "x" else scale_y_discrete
    else:
        scale = scale_x_continuous if aesthetic == "x" else scale_y_continuous
    return scale(limits=list(limits))
