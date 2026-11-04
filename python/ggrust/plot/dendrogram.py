"""Hierarchical clustering and dendrograms (R: hclust, ggdendro, legendry).

Not in lets-plot. ``hclust()`` clusters rows (or columns) in Rust with the merges,
heights and leaf order of R's ``hclust``. The tree is drawn by itself with
``geom_dendrogram()`` (as ggdendro), or beside a discrete axis that it also orders
with ``scale_x_dendro()`` / ``scale_y_dendro()`` and ``guide_axis_dendro()`` (as
the R package legendry, the successor of ggh4x's ``guide_dendro``).
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from numbers import Real
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from ggrust._data import as_arrow
from ggrust._ggrust import dendrogram_segments
from ggrust._ggrust import hclust as _hclust
from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec, aes
from ggrust.plot.geom import geom_segment
from ggrust.plot.geom_beeswarm import _check_choice
from ggrust.plot.scale import _scale

if TYPE_CHECKING:
    from ggrust._typing import Breaks, Labels, Number, PlotData
    from ggrust.plot.core import LayerSpec

__all__ = [
    "ClusterTree",
    "geom_dendrogram",
    "guide_axis_dendro",
    "hclust",
    "scale_x_dendro",
    "scale_y_dendro",
]

Linkage: TypeAlias = Literal["complete", "average", "single", "ward.D2"]
"""How cluster dissimilarities are updated after a merge (R ``hclust(method=)``)."""

Distance: TypeAlias = Literal["euclidean", "correlation"]
"""Dissimilarity between two items: straight-line, or one minus Pearson's r."""

DendrogramType: TypeAlias = Literal["rectangle", "triangle"]
"""Elbow links (``"rectangle"``) or straight lines to the children."""

_LINKAGES: tuple[str, ...] = ("complete", "average", "single", "ward.D2")
_DISTANCES: tuple[str, ...] = ("euclidean", "correlation")
_AXES: tuple[str, ...] = ("rows", "columns")
_TYPES: tuple[str, ...] = ("rectangle", "triangle")
_SIDES: tuple[str, ...] = ("axis", "opposite")
_ORIENTATIONS: tuple[str, ...] = ("x", "y")


@dataclass(frozen=True)
class ClusterTree:
    """A clustering tree in R's ``hclust`` layout.

    Build it with :func:`hclust` or :meth:`ClusterTree.from_linkage`.

    Parameters
    ----------
    labels : tuple of str
        Leaf names, in input order.
    merge : tuple of (int, int)
        One pair per merge: ``-k`` is leaf ``k`` (1-based), ``+s`` the cluster made at
        step ``s`` (1-based), as R's ``merge``.
    height : tuple of float
        Height of each merge.
    order : tuple of int
        Leaves (0-based indices into ``labels``) from left to right; R's ``order``
        minus one.
    method : str or None, default=None
        Linkage used, when known.
    distance : str or None, default=None
        Distance used, when known.

    Examples
    --------
    >>> from ggrust import hclust
    >>> tree = hclust({"name": ["a", "b", "c"], "v": [0.0, 1.0, 5.0]}, labels="name")
    >>> tree.ordered_labels
    ('c', 'a', 'b')
    >>> tree.height
    (1.0, 5.0)
    """

    labels: tuple[str, ...]
    merge: tuple[tuple[int, int], ...]
    height: tuple[float, ...]
    order: tuple[int, ...]
    method: str | None = None
    distance: str | None = None

    @property
    def ordered_labels(self) -> tuple[str, ...]:
        """Leaf names from left to right (the axis order of ``scale_*_dendro()``)."""
        return tuple(self.labels[leaf] for leaf in self.order)

    def to_dict(self) -> dict[str, Any]:
        """Return the tree as the plot spec stores it.

        Returns
        -------
        dict
            ``labels``, ``merge`` (list of pairs), ``height`` and ``order``.
        """
        return {
            "labels": list(self.labels),
            "merge": [list(pair) for pair in self.merge],
            "height": list(self.height),
            "order": list(self.order),
        }

    @classmethod
    def from_linkage(
        cls,
        linkage: Sequence[Sequence[float]],
        labels: Sequence[str],
        *,
        order: Sequence[int] | None = None,
    ) -> ClusterTree:
        """Read a SciPy linkage matrix (``scipy.cluster.hierarchy.linkage``).

        Parameters
        ----------
        linkage : array-like of shape (n - 1, 4)
            SciPy linkage rows ``[a, b, height, count]``: indices below ``n`` are
            leaves, ``n + s`` the cluster of row ``s``.
        labels : sequence of str
            Leaf names, one per observation.
        order : sequence of int, optional
            Leaf order (0-based). Default: SciPy's ``dendrogram()`` order (each
            merge's first side before its second, its ``leaves``), so scanpy trees
            keep their ``categories_ordered``.

        Returns
        -------
        ClusterTree
            The tree.

        Raises
        ------
        SpecError
            If ``linkage`` is not a valid ``(n - 1) x 4`` SciPy linkage of ``labels``
            (the error names the row and column), or ``labels`` repeat.

        Examples
        --------
        A scanpy tree (``scanpy.tl.dendrogram``) is reused as it is:

        >>> import ggrust as gg
        >>> def scanpy_tree(adata, group_by):  # doctest: +SKIP
        ...     info = adata.uns[f"dendrogram_{group_by}"]
        ...     labels = adata.obs[group_by].cat.categories
        ...     return gg.ClusterTree.from_linkage(info["linkage"], labels)

        >>> from ggrust import ClusterTree
        >>> tree = ClusterTree.from_linkage([[0, 1, 1.0, 2], [2, 3, 4.0, 3]], ["a", "b", "c"])
        >>> tree.merge
        ((-1, -2), (-3, 1))
        >>> tree.ordered_labels
        ('c', 'a', 'b')
        """
        names = tuple(str(label) for label in labels)
        merge, height = _merge_from_linkage(linkage, names)
        if order is None:
            order = _depth_first(merge, len(names))
        return cls(names, merge, height, tuple(int(leaf) for leaf in order))


def _linkage_error(path: str, reason: str) -> SpecError:
    """Build a ``from_linkage`` error at ``path`` (e.g. ``linkage[2][0]``)."""
    return SpecError(f"ClusterTree.from_linkage() at '{path}': {reason}", path=path, reason=reason)


def _merge_from_linkage(
    linkage: Sequence[Sequence[float]], labels: tuple[str, ...]
) -> tuple[tuple[tuple[int, int], ...], tuple[float, ...]]:
    """Check a SciPy linkage matrix and return R-style ``merge`` and ``height``.

    Raises
    ------
    SpecError
        If the labels repeat or are empty, the matrix is not ``(n - 1) x 4``, an
        index is not a leaf or earlier cluster (or is used twice), a height is
        negative, missing or below a child's height, or a count is wrong.
    """
    n = len(labels)
    if n == 0:
        raise _linkage_error("labels", "a tree needs at least one leaf label")
    seen_labels: set[str] = set()
    for label in labels:
        if label in seen_labels:
            raise _linkage_error(
                "labels", f"label {label!r} appears twice; leaves need distinct names"
            )
        seen_labels.add(label)
    rows = [list(row) for row in linkage]
    if len(rows) != n - 1:
        reason = (
            f"{n} labels need a linkage of {n - 1} rows, got {len(rows)}; pass one label per "
            "observation, in the order the linkage was computed on"
        )
        raise _linkage_error("linkage", reason)
    sizes = [1] * n
    heights = [0.0] * n
    used = [False] * (2 * n - 1)
    merge: list[tuple[int, int]] = []
    height: list[float] = []
    for step, row in enumerate(rows):
        at = f"linkage[{step}]"
        if len(row) != 4:
            reason = f"expected a row [a, b, height, count] of 4 values, got {len(row)}"
            raise _linkage_error(at, reason)
        sides: list[int] = []
        for column in (0, 1):
            value = row[column]
            if (
                isinstance(value, bool)
                or not isinstance(value, Real)
                or not math.isfinite(value)
                or value != int(value)
            ):
                raise _linkage_error(f"{at}[{column}]", f"expected an integer index, got {value!r}")
            index = int(value)
            if not 0 <= index < n + step:
                reason = (
                    f"index {index} is neither a leaf (0..{n - 1}) nor an earlier cluster "
                    f"({n}..{n + step - 1})"
                )
                raise _linkage_error(f"{at}[{column}]", reason)
            if used[index]:
                raise _linkage_error(f"{at}[{column}]", f"index {index} is merged twice")
            used[index] = True
            sides.append(index)
        value = row[2]
        if isinstance(value, bool) or not isinstance(value, Real) or not value >= 0:
            raise _linkage_error(f"{at}[2]", f"expected a height of 0 or more, got {value!r}")
        child_height = max(heights[index] for index in sides)
        if float(value) < child_height:
            reason = (
                f"height {value} is below its child's height {child_height}; heights must not "
                "decrease towards the root (centroid/median linkages can do that)"
            )
            raise _linkage_error(f"{at}[2]", reason)
        count = sizes[sides[0]] + sizes[sides[1]]
        if row[3] != count:
            reason = f"count {row[3]!r} does not match the {count} leaves below this merge"
            raise _linkage_error(f"{at}[3]", reason)
        sizes.append(count)
        heights.append(float(value))
        first, second = (-(index + 1) if index < n else index - n + 1 for index in sides)
        merge.append((first, second))
        height.append(float(value))
    return tuple(merge), tuple(height)


def _depth_first(merge: Sequence[tuple[int, int]], n: int) -> tuple[int, ...]:
    """Leaves of ``merge`` from the root, first side before second."""
    if not merge:
        return tuple(range(n))
    leaves: list[int] = []
    stack = [len(merge)]
    while stack:
        node = stack.pop()
        if node < 0:
            leaves.append(-node - 1)
        else:
            first, second = merge[node - 1]
            stack.extend((second, first))
    return tuple(leaves)


def hclust(
    data: PlotData,
    *,
    labels: str | None = None,
    axis: Literal["rows", "columns"] = "rows",
    method: Linkage = "complete",
    distance: Distance = "euclidean",
) -> ClusterTree:
    """Cluster the rows (or columns) of a table hierarchically, as R's ``hclust``.

    Merges, heights and leaf order match R ``hclust(dist(x), method)`` (or
    ``hclust(as.dist(1 - cor(t(x))), method)`` for correlation), ties included. One
    exception: rows whose correlation is 1 up to rounding can tie differently from
    R, as ``1 - r`` may come out as 0 on one side and 2e-16 on the other.

    Parameters
    ----------
    data : DataFrame or dict
        A wide table: every column but ``labels`` holds numbers, without missing
        values.
    labels : str, optional
        Column naming the rows (the leaves when clustering rows). Default: rows are
        named ``"1"``, ``"2"``, ...
    axis : {"rows", "columns"}, default="rows"
        Cluster the rows, or the numeric columns (leaves named by column).
    method : {"complete", "average", "single", "ward.D2"}, default="complete"
        Linkage.
    distance : {"euclidean", "correlation"}, default="euclidean"
        Dissimilarity: straight-line distance, or ``1 - r`` with Pearson's ``r``.

    Returns
    -------
    ClusterTree
        The tree.

    Raises
    ------
    SpecError
        If ``axis``, ``method`` or ``distance`` is unknown.
    DataError
        If a column is not numeric, a value is missing, a label repeats, or (for
        correlation) an item has no variance.

    Examples
    --------
    >>> from ggrust import hclust
    >>> frame = {"gene": ["CD3E", "CD8A", "MS4A1"], "t": [5.0, 4.5, 0.1], "b": [0.2, 0.1, 6.0]}
    >>> hclust(frame, labels="gene", method="average").ordered_labels
    ('MS4A1', 'CD3E', 'CD8A')
    """
    _check_choice("hclust", "axis", axis, _AXES)
    _check_choice("hclust", "method", method, _LINKAGES)
    _check_choice("hclust", "distance", distance, _DISTANCES)
    if labels is not None and not isinstance(labels, str):
        reason = f"expected a column name, got {type(labels).__name__}"
        raise UnsupportedValueError(f"hclust(labels=...): {reason}", path="labels", reason=reason)
    names, merge, height, order = _hclust(
        as_arrow(data), labels, axis == "columns", method, distance
    )
    return ClusterTree(
        tuple(names),
        tuple((first, second) for first, second in merge),
        tuple(height),
        tuple(order),
        method=method,
        distance=distance,
    )


def _check_tree(function: str, tree: object) -> ClusterTree:
    """Return ``tree`` if it is an :class:`ClusterTree`, else raise."""
    if isinstance(tree, ClusterTree):
        return tree
    reason = f"expected a tree from ggrust.hclust(), got {type(tree).__name__}"
    raise UnsupportedValueError(f"{function}(): {reason}", path="clust", reason=reason)


def geom_dendrogram(
    tree: ClusterTree,
    *,
    type: DendrogramType = "rectangle",
    orientation: Literal["x", "y"] = "x",
    color: str | None = None,
    size: Number | None = None,
    linetype: str | int | None = None,
    alpha: Number | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw a clustering tree as line segments (R ggdendro).

    Leaf ``i`` of the tree's order sits at position ``i`` (0, 1, 2, ...: the places
    of a discrete axis, so the tree lines up with ``scale_*_dendro(tree)``), the
    leaves at height 0 and each node at its merge height.

    Parameters
    ----------
    tree : ClusterTree
        The tree, from :func:`hclust`.
    type : {"rectangle", "triangle"}, default="rectangle"
        Elbow links, or straight lines from each node to its children.
    orientation : {"x", "y"}, default="x"
        Leaves along x (heights on y, root up), or along y (heights on x).
    color, size, linetype, alpha : optional
        Segment style, as ``geom_segment()``.
    **other_args
        Other ``geom_segment()`` options.

    Returns
    -------
    LayerSpec
        A segment layer with its own data (it does not inherit the plot mapping).

    Raises
    ------
    UnsupportedValueError
        If ``tree`` is not an :class:`ClusterTree`.
    SpecError
        If ``type`` or ``orientation`` is unknown.

    Examples
    --------
    >>> from ggrust import *
    >>> frame = {"car": ["a", "b", "c", "d"], "mpg": [21.0, 22.8, 18.7, 14.3]}
    >>> tree = hclust(frame, labels="car")
    >>> plot = ggplot() + geom_dendrogram(tree)
    """
    tree = _check_tree("geom_dendrogram", tree)
    _check_choice("geom_dendrogram", "type", type, _TYPES)
    _check_choice("geom_dendrogram", "orientation", orientation, _ORIENTATIONS)
    position, height, position_end, height_end = dendrogram_segments(
        list(tree.labels),
        [list(pair) for pair in tree.merge],
        list(tree.height),
        list(tree.order),
        type,
    )
    if orientation == "x":
        frame = {"x": position, "y": height, "xend": position_end, "yend": height_end}
    else:
        frame = {"x": height, "y": position, "xend": height_end, "yend": position_end}
    return geom_segment(
        aes(x="x", y="y", xend="xend", yend="yend"),
        data=frame,
        inherit_aes=False,
        color=color,
        size=size,
        linetype=linetype,
        alpha=alpha,
        **other_args,
    )


def guide_axis_dendro(
    *,
    space: Number | None = None,
    side: Literal["axis", "opposite"] | None = None,
    type: DendrogramType | None = None,
    title: str | None = None,
) -> FeatureSpec:
    """Axis guide that draws the scale's tree beside the axis (R legendry).

    Parameters
    ----------
    space : float, default=40
        Pixels from the leaves to the root.
    side : {"axis", "opposite"}, default="axis"
        ``"axis"``: past the tick labels, root outward (legendry). ``"opposite"``:
        across the panel (top for x, right for y), without labels (scanpy).
    type : {"rectangle", "triangle"}, default="rectangle"
        Elbow links, or straight lines.
    title : str, optional
        Axis title (as the scale's ``name``).

    Returns
    -------
    FeatureSpec
        Guide specification for ``scale_*_dendro(guide=...)``.

    Raises
    ------
    SpecError
        If ``side`` or ``type`` is unknown.
    UnsupportedValueError
        If ``space`` is not a number of 0 or more.

    Examples
    --------
    >>> from ggrust import *
    >>> guide = guide_axis_dendro(side="opposite", space=60)
    """
    _check_choice("guide_axis_dendro", "side", side, _SIDES)
    _check_choice("guide_axis_dendro", "type", type, _TYPES)
    if space is not None and (
        isinstance(space, bool) or not isinstance(space, Real) or not space >= 0
    ):
        reason = f"expected pixels (a number of 0 or more), got {space!r}"
        raise UnsupportedValueError(
            f"guide_axis_dendro(space={space!r}): {reason}", path="space", reason=reason
        )
    return FeatureSpec("guide", name="axis_dendro", space=space, side=side, type=type, title=title)


def _dendro_scale(
    aesthetic: Literal["x", "y"],
    clust: ClusterTree,
    name: str | None,
    breaks: Breaks | None,
    labels: Labels | None,
    expand: Sequence[Number] | None,
    reverse: bool | None,
    format: str | None,
    guide: FeatureSpec | str | None,
) -> FeatureSpec:
    """Build the ``scale_{aesthetic}_dendro()`` spec."""
    function = f"scale_{aesthetic}_dendro"
    tree = _check_tree(function, clust)
    if guide is None:
        guide = guide_axis_dendro()
    elif isinstance(guide, str):
        if guide != "axis_dendro":
            reason = f"the guide of {function}() is 'axis_dendro' or guide_axis_dendro()"
            raise SpecError(f"{function}(guide={guide!r}): {reason}", path="guide", reason=reason)
    elif not (isinstance(guide, FeatureSpec) and _guide_name(guide) == "axis_dendro"):
        reason = f"expected guide_axis_dendro(), got {type(guide).__name__}"
        raise UnsupportedValueError(f"{function}(guide=...): {reason}", path="guide", reason=reason)
    return _scale(
        aesthetic,
        name=name,
        breaks=breaks,
        labels=labels,
        expand=expand,
        guide=guide,
        format=format,
        discrete=True,
        reverse=reverse,
        dendrogram=tree.to_dict(),
    )


def _guide_name(guide: FeatureSpec) -> object:
    """Return the ``name`` of a guide spec."""
    properties: Mapping[str, object] = guide.props()
    return properties.get("name")


def scale_x_dendro(
    clust: ClusterTree,
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    expand: Sequence[Number] | None = None,
    reverse: bool | None = None,
    format: str | None = None,
    guide: FeatureSpec | str | None = None,
) -> FeatureSpec:
    """Discrete x scale ordered by a clustering tree, drawn below the axis (legendry).

    The axis levels are the tree's leaves in its order; every value mapped to x
    must be a leaf. The tree is drawn by ``guide`` (default
    ``guide_axis_dendro()``).

    Parameters
    ----------
    clust : ClusterTree
        The tree, from :func:`hclust`.
    name : str, optional
        Axis title.
    breaks, labels, expand, format : optional
        As ``scale_x_discrete()``.
    reverse : bool, default=False
        Reverse the leaf order (the tree flips with it).
    guide : FeatureSpec or "axis_dendro", optional
        ``guide_axis_dendro(...)`` options.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Raises
    ------
    UnsupportedValueError
        If ``clust`` is not an :class:`ClusterTree` or ``guide`` is another guide.

    Examples
    --------
    >>> from ggrust import *
    >>> frame = {"city": ["Oslo", "Rome", "Cairo"], "jan": [-4.0, 8.0, 14.0]}
    >>> tree = hclust(frame, labels="city")
    >>> plot = ggplot(frame, aes("city", "jan")) + geom_bar(stat="identity")
    >>> plot = plot + scale_x_dendro(tree)
    """
    return _dendro_scale("x", clust, name, breaks, labels, expand, reverse, format, guide)


def scale_y_dendro(
    clust: ClusterTree,
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    expand: Sequence[Number] | None = None,
    reverse: bool | None = None,
    format: str | None = None,
    guide: FeatureSpec | str | None = None,
) -> FeatureSpec:
    """Discrete y scale ordered by a clustering tree, drawn left of the axis (legendry).

    The first leaf of the tree's order is at the bottom, as levels of a discrete y
    axis are. See :func:`scale_x_dendro`.

    Parameters
    ----------
    clust : ClusterTree
        The tree, from :func:`hclust`.
    name : str, optional
        Axis title.
    breaks, labels, expand, format : optional
        As ``scale_y_discrete()``.
    reverse : bool, default=False
        Reverse the leaf order (the tree flips with it).
    guide : FeatureSpec or "axis_dendro", optional
        ``guide_axis_dendro(...)`` options.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Raises
    ------
    UnsupportedValueError
        If ``clust`` is not an :class:`ClusterTree` or ``guide`` is another guide.

    Examples
    --------
    >>> from ggrust import *
    >>> frame = {"gene": ["CD3E", "CD8A", "MS4A1"], "t": [5.0, 4.5, 0.1], "b": [0.2, 0.1, 6.0]}
    >>> rows = hclust(frame, labels="gene")
    >>> long = {
    ...     "gene": frame["gene"] * 2,
    ...     "cell": ["t"] * 3 + ["b"] * 3,
    ...     "value": frame["t"] + frame["b"],
    ... }
    >>> plot = ggplot(long, aes("cell", "gene", fill="value")) + geom_tile()
    >>> plot = plot + scale_y_dendro(rows)
    """
    return _dendro_scale("y", clust, name, breaks, labels, expand, reverse, format, guide)
