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
``build_reference.SECTIONS``). An example that binds ``plot`` is rendered to
``docs/plots/reference/<name>.html`` with its iframe markup in
``docs/examples/figures/reference/<name>.html``, which the reference page includes.

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
from dataclasses import dataclass
from pathlib import Path

DOCS = Path(__file__).parent
EXAMPLES = DOCS / "examples"
DATA_URL = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
START = "# --8<-- [start:code]\n"
END = "# --8<-- [end:code]"

# Gallery sections, in order. Every example listed here gets a gallery page.
GALLERY: dict[str, list[str]] = {
    "Points and lines": ["scatter", "line", "ribbon", "function", "venn"],
    "Time series": ["unemployment", "bitcoin"],
    "Bars and tiles": ["stacked_bars", "dodged_bars", "heatmap", "periodic_table"],
    "Distributions": ["histogram", "boxplot", "violin", "raincloud"],
    "Ridgelines": ["ridges", "netflix_ridges", "unknown_pleasures"],
    "Labels, facets and grids": ["text_repel", "facets", "grid"],
    "Interactivity": ["tooltips", "many_points"],
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
    plot.to_png(DOCS / "images" / "gallery" / f"{path.stem}.png")
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


def docstring_example(item: object) -> str:
    """Return the runnable code of an object's docstring ``Examples`` section.

    Lines marked ``# doctest: +SKIP`` are left out. The result is empty when the
    docstring has no examples.
    """
    parser = doctest.DocTestParser()
    examples = parser.get_examples(inspect.getdoc(item) or "")
    return "".join(example.source for example in examples if not example.options.get(doctest.SKIP))


def binds_plot(code: str) -> bool:
    """Return whether example code assigns the name ``plot`` at top level."""
    tree = ast.parse(code)
    return any(
        isinstance(node, ast.Assign)
        and any(isinstance(target, ast.Name) and target.id == "plot" for target in node.targets)
        for node in tree.body
    )


def render_reference(name: str, item: object) -> None:
    """Run one docstring example and, if it binds ``plot``, write its plot and iframe."""
    code = docstring_example(item)
    namespace: dict[str, object] = {}
    with contextlib.redirect_stdout(io.StringIO()):  # examples may print
        exec(code, namespace)
    if not binds_plot(code):
        return
    plot = namespace["plot"]
    iframe = plot.to_html(iframe=True)
    width, height = (
        int(re.search(rf'{key}="(\d+)"', iframe).group(1)) for key in ("width", "height")
    )
    plot.to_html(DOCS / "plots" / "reference" / f"{name}.html")
    (EXAMPLES / "figures" / "reference" / f"{name}.html").write_text(
        figure(f"reference/{name}", f"{name} example", width, height, prefix="../../")
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
            render_reference(path.rpartition(".")[2], build_reference.resolve(path))


if __name__ == "__main__":
    main()
