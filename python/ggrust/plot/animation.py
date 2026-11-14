"""gganimate transitions, added to a plot with ``+``.

A ggrust extension: lets-plot has no animation, ggplot2 needs the gganimate package.
Names and arguments follow R gganimate (MIT). The transition goes into the plot's
``"animation"`` key; ``animate()`` and the notebook display turn it into frames.
"""

from __future__ import annotations

from ggrust.errors import SpecError
from ggrust.plot.core import FeatureSpec

__all__ = ["transition_manual"]


def transition_manual(frames: str, cumulative: bool = False) -> FeatureSpec:
    """Animate by showing the rows of one value of ``frames`` per frame.

    Each frame draws the rows whose ``frames`` column has one value, in the
    column's order: numbers ascending, categories in their level order (a polars
    ``Enum`` or pandas categorical keeps its order, plain strings their order of
    first appearance). Stats and positions (counts, stacks, smooths) run per value.
    Axes and legends cover every frame, so they stay still. Layers whose data has
    no ``frames`` column are drawn in every frame. As gganimate, the animation has
    one frame per value when ``nframes`` is larger, and ``fps`` shrinks to keep
    the requested duration; with more values than ``nframes``, evenly spaced values
    are shown.

    Titles, subtitles and captions can show the frame with ``{current_frame}``
    (also ``{previous_frame}``, ``{next_frame}``, ``{frame}``, ``{nframes}`` and
    ``{progress}``).

    Parameters
    ----------
    frames : str
        The column whose values are the frames.
    cumulative : bool, default=False
        Keep the rows of earlier frames: frame ``k`` draws the values up to ``k``.

    Returns
    -------
    FeatureSpec
        The transition (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If ``frames`` is not a non-empty string or ``cumulative`` is not a bool.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "year": [2021, 2021, 2022, 2022, 2023, 2023],
    ...     "fruit": ["apple", "pear"] * 3,
    ...     "sold": [10, 4, 12, 7, 15, 9],
    ... }
    >>> plot = (
    ...     ggplot(data, aes("fruit", "sold"))
    ...     + geom_col()
    ...     + transition_manual("year")
    ...     + ggtitle("Sales in {current_frame}")
    ... )
    >>> animate(plot).nframes
    3
    """
    if not isinstance(frames, str) or not frames:
        message = (
            f"transition_manual(frames={frames!r}): expected the name of a column, "
            "e.g. transition_manual('year')"
        )
        raise SpecError(message, path="animation.transition.frames")
    if not isinstance(cumulative, bool):
        message = f"transition_manual(cumulative={cumulative!r}): expected True or False"
        raise SpecError(message, path="animation.transition.cumulative")
    transition = {"name": "manual", "frames": frames, "cumulative": cumulative}
    return FeatureSpec("animation", name=None, transition=transition)
