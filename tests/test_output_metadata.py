"""Every output format records ``ggrust vX.Y.Z`` in its usual metadata slot, read back
with standard tools (PIL, an XML parser), and ``ggsave(..., metadata=False)`` leaves
it out without changing the picture.
"""

import html
import re
import xml.etree.ElementTree as ElementTree
from importlib.metadata import version
from pathlib import Path

import ggrust as gg
import pytest
from ggrust.errors import ExportError
from PIL import Image, ImageChops

TAG = f"ggrust v{version('ggrust')}"


def plot() -> gg.PlotSpec:
    frame = {"x": [1.0, 2.0, 3.0], "y": [2.0, 1.0, 3.0]}
    return gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point()


def save(tmp_path: Path, name: str, metadata: bool = True) -> Path:
    return Path(gg.ggsave(plot(), name, path=str(tmp_path), metadata=metadata))


def test_tag_matches_the_installed_version() -> None:
    assert re.fullmatch(r"ggrust v\d+\.\d+\.\d+\S*", TAG)


def image_info(path: Path) -> dict[str, object]:
    with Image.open(path) as image:
        image.load()
        info: dict[str, object] = dict(image.info)
        if image.format == "TIFF":
            info["software"] = image.tag_v2.get(305)  # type: ignore[attr-defined]
        return info


@pytest.mark.parametrize(
    ("name", "key", "expected"),
    [
        ("plot.png", "Software", TAG),
        ("plot.jpg", "comment", TAG.encode()),
        ("plot.tiff", "software", TAG),
    ],
)
def test_raster_formats_carry_the_tag(
    tmp_path: Path, name: str, key: str, expected: object
) -> None:
    assert image_info(save(tmp_path, name)).get(key) == expected
    assert image_info(save(tmp_path, "plain-" + name, metadata=False)).get(key) is None


@pytest.mark.parametrize("name", ["plot.png", "plot.jpg", "plot.tiff"])
def test_tag_does_not_change_pixels(tmp_path: Path, name: str) -> None:
    with (
        Image.open(save(tmp_path, name)) as tagged,
        Image.open(save(tmp_path, "plain-" + name, metadata=False)) as plain,
    ):
        assert (
            ImageChops.difference(tagged.convert("RGBA"), plain.convert("RGBA")).getbbox() is None
        )


def test_pdf_producer(tmp_path: Path) -> None:
    assert f"/Producer ({TAG})".encode() in save(tmp_path, "plot.pdf").read_bytes()
    assert b"/Producer" not in save(tmp_path, "plain.pdf", metadata=False).read_bytes()


def test_svg_comment_keeps_the_document_valid(tmp_path: Path) -> None:
    svg = save(tmp_path, "plot.svg").read_text()
    assert f"<!-- {TAG} -->" in svg
    ElementTree.fromstring(svg)
    assert TAG not in save(tmp_path, "plain.svg", metadata=False).read_text()


def test_svg_with_physical_size_keeps_the_tag(tmp_path: Path) -> None:
    written = gg.ggsave(plot(), "plot.svg", path=str(tmp_path), w=3, h=2, unit="in")
    assert f"<!-- {TAG} -->" in Path(written).read_text()


@pytest.mark.parametrize("iframe", [False, True])
def test_html_generator_meta(tmp_path: Path, iframe: bool) -> None:
    page = Path(gg.ggsave(plot(), "plot.html", path=str(tmp_path), iframe=iframe)).read_text()
    meta = f'<meta name="generator" content="{TAG}">'
    # In the iframe the page is an escaped `srcdoc` attribute.
    assert (html.escape(meta) if iframe else meta) in page
    plain = gg.ggsave(plot(), "plain.html", path=str(tmp_path), iframe=iframe, metadata=False)
    assert TAG not in Path(plain).read_text()


def test_notebook_and_to_methods_are_tagged() -> None:
    assert f"<!-- {TAG} -->" in plot().to_svg()
    assert TAG in plot()._repr_html_()


def test_metadata_false_on_a_plot_file_is_an_error(tmp_path: Path) -> None:
    with pytest.raises(ExportError, match="'metadata' applies to rendered files"):
        save(tmp_path, "plot.ggrs", metadata=False)
