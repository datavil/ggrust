"""Guide options: what is built, what warns, what is an error."""

import warnings

import ggrust as gg
import pytest

FRAME = {"x": [1.0, 2.0, 3.0], "y": [3.0, 1.0, 2.0], "c": ["a", "b", "a"], "v": [0.0, 5.0, 9.0]}


def build_warnings(plot) -> list[str]:
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        plot.to_svg()
    return [str(warning.message) for warning in caught]


@pytest.mark.parametrize(
    "plot",
    [
        gg.ggplot(FRAME, gg.aes("x", "y", color="c"))
        + gg.geom_point()
        + gg.guides(color="colorbar"),
        gg.ggplot(FRAME, gg.aes("x", "y", size="v"))
        + gg.geom_point()
        + gg.guides(size=gg.guide_colorbar(barwidth=3)),
        gg.ggplot(FRAME, gg.aes("x", "y", color="c"))
        + gg.geom_point()
        + gg.scale_color_hue(guide="colorbar"),
    ],
)
def test_a_colorbar_needs_a_continuous_color_scale(plot):
    with pytest.raises(ValueError, match="a colorbar needs a continuous color or fill scale"):
        plot.to_svg()


@pytest.mark.parametrize(
    ("guide", "message"),
    [
        (gg.guide_colorbar(barwidth=-30), "barwidth': expected a positive size"),
        (gg.guide_colorbar(nbin=1_000_000), "nbin': at most 10000 bins"),
    ],
)
def test_colorbar_options_are_validated(guide, message):
    plot = gg.ggplot(FRAME, gg.aes("x", "y", color="v")) + gg.geom_point() + gg.guides(color=guide)
    with pytest.raises(ValueError, match=message):
        plot.to_svg()


def test_built_guide_options_do_not_warn_and_others_do():
    base = gg.ggplot(FRAME, gg.aes("x", "y", color="v")) + gg.geom_point()
    built = [
        base + gg.guides(color=gg.guide_colorbar(title="T", barwidth=10, barheight=90, nbin=5)),
        base + gg.scale_color_gradient(guide=gg.guide_colorbar(nbin=3)),
        base + gg.scale_color_gradient(guide=gg.guide_legend(title="X")),
        base + gg.guides(color=gg.guide_legend(ncol=2, nrow=3, byrow=True)),
    ]
    for plot in built:
        assert not [message for message in build_warnings(plot) if "guide" in message]
    unbuilt = base + gg.guides(color=gg.guide_legend(override_aes={"size": 3}))
    assert any("not supported yet" in message for message in build_warnings(unbuilt))
