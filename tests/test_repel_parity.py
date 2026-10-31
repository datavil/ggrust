"""geom_text_repel / geom_label_repel: lets-plot's force layout and leader segments.

The layout itself is checked bit for bit against lets-plot in Rust
(`repel_layout::tests`, fed lets-plot's text widths). Here: the rendered texts,
segment paint and geometry, options and errors. Positions are not compared with
lets-plot: the embedded font measures text a few pixels narrower, which moves
the labels.
"""

import math
import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

CITIES = {
    "area": [1.0, 1.1, 1.2, 2.0, 3.0, 3.05],
    "people": [1.0, 1.05, 0.95, 2.0, 3.0, 3.02],
    "city": ["Ghent", "Bruges", "Leuven", "Antwerp", "Brussels", "Liege"],
    "region": ["west", "west", "east", "east", "east", "west"],
}


def _plot(m, *layers):
    plot = m.ggplot(CITIES, m.aes("area", "people")) + m.geom_point()
    for layer in layers:
        plot += layer
    return plot


def _ggrust_svg(plot) -> str:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        return plot.to_svg()


def _texts(svg: str) -> list[tuple[str, float, float]]:
    """Panel texts (content, x, y) in drawing order."""
    panel = svg[svg.find('class="panel"') : svg.find('class="axis-x"')]
    return [
        (content, float(x), float(y))
        for x, y, content in re.findall(
            r'<text x="([\d.-]+)" y="([\d.-]+)"[^>]*>([^<]*)</text>', panel
        )
    ]


def _segments(svg: str) -> list[dict[str, object]]:
    """Open two-point paths in the panel: leader segments."""
    panel = svg[svg.find('class="panel"') : svg.find('class="axis-x"')]
    segments = []
    for d, attributes in re.findall(r'<path d="M([\d. L-]+)"([^>]*)/>', panel):
        points = [tuple(map(float, pair.split())) for pair in d.split("L")]
        if len(points) == 2 and 'fill="none"' in attributes:
            segments.append({"points": points, "attributes": attributes})
    return segments


def _circles(svg: str) -> list[tuple[float, float]]:
    return [
        (float(x), float(y)) for x, y in re.findall(r'<circle cx="([\d.-]+)" cy="([\d.-]+)"', svg)
    ]


def test_draws_every_label_once_as_lets_plot():
    ours = _texts(_ggrust_svg(_plot(gg, gg.geom_text_repel(gg.aes(label="city"), seed=1))))
    theirs = re.findall(
        r"<tspan>([^<]+)</tspan>",
        _plot(lp, lp.geom_text_repel(lp.aes(label="city"), seed=1)).to_svg(),
    )
    assert sorted(text for text, _, _ in ours) == sorted(set(theirs) & set(CITIES["city"]))


def test_segments_start_at_the_marker_edge_in_the_text_paint():
    svg = _ggrust_svg(_plot(gg, gg.geom_text_repel(gg.aes(label="city"), seed=1)))
    circles = _circles(svg)
    segments = _segments(svg)
    assert segments
    for segment in segments:
        start = segment["points"][0]
        distance = min(math.dist(start, circle) for circle in circles)
        assert distance == pytest.approx(3.3, abs=0.02)
        assert 'stroke="#474747"' in segment["attributes"]
        assert 'stroke-width="1.65"' in segment["attributes"]


def test_segment_style_options_as_lets_plot():
    layer = {
        "seed": 3,
        "segment_color": "red",
        "segment_size": 1.0,
        "segment_alpha": 0.5,
        "color": "blue",
        "linetype": "dashed",
    }
    svg = _ggrust_svg(_plot(gg, gg.geom_text_repel(gg.aes(label="city"), **layer)))
    theirs = _plot(lp, lp.geom_text_repel(lp.aes(label="city"), **layer)).to_svg()
    line = re.search(r'<line [^>]*stroke="rgb\(255,0,0\)"[^>]*>', theirs).group(0)
    assert 'stroke-opacity="0.50' in line and 'stroke-width="2.2"' in line
    attributes = _segments(svg)[0]["attributes"]
    assert 'stroke="#ff0000"' in attributes and 'stroke-opacity="0.502"' in attributes
    assert 'stroke-width="2.2"' in attributes and "stroke-dasharray" in attributes
    assert all(
        'fill="#0000ff"' in text for text in re.findall(r"<text [^>]*>[A-Z][a-z]+</text>", svg)
    )


def test_nudged_labels_keep_segments_at_their_points():
    svg = _ggrust_svg(_plot(gg, gg.geom_text_repel(gg.aes(label="city"), seed=1, nudge_y=0.5)))
    circles = _circles(svg)
    for segment in _segments(svg):
        assert min(math.dist(segment["points"][0], circle) for circle in circles) == pytest.approx(
            3.3, abs=0.02
        )


def test_layout_is_seeded_and_reproducible():
    def render(seed):
        return _ggrust_svg(_plot(gg, gg.geom_text_repel(gg.aes(label="city"), seed=seed)))

    assert render(5) == render(5)
    assert _texts(render(5)) != _texts(render(6))
    # Without a seed the layout is still reproducible (seed 0).
    unseeded = _plot(gg, gg.geom_text_repel(gg.aes(label="city")))
    assert _ggrust_svg(unseeded) == _ggrust_svg(unseeded)


def test_crowded_labels_are_hidden_past_max_overlaps():
    crowd = {"x": [1.0] * 12, "y": [1.0] * 12, "name": [f"label {index}" for index in range(12)]}

    def count(**options):
        plot = gg.ggplot(crowd, gg.aes("x", "y")) + gg.geom_text_repel(
            gg.aes(label="name"), seed=1, **options
        )
        return len(_texts(_ggrust_svg(plot)))

    assert count(max_overlaps=-1) == 12
    assert count(max_overlaps=2) < 12


def test_direction_x_moves_labels_sideways_only():
    svg = _ggrust_svg(_plot(gg, gg.geom_text_repel(gg.aes(label="city"), seed=1, direction="x")))
    circle_ys = [y for _, y in _circles(svg)]
    # Each baseline sits about 0.35 em below its point's centre: labels kept their y.
    for _, _, y in _texts(svg):
        assert min(abs(y - circle_y - 4.85) for circle_y in circle_ys) < 0.1


def test_unknown_direction_says_what_is_accepted():
    plot = _plot(gg, gg.geom_text_repel(gg.aes(label="city"), direction="diagonal"))
    with pytest.raises(ValueError, match=r"direction.*'diagonal'.*'both', 'x' or 'y'"):
        plot.to_svg()


def test_label_repel_draws_boxes_and_arrows():
    plot = _plot(
        gg,
        gg.geom_label_repel(
            gg.aes(label="city"), seed=1, fill="#ffffcc", arrow=gg.arrow(), min_segment_length=0
        ),
    )
    svg = _ggrust_svg(plot)
    assert svg.count('fill="#ffffcc"') == len(CITIES["city"])
    assert len(_texts(svg)) == len(CITIES["city"])


def test_legend_key_is_text_and_segment():
    plot = _plot(gg, gg.geom_text_repel(gg.aes(label="city", color="region"), seed=1))
    svg = _ggrust_svg(plot)
    legend = svg[svg.find('class="legend"') :]
    assert legend.count(">a</text>") == 2
    assert len(re.findall(r'<path d="M[\d.]+ [\d.]+L[\d.]+ [\d.]+" fill="none"', legend)) >= 2


def test_tooltips_follow_the_placed_labels():
    plot = _plot(
        gg,
        gg.geom_text_repel(
            gg.aes(label="city"), seed=1, tooltips=gg.layer_tooltips().line("@city")
        ),
    )
    assert "Ghent" in plot.to_html()


def test_label_legend_key_is_a_filled_cell_as_lets_plot():
    def plot(m):
        return _plot(m, m.geom_label_repel(m.aes(label="city", fill="region"), seed=1))

    theirs = plot(lp).to_svg()
    assert len(re.findall(r'<rect [^>]*fill="rgb\((228,26,28|55,126,184)\)"', theirs)) == 2
    svg = _ggrust_svg(plot(gg))
    legend = svg[svg.find('class="legend"') :]
    for color in ["#e41a1c", "#377eb8"]:
        assert re.search(rf'<path d="M[^"]+Z" fill="{color}"', legend)
    assert legend.count(">a</text>") == 2
