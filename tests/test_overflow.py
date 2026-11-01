"""Overflow guards: legends, axis labels and titles stay inside the canvas.

lets-plot and ggplot2 let these run off the plot; ggrust reflows, wraps or cuts
them instead (recorded in the plan.md decisions table). Plots that fit are drawn
exactly as before.
"""

import re

import ggrust as gg

NUMBER = r"([\d.\-]+)"
GROUPS = [f"group {index:02d}" for index in range(50)]


def texts(svg: str) -> list[tuple[float, float, str]]:
    """Every text element as (x, y, content)."""
    return [
        (float(x), float(y), text)
        for x, y, text in re.findall(rf'<text x="{NUMBER}" y="{NUMBER}"[^>]*>([^<]*)</text>', svg)
    ]


def many_levels(position: str) -> gg.PlotSpec:
    """50 colour levels in a 400 x 300 plot."""
    frame = {"x": list(range(50)), "y": list(range(50)), "c": GROUPS}
    return (
        gg.ggplot(frame, gg.aes("x", "y", color="c"))
        + gg.geom_point()
        + gg.theme(legend_position=position)
        + gg.ggsize(400, 300)
    )


def test_many_keys_reflow_and_cut_to_fit_the_canvas():
    for position in ["right", "bottom", "left", "top"]:
        found = texts(many_levels(position).to_svg())
        more = [text for _, _, text in found if re.fullmatch(r"\+\d+ more", text)]
        assert len(more) == 1, position
        shown = [text for _, _, text in found if text in GROUPS]
        assert len(shown) + int(more[0].split()[0][1:]) == len(GROUPS)
        # Kept keys are the first ones, in order.
        assert shown == sorted(shown)
        for x, y, text in found:
            assert 0 <= x <= 400 and 0 <= y <= 300, (position, text, x, y)


def test_a_legend_that_fits_is_untouched():
    frame = {"x": [1, 2, 3], "y": [1, 2, 3], "c": ["a", "b", "c"]}
    svg = (gg.ggplot(frame, gg.aes("x", "y", color="c")) + gg.geom_point()).to_svg()
    assert not any("more" in text for _, _, text in texts(svg))


def test_long_legend_labels_wrap_narrower_before_keys_are_dropped():
    label = "an extremely long legend label that goes on and on and on forever"
    frame = {"x": [1, 2, 3], "y": [1, 2, 3], "c": ["a", label, "c"]}
    svg = (
        gg.ggplot(frame, gg.aes("x", "y", color="c")) + gg.geom_point() + gg.ggsize(400, 300)
    ).to_svg()
    found = [text for _, _, text in texts(svg)]
    assert "a" in found and "c" in found
    assert not any("more" in text for text in found)
    assert all(len(text) <= 20 for text in found if text in label)


def test_long_discrete_axis_labels_are_cut_and_the_panel_keeps_its_height():
    long = "a really long category name that never ends at all"
    frame = {"x": ["short", long], "y": [1, 2]}
    svg = (
        gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_bar(stat="identity") + gg.ggsize(300, 250)
    ).to_svg()
    found = [text for _, _, text in texts(svg)]
    assert "short" in found
    assert long not in found
    assert any(
        "..." in text
        and long.startswith(text.split("...")[0])
        and long.endswith(text.split("...")[1])
        for text in found
    )


def test_long_titles_wrap_inside_the_canvas():
    title = "A very very long plot title that is much wider than the plot itself, really"
    svg = (
        gg.ggplot({"x": [1, 2], "y": [1, 2]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.ggtitle(title)
        + gg.ggsize(300, 250)
    ).to_svg()
    lines = [text for _, _, text in texts(svg) if len(text) > 1 and text in title]
    assert len(lines) > 1
    assert " ".join(lines) == title


def test_cut_category_labels_keep_their_distinct_ends():
    names = [f"sample_identifier_prefix_{index}" for index in range(4)]
    svg = (
        gg.ggplot({"x": names, "y": [1, 2, 3, 4]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.ggsize(300, 200)
    ).to_svg()
    cut = [text for _, _, text in texts(svg) if "..." in text]
    assert len(cut) == 4
    assert len(set(cut)) == 4
    assert [text[-1] for text in cut] == ["0", "1", "2", "3"]


def test_numeric_axis_labels_are_never_cut():
    svg = (
        gg.ggplot({"x": [1, 2, 3], "y": [1e12, 2e12, 3e12]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.scale_y_continuous(format="{.15f}")
        + gg.ggsize(150, 200)
    ).to_svg()
    assert not any("..." in text for _, _, text in texts(svg))


def test_stacked_legends_share_the_budget():
    levels = [f"level {index:02d}" for index in range(20)]
    frame = {"x": list(range(20)), "y": list(range(20)), "a": levels, "b": levels}
    svg = (
        gg.ggplot(frame, gg.aes("x", "y", color="a", shape="b"))
        + gg.geom_point()
        + gg.guides(shape=gg.guide_legend(title="b"))
        + gg.ggsize(400, 300)
    ).to_svg()
    for x, y, text in texts(svg):
        assert 0 <= x <= 400 and 0 <= y <= 300, (text, x, y)


def test_a_title_word_wider_than_the_plot_is_broken():
    svg = (
        gg.ggplot({"x": [1], "y": [1]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.ggtitle("W" * 80)
        + gg.ggsize(300, 250)
    ).to_svg()
    lines = [text for _, _, text in texts(svg) if set(text) == {"W"}]
    assert len(lines) > 1 and "".join(lines) == "W" * 80
