"""Custom fonts: ``register_font`` and ``element_text(family=...)`` in every output.

The registry is process-wide, so each test registers under its own family name.
Liberation Serif (SIL OFL 1.1, ``tests/fonts/``) is the test font.
"""

from __future__ import annotations

import io
import re
import warnings
from pathlib import Path
from typing import Literal

import pytest
from ggrust import (
    GgrustWarning,
    aes,
    element_text,
    geom_point,
    geom_text,
    ggplot,
    ggtitle,
    register_font,
    register_font_dir,
    registered_fonts,
    theme,
)
from ggrust.errors import FontError, FontNotFoundError, GgrustError
from ggrust.plot.core import PlotSpec

FONTS = Path(__file__).parent / "fonts"
REGULAR = FONTS / "LiberationSerif-Regular.ttf"
BOLD = FONTS / "LiberationSerif-Bold.ttf"
DATA = {"weight": [2.6, 2.9, 3.2], "mpg": [31.0, 28.0, 24.0], "car": ["Avanti", "Fiat", "Tavria"]}


def chart(family: str) -> PlotSpec:
    """A scatter plot with a bold title, all text in ``family``."""
    return (
        ggplot(DATA, aes("weight", "mpg"))
        + geom_point()
        + ggtitle("Fuel use")
        + theme(text=element_text(family=family), plot_title=element_text(face="bold"))
    )


def render_bytes(plot: PlotSpec, kind: Literal["png", "pdf"]) -> bytes:
    """``plot`` as PNG or PDF bytes."""
    buffer = io.BytesIO()
    if kind == "png":
        plot.to_png(buffer)
    else:
        plot.to_pdf(buffer)
    return buffer.getvalue()


def font_warnings(plot: PlotSpec) -> list[str]:
    """Messages of the ggrust warnings raised while rendering ``plot`` to SVG."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        plot.to_svg()
    return [str(item.message) for item in caught if issubclass(item.category, GgrustWarning)]


def test_register_font_reads_the_family_name() -> None:
    assert register_font(REGULAR) == ["Liberation Serif"]
    assert register_font(str(BOLD)) == ["Liberation Serif"]
    assert registered_fonts()["Liberation Serif"] == ["plain", "bold"]


def test_family_override_and_case_insensitive_match() -> None:
    assert register_font(REGULAR, family="Override Serif") == ["Override Serif"]
    assert "Override Serif" in registered_fonts()
    messages = font_warnings(chart("override serif"))
    assert not any("is not registered" in message for message in messages), messages


def test_every_output_uses_the_font() -> None:
    register_font(REGULAR, family="Outputs Serif")
    register_font(BOLD, family="Outputs Serif")
    plot = chart("Outputs Serif")
    assert font_warnings(plot) == []
    svg = plot.to_svg()
    css_names = set(re.findall(r"font-family=\"&#39;(ggrust-[0-9a-f]+)&#39;", svg))
    assert len(css_names) == 2  # the plain and the bold face
    for name in css_names:
        assert f"@font-face{{font-family:'{name}';src:url(data:font/ttf;base64," in svg
    assert "@font-face" not in (ggplot(DATA, aes("weight", "mpg")) + geom_point()).to_svg()
    html = plot._repr_html_()
    assert "@font-face" in html
    png = render_bytes(plot, "png")
    assert png != render_bytes(chart("sans"), "png")
    pdf = render_bytes(plot, "pdf")
    assert b"+LiberationSerif" in pdf
    assert b"+LiberationSerif-Bold" in pdf


def test_text_geom_family() -> None:
    register_font(REGULAR, family="Labels Serif")
    plot = ggplot(DATA, aes("weight", "mpg", label="car")) + geom_text(family="Labels Serif")
    svg = plot.to_svg()
    assert "Labels Serif" in svg
    assert "@font-face" in svg
    assert font_warnings(plot) == []


def test_unknown_family_warns_with_a_suggestion() -> None:
    register_font(REGULAR, family="Brand Serif")
    messages = font_warnings(chart("Brand Serf"))
    assert any("'Brand Serf' is not registered" in message for message in messages), messages
    assert any("did you mean 'Brand Serif'?" in message for message in messages), messages
    label_messages = font_warnings(
        ggplot(DATA, aes("weight", "mpg", label="car")) + geom_text(family="Brand Serf")
    )
    assert any("layers[0].family" in message for message in label_messages), label_messages


def test_missing_face_warns() -> None:
    register_font(REGULAR, family="Plain Only Serif")
    messages = font_warnings(chart("Plain Only Serif"))
    assert len(messages) == 1, messages
    assert "has no bold face registered; its plain face was used" in messages[0]


def test_register_font_dir() -> None:
    assert register_font_dir(FONTS) == ["Liberation Serif"]
    with pytest.raises(FontNotFoundError, match="no font files"):
        register_font_dir(Path(__file__).parent / "fixtures")


def test_register_font_dir_skips_unusable_files(tmp_path: Path) -> None:
    (tmp_path / "Serif.ttf").write_bytes(REGULAR.read_bytes())
    (tmp_path / "broken.ttf").write_text("not a font")
    with pytest.warns(GgrustWarning, match=r"broken\.ttf' was skipped: it is not a font file"):
        assert register_font_dir(tmp_path) == ["Liberation Serif"]
    (tmp_path / "Serif.ttf").unlink()
    with pytest.raises(FontError, match="none of its 1 font files could be registered"):
        register_font_dir(tmp_path)


def test_errors(tmp_path: Path) -> None:
    with pytest.raises(FontNotFoundError) as missing:
        register_font(tmp_path / "Arial.ttf")
    assert isinstance(missing.value, FileNotFoundError)
    assert isinstance(missing.value, GgrustError)
    assert missing.value.path == str(tmp_path / "Arial.ttf")

    text_file = tmp_path / "notes.ttf"
    text_file.write_text("not a font")
    with pytest.raises(FontError, match="not a font file") as not_font:
        register_font(text_file)
    assert isinstance(not_font.value, ValueError)
    assert not_font.value.path == str(text_file)

    with pytest.raises(FontError, match="register_font_dir"):
        register_font(tmp_path)
    with pytest.raises(FontError, match="non-empty"):
        register_font(REGULAR, family=" ")
    with pytest.raises(FontNotFoundError, match="no such directory"):
        register_font_dir(tmp_path / "missing")
    with pytest.raises(FontError, match="not a directory"):
        register_font_dir(REGULAR)
