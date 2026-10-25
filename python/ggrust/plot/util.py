"""Data annotation helpers: ``data_meta`` for mappings and frames.

Ported from lets-plot's ``lets_plot/plot/util.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import Any

from ggrust.mapping import MappingMeta
from ggrust.plot.core import FeatureSpec, PlotSpec, aes
from ggrust.plot.series_meta import (
    TYPE_DATE_TIME,
    TYPE_UNKNOWN,
    detect_time_zone,
    infer_types,
    is_pandas_data_frame,
    is_polars_data_frame,
)


def last_not_none(values: list) -> Any:
    """Return the last value that is not None, or None."""
    for value in reversed(values):
        if value is not None:
            return value
    return None


def key_int2str(data):
    """Use string column names when the names are integers (as lets_plot)."""
    if is_pandas_data_frame(data):
        if data.columns.inferred_type in ("integer", "mixed-integer"):
            data.columns = data.columns.astype(str)
        return data
    if isinstance(data, dict):
        return {str(key) if isinstance(key, int) else key: value for key, value in data.items()}
    return data


def update_plot_aes_mapping(plot: PlotSpec, add_mapping: FeatureSpec) -> None:
    """Merge ``plot + aes(...)`` into the plot mapping and refresh ``data_meta``."""
    existing = plot.props().get("mapping", aes())
    merged = {**existing.as_dict(), **add_mapping.as_dict()}
    data = plot.props().get("data", None)
    data, mapping, data_meta = as_annotated_data(data, aes(**merged))
    plot.props()["data"] = data
    plot.props()["mapping"] = mapping
    for key, value in data_meta.items():
        plot.props()[key] = value


def as_annotated_data(data: Any, mapping_spec: FeatureSpec | None) -> tuple:
    """Resolve ``as_discrete`` annotations and describe column types.

    Parameters
    ----------
    data : DataFrame or dict or None
        Plot or layer data.
    mapping_spec : FeatureSpec or None
        The ``aes()`` mapping, possibly holding ``MappingMeta`` values.

    Returns
    -------
    tuple
        ``(data, mapping with plain column names, {"data_meta": {...}})``.
    """
    type_by_variable: dict[str, str] = {}
    meta_by_variable: dict[str, dict[str, MappingMeta]] = {}
    mappings = {}
    if mapping_spec is not None:
        for key, spec in mapping_spec.props().items():
            if key == "name":
                continue
            if isinstance(spec, MappingMeta):
                mappings[key] = spec.variable
                meta_by_variable.setdefault(spec.variable, {})[key] = spec
                type_by_variable[spec.variable] = TYPE_UNKNOWN
            else:
                mappings[key] = spec
    type_by_variable.update(infer_types(data))

    time_zones = {}
    for variable, data_type in type_by_variable.items():
        if data_type == TYPE_DATE_TIME:
            time_zone = detect_time_zone(variable, data)
            if time_zone is not None:
                time_zones[variable] = time_zone

    series_annotations = {}
    for variable, data_type in type_by_variable.items():
        annotation = {}
        if data_type != TYPE_UNKNOWN:
            annotation["type"] = data_type
        if variable in time_zones:
            annotation["time_zone"] = time_zones[variable]
        # Same branch order as lets_plot: an unordered pandas column falls through
        # to the as_discrete levels below.
        if (
            is_pandas_data_frame(data)
            and data[variable].dtype.name == "category"
            and data[variable].dtype.ordered
        ):
            annotation["factor_levels"] = data[variable].cat.categories.to_list()
        elif is_polars_data_frame(data):
            import polars

            dtype = data[variable].dtype
            if isinstance(dtype, polars.Enum):
                annotation["factor_levels"] = list(dtype.categories)
        elif variable in meta_by_variable:
            levels = last_not_none([meta.levels for meta in meta_by_variable[variable].values()])
            if levels is not None:
                annotation["factor_levels"] = levels
        if "factor_levels" in annotation and variable in meta_by_variable:
            order = last_not_none(
                [meta.parameters["order"] for meta in meta_by_variable[variable].values()]
            )
            if order is not None:
                annotation["order"] = order
        if annotation:
            annotation["column"] = variable
            series_annotations[variable] = annotation

    mapping_annotations = []
    for variable, metas in meta_by_variable.items():
        for aesthetic, meta in metas.items():
            if meta.annotation != "as_discrete":
                continue
            if "factor_levels" in series_annotations.get(variable, {}):
                continue
            mapping_annotation = {}
            label = meta.parameters.get("label")
            if label is not None:
                mapping_annotation.setdefault("parameters", {})["label"] = label
            if meta.levels is not None:
                mapping_annotation["levels"] = meta.levels
            order_by = meta.parameters.get("order_by")
            if order_by is not None:
                mapping_annotation.setdefault("parameters", {})["order_by"] = order_by
            order = meta.parameters.get("order")
            if order is not None:
                mapping_annotation.setdefault("parameters", {})["order"] = order
            if mapping_annotation:
                mapping_annotation["aes"] = aesthetic
                mapping_annotation["annotation"] = "as_discrete"
                mapping_annotations.append(mapping_annotation)

    data_meta = {}
    if series_annotations:
        data_meta["series_annotations"] = list(series_annotations.values())
    if mapping_annotations:
        data_meta["mapping_annotations"] = mapping_annotations
    return data, aes(**mappings), {"data_meta": data_meta}
