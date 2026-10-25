"""Geom layer functions.

Ported from lets-plot's ``lets_plot/plot/geom.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Only geoms the engine draws are exported; more are added phase by phase (plan.md).
"""

from ggrust.plot.core import FeatureSpec, LayerSpec
from ggrust.plot.tooltip import layer_tooltips
from ggrust.plot.util import as_annotated_data, key_int2str

__all__ = ["geom_point"]


def geom_point(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    size_unit=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Scatter points.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``color``, ``size``,
        ``alpha``, ... Mapping data to styles arrives in Phase 1.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Stat name (``"identity"``).
    position : str or FeatureSpec, optional
        Position adjustment (``"identity"``).
    show_legend, inherit_aes, manual_key, sampling, tooltips, size_unit, na_rm : optional
        As in lets_plot. ``inherit_aes`` and ``na_rm`` are used; the others are
        reported as not supported yet.
    **other_args
        Constant aesthetics such as ``color="red"``, ``size=2``, ``alpha=0.5``, or
        constant positions ``x=1, y=2``.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "point",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        size_unit=size_unit,
        na_rm=na_rm,
        **other_args,
    )


def _geom(
    name,
    *,
    mapping=None,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    na_rm=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    **kwargs,
) -> LayerSpec:
    if mapping and not (isinstance(mapping, FeatureSpec) and mapping.kind == "mapping"):
        message = "Unexpected value for argument 'mapping'. Hint: try to use function aes()"
        raise ValueError(message)
    data = key_int2str(data)
    data, mapping, data_meta = as_annotated_data(data, mapping)
    if isinstance(tooltips, list | tuple):
        tooltips = layer_tooltips(tooltips)
    return LayerSpec(
        geom=name,
        stat=stat,
        data=data,
        mapping=mapping,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        na_rm=na_rm,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        **data_meta,
        **kwargs,
    )
