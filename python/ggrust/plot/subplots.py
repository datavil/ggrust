"""``SupPlotsSpec``: a figure made of several plots (``gggrid``).

Ported from lets-plot's ``lets_plot/plot/subplots.py`` (MIT, see THIRD_PARTY_NOTICES.md)
with identical ``as_dict()`` output and ``+`` semantics. The engine lays grids out
(``crates/ggrust-core/src/grid.rs``).
"""

from __future__ import annotations

from ggrust.plot.core import (
    DummySpec,
    FeatureSpec,
    FeatureSpecArray,
    _specs_to_dict,
    _theme_dicts_merge,
)

__all__ = ["SupPlotsSpec"]


class SupPlotsLayoutSpec:
    """Layout of a ``SupPlotsSpec`` (``name="grid"`` plus its options)."""

    def __init__(self, name: str, **kwargs):
        self.__props = dict(kwargs)
        self.__props["name"] = name

    def as_dict(self) -> dict:
        """Return the layout options without ``None`` values."""
        return _specs_to_dict(self.__props)


class SupPlotsSpec(FeatureSpec):
    """Several plots in one figure. Build it with ``gggrid()``."""

    @classmethod
    def duplicate(cls, other: SupPlotsSpec) -> SupPlotsSpec:
        """Return a shallow copy of ``other``."""
        duplicate = SupPlotsSpec(figures=other.__figures, layout=other.__layout)
        duplicate.props().update(other.props())
        return duplicate

    def __init__(self, figures: list, layout: SupPlotsLayoutSpec):
        super().__init__("subplots", name=None)
        self.__figures = list(figures)
        self.__layout = layout

    def __add__(self, other):
        if isinstance(other, DummySpec):
            return self
        if isinstance(other, FeatureSpecArray):
            supplots = SupPlotsSpec.duplicate(self)
            for element in other.elements():
                supplots = supplots.__add__(element)
            return supplots
        if isinstance(other, FeatureSpec) and other.kind in [
            "ggsize",
            "theme",
            "ggtitle",
            "caption",
            "tag",
            "ggtoolbar",
        ]:
            supplots = SupPlotsSpec.duplicate(self)
            if other.kind == "theme":
                new_options = {
                    key: value for key, value in other.props().items() if value is not None
                }
                if "name" in new_options:
                    # A named theme replaces the theme, keeping the flavor.
                    flavor = supplots.props().get("theme", {}).get("flavor")
                    if flavor is not None:
                        new_options["flavor"] = flavor
                    supplots.props()["theme"] = new_options
                else:
                    old_options = supplots.props().get("theme", {})
                    supplots.props()["theme"] = _theme_dicts_merge(old_options, new_options)
                return supplots
            supplots.props()[other.kind] = other
            return supplots
        return super().__add__(other)

    def as_dict(self) -> dict:
        """Return the figure as a lets_plot-compatible dictionary.

        Returns
        -------
        dict
            Keys as lets_plot's ``SupPlotsSpec.as_dict()``: ``kind``, ``layout``,
            ``figures`` (each ``as_dict()`` or None for an empty cell), ...
        """
        result = super().as_dict()
        result["kind"] = self.kind
        result["layout"] = self.__layout.as_dict()
        result["figures"] = [
            None if figure is None else figure.as_dict() for figure in self.__figures
        ]
        return result

    def _repr_html_(self) -> str:
        from ggrust._render import render_notebook_html

        return render_notebook_html(self)

    def to_svg(self, path=None, w=None, h=None, unit=None) -> str | None:
        """Render to SVG; parameters as ``PlotSpec.to_svg``."""
        from ggrust._render import export_svg

        return export_svg(self, path, w=w, h=h, unit=unit)

    def to_png(self, path, scale=None, w=None, h=None, unit=None, dpi=None) -> str | None:
        """Render to PNG; parameters as ``PlotSpec.to_png``."""
        from ggrust._render import export_png

        return export_png(self, path, scale=scale, w=w, h=h, unit=unit, dpi=dpi)

    def to_pdf(self, path, scale=None, w=None, h=None, unit=None, dpi=None) -> str | None:
        """Render to PDF; parameters as ``PlotSpec.to_pdf``."""
        from ggrust._render import export_pdf

        return export_pdf(self, path, scale=scale, w=w, h=h, unit=unit, dpi=dpi)

    def to_html(self, path=None, iframe: bool | None = None) -> str | None:
        """Render to standalone HTML; parameters as ``PlotSpec.to_html``."""
        from ggrust._render import export_html

        return export_html(self, path, iframe)
