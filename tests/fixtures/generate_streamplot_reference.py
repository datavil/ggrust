"""Writes streamplot_matplotlib.json: matplotlib's streamplot lines and arrows.

`case_fields()` is shared with `tests/test_streamplot.py`. Lines are stored as
summaries (vertex count, first and last vertex, coordinate sums) to keep the
fixture small; arrows are stored in full.

Run with any environment that has matplotlib and numpy (not a ggrust dependency):
``python tests/data/generate_streamplot_reference.py``.
"""

import json
from pathlib import Path

import numpy as np


def case_fields() -> list[tuple[str, dict, dict]]:
    """``(name, field, options)`` per case; the field holds x, y, u, v lists."""
    cases = []
    x = np.linspace(-3, 3, 50)
    y = np.linspace(-2, 2, 40)
    grid_x, grid_y = np.meshgrid(x, y)
    rotation = {"x": x, "y": y, "u": -grid_y, "v": grid_x}
    saddle = {"x": x, "y": y, "u": grid_x, "v": -grid_y}
    wavy = {
        "x": x,
        "y": y,
        "u": -1 - grid_x**2 + grid_y,
        "v": 1 + grid_x - grid_y**2,
    }
    holes = dict(wavy)
    holes_u = wavy["u"].copy()
    holes_u[15:22, 20:30] = np.nan
    holes_u[0, 0] = np.inf
    holes["u"] = holes_u
    rng = np.random.default_rng(7)
    coarse = rng.normal(size=(8, 9, 2))
    small = {
        "x": np.linspace(0, 8, 9),
        "y": np.linspace(10, 17, 8),
        "u": coarse[..., 0],
        "v": coarse[..., 1],
    }
    cases.append(("rotation", rotation, {}))
    cases.append(("saddle_density2", saddle, {"density": 2}))
    cases.append(("wavy_density_pair", wavy, {"density": (0.6, 1.7)}))
    cases.append(("wavy_forward", wavy, {"integration_direction": "forward", "maxlength": 2}))
    cases.append(("wavy_backward", wavy, {"integration_direction": "backward"}))
    cases.append(("wavy_unbroken", wavy, {"broken_streamlines": False, "density": 0.5}))
    cases.append(("holes", holes, {"density": 1.3, "minlength": 0.05}))
    cases.append(("random_coarse", small, {"density": 2}))
    cases.append(
        (
            "start_points",
            wavy,
            {"start_points": [[-3, -2], [0, 0], [1.5, -1.2], [3, 2]], "density": 1.2},
        )
    )
    return cases


def main() -> None:
    """Writes the fixture next to this script."""
    import matplotlib
    from matplotlib.figure import Figure

    output = {"matplotlib": matplotlib.__version__, "cases": []}
    for name, field, options in case_fields():
        axes = Figure().add_subplot()
        streams = axes.streamplot(field["x"], field["y"], field["u"], field["v"], **options)
        lines = [path.vertices.tolist() for path in streams.lines.get_paths()]
        arrows = [[*patch._posA_posB[0], *patch._posA_posB[1]] for patch in axes.patches]
        output["cases"].append(
            {
                "name": name,
                "lines": [
                    [len(line), *line[0], *line[-1], *np.sum(line, axis=0).tolist()]
                    for line in lines
                ],
                "arrows": arrows,
            }
        )
    path = Path(__file__).with_name("streamplot_matplotlib.json")
    path.write_text(json.dumps(output))


if __name__ == "__main__":
    main()
