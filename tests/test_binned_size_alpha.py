"""Binned size and alpha scales and ``guide_bins()`` (ggplot2)."""

from __future__ import annotations

import re
from pathlib import Path

import ggrust as gg
import pytest
from ggrust.errors import SpecError

DATA = {"x": [3.0, 17.0, 22.0, 48.0, 51.0, 77.0, 99.0], "y": [1.0, 5.0, 2.0, 8.0, 3.0, 9.0, 4.0]}


def plot(*parts: gg.FeatureSpec, aesthetic: str = "size", column: str = "x") -> gg.PlotSpec:
    base = gg.ggplot(DATA, gg.aes("x", "y", **{aesthetic: column})) + gg.geom_point()
    for part in parts:
        base += part
    return base


def legend_texts(svg: str) -> list[str]:
    legend = svg[svg.index('class="legend"') :] if 'class="legend"' in svg else svg
    return re.findall(r">([^<>]+)</text>", legend)


def test_sizes_step_by_bin():
    data = plot(gg.scale_size_binned()).guide_data("size")
    # Bins 0-20-40-60-80-100: five sizes growing.
    sizes = data["size"].to_list()
    assert len(sizes) == 5
    assert sizes == sorted(sizes)


def test_alpha_takes_the_value_at_the_bin_middle_like_ggplot2():
    """R: scale_alpha_binned(breaks = c(2.5, 5, 7.5), limits = c(0, 10)) on y."""
    scale = gg.scale_alpha_binned(breaks=[2.5, 5, 7.5], limits=[0, 10])
    alphas = plot(scale, aesthetic="alpha", column="y").guide_data("alpha")["alpha"].to_list()
    assert alphas == pytest.approx([0.2125, 0.4375, 0.6625, 0.8875])


def test_default_guide_labels_the_inner_edges():
    texts = legend_texts(plot(gg.scale_size_binned()).to_svg())
    for edge in ("20", "40", "60", "80"):
        assert edge in texts
    assert "0" not in texts and "100" not in texts
    assert not any(" - " in text for text in texts)


def test_show_limits_labels_the_ends():
    for scale in (
        gg.scale_size_binned(show_limits=True),
        gg.scale_size_binned(guide=gg.guide_bins(show_limits=True)),
    ):
        texts = legend_texts(plot(scale).to_svg())
        assert "0" in texts and "100" in texts


def test_guide_legend_keeps_range_labels():
    texts = legend_texts(plot(gg.scale_size_binned(guide="legend")).to_svg())
    assert "20 - 40" in texts


def test_reverse_lists_high_bins_first():
    svg = plot(gg.scale_size_binned(guide=gg.guide_bins(reverse=True))).to_svg()
    texts = [text for text in legend_texts(svg) if text.isdigit()]
    assert texts == ["80", "60", "40", "20"]


def test_guide_bins_on_a_binned_color_scale_replaces_the_colorbar():
    scale = gg.scale_color_steps(guide="bins")
    svg = plot(scale, aesthetic="color").to_svg()
    assert "20" in legend_texts(svg)
    assert plot(scale, aesthetic="color").guide_data("color").height == 5


def test_guide_bins_needs_a_binned_scale():
    with pytest.raises(SpecError, match="guide_bins\\(\\) needs a binned scale"):
        plot(gg.scale_size(guide=gg.guide_bins())).to_svg()


def test_guide_bins_checks_show_limits():
    with pytest.raises(SpecError, match="show_limits"):
        gg.guide_bins(show_limits="yes")  # ty: ignore[invalid-argument-type]


def test_horizontal_bins_render(tmp_path: Path):
    spec = plot(gg.scale_size_binned(), gg.theme(legend_position="bottom"))
    assert "40" in legend_texts(spec.to_svg())
    gg.ggsave(spec, str(tmp_path / "bins.png"))
    assert (tmp_path / "bins.png").read_bytes()[:4] == b"\x89PNG"


def test_many_bins_fall_back_to_a_legend_that_fits(tmp_path: Path):
    """Bug hunt: a tall guide_bins ran off the canvas; it now reflows as a legend."""
    spec = plot(gg.scale_size_binned(n_breaks=60)) + gg.ggsize(500, 300)
    svg = spec.to_svg()
    assert any(" - " in text or "more" in text for text in legend_texts(svg))


def test_mixed_scales_under_one_title_get_a_plain_legend():
    """Bug hunt: ggplot2 splits them into two guides; ggrust merges by title (lets-plot)."""
    both = gg.ggplot(DATA, gg.aes("x", "y", size="x", alpha="x")) + gg.geom_point()
    same = both + gg.scale_size_binned() + gg.scale_alpha_binned()
    assert "20" in legend_texts(same.to_svg())
    mixed = both + gg.scale_size_binned() + gg.scale_alpha_binned(n_breaks=3)
    assert "0 - 20" in legend_texts(mixed.to_svg())
