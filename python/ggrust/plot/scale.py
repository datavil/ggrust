"""Position scales: ``scale_x_continuous()``, ``scale_x_log10()``, ``xlim()``, ...

Ported from lets-plot's ``lets_plot/plot/scale.py``, ``scale_position.py`` and
``scale_convenience.py`` (MIT, see THIRD_PARTY_NOTICES.md). Discrete and color scales
come in later phases.
"""

from ggrust.plot.core import DummySpec, FeatureSpec, FeatureSpecArray

__all__ = [
    "lims",
    "scale_x_continuous",
    "scale_x_log2",
    "scale_x_log10",
    "scale_x_reverse",
    "scale_y_continuous",
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
    breaks, break_width, labels, lablim, na_value, format, position
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
    """Limits of the x axis, e.g. ``xlim(0, 10)`` or ``xlim(None, 10)``.

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
        message = "discrete limits (category lists) are not supported yet"
        raise NotImplementedError(message)
    scale = scale_x_continuous if aesthetic == "x" else scale_y_continuous
    return scale(limits=list(limits))
