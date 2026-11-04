"""ggrastr: ``rasterize(layer, dpi=300)``.

SVG and PDF draw a rasterized layer as one embedded PNG per panel, on the pixel grid
of a PNG at that resolution; the rest stays vector. PNG output does not change and
HTML keeps the layer vector (tooltips, zoom). Pixel-exact alignment is tested in
Rust (``render::raster``); here the API, errors and the output structure.
"""

import base64
import io
import re

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import RenderError, SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec, PlotSpec
from PIL import Image

FRAME = {
    "x": [float(value % 17) for value in range(200)],
    "y": [float(value * 7 % 23) for value in range(200)],
    "group": ["a", "b", "c", "d"] * 50,
}


def scatter(layer: FeatureSpec | None = None) -> PlotSpec:
    """A scatter of ``FRAME`` with ``layer`` (default: plain points)."""
    layer = layer if layer is not None else gg.geom_point()
    return gg.ggplot(FRAME, gg.aes("x", "y")) + layer + gg.ggtitle("points")


def png(plot: PlotSpec) -> bytes:
    """``plot`` as PNG bytes."""
    buffer = io.BytesIO()
    plot.to_png(buffer)
    return buffer.getvalue()


def pdf(plot: PlotSpec, scale: float | None = None) -> bytes:
    """``plot`` as PDF bytes."""
    buffer = io.BytesIO()
    plot.to_pdf(buffer, scale=scale)
    return buffer.getvalue()


def images(svg: str) -> list[dict[str, float]]:
    """Position and size of every ``<image>`` of ``svg``."""
    found = []
    for tag in re.findall(r"<image [^>]*>", svg):
        found.append({key: float(re.search(rf' {key}="([^"]+)"', tag)[1]) for key in "xy"})
        found[-1].update(
            {key: float(re.search(rf' {key}="([^"]+)"', tag)[1]) for key in ("width", "height")}
        )
        found[-1]["png"] = base64.b64decode(re.search(r'base64,([^"]+)"', tag)[1])
    return found


def clip_rects(svg: str) -> list[tuple[float, ...]]:
    """The clip rectangles of ``svg`` (x, y, width, height)."""
    pattern = r'<clipPath [^>]*><rect x="([^"]+)" y="([^"]+)" width="([^"]+)" height="([^"]+)"'
    return [tuple(float(value) for value in match) for match in re.findall(pattern, svg)]


def texts(svg: str) -> list[str]:
    """Text contents of ``svg``, in order."""
    return re.findall(r"<text[^>]*>([^<]*)<", svg)


class TestSpec:
    def test_layer_carries_the_dpi(self) -> None:
        layer = gg.geom_point(size=2)
        rasterized = gg.rasterize(layer, dpi=150)
        assert rasterized.as_dict()["rasterize"] == {"dpi": 150.0}
        assert "rasterize" not in layer.as_dict()  # a copy
        assert gg.rasterize(layer).as_dict()["rasterize"] == {"dpi": 300.0}

    def test_multi_layer_geoms_are_rasterized_as_a_whole(self) -> None:
        rasterized = gg.rasterize(gg.geom_boxplot())
        layers = [element["layer"] for element in rasterized.as_dict()["feature-list"]]
        assert len(layers) == 2
        assert all(layer["rasterize"] == {"dpi": 300.0} for layer in layers)

    @pytest.mark.parametrize("dpi", [0, -5, 4801, float("nan"), float("inf")])
    def test_dpi_out_of_range(self, dpi: float) -> None:
        with pytest.raises(SpecError, match=r"at 'dpi'.*\(0, 4800\]") as error:
            gg.rasterize(gg.geom_point(), dpi=dpi)
        assert error.value.path == "dpi"

    @pytest.mark.parametrize("dpi", ["300", True, None])
    def test_dpi_must_be_a_number(self, dpi: object) -> None:
        with pytest.raises(UnsupportedValueError, match="dots per inch"):
            gg.rasterize(gg.geom_point(), dpi=dpi)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "value", [gg.ggplot(), gg.labs(title="t"), gg.geom_point() + gg.labs(x="x"), 3]
    )
    def test_only_layers(self, value: object) -> None:
        with pytest.raises(
            UnsupportedValueError, match="takes a layer such as geom_point"
        ) as error:
            gg.rasterize(value)  # type: ignore[call-overload]
        assert error.value.path == "layer"

    @pytest.mark.parametrize(
        ("rasterize", "message"),
        [
            ({"dpi": "high"}, "expected a number of dots per inch"),
            ({"dpi": 0}, "expected dots per inch in"),
            ({"dpi": 300, "scale": 2}, "unknown rasterize option; accepted: 'dpi'"),
            ({}, "missing"),
            (300, "expected rasterize options"),
        ],
    )
    def test_engine_validates_hand_built_specs(self, rasterize: object, message: str) -> None:
        layer = gg.geom_point()
        layer.props()["rasterize"] = rasterize
        with pytest.raises(SpecError, match=re.escape(message)) as error:
            scatter(layer).to_svg()
        assert "layers[0].rasterize" in error.value.path


class TestOutputs:
    def test_too_many_pixels_is_a_render_error(self) -> None:
        plot = scatter(gg.rasterize(gg.geom_point(), dpi=4800)) + gg.ggsize(2000, 2000)
        with pytest.raises(RenderError, match=r"too large.*rasterize\(dpi=\.\.\.\)"):
            plot.to_svg()

    def test_svg_embeds_the_layer_and_keeps_text_vector(self) -> None:
        vector = scatter().to_svg()
        raster = scatter(gg.rasterize(gg.geom_point())).to_svg()
        assert "<circle" in vector and "<circle" not in raster
        assert texts(raster) == texts(vector)
        assert len(images(raster)) == 1
        # Finer than the screen: resampled smoothly, not pixelated.
        assert "pixelated" not in raster

    def test_svg_image_covers_the_panel_on_the_pixel_grid(self) -> None:
        for dpi in (300, 96, 150):
            svg = scatter(gg.rasterize(gg.geom_point(), dpi=dpi)).to_svg()
            (image,) = images(svg)
            (panel,) = clip_rects(svg)
            scale = dpi / 96
            x, y, width, height = panel
            # Covers the panel, at most one bitmap pixel more on each side.
            assert image["x"] <= x + 0.01 and image["y"] <= y + 0.01
            assert image["x"] + image["width"] >= x + width - 0.01
            assert image["y"] + image["height"] >= y + height - 0.01
            assert x - image["x"] < 1 / scale + 0.01
            # On the pixel grid of the plot's PNG at that scale.
            assert abs(image["x"] * scale - round(image["x"] * scale)) < 0.02
            assert abs(image["y"] * scale - round(image["y"] * scale)) < 0.02
            columns, rows = Image.open(io.BytesIO(image["png"])).size
            assert abs(image["width"] * scale - columns) < 0.02
            assert abs(image["height"] * scale - rows) < 0.02

    def test_facets_get_one_image_per_panel(self) -> None:
        plot = scatter(gg.rasterize(gg.geom_point())) + gg.facet_wrap("group", ncol=2)
        svg = plot.to_svg()
        panels = clip_rects(svg)
        found = images(svg)
        assert len(panels) == 4 and len(found) == 4
        for (x, y, width, _height), image in zip(panels, found, strict=True):
            assert image["x"] <= x + 0.01 < image["x"] + image["width"]
            assert image["y"] <= y + 0.01 < image["y"] + image["height"]
            assert image["x"] + image["width"] - (x + width) < 1

    def test_other_layers_stay_vector(self) -> None:
        plot = scatter(gg.rasterize(gg.geom_point())) + gg.geom_hline(yintercept=5)
        svg = plot.to_svg()
        assert len(images(svg)) == 1
        assert "<path" in svg or "<line" in svg

    def test_png_does_not_change(self) -> None:
        assert png(scatter(gg.rasterize(gg.geom_point()))) == png(scatter())

    def test_html_keeps_the_layer_vector(self) -> None:
        html = scatter(gg.rasterize(gg.geom_point())).to_html()
        assert "<image" not in html
        assert "<circle" in html

    def test_pdf_embeds_one_image_at_the_page_resolution(self) -> None:
        vector = pdf(scatter())
        raster = pdf(scatter(gg.rasterize(gg.geom_point(), dpi=300)))
        assert raster.count(b"/Subtype /Image") == 2  # the colors and their alpha mask
        assert b"/Subtype /Image" not in vector
        # Text is still glyphs of the embedded font.
        assert b"/FontFile2" in raster

    def test_dense_layer_shrinks_vector_files(self) -> None:
        # Rasterizing pays off for dense layers: vector PDF markers cost ~8 bytes
        # each, a bitmap a fixed amount per pixel.
        count = 400_000
        index = np.arange(count)
        frame = pl.DataFrame({"x": index * 0.6180339 % 1, "y": index * 0.4142135 % 1})

        def plot(layer: FeatureSpec) -> PlotSpec:
            return gg.ggplot(frame, gg.aes("x", "y")) + layer

        vector = plot(gg.geom_point(size=0.5))
        raster = plot(gg.rasterize(gg.geom_point(size=0.5), dpi=150))
        assert len(raster.to_svg()) * 10 < len(vector.to_svg())
        assert len(pdf(raster)) * 2 < len(pdf(vector))


def test_pdf_image_has_the_svg_image_size() -> None:
    """At the same inch per layout pixel, SVG and PDF embed bitmaps of one size."""
    plot = scatter(gg.rasterize(gg.geom_point(size=3), dpi=192))
    (image,) = images(plot.to_svg())
    svg_pixels = Image.open(io.BytesIO(image["png"])).convert("RGBA")
    # PDF at scale 1, dpi 96: a layout pixel is 1/96 inch, as in SVG.
    document = pdf(plot, scale=1)
    match = re.search(rb"/Width (\d+)\s*/Height (\d+)", document) or re.search(
        rb"/Height (\d+)\s*/Width (\d+)", document
    )
    assert match is not None
    sizes = {int(match[1]), int(match[2])}
    assert sizes == set(svg_pixels.size)
