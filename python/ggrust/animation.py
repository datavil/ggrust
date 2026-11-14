"""``animate()``, ``Animation`` and ``anim_save()``: turn a plot with a transition into frames.

A ggrust extension: lets-plot has no animation, ggplot2 needs the gganimate package.
Arguments follow R gganimate's ``animate()`` (MIT) without ``renderer`` and ``device``:
the format comes from the file name or the display. Frames are planned and drawn in
Rust, in parallel. Outputs: GIF, APNG (``.png``) and an HTML player (notebooks).
"""

from __future__ import annotations

import base64
import json
import math
import numbers
import os
from typing import TYPE_CHECKING, Literal, TypedDict, cast

from ggrust.errors import ExportError, PlotIndexError, SpecError
from ggrust.plot.core import FeatureSpec, PlotSpec

if TYPE_CHECKING:
    from ggrust._typing import Target

__all__ = ["Animation", "anim_save", "animate", "last_animation"]

SCREEN_DPI = 96.0
FORMATS: dict[str, Literal["gif", "apng", "html"]] = {
    "gif": "gif",
    "png": "apng",
    "apng": "apng",
    "html": "html",
    "htm": "html",
}


class AnimateOptions(TypedDict):
    """``animate()`` options sent to the engine (None: gganimate's default)."""

    nframes: int | None
    fps: float | None
    duration: float | None
    detail: int | None
    start_pause: int
    end_pause: int
    rewind: bool


class _Info(TypedDict):
    """The engine's plan: output frames, final fps, scene frames, timeline."""

    frames: int
    fps: float
    nframes: int
    timeline: list[int]


_last: list[Animation] = []


class Animation:
    """A planned animation of a plot: frames, speed and size, rendered on demand.

    Made by :func:`animate`. Displays as an HTML player in notebooks, saves as GIF,
    APNG or HTML, and gives single frames as plots for inspection. Immutable.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"year": [2021, 2022, 2023], "sold": [10, 12, 15], "fruit": ["apple"] * 3}
    >>> plot = ggplot(data, aes("fruit", "sold")) + geom_col() + transition_manual("year")
    >>> animation = animate(plot, fps=2)
    >>> animation.nframes, animation.fps
    (3, 0.06)
    """

    __slots__ = ("__dpi", "__info", "__options", "__plot", "__size")

    def __init__(
        self,
        plot: PlotSpec,
        options: AnimateOptions,
        size: tuple[float, float] | None,
        dpi: float | None,
    ) -> None:
        self.__plot = plot
        self.__options = options
        self.__size = size
        self.__dpi = dpi
        self.__info: _Info | None = None

    @property
    def plot(self) -> PlotSpec:
        """The animated plot (with its transition)."""
        return self.__plot

    @property
    def options(self) -> AnimateOptions:
        """The ``animate()`` options, a copy (None: gganimate's default)."""
        return cast("AnimateOptions", dict(self.__options))

    @property
    def nframes(self) -> int:
        """Number of frames the animation shows, pauses and rewind included.

        As gganimate, a transition with fewer frames than ``nframes`` asked for
        (``transition_manual()``: one per value) shows only those.
        """
        return self._info()["frames"]

    @property
    def fps(self) -> float:
        """Frames per second, lowered as gganimate when the transition has fewer frames."""
        return self._info()["fps"]

    def frame(self, index: int) -> PlotSpec:
        """Return frame ``index`` as a still plot (render it with ``to_png()`` etc.).

        Parameters
        ----------
        index : int
            0-based frame number; negative numbers count from the end.

        Returns
        -------
        PlotSpec
            The plot drawing only that frame.

        Raises
        ------
        PlotIndexError
            If ``index`` is not an integer within ``-nframes`` to ``nframes - 1``.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"year": [2021, 2022], "sold": [10, 12], "fruit": ["apple"] * 2}
        >>> plot = ggplot(data, aes("fruit", "sold")) + geom_col() + transition_manual("year")
        >>> svg = animate(plot).frame(-1).to_svg()
        """
        count = self.nframes
        if isinstance(index, bool) or not isinstance(index, numbers.Integral):
            message = f"Animation.frame({index!r}): expected an integer frame number"
            raise PlotIndexError(message, index=index, length=count)
        position = int(index) + count if index < 0 else int(index)
        if not 0 <= position < count:
            message = (
                f"Animation.frame({index}): the animation has {count} frames "
                f"(0 to {count - 1}, or -{count} to -1)"
            )
            raise PlotIndexError(message, index=index, length=count)
        still = self.__plot + FeatureSpec(
            "animation", name=None, animate=self._engine_options(), frame=position
        )
        if self.__size is not None:
            from ggrust.plot.plot import ggsize

            still = still + ggsize(*self.__size)
        return still

    def save(self, filename: str | os.PathLike[str], *, path: str | None = None) -> str:
        """Write the animation to ``filename``; the extension picks the format.

        Parameters
        ----------
        filename : str
            ``.gif``, ``.png`` / ``.apng`` (animated PNG) or ``.html`` / ``.htm`` (a
            page with the player).
        path : str, optional
            Directory to join in front of ``filename``.

        Returns
        -------
        str
            Absolute path of the written file.

        Raises
        ------
        ExportError
            If the extension is missing or not an animation format.
        """
        pathname = os.fspath(filename) if path is None else os.path.join(path, filename)
        extension = os.path.splitext(pathname)[1][1:].lower()
        output = FORMATS.get(extension)
        if output is None:
            from ggrust.errors import did_you_mean

            accepted = ", ".join(f"'.{name}'" for name in FORMATS)
            message = (
                f"anim_save(): cannot write an animation to {os.fspath(filename)!r}"
                f"{did_you_mean(extension, FORMATS)}; accepted extensions: {accepted}"
            )
            raise ExportError(message)
        if output == "html":
            return cast("str", self.to_html(pathname))
        return cast("str", self._write(pathname, output))

    def to_gif(self, path: Target | None = None) -> bytes | str:
        """Return the GIF bytes, or write them to ``path`` and return its absolute path.

        Parameters
        ----------
        path : str or file-like, optional
            Where to write: a file path or a binary file-like object.

        Returns
        -------
        bytes or str
            The GIF when ``path`` is None, else the written path (empty for a file object).
        """
        return self._write(path, "gif")

    def to_apng(self, path: Target | None = None) -> bytes | str:
        """Return the animated PNG bytes, or write them to ``path``.

        Parameters
        ----------
        path : str or file-like, optional
            Where to write: a file path or a binary file-like object.

        Returns
        -------
        bytes or str
            The APNG when ``path`` is None, else the written path (empty for a file object).
        """
        return self._write(path, "apng")

    def to_html(
        self, path: Target | None = None, iframe: bool | None = None, metadata: bool = True
    ) -> str | None:
        """Return a standalone HTML page with the player, or write it to ``path``.

        Parameters
        ----------
        path : str or file-like, optional
            Where to write: a file path or a binary file-like object.
        iframe : bool, optional
            Return an ``<iframe srcdoc>`` sized to the player instead of a page.
        metadata : bool, default=True
            Record ``ggrust vX.Y.Z`` in the page's ``generator`` meta tag.

        Returns
        -------
        str or None
            The page when ``path`` is None, else the written path (None for a file object).
        """
        from ggrust._render import _write, render_animation_bytes

        output = "html_iframe" if iframe else "html_page"
        page = render_animation_bytes(
            self.__plot, output, self._engine_options(), size=self.__size, metadata=metadata
        ).decode()
        return page if path is None else _write(path, page)

    def _repr_html_(self) -> str:
        """Return the notebook display: the player, or a GIF in ``"svg"`` / ``"png"`` mode."""
        from ggrust._display import current_display
        from ggrust._render import render_animation_bytes

        if current_display() == "html":
            return render_animation_bytes(
                self.__plot, "html", self._engine_options(), size=self.__size
            ).decode()
        encoded = base64.b64encode(cast("bytes", self.to_gif())).decode()
        return f'<img src="data:image/gif;base64,{encoded}">'

    def __repr__(self) -> str:
        """Return ``<Animation: 3 frames at 0.3 fps>``."""
        return f"<Animation: {self.nframes} frames at {self.fps:g} fps>"

    def _engine_options(self) -> dict[str, object]:
        return {key: value for key, value in self.__options.items() if value is not None}

    def _info(self) -> _Info:
        if self.__info is None:
            from ggrust._render import render_animation_bytes

            encoded = render_animation_bytes(
                self.__plot, "info", self._engine_options(), size=self.__size
            )
            self.__info = cast("_Info", json.loads(encoded))
        return self.__info

    def _write(self, path: Target | None, output: Literal["gif", "apng"]) -> bytes | str:
        from ggrust._render import _write, render_animation_bytes

        scale = 1.0 if self.__dpi is None else self.__dpi / SCREEN_DPI
        encoded = render_animation_bytes(
            self.__plot,
            output,
            self._engine_options(),
            scale=scale,
            size=self.__size,
            dpi=self.__dpi,
        )
        if path is None:
            return encoded
        return _write(path, encoded) or ""


def animate(
    plot: PlotSpec,
    nframes: int | None = None,
    fps: float | None = None,
    duration: float | None = None,
    detail: int | None = None,
    start_pause: int = 0,
    end_pause: int = 0,
    rewind: bool = False,
    width: float | None = None,
    height: float | None = None,
    dpi: float | None = None,
) -> Animation:
    """Plan the frames of a plot with a transition (gganimate ``animate()``).

    Nothing is drawn yet: the returned :class:`Animation` renders when displayed,
    saved or asked for a frame. Two of ``nframes``, ``fps`` and ``duration`` set
    the third, as gganimate: with ``duration``, a given ``fps`` sets the frame
    count, otherwise the frame count sets ``fps``.

    Parameters
    ----------
    plot : PlotSpec
        A plot with a transition, e.g. ``+ transition_manual("year")``.
    nframes : int, default=100
        Frames to show, pauses and rewind included. A transition with fewer frames
        shows those and lowers ``fps`` so the duration stays.
    fps : float, default=10
        Frames per second.
    duration : float, optional
        Length in seconds.
    detail : int, default=1
        Frames computed per shown frame (for smoother tweening in later
        transitions).
    start_pause : int, default=0
        Frames the first frame is held at the start.
    end_pause : int, default=0
        Frames the last frame is held at the end.
    rewind : bool, default=False
        Play the frames backwards after playing them forwards.
    width, height : float, optional
        Frame size in pixels (both or neither); the plot's ``ggsize`` by default.
    dpi : float, optional
        Resolution of GIF and APNG frames: the pixel size is multiplied by
        ``dpi / 96`` (96 by default, one pixel per layout pixel).

    Returns
    -------
    Animation
        The planned animation; also kept for :func:`anim_save` and
        :func:`last_animation`.

    Raises
    ------
    SpecError
        If ``plot`` has no transition, or an option is out of range.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "year": [2021, 2021, 2022, 2022, 2023, 2023],
    ...     "fruit": ["apple", "pear"] * 3,
    ...     "sold": [10, 4, 12, 7, 15, 9],
    ... }
    >>> plot = ggplot(data, aes("fruit", "sold")) + geom_col() + transition_manual("year")
    >>> animation = animate(plot, fps=1, end_pause=2)
    >>> animation.nframes
    5
    """
    if not isinstance(plot, PlotSpec):
        message = f"animate(): 'plot' must be a PlotSpec (from ggplot()), got {type(plot).__name__}"
        raise SpecError(message, path="animate.plot")
    if "animation" not in plot.props():
        message = (
            "animate(): the plot has no transition to animate; add one, "
            "e.g. + transition_manual('year')"
        )
        raise SpecError(message, path="animation")
    _check_number("nframes", nframes, whole=True, minimum=1)
    _check_number("fps", fps)
    _check_number("duration", duration)
    _check_number("detail", detail, whole=True, minimum=1)
    _check_number("start_pause", start_pause, whole=True, minimum=0)
    _check_number("end_pause", end_pause, whole=True, minimum=0)
    _check_number("width", width)
    _check_number("height", height)
    _check_number("dpi", dpi)
    if not isinstance(rewind, bool):
        message = f"animate(rewind={rewind!r}): expected True or False"
        raise SpecError(message, path="animation.animate.rewind")
    if (width is None) != (height is None):
        message = f"animate(): give both 'width' and 'height', or neither; got {width}, {height}"
        raise SpecError(message, path="animate.width")
    options: AnimateOptions = {
        "nframes": None if nframes is None else int(nframes),
        "fps": None if fps is None else float(fps),
        "duration": None if duration is None else float(duration),
        "detail": None if detail is None else int(detail),
        "start_pause": int(start_pause),
        "end_pause": int(end_pause),
        "rewind": rewind,
    }
    size = None if width is None or height is None else (float(width), float(height))
    animation = Animation(plot, options, size, None if dpi is None else float(dpi))
    _last[:] = [animation]
    return animation


def anim_save(
    filename: str | os.PathLike[str],
    animation: Animation | None = None,
    *,
    path: str | None = None,
) -> str:
    """Save an animation to a file; the extension picks the format (gganimate ``anim_save``).

    Parameters
    ----------
    filename : str
        ``.gif``, ``.png`` / ``.apng`` (animated PNG) or ``.html`` / ``.htm``.
    animation : Animation, optional
        The animation; the last one :func:`animate` made when None.
    path : str, optional
        Directory to join in front of ``filename``.

    Returns
    -------
    str
        Absolute path of the written file.

    Raises
    ------
    ExportError
        If there is no animation yet or the extension is not an animation format.

    Examples
    --------
    >>> from ggrust import *
    >>> import tempfile
    >>> data = {"year": [2021, 2022], "sold": [10, 12], "fruit": ["apple"] * 2}
    >>> plot = ggplot(data, aes("fruit", "sold")) + geom_col() + transition_manual("year")
    >>> written = anim_save("sales.gif", animate(plot), path=tempfile.mkdtemp())
    """
    if animation is None:
        animation = last_animation()
    if not isinstance(animation, Animation):
        message = (
            f"anim_save(): 'animation' must be an Animation (from animate()), "
            f"got {type(animation).__name__}"
        )
        raise ExportError(message)
    return animation.save(filename, path=path)


def last_animation() -> Animation:
    """Return the animation :func:`animate` made last (gganimate ``last_animation``).

    Returns
    -------
    Animation
        The last animation.

    Raises
    ------
    ExportError
        If no animation was made yet in this session.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"year": [2021, 2022, 2023], "sold": [10, 12, 15]}
    >>> plot = ggplot(data, aes("year", "sold")) + geom_col() + transition_manual("year")
    >>> animation = animate(plot)
    >>> last_animation() is animation
    True
    """
    if not _last:
        message = "no animation yet; call animate(plot) first or pass the animation"
        raise ExportError(message)
    return _last[0]


def _check_number(
    name: str,
    value: object,
    *,
    whole: bool = False,
    minimum: float | None = None,
) -> None:
    """Raise ``SpecError`` unless ``value`` is None or a fitting finite number."""
    if value is None:
        return
    path = f"animation.animate.{name}"
    if isinstance(value, bool) or not isinstance(value, numbers.Real):
        message = f"animate({name}={value!r}): expected a number"
        raise SpecError(message, path=path)
    number = float(value)
    if whole:
        if not (math.isfinite(number) and number == int(number) and number >= (minimum or 0)):
            message = f"animate({name}={value!r}): expected a whole number >= {minimum:g}"
            raise SpecError(message, path=path)
    elif not (math.isfinite(number) and number > 0):
        message = f"animate({name}={value!r}): expected a positive number"
        raise SpecError(message, path=path)
