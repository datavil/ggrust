"""Docs index check: what it does and does not do.

Fails when a markdown file under `notes/` is not listed in `notes/README.md`, or when a
relative markdown link in the indexed docs points nowhere. It does not check anchors,
external URLs, or the `docs/` site (zensical checks that with `--strict`).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LINK = re.compile(r"\]\(([^)#\s]+\.md)\)")
CHECKED_LINK_FILES = [
    "plan.md",
    "AGENTS.md",
    "benchmarks/README.md",
    "benchmarks/results/README.md",
]


def main() -> int:
    """Print every docs problem found and return the process exit code (1 if any)."""
    errors: list[str] = []
    index = (ROOT / "notes" / "README.md").read_text()
    notes = sorted(
        p
        for p in (ROOT / "notes").rglob("*.md")
        if p.name != "README.md" or p.parent != ROOT / "notes"
    )
    for path in notes:
        relative = path.relative_to(ROOT / "notes").as_posix()
        if relative != "README.md" and relative not in index:
            errors.append(f"not indexed in notes/README.md: {relative}")
    link_files = (
        [p for p in notes] + [ROOT / "notes" / "README.md"] + [ROOT / f for f in CHECKED_LINK_FILES]
    )
    for path in link_files:
        for target in LINK.findall(path.read_text()):
            if target.startswith("http"):
                continue
            if not (path.parent / target).resolve().exists():
                errors.append(f"broken link in {path.relative_to(ROOT)}: {target}")
    for error in errors:
        print(error)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
