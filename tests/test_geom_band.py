"""geom_band against lets-plot 4.11 (`BandGeom`).

A band fills the whole panel between two bounds, without an outline; its two
edges are lines in `color`, `size` x 2.2 px and `linetype`; `alpha` fades the fill
only. Measured from lets-plot SVG: pen fill at 10% opacity by default, 1.65 px
pen edges, opaque mapped fills, bounds train the axis. The edges must fall where
`geom_vline` / `geom_hline` draw the same values.
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

POINTS = {"x": [0.0, 5.0], "y": [0.0, 5.0]}
PATH = re.compile(r'<path d="([^"]+)"([^>]*)/>')


def paths(svg: str) -> list[tuple[list[tuple[float, float]], str]]:
    found = []
    for data, attributes in PATH.findall(svg):
        numbers = [float(value) for value in re.findall(r"-?[\d.]+", data)]
        found.append((list(zip(numbers[::2], numbers[1::2], strict=True)), attributes))
    return found


def base(flip: bool = False):
    plot = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point() + gg.theme_void()
    return plot + gg.coord_flip() if flip else plot


@pytest.mark.parametrize("flip", [False, True])
@pytest.mark.parametrize(
    ("bounds", "line"),
    [
        ({"xmin": 1, "xmax": 2}, lambda value: gg.geom_vline(xintercept=value)),
        ({"ymin": 1, "ymax": 3}, lambda value: gg.geom_hline(yintercept=value)),
    ],
)
def test_edges_fall_on_reference_lines(bounds: dict, line, flip: bool) -> None:
    band = paths((base(flip) + gg.geom_band(**bounds)).to_svg())
    low, high = list(bounds.values())
    lines = paths((base(flip) + line(low) + line(high)).to_svg())
    fills = [
        points for points, attributes in band if "Z" in attributes or 'stroke="none"' in attributes
    ]
    edges = [points for points, attributes in band if 'fill="none"' in attributes]
    references = [points for points, attributes in lines if 'fill="none"' in attributes]
    assert len(fills) == 1 and edges == references
    # The fill spans the lines' whole length and the gap between them.
    xs = {x for points in references for x, _ in points}
    ys = {y for points in references for _, y in points}
    fill_xs = {x for x, _ in fills[0]}
    fill_ys = {y for _, y in fills[0]}
    assert fill_xs == xs and fill_ys == ys


def test_styles_match_lets_plot() -> None:
    svg = (base() + gg.geom_band(xmin=1, xmax=2)).to_svg()
    assert 'fill="#474747" fill-opacity="0.102" stroke="none"' in svg
    assert svg.count('fill="none" stroke="#474747" stroke-width="1.65"') == 2
    faded = (
        base() + gg.geom_band(xmin=1, xmax=2, alpha=0.3, color="red", linetype="dashed")
    ).to_svg()
    # alpha on the fill only; the edges stay opaque and dashed, as lets-plot.
    assert 'fill-opacity="0.302"' in faded
    assert re.search(r'stroke="#ff0000" stroke-width="1.65" stroke-dasharray="6.6', faded)
    assert "stroke-opacity" not in faded


def test_lets_plot_has_the_same_marks() -> None:
    def build(module):
        return (
            module.ggplot(POINTS, module.aes("x", "y"))
            + module.geom_band(xmin=1, xmax=2, size=0)
            + module.geom_band(
                module.aes(ymin="low", ymax="high", fill="kind"),
                data={"low": [1.0, 3.0], "high": [2.0, 3.5], "kind": ["u", "v"]},
            )
        )

    theirs = build(lp).to_svg()
    ours = build(gg).to_svg()
    # Fills: one grey band, two mapped (opaque) ones; edges: none for size=0, two each.
    their_fills = re.findall(
        r'fill="(rgb\([^)]*\))" fill-opacity="([\d.]+)" stroke-width="0.0"', theirs
    )
    assert [opacity[:4] for _, opacity in their_fills] == ["0.10", "1.0", "1.0"]
    our_fills = re.findall(
        r'Z" fill="(#[0-9a-f]{6})"(?: fill-opacity="([\d.]+)")? stroke="none"', ours
    )
    assert [opacity for _, opacity in our_fills] == ["0.102", "", ""]
    assert (
        theirs.count('stroke-width="1.6500000000000001"')
        == ours.count('stroke-width="1.65"')
        == 4 + 2
    )


def test_bounds_train_the_axis() -> None:
    svg = (base() + gg.geom_band(xmin=6, xmax=7)).to_svg()
    edges = [points for points, attributes in paths(svg) if 'fill="none"' in attributes]
    panel_right = max(x for points, _ in paths(svg) for x, _ in points)
    assert max(x for points in edges for x, _ in points) <= panel_right


def test_mapped_bounds_and_missing_rows() -> None:
    data = {"low": [1.0, None, 4.0, 3.0], "high": [2.0, 3.0, 4.5, 2.0]}
    svg = (base() + gg.geom_band(gg.aes(ymin="low", ymax="high"), data=data)).to_svg()
    # The second row has no low bound; swapped bounds still draw (see deviations.md).
    assert len(re.findall(r'Z" fill="#474747" fill-opacity="0.102" stroke="none"', svg)) == 3


@pytest.mark.parametrize(
    ("band", "scale"),
    [
        ({"xmin": 2, "xmax": 4}, gg.scale_x_reverse),
        ({"ymin": 2, "ymax": 3}, gg.scale_y_reverse),
    ],
)
def test_reversed_scale_draws_the_band(band: dict, scale) -> None:
    svg = (base() + gg.geom_band(**band) + scale()).to_svg()
    assert len(re.findall(r'Z" fill="#474747" fill-opacity="0.102" stroke="none"', svg)) == 1


def test_both_directions_is_an_error() -> None:
    with pytest.raises(SpecError, match=r"either xmin/xmax.*or ymin/ymax"):
        (base() + gg.geom_band(xmin=1, xmax=2, ymin=1, ymax=2)).to_svg()


def test_no_unsupported_warning() -> None:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        (gg.ggplot() + gg.geom_band(xmin=-1, xmax=1)).to_svg()
