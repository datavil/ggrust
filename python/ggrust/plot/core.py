"""Plot specification objects: ``FeatureSpec``, ``PlotSpec``, ``LayerSpec``, ...

Ported from lets-plot's ``lets_plot/plot/core.py`` (MIT, see THIRD_PARTY_NOTICES.md)
with identical ``as_dict()`` output and ``+`` semantics, so code written for lets_plot
(cellestial) works unchanged. Livemap/geocoding support is not ported. Rendering
(``to_svg``, ``to_png``, ``_repr_html_``) goes through the Rust engine.
"""

from __future__ import annotations

from collections.abc import Iterable, Iterator
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Number, PlotData, Position, Target


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

        return json.dumps(self.as_dict(), indent=2)

    def __add__(self, other: object) -> FeatureSpec:
        if isinstance(other, DummySpec):
            return self
        if (
            self.kind not in ["plot", "subplots"]
            and isinstance(other, FeatureSpec)
            and other.kind not in ["plot", "subplots"]
        ):
            return FeatureSpecArray(self, other)
        message = f"unsupported operand type(s) for +: {self.__class__} and {other.__class__}"
        raise TypeError(message)


class PlotSpec(FeatureSpec):
    """A plot: data, mapping, layers, scales and features. Build it with ``ggplot()``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + geom_smooth(method="lm") + ggtitle("Heavier cars use more fuel")
    >>> spec = plot.as_dict()
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

    def get_plot_shared_data(self) -> PlotData | None:
        """Return the data shared by all layers."""
        return self.props()["data"]

    def has_layers(self) -> bool:
        """Return whether the plot has at least one layer."""
        return bool(self.__layers)

    def __add__(self, other: object) -> PlotSpec:
        if isinstance(other, DummySpec):
            return self
        # lets_plot: plot + plot is not rejected; it is stored under "plot".
        if isinstance(other, FeatureSpec):
            plot = PlotSpec.duplicate(self)
            if other.kind == "layer":
                plot.__layers.append(other)
                return plot
            if other.kind == "scale":
                plot.__scales.append(other)
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
            if other.kind == "metainfo":
                plot.__metainfo_list.append(other)
                return plot
            if isinstance(other, FeatureSpecArray):
                for element in other.elements():
                    plot = plot.__add__(element)
                return plot
            if other.kind == "guides":
                existing = plot.props().get("guides", {})
                plot.props()["guides"] = _merge_dicts_recursively(existing, other.as_dict())
                return plot
            if other.kind == "mapping":
                from ggrust.plot.util import update_plot_aes_mapping

                update_plot_aes_mapping(plot, other)
                return plot
            plot.props()[other.kind] = other
            return plot
        message = f"unsupported operand type(s) for +: {self.__class__} and {other.__class__}"
        raise TypeError(message)

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
        super().__init__("layer", name=None, **kwargs)

    def get_plot_layer_data(self) -> PlotData | None:
        """Return the layer's own data."""
        return self.props()["data"]

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
