"""Plot specification objects: ``FeatureSpec``, ``PlotSpec``, ``LayerSpec``, ...

Ported from lets-plot's ``lets_plot/plot/core.py`` (MIT, see THIRD_PARTY_NOTICES.md)
with identical ``as_dict()`` output and ``+`` semantics, so code written for lets_plot
(cellestial) works unchanged. Livemap/geocoding support is not ported. Rendering
(``to_svg``, ``to_png``, ``_repr_html_``) goes through the Rust engine.
"""

from __future__ import annotations

import copy
from collections.abc import Iterable, Iterator
from types import NotImplementedType
from typing import TYPE_CHECKING, Any, Literal, cast

if TYPE_CHECKING:
    from collections.abc import Sequence

    import polars as pl

    from ggrust._typing import AxisScale, Number, PlotBuild, PlotData, Position, Target


__all__ = ["aes", "layer"]


def aes(
    x: str | Sequence[Any] | None = None, y: str | Sequence[Any] | None = None, **kwargs: object
) -> FeatureSpec:
    """Define aesthetic mappings.

    Parameters
    ----------
    x, y : str, optional
        Columns mapped to the x and y positions.
    **kwargs
        Other aesthetics, e.g. ``color="cluster"``, ``size="count"``, ``group=[...]``.

    Returns
    -------
    FeatureSpec
        Aesthetic mapping specification (kind ``"mapping"``).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [150, 165, 180, 172], "weight": [52, 61, 80, 70],
    ...         "sex": ["f", "f", "m", "m"]}
    >>> mapping = aes(x="height", y="weight", color="sex")
    >>> plot = ggplot(data, mapping) + geom_point(size=5)
    """
    return FeatureSpec("mapping", name=None, x=x, y=y, **kwargs)


def layer(
    geom: str | None = None,
    stat: str | None = None,
    data: PlotData | None = None,
    mapping: FeatureSpec | None = None,
    position: Position | None = None,
    **kwargs: object,
) -> LayerSpec:
    """Create a layer from a geom name, stat, data, mapping and position.

    Parameters
    ----------
    geom : str
        Geom name, e.g. ``"point"``.
    stat : str, optional
        Stat name, e.g. ``"identity"``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    position : str or FeatureSpec, optional
        Position adjustment.
    **kwargs
        Constant aesthetics (``color="red"``) and geom/stat options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + layer(geom="point", stat="identity", size=4)
    """
    # lets_plot sends **kwargs as one ``kwargs`` option, which drops them; spread them
    # like ``geom_*`` does (see deviations.md).
    return LayerSpec(geom=geom, stat=stat, data=data, mapping=mapping, position=position, **kwargs)


def _mapping_dict(mapping: object) -> dict[str, Any]:
    """Return a mapping feature as ``{aesthetic: column}`` (empty without one)."""
    return mapping.as_dict() if isinstance(mapping, FeatureSpec) else {}


def _feature_dict(feature: object) -> dict[str, Any] | None:
    """Return a feature's ``as_dict()``, or None when it is not set."""
    return feature.as_dict() if isinstance(feature, FeatureSpec) else None


def _size_of(props: dict[str, Any]) -> tuple[Number, Number] | None:
    """``(width, height)`` of a ``ggsize`` feature in ``props``, or None."""
    size = props.get("ggsize")
    if not isinstance(size, FeatureSpec):
        return None
    return size.props()["width"], size.props()["height"]


def _text_labels(props: dict[str, Any]) -> dict[str, str]:
    """Title, subtitle, caption and tag set on a plot or grid."""
    labels: dict[str, str] = {}
    for kind, key, label in [
        ("ggtitle", "text", "title"),
        ("ggtitle", "subtitle", "subtitle"),
        ("caption", "text", "caption"),
        ("tag", "text", "tag"),
    ]:
        feature = props.get(kind)
        if isinstance(feature, FeatureSpec) and isinstance(feature.props().get(key), str):
            labels[label] = feature.props()[key]
    return labels


def _layer_position(index: object, count: int) -> int:
    """Index ``index`` into ``count`` layers (negative from the end), or raise."""
    from ggrust.errors import PlotIndexError

    if isinstance(index, bool) or not isinstance(index, int):
        message = (
            f"layer index must be an int, got {type(index).__name__} {index!r}; "
            "pass the position in plot.layers, e.g. layer_data(0)"
        )
        raise TypeError(message)
    if not -count <= index < count:
        valid = f"0..{count - 1} (or -{count}..-1)" if count else "none: the plot has no layers"
        message = (
            f"layer index {index} is out of range: the plot has {count} "
            f"layer{'s' if count != 1 else ''}; valid indices: {valid}"
        )
        raise PlotIndexError(message, index=index, length=count)
    return index % count


def _filter_none(original: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in original.items() if value is not None}


def _specs_to_dict(raw_options: dict[str, Any]) -> dict[str, Any]:
    options: dict[str, Any] = {}
    for key, value in raw_options.items():
        if isinstance(value, FeatureSpec):
            options[key] = value.as_dict()
        elif isinstance(value, dict):
            options[key] = _specs_to_dict(value)
        else:
            options[key] = value
    return _filter_none(options)


class FeatureSpec:
    """Base class of plot objects. Build them with ``ggplot()``, ``geom_*()``, ...."""

    def __init__(self, kind: str, name: str | None, **kwargs: object) -> None:
        self.kind = kind
        self.__props: dict[str, Any] = {}
        if name is not None:
            self.__props["name"] = name
        self.__props.update(**kwargs)

    def props(self) -> dict[str, Any]:
        """Return the raw properties (mutable)."""
        return self.__props

    def as_dict(self) -> dict[str, Any]:
        """Return all properties, with ``as_dict()`` applied to nested specs.

        Returns
        -------
        dict
            Properties; ``None`` values are dropped and data frames are kept as is.
        """
        return _specs_to_dict(self.props())

    def __str__(self) -> str:
        import json

        # A secondary axis transform is a callable: shown by its repr.
        return json.dumps(self.as_dict(), indent=2, default=repr)

    def __add__(self, other: object) -> FeatureSpec:
        """Return ``self + other``, or ``NotImplemented`` so ``other.__radd__`` runs.

        Python raises ``TypeError`` when neither side handles the operands.
        """
        if isinstance(other, DummySpec):
            return self
        if (
            self.kind not in ["plot", "subplots"]
            and isinstance(other, FeatureSpec)
            and other.kind not in ["plot", "subplots"]
        ):
            return FeatureSpecArray(self, other)
        return NotImplemented


class PlotSpec(FeatureSpec):
    """A plot: data, mapping, layers, scales and features. Build it with ``ggplot()``.

    ``+`` returns a new plot and never changes this one. Read the parts back with the
    properties ``data`` (the same object), ``mapping``, ``layers`` (``LayerSpec``
    objects with ``geom``, ``stat``, ``data``, ``mapping`` and ``params``),
    ``scales``, ``theme``, ``labels``, ``size``, ``facet``, ``coord`` and ``guides``;
    they return plain dicts and tuples. ``with_data(frame)`` swaps the data.
    ``layer_data(i)`` and ``build()`` run the engine and return what it computed
    (as ggplot2's ``layer_data`` and ``ggplot_build``).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + geom_smooth(method="lm") + ggtitle("Heavier cars use more fuel")
    >>> spec = plot.as_dict()
    >>> [layer.geom for layer in plot.layers], plot.labels["title"]
    (['point', 'smooth'], 'Heavier cars use more fuel')
    >>> fitted = plot.layer_data(1)  # the smooth's x, y, ymin, ymax, se
    """

    @classmethod
    def duplicate(cls, other: PlotSpec) -> PlotSpec:
        """Return a shallow copy of ``other``.

        Parameters
        ----------
        other : PlotSpec
            The spec to copy.
        """
        duplicate = PlotSpec(
            data=None,
            mapping=None,
            scales=other.__scales,
            layers=other.__layers,
            metainfo_list=other.__metainfo_list,
        )
        duplicate.props().update(other.props())
        duplicate.__scale_groups = dict(other.__scale_groups)
        return duplicate

    def __init__(
        self,
        data: PlotData | None,
        mapping: FeatureSpec | None,
        scales: Iterable[FeatureSpec],
        layers: Iterable[FeatureSpec],
        metainfo_list: Iterable[FeatureSpec] = (),
        **kwargs: object,
    ) -> None:
        super().__init__("plot", name=None, data=data, mapping=mapping, **kwargs)
        self.__scales = list(scales)
        self.__layers = list(layers)
        self.__metainfo_list = list(metainfo_list)
        # ggnewscale: how many ``new_scale()`` markers per aesthetic were added.
        self.__scale_groups: dict[str, int] = {}

    def get_plot_shared_data(self) -> PlotData | None:
        """Return the data shared by all layers (lets_plot name of ``data``)."""
        return self.props()["data"]

    @property
    def data(self) -> PlotData | None:
        """The data given to ``ggplot()``: the same object, not a copy.

        Examples
        --------
        >>> import polars as pl
        >>> from ggrust import *
        >>> frame = pl.DataFrame({"weight": [2.6, 2.9, 3.2], "mpg": [31, 28, 24]})
        >>> plot = ggplot(frame, aes("weight", "mpg")) + geom_point()
        >>> plot.data is frame
        True
        """
        return self.props().get("data")

    @property
    def mapping(self) -> dict[str, Any]:
        """The plot's aesthetic mapping, as ``{aesthetic: column}`` (a new dict).

        Examples
        --------
        >>> from ggrust import *
        >>> plot = ggplot(None, aes("weight", "mpg", color="origin"))
        >>> plot.mapping
        {'x': 'weight', 'y': 'mpg', 'color': 'origin'}
        """
        return copy.deepcopy(_mapping_dict(self.props().get("mapping")))

    @property
    def layers(self) -> tuple[LayerSpec, ...]:
        """The layers in ``+`` order.

        ``geom_boxplot()`` adds two layers (the boxes, then the outlier points), as in
        ``as_dict()``, so indices match ``layer_data(i)`` and spec error paths
        (``layers[1]``).

        Examples
        --------
        >>> from ggrust import *
        >>> plot = ggplot() + geom_point() + geom_line()
        >>> [layer.geom for layer in plot.layers]
        ['point', 'line']
        """
        return tuple(cast("list[LayerSpec]", self.__layers))

    @property
    def scales(self) -> tuple[dict[str, Any], ...]:
        """The ``scale_*()`` calls in ``+`` order, each as its ``as_dict()``.

        Examples
        --------
        >>> from ggrust import *
        >>> plot = ggplot() + scale_x_log10()
        >>> plot.scales[0]["aesthetic"], plot.scales[0]["trans"]
        ('x', 'log10')
        """
        return tuple(copy.deepcopy(scale.as_dict()) for scale in self.__scales)

    @property
    def theme(self) -> dict[str, Any]:
        """The theme options added so far, merged as ``+`` merges them (a new dict).

        Only what was added to this plot: defaults of the named theme are not filled
        in.

        Examples
        --------
        >>> from ggrust import *
        >>> plot = ggplot() + theme_bw() + theme(legend_position="bottom")
        >>> plot.theme
        {'name': 'bw', 'legend_position': 'bottom'}
        """
        return copy.deepcopy(_specs_to_dict(dict(self.props().get("theme", {}))))

    @property
    def labels(self) -> dict[str, str]:
        """Title, subtitle, caption, tag and axis or legend titles from ``labs()``.

        Aesthetic titles are keyed as in the ``guides`` spec (``"color:1"`` after
        ``new_scale_color()``). A scale's ``name=`` is not included.

        Examples
        --------
        >>> from ggrust import *
        >>> plot = ggplot() + labs(title="Fuel use", x="Weight", color="Origin")
        >>> plot.labels
        {'title': 'Fuel use', 'x': 'Weight', 'color': 'Origin'}
        """
        labels = _text_labels(self.props())
        for key, guide in self.guides.items():
            if isinstance(guide, dict) and isinstance(guide.get("title"), str):
                labels[key] = guide["title"]
        return labels

    @property
    def size(self) -> tuple[Number, Number] | None:
        """``(width, height)`` from ``ggsize()``, or None for the default size.

        Examples
        --------
        >>> from ggrust import *
        >>> (ggplot() + ggsize(700, 300)).size
        (700, 300)
        """
        return _size_of(self.props())

    @property
    def facet(self) -> dict[str, Any] | None:
        """The ``facet_*()`` spec as a dict, or None.

        Examples
        --------
        >>> from ggrust import *
        >>> (ggplot() + facet_wrap("origin", ncol=2)).facet["ncol"]
        2
        """
        return copy.deepcopy(_feature_dict(self.props().get("facet")))

    @property
    def coord(self) -> dict[str, Any] | None:
        """The ``coord_*()`` spec as a dict, or None.

        Examples
        --------
        >>> from ggrust import *
        >>> (ggplot() + coord_flip()).coord
        {'name': 'flip', 'flip': True}
        """
        return copy.deepcopy(_feature_dict(self.props().get("coord")))

    @property
    def guides(self) -> dict[str, Any]:
        """The merged ``guides()`` and ``labs()`` titles by aesthetic (a new dict).

        Examples
        --------
        >>> from ggrust import *
        >>> (ggplot() + guides(color="none")).guides
        {'color': 'none'}
        """
        return copy.deepcopy(_specs_to_dict(dict(self.props().get("guides", {}))))

    def with_data(self, data: PlotData | None) -> PlotSpec:
        """Return a copy of the plot with new shared data (ggplot2's ``p %+% data``).

        Layers, scales and features are kept; this plot is not changed.

        Parameters
        ----------
        data : DataFrame or dict, optional
            The new data for ``ggplot()``; None removes it. A frame is kept as the
            same object; a dict is annotated as ``ggplot()`` does (``data_meta``).

        Returns
        -------
        PlotSpec
            The new plot.

        Examples
        --------
        >>> import polars as pl
        >>> from ggrust import *
        >>> cars = pl.DataFrame({"weight": [2.6, 2.9, 3.2], "mpg": [31, 28, 24]})
        >>> trucks = pl.DataFrame({"weight": [4.5, 5.0, 5.6], "mpg": [17, 15, 13]})
        >>> plot = ggplot(cars, aes("weight", "mpg")) + geom_point()
        >>> truck_plot = plot.with_data(trucks)
        >>> plot.data is cars, truck_plot.data is trucks
        (True, True)
        """
        from ggrust.plot.util import as_annotated_data, key_int2str

        plot = PlotSpec.duplicate(self)
        # Column types, time zones and Enum levels follow the new data, as ggplot().
        data, mapping, data_meta = as_annotated_data(key_int2str(data), self.props().get("mapping"))
        plot.props()["data"] = data
        plot.props()["mapping"] = mapping
        plot.props().update(data_meta)
        return plot

    @property
    def columns(self) -> tuple[str, ...]:
        """Every data column the plot reads, sorted, tooltip variables included.

        Columns named by mappings (plot and layers), facets, ``as_discrete(order_by=)``
        and ``layer_tooltips()``. Reads the spec only: a name missing from the data is
        listed too, and is reported when the plot is built.

        Raises
        ------
        SpecError
            If the spec is invalid.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"mpg": [31, 22], "hp": [65, 110], "origin": ["EU", "US"], "year": [1, 2]}
        >>> plot = ggplot(data, aes("mpg", "hp")) + geom_point(aes(color="origin"))
        >>> plot.columns
        ('hp', 'mpg', 'origin')
        """
        from ggrust._render import plot_columns

        return plot_columns(self)

    def validate(self) -> list[str]:
        """Build the plot without drawing it and return its warnings.

        Runs the engine as rendering does: everything that would stop the plot raises
        here, and every message rendering would emit as ``GgrustWarning`` (ignored
        options, removed rows, ...) is returned instead of emitted.

        Returns
        -------
        list of str
            The warnings, empty for a plot with nothing to report.

        Raises
        ------
        SpecError, DataError
            If the plot cannot be built, as when rendering it.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"mpg": [31, 22], "hp": [65, 110]}
        >>> (ggplot(data, aes("mpg", "hp")) + geom_point()).validate()
        []
        """
        from ggrust._render import plot_warnings

        return plot_warnings(self)

    def layer_data(self, index: int = 0) -> pl.DataFrame:
        """Return a layer's data after its stat and position (ggplot2's ``layer_data``).

        Runs the engine's stats, positions and scales (no drawing). Columns: ``panel``
        and ``group`` (0-based), then the layer's aesthetics (``x``, ``ymin``,
        ``color``, ...) and stat variables (``count``, ``density``, ...; a stat
        variable whose name an aesthetic uses keeps its dots, ``..height..``).
        Positions are in transformed units (``log10`` of the data on a log axis), and
        categories on a discrete axis are 0, 1, 2, ... (see ``build()`` for the
        levels). Style aesthetics hold the data values, not colors or sizes. Rows are
        the engine's: a stat that summarises a group carries other mapped columns as
        the group's mean or first value (a boxplot's ``y``), the boxplot outlier layer
        has a NaN row for a box without outliers, and dates and times are epoch
        milliseconds.

        Parameters
        ----------
        index : int, default=0
            The layer, as in ``layers``; negative values count from the end.

        Returns
        -------
        polars.DataFrame
            One row per drawn element (a bin, a box, a point).

        Raises
        ------
        PlotIndexError
            If ``index`` is not a layer of the plot.
        SpecError, DataError
            If the plot cannot be built, as when rendering it.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"mpg": [31, 28, 24, 22, 19, 15, 18, 21]}
        >>> plot = ggplot(data, aes("mpg")) + geom_histogram(bins=3)
        >>> bins = plot.layer_data(0)
        >>> bins["count"].to_list()
        [3.0, 3.0, 2.0]
        """
        position = _layer_position(index, len(self.__layers))
        return self.build()["layers"][position]

    def build(self) -> PlotBuild:
        """Run the engine on the plot without drawing it (ggplot2's ``ggplot_build``).

        Returns
        -------
        PlotBuild
            ``{"layers": [...], "panels": [...]}``: every layer's data as
            ``layer_data(i)`` returns it, and per panel (one without facets, in panel
            order otherwise) the trained ``x`` and ``y`` scales: ``discrete``,
            ``domain`` (the range the panel spans, in transformed units), and the
            ``breaks`` and ``labels`` drawn at the plot's size. ``x`` is the data x,
            also under ``coord_flip()``.

        Raises
        ------
        SpecError, DataError
            If the plot cannot be built, as when rendering it.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"origin": ["EU", "US", "US", "JP"], "mpg": [31, 22, 19, 28]}
        >>> built = (ggplot(data, aes("origin", "mpg")) + geom_point()).build()
        >>> built["panels"][0]["x"]["labels"]
        ['EU', 'US', 'JP']
        """
        from ggrust._render import build_plot

        return build_plot(self)

    def guide_data(self, aesthetic: str = "color", scale_group: int = 0) -> pl.DataFrame:
        """Return the keys of a legend or colorbar (ggplot2's ``get_guide_data``).

        Runs the engine without drawing. One row per key, in legend order, or per
        break of a colorbar. With a scale ``na_value`` and missing data, the last key
        is labelled ``NA``.

        Parameters
        ----------
        aesthetic : str, default="color"
            The style aesthetic the guide shows: ``"color"`` (or ``"colour"``),
            ``"fill"``, ``"size"``, ``"shape"``, ...
        scale_group : int, default=0
            Which scale of the aesthetic after ``new_scale_color()`` and friends:
            how many ``new_scale_*()`` of the aesthetic came before it.

        Returns
        -------
        polars.DataFrame
            Two columns: one named after the aesthetic with the mapped values (hex
            colors, sizes, shape codes, dash patterns), and ``label`` with the text
            drawn next to the key.

        Raises
        ------
        GuideNotFoundError
            If the plot shows no guide for ``aesthetic``: it is not mapped, its guide
            is hidden, or the scale has more keys than a legend can show.
        SpecError, DataError
            If the plot cannot be built, as when rendering it.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"mpg": [31, 22, 19], "origin": ["EU", "US", "US"]}
        >>> plot = ggplot(data, aes("mpg", "mpg", color="origin")) + geom_point()
        >>> plot.guide_data("color")["label"].to_list()
        ['EU', 'US']
        """
        import polars as pl

        from ggrust.errors import GuideNotFoundError, did_you_mean

        name = "color" if aesthetic in ("colour", "col") else aesthetic
        guides = self.build()["guides"]
        for guide in guides:
            if guide["aesthetic"] == name and guide["group"] == scale_group:
                return pl.DataFrame({name: guide["values"], "label": guide["labels"]})
        available = sorted({guide["aesthetic"] for guide in guides})
        shown = ", ".join(repr(known) for known in available) or "none"
        where = f"aesthetic {aesthetic!r}" + (
            f" in scale group {scale_group}" if scale_group else ""
        )
        message = (
            f"guide_data: the plot shows no guide for {where}{did_you_mean(name, available)}; "
            f"guides shown: {shown}. "
            "Map the aesthetic to a column and check it is not hidden with guides(...='none')."
        )
        raise GuideNotFoundError(message, aesthetic=aesthetic, available=available)

    def scale_info(self, aesthetic: Literal["x", "y"] = "x") -> list[AxisScale]:
        """Return a position scale after the build, one entry per panel.

        As ggplot2's ``layer_scales``. Runs the engine without drawing. Units are the
        scale's transformed units (``log10`` of the data on a log axis; 0, 1, 2, ...
        on a discrete axis). ``x`` is the data x, also under ``coord_flip()``.

        Parameters
        ----------
        aesthetic : {"x", "y"}, default="x"
            The position scale.

        Returns
        -------
        list of AxisScale
            Per panel, in panel order (one without facets): ``discrete``, ``limits``
            (trained range after ``limits=``, before expansion), ``levels`` (the
            categories, empty on a continuous axis), ``domain`` (the range the panel
            spans), and the ``breaks`` and ``labels`` drawn at the plot's size.

        Raises
        ------
        SpecError
            If ``aesthetic`` is not ``"x"`` or ``"y"``.
        SpecError, DataError
            If the plot cannot be built, as when rendering it.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"mpg": [20.0, 30.0], "origin": ["EU", "US"]}
        >>> plot = ggplot(data, aes("origin", "mpg")) + geom_point()
        >>> plot.scale_info("x")[0]["levels"]
        ['EU', 'US']
        >>> plot.scale_info("y")[0]["limits"]
        (20.0, 30.0)
        """
        from ggrust.errors import SpecError, did_you_mean

        if aesthetic not in ("x", "y"):
            hint = did_you_mean(str(aesthetic), ("x", "y"))
            message = (
                f"scale_info: unsupported aesthetic {aesthetic!r}{hint}; "
                "expected 'x' or 'y'. For a legend or colorbar use guide_data()"
            )
            raise SpecError(message, path="scale_info.aesthetic")
        return [panel[aesthetic] for panel in self.build()["panels"]]

    def has_layers(self) -> bool:
        """Return whether the plot has at least one layer."""
        return bool(self.__layers)

    def __add__(self, other: object) -> PlotSpec:
        """Return a new plot with ``other`` added, or ``NotImplemented`` for other operands.

        Any object that is not a ``FeatureSpec`` is left to its own ``__radd__(plot)``,
        so third-party layers can extend ``+``. Python raises ``TypeError`` when that
        side does not handle the plot either.
        """
        if isinstance(other, DummySpec):
            return self
        # lets_plot: plot + plot is not rejected; it is stored under "plot".
        if isinstance(other, FeatureSpec):
            plot = PlotSpec.duplicate(self)
            if other.kind == "new_scale":
                aesthetic = other.props()["aesthetic"]
                plot.__scale_groups[aesthetic] = plot.__scale_groups.get(aesthetic, 0) + 1
                return plot
            if other.kind == "layer":
                plot.__layers.append(plot.__tagged_layer(other))
                return plot
            if other.kind == "gghighlight":
                return plot.__highlighted(other)
            if other.kind == "inset":
                plot.props()["insets"] = [*plot.props().get("insets", []), other]
                return plot
            if other.kind == "scale":
                plot.__scales.append(plot.__tagged_scale(other))
                return plot
            if other.kind == "theme":
                new_options = {
                    key: value for key, value in other.props().items() if value is not None
                }
                if "name" in new_options:
                    # A named theme replaces the theme, keeping the flavor.
                    flavor = plot.props().get("theme", {}).get("flavor")
                    if flavor is not None:
                        new_options["flavor"] = flavor
                    plot.props()["theme"] = new_options
                else:
                    old_options = plot.props().get("theme", {})
                    plot.props()["theme"] = _theme_dicts_merge(old_options, new_options)
                return plot
            if other.kind == "ggside":
                # Later ggside() options win key by key; unset ones keep earlier values.
                earlier = plot.props().get("ggside")
                merged = dict(earlier.props()) if isinstance(earlier, FeatureSpec) else {}
                merged.pop("name", None)
                merged.update(
                    {key: value for key, value in other.props().items() if value is not None}
                )
                plot.props()["ggside"] = FeatureSpec("ggside", name=None, **merged)
                return plot
            if other.kind == "metainfo":
                plot.__metainfo_list.append(other)
                return plot
            if isinstance(other, FeatureSpecArray):
                for element in other.elements():
                    added = plot.__add__(element)
                    if isinstance(added, NotImplementedType):
                        return NotImplemented
                    plot = added
                return plot
            if other.kind == "guides":
                existing = plot.props().get("guides", {})
                added = plot.__tagged_guides(other.as_dict())
                plot.props()["guides"] = _merge_dicts_recursively(existing, added)
                return plot
            if other.kind == "mapping":
                from ggrust.plot.util import update_plot_aes_mapping

                update_plot_aes_mapping(plot, other)
                return plot
            if other.kind == "facet" and other.props().get("name") == "null":
                # facet_null(): back to a single panel.
                plot.props().pop("facet", None)
                return plot
            plot.props()[other.kind] = other
            return plot
        return NotImplemented

    def __highlighted(self, highlight: FeatureSpec) -> PlotSpec:
        """Return this plot (already a copy) rewritten by ``+ gghighlight()``."""
        from ggrust.plot.gghighlight import HighlightSpec, highlight_layers

        rewrite = highlight_layers(self.props(), self.__layers, cast("HighlightSpec", highlight))
        self.__layers = rewrite.layers
        self.props()["data"] = rewrite.data
        if rewrite.hide_legends:
            from ggrust.plot.guide import guides

            return self + guides(color="none", fill="none")
        return self

    def __group_of(self, aesthetic: str) -> int:
        """Return the current ggnewscale group of ``aesthetic`` (0: no marker yet)."""
        from ggrust.plot.new_scale import canonical_aesthetic

        return self.__scale_groups.get(canonical_aesthetic(aesthetic), 0)

    def __tagged_layer(self, layer: FeatureSpec) -> FeatureSpec:
        """Return ``layer`` with the current non-zero scale groups as ``scale_group``."""
        groups = {aesthetic: group for aesthetic, group in self.__scale_groups.items() if group}
        if not groups or not isinstance(layer, LayerSpec):
            return layer
        tagged = LayerSpec.duplicate(layer)
        tagged.props()["scale_group"] = groups
        return tagged

    def __tagged_scale(self, scale: FeatureSpec) -> FeatureSpec:
        """Return ``scale`` with its aesthetics' non-zero groups as ``scale_group``."""
        from ggrust.plot.new_scale import canonical_aesthetic

        aesthetics = scale.props().get("aesthetic")
        names = [aesthetics] if isinstance(aesthetics, str) else list(aesthetics or [])
        groups = {
            canonical_aesthetic(name): self.__group_of(name)
            for name in names
            if isinstance(name, str) and self.__group_of(name)
        }
        if not groups:
            return scale
        tagged = FeatureSpec(scale.kind, name=None)
        tagged.props().update(scale.props())
        tagged.props()["scale_group"] = groups
        return tagged

    def __tagged_guides(self, guides: dict[str, Any]) -> dict[str, Any]:
        """Return ``guides`` with keys of non-zero groups as ``"color:1"``."""
        from ggrust.plot.new_scale import canonical_aesthetic

        tagged: dict[str, Any] = {}
        for key, guide in guides.items():
            group = self.__group_of(key)
            tagged[f"{canonical_aesthetic(key)}:{group}" if group else key] = guide
        return tagged

    def as_dict(self) -> dict[str, Any]:
        """Return the plot as a lets_plot-compatible dictionary.

        Returns
        -------
        dict
            Keys as lets_plot's ``PlotSpec.as_dict()``: ``data`` (the frame object
            itself), ``mapping``, ``kind``, ``scales``, ``layers``, ``metainfo_list``, ...
        """
        result = super().as_dict()
        result["kind"] = self.kind
        result["scales"] = [scale.as_dict() for scale in self.__scales]
        result["layers"] = [layer.as_dict() for layer in self.__layers]
        result["metainfo_list"] = [metainfo.as_dict() for metainfo in self.__metainfo_list]
        if "insets" in result:
            result["insets"] = [inset.as_dict() for inset in result["insets"]]
        return result

    def __str__(self) -> str:
        lines = ["plot"]
        lines.extend(f"{key}: {value}" for key, value in self.props().items())
        for title, items in [
            ("scales", self.__scales),
            ("layers", self.__layers),
            ("metainfo_list", self.__metainfo_list),
        ]:
            lines.append(f"{title} [{len(items)}]")
            for item in items:
                lines.append(str(item))
                lines.append("-" * 34)
        lines.append("")
        return "\n".join(lines)

    def _repr_html_(self) -> str:
        from ggrust._render import render_notebook_html

        return render_notebook_html(self)

    def show(self) -> None:
        """Draw the plot: inline in Jupyter/IPython, else in the default browser.

        Outside IPython the interactive HTML page is written to a temporary file
        and opened in the browser (lets_plot prints the spec dict there).

        Examples
        --------
        >>> from ggrust import *
        >>> (ggplot() + geom_point(x=0, y=0)).show()  # doctest: +SKIP
        """
        from ggrust._render import show

        show(self)

    def to_svg(
        self,
        path: Target | None = None,
        w: Number | None = None,
        h: Number | None = None,
        unit: str | None = None,
    ) -> str | None:
        """Render to SVG.

        Parameters
        ----------
        path : str or file-like, optional
            Where to write. When None, the SVG text is returned.
        w, h : float, optional
            Override the plot size (give both).
        unit : {"in", "cm", "mm", "px"}, optional
            Unit of ``w`` and ``h``; inches when None. Physical units become the SVG's
            ``width`` / ``height`` (laid out at 96 px per inch).

        Returns
        -------
        str or None
            The SVG text, the absolute path written to, or None for a file-like target.
        """
        from ggrust._render import export_svg

        return export_svg(self, path, w=w, h=h, unit=unit)

    def to_png(
        self,
        path: Target,
        scale: Number | None = None,
        w: Number | None = None,
        h: Number | None = None,
        unit: str | None = None,
        dpi: Number | None = None,
    ) -> str | None:
        """Render to PNG, sized as lets_plot.

        Parameters
        ----------
        path : str or file-like
            Where to write.
        scale : float, optional
            Pixel scale: 2.0 by default, 1.0 once ``w`` and ``h``, ``dpi`` or ``unit`` is
            given.
        w, h : float, optional
            Output size (give both). In px: that many pixels (times ``scale``). In in / cm
            / mm (inches when ``unit`` is None): laid out at 96 px per inch, rendered at
            ``dpi`` (times ``scale``).
        unit : {"in", "cm", "mm", "px"}, optional
            Unit of ``w`` and ``h``.
        dpi : float, optional
            Resolution: 300 by default for physical sizes. Without ``w`` and ``h`` it
            scales the plot's own size by ``dpi / 96``. Written to the PNG (``pHYs``).

        Returns
        -------
        str or None
            The absolute path written to, or None for a file-like target.

        Raises
        ------
        ValueError
            On an unknown ``unit``, only one of ``w`` / ``h``, a unitless size of 20 or
            more (looks like pixels), or a non-positive ``dpi`` / ``scale``.
        """
        from ggrust._render import export_png

        return export_png(self, path, scale=scale, w=w, h=h, unit=unit, dpi=dpi)

    def to_pdf(
        self,
        path: Target,
        scale: Number | None = None,
        w: Number | None = None,
        h: Number | None = None,
        unit: str | None = None,
        dpi: Number | None = None,
    ) -> str | None:
        """Render to a one-page vector PDF (paths, selectable text in embedded fonts).

        Parameters
        ----------
        path : str or file-like
            Where to write.
        scale, w, h, unit, dpi : optional
            As ``to_png``; the page size is the PNG's pixel size divided by the
            resolution, the same page as lets_plot's (raster) PDF.

        Returns
        -------
        str or None
            The absolute path written to, or None for a file-like target.

        Raises
        ------
        ValueError
            As ``to_png``.
        """
        from ggrust._render import export_pdf

        return export_pdf(self, path, scale=scale, w=w, h=h, unit=unit, dpi=dpi)

    def to_html(self, path: Target | None = None, iframe: bool | None = None) -> str | None:
        """Render to a standalone HTML page (inline SVG + the interactive runtime).

        Parameters
        ----------
        path : str or file-like, optional
            Where to write. When None, the HTML text is returned.
        iframe : bool, optional
            Return an ``<iframe srcdoc>`` sized to the plot instead of a page, as
            lets_plot. Default False.

        Returns
        -------
        str or None
            The HTML text, the absolute path written to, or None for a file-like target.
        """
        from ggrust._render import export_html

        return export_html(self, path, iframe)


class LayerSpec(FeatureSpec):
    """A plot layer. Build it with ``geom_*()`` functions or ``layer()``."""

    __own_features = ("geom", "stat", "mapping", "position")

    @classmethod
    def duplicate(cls, other: LayerSpec) -> LayerSpec:
        """Return a shallow copy of ``other``.

        Parameters
        ----------
        other : LayerSpec
            The spec to copy.
        """
        return LayerSpec(**other.props())

    def __init__(self, **kwargs: object) -> None:
        if kwargs.get("sampling") is not None:
            from ggrust.plot.sampling import resolve_sampling

            kwargs["sampling"] = resolve_sampling(kwargs["sampling"])
        super().__init__("layer", name=None, **kwargs)

    def get_plot_layer_data(self) -> PlotData | None:
        """Return the layer's own data (lets_plot name of ``data``)."""
        return self.props()["data"]

    @property
    def geom(self) -> str | None:
        """The geom name, e.g. ``"point"``.

        Examples
        --------
        >>> from ggrust import *
        >>> geom_histogram().geom
        'histogram'
        """
        return self.props().get("geom")

    @property
    def stat(self) -> str | None:
        """The stat name given to the layer, or None for the geom's default stat.

        Examples
        --------
        >>> from ggrust import *
        >>> geom_point(stat="sum").stat
        'sum'
        """
        stat = self.props().get("stat")
        return stat.props().get("name") if isinstance(stat, FeatureSpec) else stat

    @property
    def data(self) -> PlotData | None:
        """The layer's own data (the same object), or None when it uses the plot's.

        Examples
        --------
        >>> import polars as pl
        >>> from ggrust import *
        >>> frame = pl.DataFrame({"x": [1, 2], "y": [3, 4]})
        >>> geom_point(aes("x", "y"), data=frame).data is frame
        True
        """
        return self.props().get("data")

    @property
    def mapping(self) -> dict[str, Any]:
        """The layer's own aesthetic mapping, as ``{aesthetic: column}`` (a new dict).

        Examples
        --------
        >>> from ggrust import *
        >>> geom_point(aes(color="origin")).mapping
        {'color': 'origin'}
        """
        return copy.deepcopy(_mapping_dict(self.props().get("mapping")))

    @property
    def params(self) -> dict[str, Any]:
        """Every other layer option, as a new dict.

        Constants (``size=3``), stat and geom options, ``position``, ``tooltips``, ...;
        unset options and lets_plot's ``data_meta`` are left out.

        Examples
        --------
        >>> from ggrust import *
        >>> geom_point(size=3, alpha=0.5).params
        {'size': 3, 'alpha': 0.5}
        """
        options = {
            key: value
            for key, value in self.props().items()
            if key not in ("geom", "stat", "data", "mapping", "data_meta")
        }
        return copy.deepcopy(_specs_to_dict(options))

    def __add__(self, other: object) -> FeatureSpec:
        if isinstance(other, DummySpec):
            return self
        if isinstance(other, FeatureSpec) and other.kind in LayerSpec.__own_features:
            duplicate = LayerSpec.duplicate(self)
            duplicate.props()[other.kind] = other
            return duplicate
        return super().__add__(other)


class FeatureSpecArray(FeatureSpec):
    """A flat list of features added together (e.g. ``labs()`` with several labels)."""

    def __init__(self, *features: FeatureSpec) -> None:
        super().__init__("feature-list", name=None)
        self.__elements: list[FeatureSpec] = []
        self._flatten(list(features), self.__elements)

    def __len__(self) -> int:
        return len(self.__elements)

    def __iter__(self) -> Iterator[FeatureSpec]:
        return self.__elements.__iter__()

    def __getitem__(self, item: int) -> FeatureSpec:
        return self.__elements[item]

    def elements(self) -> list[FeatureSpec]:
        """Return the features."""
        return self.__elements

    def as_dict(self) -> dict[str, Any]:
        """Return ``{"feature-list": [{kind: as_dict()}, ...]}``."""
        return {"feature-list": [{element.kind: element.as_dict()} for element in self.__elements]}

    def __add__(self, other: object) -> FeatureSpec:
        if isinstance(other, DummySpec):
            return self
        if isinstance(other, FeatureSpecArray):
            return FeatureSpecArray(*self.__elements, *other.__elements)
        if isinstance(other, FeatureSpec) and other.kind != "plot":
            return FeatureSpecArray(*self.__elements, other)
        return super().__add__(other)

    def _flatten(self, features: Iterable[FeatureSpec], out: list[FeatureSpec]) -> None:
        for feature in features:
            if isinstance(feature, FeatureSpecArray):
                self._flatten(feature.elements(), out)
            else:
                out.append(feature)


class DummySpec(FeatureSpec):
    """A feature that does nothing when added."""

    def __init__(self) -> None:
        super().__init__("dummy", name=None)

    def as_dict(self) -> dict[str, Any]:
        """Return ``{"dummy-feature": True}``."""
        return {"dummy-feature": True}

    def __add__(self, other: object) -> Any:  # noqa: ANN401 (returns ``other`` as is)
        return other


def _merge_dicts_recursively(first: dict[str, Any], second: dict[str, Any]) -> dict[str, Any]:
    merged = first.copy()
    for key, value in second.items():
        if isinstance(value, dict) and isinstance(merged.get(key), dict):
            merged[key] = _merge_dicts_recursively(merged[key], value)
        else:
            merged[key] = value
    return merged


def _theme_dicts_merge(first: dict[str, Any], second: dict[str, Any]) -> dict[str, Any]:
    overlapping = first.keys() & second.keys()
    nested = {
        key: {**first[key], **second[key]}
        for key in overlapping
        if type(first[key]) is dict and type(second[key]) is dict
    }
    return {**first, **second, **nested}
