"""Per-guide options: reverse, direction, order, theme; colorbar display, alpha, limits."""

from __future__ import annotations

import re

import pytest
from ggrust import (
    aes,
    element_text,
    geom_point,
    ggplot,
    guide_colorbar,
    guide_legend,
    guides,
    scale_color_gradient,
    theme,
)
from ggrust.errors import SpecError

DATA = {
    "x": [1, 2, 3, 4, 5, 6],
    "y": [2, 4, 3, 5, 6, 1],
    "pet": ["cat", "cat", "dog", "dog", "fox", "fox"],
    "size": ["s", "m", "l", "s", "m", "l"],
}


def svg(*specs: object, color: str = "pet") -> str:
    plot = ggplot(DATA, aes("x", "y", color=color, shape="size")) + geom_point()
    for spec in specs:
        plot = plot + spec
    return plot.to_svg()


def order_of(out: str, *labels: str) -> list[int]:
    return [out.index(f">{label}<") for label in labels]


def test_reverse_lists_keys_backwards() -> None:
    out = svg(guides(color=guide_legend(reverse=True)))
    first, last = order_of(out, "fox", "cat")
    assert first < last


def test_order_puts_legends_first() -> None:
    default = svg()
    assert default.index(">pet<") < default.index(">size<")
    out = svg(guides(size=guide_legend(), shape=guide_legend(order=1)))
    assert out.index(">size<") < out.index(">pet<")


def test_direction_lays_one_legend_in_a_row() -> None:
    out = svg(guides(color=guide_legend(direction="horizontal")))
    y_of = [
        float(y) for y in re.findall(r'<text x="[0-9.]+" y="([0-9.]+)"[^>]*>(?:cat|dog|fox)<', out)
    ]
    assert len(set(y_of)) == 1


def test_theme_styles_only_its_legend() -> None:
    out = svg(guides(color=guide_legend(theme=theme(legend_text=element_text(color="#ff0000")))))
    assert re.search(r'fill="#ff0000"[^>]*>cat<', out)
    assert not re.search(r'fill="#ff0000"[^>]*>s<', out)


def colorbar_svg(**options: object) -> str:
    plot = (
        ggplot(DATA, aes("x", "y", color="y"))
        + geom_point()
        + scale_color_gradient(limits=[1, 6], breaks=[1, 3, 6])
        + guides(color=guide_colorbar(**options))  # type: ignore[arg-type]
    )
    return plot.to_svg()


def test_colorbar_reverse_puts_high_values_at_the_bottom() -> None:
    plain = colorbar_svg()
    reversed_ = colorbar_svg(reverse=True)

    def label_y(out: str, label: str) -> float:
        return float(re.findall(rf'y="([0-9.]+)"[^>]*>{label}<', out)[-1])

    assert label_y(plain, "6") < label_y(plain, "1")
    assert label_y(reversed_, "6") > label_y(reversed_, "1")


def test_colorbar_limits_alpha_and_gradient() -> None:
    ticks = colorbar_svg().count("<path")
    assert colorbar_svg(draw_ulim=False, draw_llim=False).count("<path") == ticks - 4
    assert "fill-opacity" in colorbar_svg(alpha=0.5) or "rgba" in colorbar_svg(alpha=0.5)
    assert colorbar_svg(display="gradient") != colorbar_svg()
    assert colorbar_svg(display="rectangles") == colorbar_svg()


def test_errors() -> None:
    with pytest.raises(SpecError, match="did you mean 'vertical'"):
        guide_legend(direction="verticle")  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="order"):
        guide_legend(order=100)
    with pytest.raises(SpecError, match="legend_\\* options"):
        guide_legend(theme=theme(axis_text=element_text(size=3)))
    with pytest.raises(SpecError, match="complete theme"):
        from ggrust import theme_bw

        guide_legend(theme=theme_bw())
    with pytest.raises(SpecError, match="did you mean 'gradient'"):
        guide_colorbar(display="gradiant")  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="alpha"):
        guide_colorbar(alpha=2)
    with pytest.raises(SpecError, match="draw_ulim"):
        guide_colorbar(draw_ulim="no")  # type: ignore[arg-type]


def test_guides_override_the_scale_guide_even_with_false() -> None:
    from ggrust import scale_color_discrete

    out = svg(
        scale_color_discrete(guide=guide_legend(reverse=True)),
        guides(color=guide_legend(reverse=False)),
    )
    first, last = order_of(out, "cat", "fox")
    assert first < last


def test_zero_bins_fail() -> None:
    with pytest.raises(ValueError, match="nbin"):
        colorbar_svg(nbin=0)
