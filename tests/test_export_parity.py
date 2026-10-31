"""PNG / PDF / SVG export sizes must match lets-plot: pixel size, resolution (``pHYs``)
and PDF page size for every ``w`` / ``h`` / ``unit`` / ``dpi`` / ``scale`` combination,
and the physical ``width`` / ``height`` of SVG roots.
"""

import io
import re
import warnings

import pytest
from PIL import Image

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

FRAME = {"x": [1.0, 2.0, 3.0], "y": [1.0, 3.0, 2.0]}

OPTIONS = [
    {},
    {"scale": 1},
    {"scale": 3},
    {"scale": 1.5},
    {"dpi": 150},
    {"dpi": 72, "scale": 1},
    {"dpi": 300, "scale": 3},
    {"unit": "in"},
    {"unit": "cm"},
    {"unit": "px"},
    {"unit": "in", "scale": 2},
    {"unit": "in", "dpi": 100},
    {"unit": "px", "dpi": 150},
    {"w": 4, "h": 3, "unit": "in"},
    {"w": 4, "h": 3, "unit": "IN"},
    {"w": 4, "h": 3, "unit": "in", "dpi": 100},
    {"w": 4, "h": 3, "unit": "in", "scale": 1},
    {"w": 4, "h": 3, "unit": "in", "dpi": 100, "scale": 2},
    {"w": 10, "h": 5, "unit": "cm"},
    {"w": 100, "h": 50, "unit": "mm", "dpi": 200},
    {"w": 300, "h": 200, "unit": "px"},
    {"w": 300, "h": 200, "unit": "px", "dpi": 192},
    {"w": 300, "h": 200, "unit": "px", "scale": 1},
    {"w": 300, "h": 200, "unit": "px", "scale": 2, "dpi": 192},
    {"w": 4, "h": 3, "unit": "in", "scale": 2},
    {"w": 4, "h": 3},
    {"w": 4, "h": 3, "dpi": 100},
    # Fractional layout sizes: lets-plot rounds them up before scaling.
    {"w": 5, "h": 4, "unit": "cm"},
    {"w": 3, "h": 2, "unit": "mm"},
    {"w": 100.5, "h": 50.7, "unit": "px"},
]


def _plot(module):
    return (
        module.ggplot(FRAME, module.aes("x", "y")) + module.geom_point() + module.ggsize(400, 300)
    )


def _png(module, options: dict) -> Image.Image:
    buffer = io.BytesIO()
    _plot(module).to_png(buffer, **options)
    return Image.open(io.BytesIO(buffer.getvalue()))


@pytest.mark.parametrize("options", OPTIONS, ids=str)
def test_png_size_and_resolution_match_lets_plot(options):
    expected, actual = _png(lp, options), _png(gg, options)
    assert actual.size == expected.size
    assert actual.info["dpi"] == pytest.approx(expected.info["dpi"])


@pytest.mark.parametrize("scale", [None, 1, 1.5])
def test_fractional_plot_size_matches_lets_plot(scale):
    def size(module) -> tuple[int, int]:
        buffer = io.BytesIO()
        plot = (
            module.ggplot(FRAME, module.aes("x", "y"))
            + module.geom_point()
            + module.ggsize(400.5, 300.5)
        )
        plot.to_png(buffer, scale=scale)
        return Image.open(io.BytesIO(buffer.getvalue())).size

    assert size(gg) == size(lp)


@pytest.mark.parametrize("options", OPTIONS[::3], ids=str)
def test_pdf_page_matches_lets_plot(options, tmp_path):
    lp.ggsave(_plot(lp), "expected.pdf", path=str(tmp_path), **options)
    gg.ggsave(_plot(gg), "actual.pdf", path=str(tmp_path), **options)

    def media_box(name: str) -> list[float]:
        data = (tmp_path / name).read_bytes()
        box = re.search(rb"/MediaBox\s*\[\s*([\d.\s]+)\]", data).group(1)
        return [float(value) for value in box.split()]

    def image_size(name: str) -> tuple[int, int]:
        data = (tmp_path / name).read_bytes()
        width = re.search(rb"/Width (\d+)", data).group(1)
        height = re.search(rb"/Height (\d+)", data).group(1)
        return int(width), int(height)

    assert image_size("actual.pdf") == image_size("expected.pdf")
    # Pillow reads the PNG's rounded pHYs (96.012 dpi): pages agree to 0.1 %.
    assert media_box("actual.pdf") == pytest.approx(media_box("expected.pdf"), rel=1e-3)


def test_pdf_shows_the_plot(tmp_path):
    path = gg.ggsave(_plot(gg), "plot.pdf", path=str(tmp_path), scale=1)
    with open(path, "rb") as file:
        data = file.read()
    assert data.startswith(b"%PDF-") and data.rstrip().endswith(b"%%EOF")
    # The image stream decodes (PNG predictor) to the PNG export's pixels over white.
    import zlib

    stream = re.search(rb"/Length (\d+) >>\nstream\n", data)
    start = stream.end()
    raw = zlib.decompress(data[start : start + int(stream.group(1))])
    png = _png(gg, {"scale": 1}).convert("RGB")
    width, height = png.size
    assert len(raw) == height * (width * 3 + 1)
    # Every scanline starts with a PNG filter type (0..4), as /Predictor 15 reads.
    assert all(raw[row * (width * 3 + 1)] <= 4 for row in range(height))


@pytest.mark.parametrize(
    "options",
    [
        {"w": 4, "h": 3, "unit": "in"},
        {"w": 4.5, "h": 3},
        {"w": 10, "h": 5, "unit": "cm"},
        {"w": 100, "h": 50, "unit": "mm"},
    ],
    ids=str,
)
def test_svg_physical_size_matches_lets_plot(options):
    def root(module) -> dict:
        tag = re.search(r"<svg[^>]*>", _plot(module).to_svg(**options)).group(0)
        attributes = dict(re.findall(r'([\w-]+)="([^"]*)"', tag))
        return {
            "width": attributes["width"],
            "height": attributes["height"],
            "viewBox": [float(value) for value in attributes["viewBox"].split()],
        }

    expected, actual = root(lp), root(gg)
    assert actual["width"] == expected["width"]
    assert actual["height"] == expected["height"]
    assert actual["viewBox"] == pytest.approx(expected["viewBox"], abs=0.01)


def test_ggsave_writes_every_format(tmp_path):
    plot = _plot(gg)
    for extension in ["svg", "png", "pdf", "html", "htm"]:
        path = gg.ggsave(
            plot, f"plot.{extension}", path=str(tmp_path), w=4, h=3, unit="in", dpi=300
        )
        with open(path, "rb") as file:
            head = file.read(4)
        assert head in (b"<svg", b"\x89PNG", b"%PDF", b"<htm", b"<ifr", b"<!DO")
    grid = gg.gggrid([plot, plot])
    assert gg.ggsave(grid, "grid.pdf", path=str(tmp_path)).endswith("grid.pdf")
    with pytest.raises(ValueError, match="'svg', 'png', 'pdf', 'html', 'htm'"):
        gg.ggsave(plot, "plot.jpg", path=str(tmp_path))
