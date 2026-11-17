"""Render every docs example and write the gallery pages.

Each ``docs/examples/<name>.py`` holds one example: a docstring (title on the first
line, a short description after it) and the user-facing code between the
``# --8<-- [start:code]`` and ``# --8<-- [end:code]`` markers, which binds ``plot``.
The code reads its data from GitHub so readers can run it as is; here it reads the
same files from ``docs/data/``.

For each example this writes:

- ``docs/plots/<name>.html``: the interactive plot, embedded in pages as an iframe;
- ``docs/images/gallery/<name>.png``: the gallery thumbnail;
- ``docs/examples/figures/<name>.html``: the iframe markup, for guide pages to
  include with ``--8<-- "figures/<name>.html"``;
- ``docs/gallery/<name>.md``: the gallery page.

It also writes ``docs/gallery/index.md``.

It also runs the ``Examples`` section of every API reference docstring (see
``build_reference.SECTIONS``). A docstring can hold several examples: the section is
one doctest, and each further example starts with a sentence that introduces it.
Every example that binds ``plot`` is rendered to ``docs/plots/reference/<name>.html``
(``<name>_2.html``, ``<name>_3.html``, ... for the later ones) with its iframe markup
under the same name in ``docs/examples/figures/reference/``, which the reference page
includes in order, each later figure under its introducing sentence.

Run after changing an example or a docstring example:
``uv run python docs/build_examples.py``.
"""

import ast
import contextlib
import doctest
import inspect
import io
import re
import warnings
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

from ggrust import animate
from ggrust._render import is_animated

DOCS = Path(__file__).parent
EXAMPLES = DOCS / "examples"
DATA_URL = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
START = "# --8<-- [start:code]\n"
END = "# --8<-- [end:code]"

# Gallery sections, in order. Every example listed here gets a gallery page.
GALLERY: dict[str, list[str]] = {
    "Points and lines": ["scatter", "line", "ribbon", "function", "manual_legend", "venn"],
    "Time series": ["unemployment", "oil_price"],
    "Bars and tiles": ["stacked_bars", "dodged_bars", "heatmap", "periodic_table"],
    "Distributions": ["histogram", "boxplot", "violin", "raincloud"],
    "Ridgelines": ["ridges", "netflix_ridges", "unknown_pleasures"],
    "Labels, facets and grids": ["text_repel", "annotations", "facets", "grid"],
    "Legends": ["legends", "colorbar"],
    "Prebuilt plots": ["joint_plot", "residual_plot"],
    "Animation": [
        "animation_states",
        "animation_time",
        "animation_reveal",
        "animation_enter_exit",
        "animation_shadow",
    ],
    "Interactivity": ["tooltips", "many_points"],
    "3D": ["surface"],
    "ggplot2 extensions": [
        "beeswarm",
        "compare_means",
        "pairwise_tests",
        "cluster_marks",
        "highlight",
        "point_density",
        "side_panels",
        "clustered_heatmap",
        "figure_layout",
        "funky_heatmap",
    ],
}


@dataclass(frozen=True)
class Example:
    """One rendered example."""

    name: str
    title: str
    description: str
    code: str
    width: int
    height: int


def render(path: Path) -> Example:
    """Run one example file and write its HTML, thumbnail and iframe markup."""
    source = path.read_text()
    docstring = ast.get_docstring(ast.parse(source)) or path.stem
    title, _, description = docstring.partition("\n")
    code = source.split(START, 1)[1].split(END, 1)[0].rstrip() + "\n"
    namespace: dict[str, object] = {}
    exec(code.replace(DATA_URL, (DOCS / "data").as_posix() + "/"), namespace)
    plot = namespace["plot"]
    iframe = plot.to_html(iframe=True)
    width, height = (
        int(re.search(rf'{key}="(\d+)"', iframe).group(1)) for key in ("width", "height")
    )
    plot.to_html(DOCS / "plots" / f"{path.stem}.html")
    # An animation's thumbnail is its first frame.
    still = animate(plot).frame(0) if is_animated(plot) else plot
    still.to_png(DOCS / "images" / "gallery" / f"{path.stem}.png")
    (EXAMPLES / "figures" / f"{path.stem}.html").write_text(figure(path.stem, title, width, height))
    return Example(path.stem, title.strip(), description.strip(), code, width, height)


def figure(name: str, title: str, width: int, height: int, prefix: str = "../") -> str:
    """Return the iframe markup that embeds one interactive plot.

    Zensical resolves the ``src`` from the page's Markdown file, so the path is
    relative to the page: ``prefix`` leads from the page's folder to ``docs/``
    (``../`` for ``guide/`` and ``gallery/`` pages).
    """
    return (
        f'<div class="ggrust-figure"><iframe src="{prefix}plots/{name}.html" title="{title}" '
        f'width="{width}" height="{height}" scrolling="no" loading="lazy"></iframe></div>\n'
    )


@dataclass(frozen=True)
class ExampleBlock:
    """One example of a docstring: a run of ``>>>`` lines and the text that introduces it."""

    label: str
    code: str


def example_blocks(item: object) -> list[ExampleBlock]:
    """Return the examples of an object's docstring, in order.

    A block is a run of ``>>>`` lines with no prose in between. Its label is the
    paragraph right before it, on one line and without the final colon; the first
    block has none (the text before it is the docstring itself). Lines marked
    ``# doctest: +SKIP`` are left out, and so is a block left empty by that.
    """
    blocks: list[ExampleBlock] = []
    text, sources = "", []

    def close() -> None:
        if sources:
            label = "" if not blocks else " ".join(text.strip().split("\n\n")[-1].split())
            blocks.append(
                ExampleBlock(label.removesuffix(":").replace("``", "`"), "".join(sources))
            )
            sources.clear()

    for part in doctest.DocTestParser().parse(inspect.getdoc(item) or ""):
        if isinstance(part, str):
            if part.strip():
                close()
                text = part
        elif not part.options.get(doctest.SKIP):
            sources.append(part.source)
    close()
    return blocks


def docstring_example(item: object) -> str:
    """Return the runnable code of an object's docstring ``Examples`` section.

    Lines marked ``# doctest: +SKIP`` are left out. The result is empty when the
    docstring has no examples.
    """
    return "".join(block.code for block in example_blocks(item))


def binds_plot(code: str) -> bool:
    """Return whether example code assigns the name ``plot`` at top level."""
    tree = ast.parse(code)
    return any(
        isinstance(node, ast.Assign)
        and any(isinstance(target, ast.Name) and target.id == "plot" for target in node.targets)
        for node in tree.body
    )


def reference_figures(name: str, item: object) -> list[tuple[str, str]]:
    """Return the (file stem, label) of each figure on an object's reference page.

    One figure per example block that binds ``plot``: ``<name>`` for the first,
    ``<name>_2``, ``<name>_3``, ... for the later ones. Nothing is run.
    """
    labels = [block.label for block in example_blocks(item) if binds_plot(block.code)]
    return [
        (name if index == 0 else f"{name}_{index + 1}", label) for index, label in enumerate(labels)
    ]


def example_plots(item: object) -> Iterator[object]:
    """Run an object's docstring examples and yield the ``plot`` of each block that binds it.

    The blocks run in order in one namespace, as the doctest does, so a later block
    can use names from an earlier one.
    """
    namespace: dict[str, object] = {}
    for block in example_blocks(item):
        with contextlib.redirect_stdout(io.StringIO()):  # examples may print
            exec(block.code, namespace)
        if binds_plot(block.code):
            yield namespace["plot"]


def render_reference(name: str, item: object) -> None:
    """Run one docstring's examples and write the plot and iframe of each that binds ``plot``."""
    for (stem, _), plot in zip(reference_figures(name, item), example_plots(item), strict=True):
        iframe = plot.to_html(iframe=True)
        width, height = (
            int(re.search(rf'{key}="(\d+)"', iframe).group(1)) for key in ("width", "height")
        )
        plot.to_html(DOCS / "plots" / "reference" / f"{stem}.html")
        (EXAMPLES / "figures" / "reference" / f"{stem}.html").write_text(
            figure(f"reference/{stem}", f"{name} example", width, height, prefix="../../")
        )


def gallery_page(example: Example) -> str:
    """Return the Markdown of one gallery page."""
    return (
        f"# {example.title}\n\n"
        f"{example.description}\n\n"
        f"{figure(example.name, example.title, example.width, example.height)}\n"
        f"```python\n{example.code}```\n\n"
        f"[:lucide-download: Download the script](../examples/{example.name}.py){{ .md-button }}\n"
    )


def gallery_index(sections: dict[str, list[Example]]) -> str:
    """Return the Markdown of the gallery overview."""
    lines = [
        "---",
        "hide: [navigation, toc]",
        "---",
        "",
        "# Gallery",
        "",
        "Every plot here is rendered by ggrust from the code on its page.",
        "Open one to hover, zoom and pan, and copy the code: it reads small",
        "everyday datasets straight from GitHub, so it runs as is.",
        "",
    ]
    for section, examples in sections.items():
        lines += [f"## {section}", "", '<div class="gallery" markdown>', ""]
        for example in examples:
            lines.append(
                f"[![{example.title}](../images/gallery/{example.name}.png)"
                f"<span>{example.title}</span>]({example.name}.md)"
            )
        lines += ["", "</div>", ""]
    return "\n".join(lines)


def main() -> None:
    """Render every example and write the gallery."""
    import build_reference  # imports this module for the docstring helpers

    warnings.simplefilter("error")  # examples must render without warnings
    for directory in (
        DOCS / "plots" / "reference",
        DOCS / "images" / "gallery",
        EXAMPLES / "figures" / "reference",
        DOCS / "gallery",
    ):
        directory.mkdir(parents=True, exist_ok=True)
    examples = {path.stem: render(path) for path in sorted(EXAMPLES.glob("*.py"))}
    sections = {section: [examples[name] for name in names] for section, names in GALLERY.items()}
    for section_examples in sections.values():
        for example in section_examples:
            (DOCS / "gallery" / f"{example.name}.md").write_text(gallery_page(example))
    (DOCS / "gallery" / "index.md").write_text(gallery_index(sections))
    for _, paths in build_reference.SECTIONS.values():
        for path in paths:
            name = path.rpartition(".")[2]
            if name not in build_reference.ALIASES:  # alias pages link to the target
                render_reference(name, build_reference.resolve(path))


if __name__ == "__main__":
    main()
