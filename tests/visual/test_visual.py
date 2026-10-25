"""Golden-image tests: every case in ``cases.py`` must render as its golden PNG.

On failure the actual image and a difference image are written to
``tests/visual/output/``. Bless new goldens on purpose (and say so in the commit):
``GGRUST_BLESS=1 uv run pytest tests/visual``.
"""

import io
import os
import sys
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

sys.path.insert(0, str(Path(__file__).parent))
import ggrust
from cases import CASES

GOLDEN = Path(__file__).parent / "golden"
OUTPUT = Path(__file__).parent / "output"

# Anti-aliasing can differ in the last bit between platforms; shapes must not.
MAX_MEAN_DIFFERENCE = 0.5
MAX_DIFFERING_FRACTION = 0.001


def render(case) -> np.ndarray:
    buffer = io.BytesIO()
    case(ggrust).to_png(buffer, scale=1)
    return np.asarray(Image.open(io.BytesIO(buffer.getvalue())).convert("RGBA"), dtype=np.int16)


@pytest.mark.parametrize("name", CASES)
def test_matches_golden(name):
    actual = render(CASES[name])
    golden_path = GOLDEN / f"{name}.png"
    if os.environ.get("GGRUST_BLESS"):
        GOLDEN.mkdir(exist_ok=True)
        Image.fromarray(actual.astype(np.uint8)).save(golden_path)
    assert golden_path.exists(), f"no golden for {name}; bless with GGRUST_BLESS=1"
    expected = np.asarray(Image.open(golden_path).convert("RGBA"), dtype=np.int16)
    if actual.shape != expected.shape:
        pytest.fail(f"{name}: size {actual.shape} differs from golden {expected.shape}")
    difference = np.abs(actual - expected).max(axis=2)
    mean = difference.mean()
    differing = (difference > 32).mean()
    if mean > MAX_MEAN_DIFFERENCE or differing > MAX_DIFFERING_FRACTION:
        OUTPUT.mkdir(exist_ok=True)
        Image.fromarray(actual.astype(np.uint8)).save(OUTPUT / f"{name}.actual.png")
        Image.fromarray((np.clip(difference * 4, 0, 255)).astype(np.uint8)).save(
            OUTPUT / f"{name}.diff.png"
        )
        pytest.fail(
            f"{name}: mean difference {mean:.3f}, {differing:.2%} pixels differ; "
            f"see {OUTPUT}/{name}.actual.png and .diff.png"
        )
