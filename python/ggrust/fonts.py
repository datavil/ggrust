"""Custom fonts: register font files, then name their family in ``element_text``.

ggrust draws with Liberation Sans and Liberation Mono (embedded, metric-compatible
with Arial and Courier New) unless you register fonts. A registered font is used
for measuring the layout and for drawing in every output: PNG, PDF (subset
embedded), SVG and HTML (subset embedded as ``@font-face``). Only the files you
register are used, never system fonts, so output is the same on every machine.
ggrust bundles no commercial fonts: register the files you have a license for.

The registry belongs to the Python process: worker processes (multiprocessing,
joblib) register their fonts again.
"""

from __future__ import annotations

import warnings
from typing import TYPE_CHECKING, Literal, cast

from ggrust import _ggrust
from ggrust._render import GgrustWarning

if TYPE_CHECKING:
    from os import PathLike

__all__ = ["register_font", "register_font_dir", "registered_fonts"]

FontFace = Literal["plain", "bold", "italic", "bold_italic"]


def register_font(path: str | PathLike[str], family: str | None = None) -> list[str]:
    """Register a font file so plots can draw with it.

    Every face in the file is registered: a ``.ttf`` or ``.otf`` holds one, a
    ``.ttc``/``.otc`` collection several. The family name and the face (plain,
    bold, italic, bold italic) are read from the font. Register one file per face
    (or use ``register_font_dir``). A later registration of the same family and
    face replaces the earlier one. Names match case-insensitively, and a
    registered family wins over the built-in aliases (``"Arial"``, ``"sans"``, ...),
    which otherwise draw with Liberation Sans.

    Parameters
    ----------
    path : str or os.PathLike
        A TrueType or OpenType font file (``.ttf``, ``.otf``, ``.ttc``, ``.otc``).
    family : str or None, default=None
        Family name to register the faces under, instead of the name in the file.

    Returns
    -------
    list of str
        The family names registered, in file order.

    Raises
    ------
    FontNotFoundError
        If ``path`` does not exist.
    FontError
        If the file is not a usable font or ``family`` is empty.

    Notes
    -----
    A family without the requested face draws its nearest registered face and
    warns. Characters a font lacks draw as its missing-glyph box. SVG and HTML
    embed a subset without ligatures; PNG and PDF are exact.

    Examples
    --------
    >>> from ggrust import *
    >>> register_font("fonts/Arial.ttf")  # doctest: +SKIP
    ['Arial']
    >>> register_font("fonts/Arial Bold.ttf")  # doctest: +SKIP
    ['Arial']
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> arial = theme(text=element_text(family="Arial"))
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + arial  # doctest: +SKIP
    """
    return _ggrust.register_font(path, family)


def register_font_dir(path: str | PathLike[str]) -> list[str]:
    """Register every font file under a directory, searched recursively.

    Files ending in ``.ttf``, ``.otf``, ``.ttc`` or ``.otc`` (any case) are
    registered in sorted path order with their own family names, as
    ``register_font`` does. Files that are not usable fonts are skipped with a
    ``GgrustWarning`` each.

    Parameters
    ----------
    path : str or os.PathLike
        The directory, for example an unpacked font family.

    Returns
    -------
    list of str
        The family names registered, each once, in order.

    Raises
    ------
    FontNotFoundError
        If the directory does not exist or holds no font files.
    FontError
        If ``path`` is not a directory or none of its font files is usable.

    Examples
    --------
    >>> from ggrust import register_font_dir
    >>> from ggrust.errors import FontNotFoundError
    >>> register_font_dir("fonts/Source Serif")  # doctest: +SKIP
    ['Source Serif 4']
    >>> try:
    ...     register_font_dir("missing/fonts")
    ... except FontNotFoundError:
    ...     print("no such directory")
    no such directory
    """
    families, skipped = _ggrust.register_font_dir(path)
    for file, reason in skipped:
        warnings.warn(
            f"font file '{file}' was skipped: {reason}",
            GgrustWarning,
            stacklevel=2,
        )
    return families


def registered_fonts() -> dict[str, list[FontFace]]:
    """Return the registered font families and their faces.

    Returns
    -------
    dict of str to list of str
        Family name to its faces (``"plain"``, ``"bold"``, ``"italic"``,
        ``"bold_italic"``), in registration order.

    Examples
    --------
    >>> from ggrust import registered_fonts
    >>> fonts = registered_fonts()
    >>> isinstance(fonts, dict)
    True
    """
    return {family: cast("list[FontFace]", faces) for family, faces in _ggrust.registered_fonts()}
