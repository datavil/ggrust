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


def test_long_discrete_axis_labels_wrap_into_their_slots():
    long = "a really long category name that never ends at all"
    frame = {"x": ["short", long], "y": [1, 2]}
    svg = (
        gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_bar(stat="identity") + gg.ggsize(300, 250)
    ).to_svg()
    found = [text for _, _, text in texts(svg)]
    assert "short" in found
    lines = [text for text in found if len(text) > 1 and text in long]
    assert len(lines) > 1 and " ".join(lines) == long
    assert not any("..." in text for text in found)


def test_long_y_labels_wrap_within_the_label_budget():
    cells = ["CD4 positive naive T cell", "CD8 positive effector memory T cell", "B cell"]
    svg = (
        gg.ggplot({"y": cells, "x": [1, 2, 3]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.ggsize(300, 300)
    ).to_svg()
    found = [text for _, _, text in texts(svg)]
    assert "B cell" in found
    assert "CD8 positive effector memory T cell" not in found
    assert not any("..." in text for text in found)


def test_vertical_labels_that_fit_are_not_wrapped():
    names = ["first long-ish label", "second long-ish label", "third long-ish label"]
    svg = (
        gg.ggplot({"x": names, "y": [1, 2, 3]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.ggsize(250, 600)
    ).to_svg()
    found = [text for _, _, text in texts(svg)]
    assert all(name in found for name in names)


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


def test_a_users_newline_in_a_label_is_drawn_as_before():
    frame = {"x": ["a\nb\nc\nd\ne", "a really long category name that never ends at all", "z"]}
    frame["y"] = [1, 2, 3]
    svg = (gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point() + gg.ggsize(400, 300)).to_svg()
    assert "a b c d e" in [text for _, _, text in texts(svg)]


def test_facet_axis_labels_wrap_like_a_single_plot():
    cells = [
        "CD4 positive naive T cell",
        "CD8 positive effector memory T cell",
        "classical monocyte",
        "natural killer cell",
    ]
    frame = {"cell": cells * 2, "n": [3, 5, 2, 4, 1, 2, 3, 4], "f": ["A"] * 4 + ["B"] * 4}
    svg = (
        gg.ggplot(frame, gg.aes("cell", "n"))
        + gg.geom_bar(stat="identity")
        + gg.facet_wrap("f")
        + gg.ggsize(500, 300)
    ).to_svg()
    found = texts(svg)
    assert "CD8 positive effector memory T cell" not in [text for _, _, text in found]
    assert not any("..." in text for _, _, text in found)
    for x, y, text in found:
        assert 0 <= x <= 500 and 0 <= y <= 300, (text, x, y)


def test_stacked_legends_wrap_into_columns_before_keys_are_cut():
    # lets-plot#1506: several short legends too tall for one stack sit side by side.
    frame = {"x": list(range(6)), "y": list(range(6))}
    for name in "abcd":
        frame[name] = [f"{name}{index}" for index in range(6)]
    svg = (
        gg.ggplot(frame, gg.aes("x", "y", color="a", shape="b", fill="c"))
        + gg.geom_point()
        + gg.geom_tile(gg.aes(alpha="d"))
        + gg.ggsize(500, 350)
    ).to_svg()
    shown = [text for _, _, text in texts(svg)]
    assert "more" not in " ".join(shown)
    for name in "abcd":
        assert all(f"{name}{index}" in shown for index in range(6)), name
    for x, y, text in texts(svg):
        assert 0 <= x <= 500 and 0 <= y <= 350, (text, x, y)
    titles = {text: x for x, _, text in texts(svg) if text in "abcd"}
    assert len(set(titles.values())) > 1


def test_axis_title_angle_rotates_the_title():
    # lets-plot#1504: lets-plot ignores the angle; ggplot2 turns the title.
    import warnings

    frame = {"x": [1, 2], "y": [1, 4]}
    plot = (
        gg.ggplot(frame, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.theme(axis_title_x=gg.element_text(angle=90, vjust=0.5))
    )
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        svg = plot.to_svg()
    title = re.search(r"<text[^>]*>x</text>", svg)
    assert title is not None and "rotate(-90" in title.group(0)
