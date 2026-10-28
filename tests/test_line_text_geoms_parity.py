"""Paint of path, line, segment, polygon, reference lines and text must match lets-plot.

Both SVGs are reduced to the layer's drawn elements (axes, grid and background
removed) and compared by paint and size; positions differ by a few px because the
panels do (real fonts), so only shape sizes are compared (arrow barbs, label boxes).
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {"x": [1.0, 2.0, 3.0, 4.0], "y": [1.0, 3.0, 2.0, 4.0], "g": ["a", "a", "b", "b"]}
ELEMENT = re.compile(r"<(path|line|text)\b([^>]*)>")


def _attributes(tag: str) -> dict:
    return dict(re.findall(r'([\w-]+)="([^"]*)"', tag))


def _hex(color: str | None) -> str | None:
    match = re.fullmatch(r"rgb\((\d+),\s*(\d+),\s*(\d+)\)", color or "")
    if match:
        return "#" + "".join(f"{int(part):02x}" for part in match.groups())
    return color.lower() if color else None


def _numbers(text: str) -> list[float]:
    return [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?", text)]


def _is_box(d: str) -> bool:
    """A rounded label box: lets-plot draws Bezier corners, ggrust many vertices."""
    return " C" in d or d.count("L") > 12


def _layer_elements(svg: str) -> list[tuple]:
    """Paint tuples of the marks drawn by the layers (stroke width != 1 or text 14px+)."""
    found = []
    for kind, tag in ELEMENT.findall(svg):
        attributes = _attributes(tag)
        if "class" in attributes or attributes.get("pointer-events") == "none":
            continue  # axis text, plot border
        width = float(attributes.get("stroke-width", 0))
        if kind == "text":
            size = attributes.get("font-size") or re.search(
                r"font-size:([\d.]+)px", attributes.get("style", "")
            )
            size = float(size if isinstance(size, str) else size.group(1)) if size else None
            if size and size not in (13.0, 15.0, 16.0):
                found.append(("text", _hex(attributes.get("fill")), size))
        elif kind in ("path", "line") and width not in (0.0, 1.0):
            closed = "Z" in attributes.get("d", "")
            dash = tuple(round(v, 3) for v in _numbers(attributes.get("stroke-dasharray", "")))
            found.append(
                (
                    kind if kind == "line" else "path",
                    _hex(attributes.get("stroke")),
                    round(width, 3),
                    dash,
                    _hex(attributes.get("fill")) if closed else None,
                    round(float(attributes.get("fill-opacity", 1)), 2) if closed else None,
                )
            )
        elif kind == "path" and attributes.get("fill") not in (None, "none") and width == 1.0:
            # label boxes: rounded paths with a fill and a 1 px border
            if _is_box(attributes.get("d", "")):
                found.append(("box", _hex(attributes.get("stroke")), _hex(attributes.get("fill"))))
    return found


def _both(build):
    return (
        _layer_elements(build(gg).to_svg()),
        _layer_elements(build(lp).to_svg()),
    )


def _plot(module, *layers, data=DATA):
    plot = module.ggplot(data, module.aes("x", "y")) + module.ggsize(400, 300)
    for layer in layers:
        plot = plot + layer(module)
    return plot


def _normalise(elements):
    """Lines of one mark may be split differently (lets-plot draws segments as lines)."""
    return sorted((e[0] if e[0] != "line" else "path", *e[1:]) for e in elements)


CASES = {
    "path": lambda m: m.geom_path(),
    "path styled": lambda m: m.geom_path(color="red", size=2, linetype="dashed"),
    "line mapped": lambda m: m.geom_line(m.aes(color="g")),
    "vline": lambda m: m.geom_vline(xintercept=2, linetype="dotted", color="blue"),
    "hline": lambda m: m.geom_hline(yintercept=2, size=1.5),
    "polygon": lambda m: m.geom_polygon(m.aes(group="g", fill="g"), alpha=0.5, color="black"),
    "text": lambda m: m.geom_text(m.aes(label="g"), size=9, color="red"),
    "text default": lambda m: m.geom_text(m.aes(label="g")),
    "label": lambda m: m.geom_label(m.aes(label="g"), size=6, color="green"),
    "segment": lambda m: m.geom_segment(m.aes(xend="y", yend="x"), color="purple", size=1),
}


@pytest.mark.parametrize("name", CASES)
def test_paint_matches_lets_plot(name):
    ours, reference = _both(lambda m: _plot(m, CASES[name]))
    assert ours, "ggrust drew nothing"
    assert _normalise(ours) == _normalise(reference)


def _arrow_barbs(svg: str) -> list[list[float]]:
    """Barb lengths and angles of every open 3-point arrow head path."""
    barbs = []
    for kind, tag in ELEMENT.findall(svg):
        attributes = _attributes(tag)
        d = attributes.get("d", "")
        if kind != "path" or d.count("L") != 2 or d.endswith("Z ") or "Z" in d:
            continue
        x0, y0, x1, y1, x2, y2 = _numbers(d)
        import math

        barbs.append(
            [
                round(math.hypot(x0 - x1, y0 - y1), 2),
                round(math.hypot(x2 - x1, y2 - y1), 2),
                round(
                    abs(math.degrees(math.atan2(y0 - y1, x0 - x1) - math.atan2(y2 - y1, x2 - x1)))
                    % 360,
                    1,
                ),
            ]
        )
    return barbs


def test_arrow_heads_match_lets_plot():
    def build(m):
        return _plot(
            m,
            lambda m: m.geom_segment(
                m.aes(xend="y", yend="x"), arrow=m.arrow(angle=25, length=12), size=1
            ),
        )

    ours = _arrow_barbs(build(gg).to_svg())
    reference = _arrow_barbs(build(lp).to_svg())
    assert ours
    assert sorted(ours) == sorted(reference)


def test_label_box_is_as_tall_as_lets_plot():
    def heights(svg):
        boxes = [
            d
            for d in re.findall(r'<path d="([^"]*)"[^>]*stroke-width="1(?:\.0)?"', svg)
            if _is_box(d)
        ]
        return sorted(round(max(_numbers(d)[1::2]) - min(_numbers(d)[1::2]), 1) for d in boxes)

    def build(m):
        return _plot(m, lambda m: m.geom_label(m.aes(label="g"), size=6))

    assert heights(build(gg).to_svg()) == heights(build(lp).to_svg())


def _text_contents(svg: str) -> list[str]:
    """Visible text of every label-sized <text> (tspans joined, powers as ^n)."""
    found = []
    for tag, body in re.findall(r"<text\b([^>]*)>(.*?)</text>", svg, re.S):
        attributes = _attributes(tag)
        size = attributes.get("font-size") or re.search(
            r"font-size:([\d.]+)px", attributes.get("style", "")
        )
        size = float(size if isinstance(size, str) else size.group(1)) if size else 0
        if size != 14.0:
            continue
        # lets-plot: tspans, the power in a 0.7em one; ggrust: text, then the
        # power in a raised (negative dy) tspan.
        text = ""
        for piece in re.split(r"(<tspan[^>]*>.*?</tspan>)", body, flags=re.S):
            match = re.fullmatch(r"<tspan([^>]*)>(.*?)</tspan>", piece, re.S)
            if not match:
                text += piece.strip()
                continue
            tspan, content = match.groups()
            content = content.replace("\u200b", "")
            if 'font-size="0.1em"' in tspan:
                continue
            raised = 'font-size="0.7em"' in tspan or re.search(r'dy="-', tspan)
            text += ("^" + content) if raised and content else content
        found.append(text.strip())
    return found


def test_numeric_labels_format_like_lets_plot():
    values = [1.0, 0.1 + 0.2, 1e20, float("nan"), 1e-7, 123456789.0, 1234.5]
    data = {"x": list(range(7)), "y": [0.0] * 7, "v": values}

    def build(m):
        return m.ggplot(data, m.aes("x", "y", label="v")) + m.geom_text()

    ours = _text_contents(build(gg).to_svg())
    reference = _text_contents(build(lp).to_svg())
    assert ours == reference, (ours, reference)


def test_newlines_make_separate_lines():
    def build(m):
        return _plot(m, lambda m: m.geom_text(label="a\nb", x=1, y=1), data={"x": [1], "y": [1]})

    assert _text_contents(build(gg).to_svg()) == ["a", "b"]
    assert _text_contents(build(lp).to_svg()) == ["a", "b"]


@pytest.mark.parametrize("hjust", ["left", "middle", "center", "right"])
def test_justification_names_are_accepted(hjust):
    svg = _plot(gg, lambda m: m.geom_text(m.aes(label="g"), hjust=hjust, vjust="top")).to_svg()
    assert _text_contents(svg)


def test_label_alpha_fades_the_box_fill_only():
    def opacities(svg):
        texts = re.findall(r"<text\b([^>]*)>\s*(?:<tspan>)?a<", svg)
        text_opacity = {round(float(_attributes(tag).get("fill-opacity", 1)), 2) for tag in texts}
        return text_opacity

    def build(m):
        return _plot(m, lambda m: m.geom_label(label="a", x=1, y=1, alpha=0.5, fill="yellow"))

    assert opacities(build(gg).to_svg()) == opacities(build(lp).to_svg()) == {1.0}


FACES = ["bold", "bold italic", "italic-bold", "plain bold", "Bold", "Bold Italic", "bolditalic"]


def _ggrust_face(svg: str, content: str) -> tuple[bool, bool]:
    tag = re.search(rf"<text\b([^>]*)>{content}<", svg).group(1)
    return 'font-weight="bold"' in tag, 'font-style="italic"' in tag


def _lets_plot_text_face(svg: str) -> tuple[bool, bool]:
    position = svg.find(">w<")
    style = svg[max(0, position - 400) : position]
    return "font-weight:bold" in style, "font-style:italic" in style


def _lets_plot_title_face(svg: str) -> tuple[bool, bool]:
    css = re.search(r"\.legend-title \{[^}]*\}", svg).group(0)
    return "font-weight: bold" in css, "font-style: italic" in css


@pytest.mark.parametrize("face", FACES)
def test_font_faces_parse_like_lets_plot(face):
    """lets-plot splits on space, `_`, `.`, `-` and matches `bold`/`italic` by case."""

    def build(module):
        return (
            module.ggplot({"x": [1.0], "c": ["a"]})
            + module.geom_point(module.aes("x", "x", color="c"))
            + module.geom_text(x=1, y=1, label="w", fontface=face)
            + module.theme(legend_title=module.element_text(face=face))
        )

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        ours = build(gg).to_svg()
    theirs = build(lp).to_svg()
    assert _ggrust_face(ours, "w") == _lets_plot_text_face(theirs)
    assert _ggrust_face(ours, "c") == _lets_plot_title_face(theirs)
